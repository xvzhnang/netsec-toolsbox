/// 优化版 Gateway Pool - 高并发架构设计
/// 
/// 核心优化：
/// 1. 一次性初始化（OnceLock）
/// 2. 无 HTTP 健康检查（基于进程状态 + 心跳）
/// 3. 模型列表缓存 + 限频
/// 4. 异步任务管理（避免堆积）
/// 5. 内存优化（及时释放资源）

use std::sync::{Arc, Mutex, OnceLock, atomic::{AtomicU8, AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::process::{Command, Child, Stdio};
use std::io::{BufRead, BufReader};
use std::thread;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Serialize, Deserialize};

use crate::utils::get_app_base_dir;

/// 优化后的 Worker 状态机
/// INIT -> READY -> IDLE -> BUSY -> IDLE (正常流程)
/// INIT -> READY -> DEGRADED -> UNHEALTHY -> DEAD (异常流程)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum OptimizedWorkerState {
    /// 启动中（进程启动，等待就绪）
    Init = 0,
    /// 就绪（进程已启动，HTTP 服务器已就绪，但未通过完整健康检查）
    Ready = 1,
    /// 空闲，可接单
    Idle = 2,
    /// 正在正常输出 token（SSE streaming）
    BusyStreaming = 3,
    /// 上游卡死 / 下游断流（异常但未死）
    BusyBlocked = 4,
    /// 慢 / 异常（但未死），可降级使用
    Degraded = 5,
    /// 健康检查失败，需要重启
    Unhealthy = 6,
    /// 正在重启
    Restarting = 7,
    /// 确认死亡，永不复用
    Dead = 8,
}

impl From<u8> for OptimizedWorkerState {
    fn from(value: u8) -> Self {
        match value {
            0 => OptimizedWorkerState::Init,
            1 => OptimizedWorkerState::Ready,
            2 => OptimizedWorkerState::Idle,
            3 => OptimizedWorkerState::BusyStreaming,
            4 => OptimizedWorkerState::BusyBlocked,
            5 => OptimizedWorkerState::Degraded,
            6 => OptimizedWorkerState::Unhealthy,
            7 => OptimizedWorkerState::Restarting,
            8 => OptimizedWorkerState::Dead,
            _ => OptimizedWorkerState::Dead,
        }
    }
}

impl OptimizedWorkerState {
    pub fn can_accept_request(&self) -> bool {
        matches!(self, OptimizedWorkerState::Idle | OptimizedWorkerState::Degraded | OptimizedWorkerState::Ready)
    }
    
    pub fn is_available(&self) -> bool {
        matches!(
            self,
            OptimizedWorkerState::Idle 
            | OptimizedWorkerState::Degraded 
            | OptimizedWorkerState::Ready
            | OptimizedWorkerState::BusyStreaming 
            | OptimizedWorkerState::BusyBlocked
        )
    }
    
    pub fn is_busy(&self) -> bool {
        matches!(self, OptimizedWorkerState::BusyStreaming | OptimizedWorkerState::BusyBlocked)
    }
}

/// 优化后的 Worker 指标（轻量级，避免内存泄漏）
#[derive(Debug, Clone)]
pub struct OptimizedWorkerMetrics {
    /// 活跃请求数（原子操作，无锁）
    pub active_requests: Arc<AtomicU8>,
    /// 最后心跳时间（从 stderr 读取的 READY 消息）
    pub last_heartbeat: Arc<Mutex<Option<Instant>>>,
    /// 连续失败次数
    pub consecutive_failures: Arc<AtomicU8>,
    /// 最近失败率（0.0-1.0）
    pub recent_fail_rate: Arc<Mutex<f64>>,
    /// 最后成功时间
    pub last_success: Arc<Mutex<Option<Instant>>>,
}

impl Default for OptimizedWorkerMetrics {
    fn default() -> Self {
        Self {
            active_requests: Arc::new(AtomicU8::new(0)),
            last_heartbeat: Arc::new(Mutex::new(None)),
            consecutive_failures: Arc::new(AtomicU8::new(0)),
            recent_fail_rate: Arc::new(Mutex::new(0.0)),
            last_success: Arc::new(Mutex::new(None)),
        }
    }
}

impl OptimizedWorkerMetrics {
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Relaxed);
        *crate::utils::lock_or_recover(self.last_success.as_ref(), "OptimizedWorkerMetrics.last_success") =
            Some(Instant::now());
        let mut rate = crate::utils::lock_or_recover(self.recent_fail_rate.as_ref(), "OptimizedWorkerMetrics.recent_fail_rate");
        *rate = (*rate * 0.9).max(0.0);
    }
    
    pub fn record_failure(&self) {
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
        let mut rate = crate::utils::lock_or_recover(self.recent_fail_rate.as_ref(), "OptimizedWorkerMetrics.recent_fail_rate");
        *rate = (*rate * 0.9 + 0.1).min(1.0);
    }
    
    pub fn update_heartbeat(&self) {
        *crate::utils::lock_or_recover(self.last_heartbeat.as_ref(), "OptimizedWorkerMetrics.last_heartbeat") =
            Some(Instant::now());
    }
}

/// 优化后的 Worker
pub struct OptimizedWorker {
    pub id: usize,
    pub port: u16,
    pub process: Arc<Mutex<Option<Child>>>,
    pub state: Arc<AtomicU8>,
    pub metrics: OptimizedWorkerMetrics,
    /// 进程启动时间
    pub started_at: Arc<Mutex<Option<Instant>>>,
    /// 是否已就绪（从 stderr 读取到 [READY] 消息）
    pub is_ready: Arc<AtomicBool>,
    /// 熔断器状态
    pub circuit_breaker_open: Arc<AtomicBool>,
    pub circuit_breaker_opened_at: Arc<Mutex<Option<Instant>>>,
}

impl OptimizedWorker {
    pub fn new(id: usize, port: u16) -> Self {
        Self {
            id,
            port,
            process: Arc::new(Mutex::new(None)),
            state: Arc::new(AtomicU8::new(OptimizedWorkerState::Init as u8)),
            metrics: OptimizedWorkerMetrics::default(),
            started_at: Arc::new(Mutex::new(None)),
            is_ready: Arc::new(AtomicBool::new(false)),
            circuit_breaker_open: Arc::new(AtomicBool::new(false)),
            circuit_breaker_opened_at: Arc::new(Mutex::new(None)),
        }
    }
    
    pub fn status(&self) -> OptimizedWorkerState {
        OptimizedWorkerState::from(self.state.load(Ordering::Relaxed))
    }
    
    pub fn set_state(&self, state: OptimizedWorkerState) {
        self.state.store(state as u8, Ordering::Relaxed);
    }
    
    pub fn is_healthy(&self) -> bool {
        let state = self.status();
        let is_ready = self.is_ready.load(Ordering::Relaxed);
        let circuit_open = self.circuit_breaker_open.load(Ordering::Relaxed);
        let failures = self.metrics.consecutive_failures.load(Ordering::Relaxed);
        
        state != OptimizedWorkerState::Dead
            && !circuit_open
            && is_ready
            && failures < 5
    }
    
    /// 无 HTTP 健康检查：基于进程状态和心跳
    pub fn health_check_no_http(&self) -> bool {
        // 1. 检查进程是否存在且运行
        let process_guard = crate::utils::lock_or_recover(self.process.as_ref(), "OptimizedWorker.process");
        let child = match process_guard.as_ref() {
            Some(child) => child,
            None => return false,
        };
        match child.try_wait() {
            Ok(Some(_)) => {
                // 进程已退出
                return false;
            }
            Ok(None) => {
                // 进程仍在运行
            }
            Err(_) => {
                return false;
            }
        }
        drop(process_guard);
        
        // 2. 检查心跳（从 stderr 读取的 [READY] 消息）
        let heartbeat_guard = crate::utils::lock_or_recover(
            self.metrics.last_heartbeat.as_ref(),
            "OptimizedWorkerMetrics.last_heartbeat",
        );
        if let Some(last_heartbeat) = *heartbeat_guard {
            // 如果超过 30 秒没有心跳，认为不健康
            if last_heartbeat.elapsed() > Duration::from_secs(30) {
                return false;
            }
        } else {
            // 如果从未收到心跳，检查启动时间
            let started_guard = crate::utils::lock_or_recover(self.started_at.as_ref(), "OptimizedWorker.started_at");
            if let Some(started_at) = *started_guard {
                // 如果启动超过 60 秒还没有心跳，认为不健康
                if started_at.elapsed() > Duration::from_secs(60) {
                    return false;
                }
            }
        }
        
        // 3. 检查就绪标志
        if !self.is_ready.load(Ordering::Relaxed) {
            return false;
        }
        
        true
    }
}

/// 模型列表缓存（限频 + 缓存）
pub struct ModelListCache {
    /// 缓存的模型列表
    models: Arc<Mutex<Option<(Vec<String>, Instant)>>>,
    /// 缓存有效期（秒）
    cache_ttl: Duration,
    /// 最后请求时间（用于限频）
    last_request: Arc<Mutex<Option<Instant>>>,
    /// 最小请求间隔（秒）
    min_request_interval: Duration,
}

impl ModelListCache {
    pub fn new() -> Self {
        Self {
            models: Arc::new(Mutex::new(None)),
            cache_ttl: Duration::from_secs(300), // 5 分钟缓存
            last_request: Arc::new(Mutex::new(None)),
            min_request_interval: Duration::from_secs(10), // 10 秒限频
        }
    }
    
    /// 获取缓存的模型列表（如果有效）
    pub fn get_cached(&self) -> Option<Vec<String>> {
        let guard = crate::utils::lock_or_recover(self.models.as_ref(), "ModelListCache.models");
        if let Some((models, cached_at)) = guard.as_ref() {
            if cached_at.elapsed() < self.cache_ttl {
                return Some(models.clone());
            }
        }
        None
    }
    
    /// 检查是否可以请求（限频）
    pub fn can_request(&self) -> bool {
        let guard = crate::utils::lock_or_recover(self.last_request.as_ref(), "ModelListCache.last_request");
        if let Some(last) = *guard {
            last.elapsed() >= self.min_request_interval
        } else {
            true
        }
    }
    
    /// 更新缓存
    pub fn update_cache(&self, models: Vec<String>) {
        *crate::utils::lock_or_recover(self.models.as_ref(), "ModelListCache.models") = Some((models, Instant::now()));
        *crate::utils::lock_or_recover(self.last_request.as_ref(), "ModelListCache.last_request") = Some(Instant::now());
    }
}

/// 优化后的 Gateway Pool
pub struct OptimizedGatewayPool {
    workers: Vec<Arc<OptimizedWorker>>,
    pool_size: usize,
    base_port: u16,
    model_cache: ModelListCache,
    /// 初始化标志（确保只初始化一次）
    initialized: Arc<AtomicBool>,
}

impl OptimizedGatewayPool {
    /// 全局单例（使用 OnceLock）
    pub static INSTANCE: OnceLock<Arc<Mutex<Option<OptimizedGatewayPool>>>> = OnceLock::new();
    
    pub fn get_instance() -> Arc<Mutex<Option<OptimizedGatewayPool>>> {
        Self::INSTANCE.get_or_init(|| {
            Arc::new(Mutex::new(None))
        }).clone()
    }
    
    /// 确保只初始化一次
    pub fn ensure_initialized(pool_size: usize, base_port: u16) -> Result<(), String> {
        let instance = Self::get_instance();
        let mut guard = crate::utils::lock_or_recover(instance.as_ref(), "OptimizedGatewayPool.INSTANCE");
        
        if guard.is_some() {
            return Ok(()); // 已初始化
        }
        
        let mut pool = OptimizedGatewayPool {
            workers: Vec::new(),
            pool_size,
            base_port,
            model_cache: ModelListCache::new(),
            initialized: Arc::new(AtomicBool::new(false)),
        };
        
        // 创建 Workers
        for i in 0..pool_size {
            let port = base_port + i as u16;
            pool.workers.push(Arc::new(OptimizedWorker::new(i, port)));
        }
        
        // 启动所有 Workers
        pool.start_all()?;
        
        // 启动无 HTTP 健康检查线程
        pool.start_health_check_thread_no_http();
        
        *guard = Some(pool);
        Ok(())
    }
    
    /// 启动所有 Workers
    fn start_all(&mut self) -> Result<Vec<String>, String> {
        let mut results = Vec::new();
        
        for worker in &self.workers {
            match Self::start_worker(worker.clone()) {
                Ok(msg) => {
                    results.push(msg);
                }
                Err(e) => {
                    return Err(format!("启动 Worker-{} 失败: {}", worker.id, e));
                }
            }
        }
        
        Ok(results)
    }
    
    /// 启动单个 Worker（优化版：监听 stderr 获取 READY 状态）
    fn start_worker(worker: Arc<OptimizedWorker>) -> Result<String, String> {
        let python_path = Self::get_python_path();
        let service_path = Self::get_ai_service_path();
        let base_dir = get_app_base_dir();
        
        if !python_path.exists() {
            return Err(format!("Python 可执行文件不存在: {}", python_path.display()));
        }
        
        if !service_path.exists() {
            return Err(format!("AI Gateway 服务脚本不存在: {}", service_path.display()));
        }
        
        log::info!("[Optimized Pool] 启动 Worker-{} 在端口 {}", worker.id, worker.port);
        
        let config_path = base_dir.join("ai_service").join("config").join("models.json");
        
        let mut cmd = Command::new(&python_path);
        cmd.arg(&service_path)
            .arg("--port")
            .arg(worker.port.to_string());
        
        if config_path.exists() {
            cmd.arg("--config")
               .arg(config_path.to_str().unwrap_or(""));
        }
        
        let mut child = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动 Worker-{} 失败: {}", worker.id, e))?;
        
        // 记录启动时间
        *crate::utils::lock_or_recover(worker.started_at.as_ref(), "OptimizedWorker.started_at") = Some(Instant::now());
        worker.set_state(OptimizedWorkerState::Init);
        
        // 启动后台线程读取 stderr（检测 READY 状态）
        if let Some(stderr) = child.stderr.take() {
            let worker_clone = worker.clone();
            let stderr_reader = BufReader::new(stderr);
            thread::spawn(move || {
                for line in stderr_reader.lines() {
                    match line {
                        Ok(line) => {
                            // 检测 [READY] 消息
                            if line.contains("[READY]") {
                                worker_clone.is_ready.store(true, Ordering::Relaxed);
                                worker_clone.set_state(OptimizedWorkerState::Ready);
                                worker_clone.metrics.update_heartbeat();
                                log::info!("[Optimized Pool] Worker-{} 已就绪", worker_clone.id);
                                
                                // 等待一小段时间后转为 Idle
                                thread::sleep(Duration::from_millis(500));
                                worker_clone.set_state(OptimizedWorkerState::Idle);
                            }
                            
                            // 检测错误消息
                            if line.contains("[FATAL]") || line.contains("[EXIT]") {
                                log::error!("[Optimized Pool] Worker-{}: {}", worker_clone.id, line);
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }
        
        // 等待进程启动
        thread::sleep(Duration::from_millis(2000));
        
        match child.try_wait() {
            Ok(Some(_)) => {
                return Err(format!("Worker-{} 进程立即退出", worker.id));
            }
            Ok(None) => {
                // 进程仍在运行
            }
            Err(e) => {
                return Err(format!("检查 Worker-{} 状态失败: {}", worker.id, e));
            }
        }
        
        *crate::utils::lock_or_recover(worker.process.as_ref(), "OptimizedWorker.process") = Some(child);
        
        Ok(format!("Worker-{} 已启动在端口 {}", worker.id, worker.port))
    }
    
    /// 无 HTTP 健康检查线程（基于进程状态和心跳）
    fn start_health_check_thread_no_http(&self) {
        let workers = self.workers.clone();
        let pool_size = self.pool_size;
        
        thread::spawn(move || {
            log::info!("[Optimized Pool] 无 HTTP 健康检查线程已启动");
            
            loop {
                thread::sleep(Duration::from_secs(30)); // 每 30 秒检查一次
                
                for idx in 0..pool_size {
                    let worker = &workers[idx];
                    
                    // 无 HTTP 健康检查
                    let is_healthy = worker.health_check_no_http();
                    
                    if is_healthy {
                        worker.metrics.record_success();
                        if worker.status() == OptimizedWorkerState::Unhealthy {
                            worker.set_state(OptimizedWorkerState::Idle);
                            worker.circuit_breaker_open.store(false, Ordering::Relaxed);
                            *crate::utils::lock_or_recover(
                                worker.circuit_breaker_opened_at.as_ref(),
                                "OptimizedWorker.circuit_breaker_opened_at",
                            ) = None;
                        }
                    } else {
                        worker.metrics.record_failure();
                        let failures = worker.metrics.consecutive_failures.load(Ordering::Relaxed);
                        
                        if failures >= 5 {
                            worker.circuit_breaker_open.store(true, Ordering::Relaxed);
                            *crate::utils::lock_or_recover(
                                worker.circuit_breaker_opened_at.as_ref(),
                                "OptimizedWorker.circuit_breaker_opened_at",
                            ) = Some(Instant::now());
                            worker.set_state(OptimizedWorkerState::Unhealthy);
                            log::warn!("[Optimized Pool] Worker-{} 连续失败 {} 次，标记为 Unhealthy", idx, failures);
                        } else if worker.status() != OptimizedWorkerState::Unhealthy {
                            worker.set_state(OptimizedWorkerState::Degraded);
                        }
                    }
                }
            }
        });
    }
    
    /// 获取模型列表（带缓存和限频）
    pub fn get_models_cached(&self) -> Result<Vec<String>, String> {
        // 1. 先检查缓存
        if let Some(models) = self.model_cache.get_cached() {
            return Ok(models);
        }
        
        // 2. 检查限频
        if !self.model_cache.can_request() {
            // 如果缓存过期但还在限频期内，返回空列表（前端可以显示"加载中"）
            return Ok(vec![]);
        }
        
        // 3. 选择一个健康的 Worker
        let worker = self.select_healthy_worker()
            .ok_or("没有可用的 Worker")?;
        
        // 4. 请求模型列表
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
        
        let url = format!("http://127.0.0.1:{}/v1/models", worker.port);
        let response = client.get(&url).send()
            .map_err(|e| format!("请求失败: {}", e))?;
        
        if response.status() != StatusCode::OK {
            return Err(format!("HTTP 状态码: {}", response.status()));
        }
        
        let data: serde_json::Value = response.json()
            .map_err(|e| format!("解析 JSON 失败: {}", e))?;
        
        let models: Vec<String> = if let Some(data_array) = data.get("data").and_then(|d| d.as_array()) {
            data_array.iter()
                .filter_map(|item| item.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
                .collect()
        } else {
            vec![]
        };
        
        // 5. 更新缓存
        self.model_cache.update_cache(models.clone());
        
        Ok(models)
    }
    
    /// 选择健康的 Worker
    fn select_healthy_worker(&self) -> Option<Arc<OptimizedWorker>> {
        for worker in &self.workers {
            if worker.is_healthy() && worker.status().can_accept_request() {
                return Some(worker.clone());
            }
        }
        None
    }
    
    fn get_python_path() -> std::path::PathBuf {
        // 实现获取 Python 路径的逻辑
        // 这里简化处理
        std::path::PathBuf::from("python")
    }
    
    fn get_ai_service_path() -> std::path::PathBuf {
        let base_dir = get_app_base_dir();
        base_dir.join("ai_service").join("main_gateway.py")
    }
}

