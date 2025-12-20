use std::process::{Command, Child, Stdio};
use std::sync::{Arc, Mutex, atomic::{AtomicU8, Ordering}};
use std::time::{Duration, Instant};
use std::io::{BufRead, BufReader};
use std::thread;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Serialize, Deserialize};

use crate::utils::get_app_base_dir;

/// Gateway Worker 状态（生产级状态机）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum WorkerState {
    /// 启动中（进程启动，等待就绪）
    Init = 0,
    /// 空闲，可接单
    Idle = 1,
    /// 正在正常输出 token（SSE streaming）
    BusyStreaming = 2,
    /// 上游卡死 / 下游断流（异常但未死）
    BusyBlocked = 3,
    /// 慢 / 异常（但未死），可降级使用
    Degraded = 4,
    /// 健康检查失败，需要重启
    Unhealthy = 5,
    /// 正在重启
    Restarting = 6,
    /// 确认死亡，永不复用
    Dead = 7,
}

/// Gateway 状态（调度核心）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayState {
    /// 有可用 worker
    Healthy,
    /// worker 都忙，但还能排队
    Busy,
    /// worker 还能用，但整体慢
    Degraded,
    /// 没有可用 worker
    Unavailable,
}

/// Worker 核心指标
#[derive(Debug, Clone)]
pub struct WorkerMetrics {
    /// 活跃请求数
    pub active_requests: u32,
    /// 最后心跳时间
    pub last_heartbeat: Option<Instant>,
    /// 最后 token 时间（用于检测 BusyBlocked）
    pub last_token_at: Option<Instant>,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: u64,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 最近失败率（0.0-1.0）
    pub recent_fail_rate: f64,
    /// 退化系数（0.0-1.0，越高越差）
    pub degrade_score: f64,
    /// 最近请求历史（用于计算失败率，保留最近 10 个）
    recent_requests: Vec<bool>, // true=成功, false=失败
}

impl Default for WorkerMetrics {
    fn default() -> Self {
        Self {
            active_requests: 0,
            last_heartbeat: None,
            last_token_at: None,
            avg_latency_ms: 0,
            consecutive_failures: 0,
            recent_fail_rate: 0.0,
            degrade_score: 0.0,
            recent_requests: Vec::new(),
        }
    }
}

impl WorkerMetrics {
    /// 记录请求结果（用于更新指标）
    pub fn record_request(&mut self, success: bool, latency_ms: u64) {
        // 更新最近请求历史
        self.recent_requests.push(success);
        if self.recent_requests.len() > 10 {
            self.recent_requests.remove(0);
        }
        
        // 计算失败率
        if !self.recent_requests.is_empty() {
            let failures = self.recent_requests.iter().filter(|&&x| !x).count();
            self.recent_fail_rate = failures as f64 / self.recent_requests.len() as f64;
        }
        
        if success {
            // 更新平均延迟（滑动平均）
            self.avg_latency_ms = (self.avg_latency_ms * 9 + latency_ms) / 10;
            // 降低退化系数
            self.degrade_score = (self.degrade_score - 0.1).max(0.0);
            self.consecutive_failures = 0;
        } else {
            // 增加退化系数
            self.degrade_score = (self.degrade_score + 0.2).min(1.0);
            self.consecutive_failures += 1;
        }
    }
    
    /// 检查是否应该跳过（预测失败）
    pub fn should_skip(&self) -> bool {
        self.recent_fail_rate > 0.3 || self.degrade_score > 0.7
    }
    
    /// 检查是否阻塞（用于检测 BusyBlocked）
    pub fn is_blocked(&self, timeout: Duration) -> bool {
        if let Some(last_token) = self.last_token_at {
            last_token.elapsed() > timeout
        } else {
            false
        }
    }
}

/// Worker 能力标识（未来扩展）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCapability {
    /// 支持的模型列表
    pub supported_models: Vec<String>,
    /// 是否支持流式输出
    pub supports_stream: bool,
    /// 是否支持工具调用
    pub supports_tools: bool,
    /// 最大上下文长度
    pub max_context: usize,
    /// 模型版本
    pub model_version: String,
}

impl Default for WorkerCapability {
    fn default() -> Self {
        Self {
            supported_models: vec![],
            supports_stream: true,
            supports_tools: false,
            max_context: 8192,
            model_version: "unknown".to_string(),
        }
    }
}

/// 兼容旧版 WorkerStatus（保持向后兼容）
pub type WorkerStatus = WorkerState;

impl From<u8> for WorkerState {
    fn from(value: u8) -> Self {
        match value {
            0 => WorkerState::Init,
            1 => WorkerState::Idle,
            2 => WorkerState::BusyStreaming,
            3 => WorkerState::BusyBlocked,
            4 => WorkerState::Degraded,
            5 => WorkerState::Unhealthy,
            6 => WorkerState::Restarting,
            7 => WorkerState::Dead,
            _ => WorkerState::Dead,
        }
    }
}

impl WorkerState {
    /// 检查状态是否可以接受请求
    pub fn can_accept_request(&self) -> bool {
        matches!(self, WorkerState::Idle | WorkerState::Degraded)
    }
    
    /// 检查是否可用（可接受请求或正在处理）
    pub fn is_available(&self) -> bool {
        matches!(self, WorkerState::Idle | WorkerState::Degraded | WorkerState::BusyStreaming | WorkerState::BusyBlocked)
    }
    
    /// 检查是否忙碌
    pub fn is_busy(&self) -> bool {
        matches!(self, WorkerState::BusyStreaming | WorkerState::BusyBlocked)
    }
}

/// Gateway Worker 信息（生产级）
#[derive(Debug)]
pub struct GatewayWorker {
    /// Worker ID
    pub id: usize,
    /// 端口号
    pub port: u16,
    /// 进程句柄
    pub process: Option<Child>,
    /// 状态（使用原子类型，轻量级锁）
    pub state: Arc<AtomicU8>,
    /// 核心指标
    pub metrics: Arc<Mutex<WorkerMetrics>>,
    /// Worker 能力标识
    pub capability: Arc<Mutex<WorkerCapability>>,
    /// 最后健康检查时间
    pub last_health_check: Option<Instant>,
    /// 最后成功时间
    pub last_success: Option<Instant>,
    /// 总请求数
    pub total_requests: u64,
    /// 总错误数
    pub total_errors: u64,
    /// 是否熔断（超时或错误过多）
    pub circuit_breaker_open: bool,
    /// 熔断开始时间
    pub circuit_breaker_opened_at: Option<Instant>,
    /// 半开熔断状态（只接 1 个请求测试）
    pub half_open_testing: bool,
    /// Trace ID（当前请求的追踪 ID）
    pub current_trace_id: Option<String>,
}

/// 兼容旧版字段访问
impl GatewayWorker {
    /// 获取状态（兼容旧版）
    pub fn status(&self) -> WorkerState {
        WorkerState::from(self.state.load(Ordering::Relaxed))
    }
    
    /// 设置状态
    pub fn set_state(&self, state: WorkerState) {
        self.state.store(state as u8, Ordering::Relaxed);
    }
    
    /// 获取活跃请求数
    pub fn active_requests(&self) -> u32 {
        self.metrics.lock().unwrap().active_requests
    }
    
    /// 记录成功请求
    pub fn record_success(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.consecutive_failures = 0;
        self.last_success = Some(Instant::now());
        // 使用默认延迟 0，因为健康检查不需要延迟信息
        metrics.record_request(true, 0);
        
        // 如果熔断器打开，尝试关闭
        if self.circuit_breaker_open {
            self.circuit_breaker_open = false;
            self.circuit_breaker_opened_at = None;
        }
    }
    
    /// 记录失败请求
    pub fn record_failure(&self, is_timeout: bool) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.consecutive_failures += 1;
        self.total_errors += 1;
        // 使用默认延迟 0
        metrics.record_request(false, 0);
        
        // 如果连续失败过多，打开熔断器
        if metrics.consecutive_failures >= 5 {
            self.circuit_breaker_open = true;
            self.circuit_breaker_opened_at = Some(Instant::now());
        }
    }
}

impl GatewayWorker {
    pub fn new(id: usize, port: u16) -> Self {
        Self {
            id,
            port,
            process: None,
            state: Arc::new(AtomicU8::new(WorkerState::Dead as u8)),
            metrics: Arc::new(Mutex::new(WorkerMetrics::default())),
            capability: Arc::new(Mutex::new(WorkerCapability::default())),
            last_health_check: None,
            last_success: None,
            total_requests: 0,
            total_errors: 0,
            circuit_breaker_open: false,
            circuit_breaker_opened_at: None,
            half_open_testing: false,
            current_trace_id: None,
        }
    }

    /// 检查是否应该尝试恢复（半开熔断）
    pub fn should_attempt_recovery(&self) -> bool {
        if !self.circuit_breaker_open {
            return false;
        }
        
        // 如果熔断器打开超过 30 秒，进入半开状态
        if let Some(opened_at) = self.circuit_breaker_opened_at {
            opened_at.elapsed() >= Duration::from_secs(30)
        } else {
            false
        }
    }

    /// 检查是否健康（可用于调度）
    pub fn is_healthy(&self) -> bool {
        let state = self.status();
        let metrics = self.metrics.lock().unwrap();
        
        state != WorkerState::Dead 
            && !self.circuit_breaker_open
            && !metrics.should_skip()
            && (state.is_available() || state.is_busy())
    }
    
    /// 状态转换：Init -> Idle
    pub fn transition_to_idle(&self) {
        self.set_state(WorkerState::Idle);
        let mut metrics = self.metrics.lock().unwrap();
        metrics.last_heartbeat = Some(Instant::now());
    }
    
    /// 状态转换：Idle -> BusyStreaming
    pub fn transition_to_busy_streaming(&self, trace_id: String) {
        self.set_state(WorkerState::BusyStreaming);
        let mut metrics = self.metrics.lock().unwrap();
        metrics.active_requests += 1;
        metrics.last_token_at = Some(Instant::now());
        // 注意：current_trace_id 需要是 Arc<Mutex<Option<String>>> 或使用其他方式
        // 暂时简化，后续优化
    }
    
    /// 状态转换：BusyStreaming -> Idle（正常完成）
    pub fn transition_to_idle_from_busy(&self, success: bool, latency_ms: u64) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.active_requests = metrics.active_requests.saturating_sub(1);
        metrics.record_request(success, latency_ms);
        
        if success {
            self.set_state(WorkerState::Idle);
            self.last_success = Some(Instant::now());
        } else {
            // 根据失败情况决定状态
            if metrics.degrade_score > 0.5 {
                self.set_state(WorkerState::Degraded);
            } else {
                self.set_state(WorkerState::Idle);
            }
        }
    }
    
    /// 检查并更新 BusyBlocked 状态
    pub fn check_blocked(&self, token_timeout: Duration) -> bool {
        let state = self.status();
        if state == WorkerState::BusyStreaming {
            let metrics = self.metrics.lock().unwrap();
            if metrics.is_blocked(token_timeout) {
                self.set_state(WorkerState::BusyBlocked);
                return true;
            }
        }
        false
    }
    
    /// 状态转换：进入 Degraded
    pub fn transition_to_degraded(&self) {
        let state = self.status();
        if state != WorkerState::Dead && state != WorkerState::Unhealthy {
            self.set_state(WorkerState::Degraded);
        }
    }
    
    /// 状态转换：进入 Unhealthy
    pub fn transition_to_unhealthy(&self) {
        self.set_state(WorkerState::Unhealthy);
        self.circuit_breaker_open = true;
        self.circuit_breaker_opened_at = Some(Instant::now());
    }
    
    /// 状态转换：进入 Restarting
    pub fn transition_to_restarting(&self) {
        self.set_state(WorkerState::Restarting);
    }
    
    /// 状态转换：进入 Dead
    pub fn transition_to_dead(&self) {
        self.set_state(WorkerState::Dead);
        // Dead Worker 永不复用，但保留在池中用于监控
    }

    /// 获取健康检查 URL
    pub fn health_url(&self) -> String {
        format!("http://127.0.0.1:{}/health", self.port)
    }

    /// 获取 API URL
    pub fn api_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

/// Gateway 连接池
#[derive(Debug, Clone)]
pub struct GatewayPool {
    /// Worker 列表
    workers: Vec<Arc<Mutex<GatewayWorker>>>,
    /// 当前轮询索引
    current_index: usize,
    /// 基础端口
    base_port: u16,
    /// Worker 数量
    pool_size: usize,
}

impl GatewayPool {
    /// 创建新的连接池
    pub fn new(pool_size: usize, base_port: u16) -> Self {
        let mut workers = Vec::new();
        for i in 0..pool_size {
            let port = base_port + i as u16;
            workers.push(Arc::new(Mutex::new(GatewayWorker::new(i, port))));
        }

        Self {
            workers,
            current_index: 0,
            base_port,
            pool_size,
        }
    }

    /// 获取所有 Worker
    pub fn get_workers(&self) -> &Vec<Arc<Mutex<GatewayWorker>>> {
        &self.workers
    }


    /// 选择一个可用的 Worker（优化调度：最少活跃请求 + 退化系数）
    pub fn select_worker(&mut self, client_id: Option<&str>) -> Option<Arc<Mutex<GatewayWorker>>> {
        // 粘性会话：如果有 client_id，优先选择同一个 worker
        if let Some(cid) = client_id {
            let hash = cid.len() % self.pool_size;
            let worker = &self.workers[hash];
            let worker_guard = worker.lock().unwrap();
            let state = worker_guard.status();
            if state.can_accept_request() && worker_guard.is_healthy() {
                let metrics = worker_guard.metrics.lock().unwrap();
                if metrics.recent_fail_rate < 0.3 {
                    return Some(Arc::clone(worker));
                }
            }
        }
        
        // 最少活跃请求算法（考虑退化系数）
        let mut best_worker: Option<Arc<Mutex<GatewayWorker>>> = None;
        let mut best_score = f64::MAX;
        
        for worker in &self.workers {
            let worker_guard = worker.lock().unwrap();
            let state = worker_guard.status();
            
            // 只选择可以接受请求的状态
            if !state.can_accept_request() {
                continue;
            }
            
            // 跳过不健康的 worker
            if !worker_guard.is_healthy() {
                continue;
            }
            
            // 跳过最近失败率过高的 worker
            let metrics = worker_guard.metrics.lock().unwrap();
            if metrics.recent_fail_rate > 0.3 {
                continue;
            }
            
            // 计算有效权重：基础权重 * (1.0 - 退化系数)
            let effective_weight = (1.0 + metrics.active_requests as f64) * (1.0 - metrics.degrade_score);
            
            if effective_weight < best_score {
                best_score = effective_weight;
                best_worker = Some(Arc::clone(worker));
            }
        }
        
        // 如果找到合适的 worker，更新轮询索引
        if best_worker.is_some() {
            self.current_index = (self.current_index + 1) % self.pool_size;
        }
        
        best_worker
    }
    
    /// 软队列等待（等待可用 worker，最多等待指定时间）
    pub fn select_worker_with_queue(&mut self, client_id: Option<&str>, max_wait_ms: u64) -> Option<Arc<Mutex<GatewayWorker>>> {
        let start = Instant::now();
        let max_wait = Duration::from_millis(max_wait_ms);
        
        loop {
            if let Some(worker) = self.select_worker(client_id) {
                return Some(worker);
            }
            
            if start.elapsed() >= max_wait {
                return None; // 超时，返回 429
            }
            
            thread::sleep(Duration::from_millis(50)); // 等待 50ms 后重试
        }
    }

    /// 启动所有 Worker
    pub fn start_all(&self) -> Result<Vec<String>, String> {
        let mut results = Vec::new();
        
        for worker in &self.workers {
            let mut worker_guard = worker.lock().unwrap();
            if worker_guard.process.is_none() {
                match Self::start_worker(&mut worker_guard) {
                    Ok(msg) => {
                        let msg_clone = msg.clone();
                        results.push(msg);
                        log::info!("[Gateway Pool] Worker-{} 启动成功: {}", worker_guard.id, msg_clone);
                    }
                    Err(e) => {
                        let error_msg = format!("Worker-{} 启动失败: {}", worker_guard.id, e);
                        results.push(error_msg.clone());
                        log::error!("[Gateway Pool] {}", error_msg);
                        worker_guard.set_state(WorkerState::Dead);
                    }
                }
            }
        }

        Ok(results)
    }

    /// 停止所有 Worker
    pub fn stop_all(&self) -> Result<Vec<String>, String> {
        let mut results = Vec::new();
        
        for worker in &self.workers {
            let mut worker_guard = worker.lock().unwrap();
            if let Some(mut child) = worker_guard.process.take() {
                #[cfg(target_os = "windows")]
                {
                    if let Err(e) = child.kill() {
                        log::warn!("终止 Worker-{} 失败: {}", worker_guard.id, e);
                    }
                    
                    if let Ok(output) = Command::new("taskkill")
                        .args(&["/F", "/T", "/PID", &child.id().to_string()])
                        .output()
                    {
                        if output.status.success() {
                            log::info!("Worker-{} 进程已终止", worker_guard.id);
                        }
                    }
                }
                
                #[cfg(not(target_os = "windows"))]
                {
                    if let Err(e) = child.kill() {
                        log::warn!("终止 Worker-{} 失败: {}", worker_guard.id, e);
                    }
                }
                
                worker_guard.set_state(WorkerState::Dead);
                results.push(format!("Worker-{} 已停止", worker_guard.id));
            }
        }

        Ok(results)
    }

    /// 启动单个 Worker
    fn start_worker(worker: &mut GatewayWorker) -> Result<String, String> {
        let python_path = Self::get_python_path();
        let service_path = Self::get_ai_service_path();
        let base_dir = get_app_base_dir();
        
        if !python_path.exists() {
            return Err(format!("Python 可执行文件不存在: {}", python_path.display()));
        }
        
        if !service_path.exists() {
            return Err(format!("AI Gateway 服务脚本不存在: {}", service_path.display()));
        }
        
        log::info!("[Gateway Pool] 启动 Worker-{} 在端口 {}", worker.id, worker.port);
        
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
        
        // 启动后台线程读取 stderr
        if let Some(stderr) = child.stderr.take() {
            let worker_id = worker.id;
            let stderr_reader = BufReader::new(stderr);
            thread::spawn(move || {
                log::info!("[Gateway Pool] [Worker-{}] 开始读取 stderr 输出...", worker_id);
                for line in stderr_reader.lines() {
                    match line {
                        Ok(line) => {
                            if line.contains("[FATAL]") || line.contains("[EXIT]") || line.contains("[UNHANDLED]") {
                                log::error!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                            } else if line.contains("[ERROR]") {
                                log::error!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                            } else if line.contains("[WARN]") {
                                log::warn!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                            } else if line.contains("[INIT]") || line.contains("[READY]") || line.contains("[SERVER]") || line.contains("[MAIN]") {
                                log::info!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                            } else if line.contains("[REQUEST-") || line.contains("[HANDLER]") || line.contains("[STEP-") {
                                log::info!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                            } else if !line.trim().is_empty() {
                                log::info!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                            }
                        }
                        Err(e) => {
                            log::warn!("[Gateway Pool] [Worker-{}] 读取 stderr 失败: {}", worker_id, e);
                            break;
                        }
                    }
                }
                log::warn!("[Gateway Pool] [Worker-{}] stderr 读取线程结束", worker_id);
            });
        }
        
        // 等待服务启动
        std::thread::sleep(Duration::from_millis(2000));
        
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!("Worker-{} 进程立即退出，退出状态: {:?}", worker.id, status));
            }
            Ok(None) => {
                log::info!("[Gateway Pool] Worker-{} 进程已启动，等待 HTTP 服务器就绪...", worker.id);
                std::thread::sleep(Duration::from_millis(3000));
                
                match child.try_wait() {
                    Ok(Some(status)) => {
                        return Err(format!("Worker-{} 进程在启动后退出，退出状态: {:?}", worker.id, status));
                    }
                    Ok(None) => {
                        log::info!("[Gateway Pool] Worker-{} 已启动并运行中", worker.id);
                    }
                    Err(e) => {
                        log::warn!("[Gateway Pool] 检查 Worker-{} 状态失败: {}", worker.id, e);
                    }
                }
            }
            Err(e) => {
                log::warn!("[Gateway Pool] 检查 Worker-{} 状态失败: {}", worker.id, e);
            }
        }
        
        worker.process = Some(child);
        worker.set_state(WorkerState::Init);
        worker.last_health_check = Some(Instant::now());
        let mut metrics = worker.metrics.lock().unwrap();
        metrics.consecutive_failures = 0;
        drop(metrics);
        worker.circuit_breaker_open = false;
        worker.circuit_breaker_opened_at = None;
        // 等待健康检查通过后转为 Idle
        std::thread::sleep(Duration::from_millis(2000));
        worker.set_state(WorkerState::Idle);
        
        Ok(format!("Worker-{} 已启动在端口 {}", worker.id, worker.port))
    }

    /// 重启单个 Worker
    pub fn restart_worker(&self, worker_id: usize) -> Result<String, String> {
        if worker_id >= self.workers.len() {
            return Err(format!("Worker ID {} 不存在", worker_id));
        }

        let worker = &self.workers[worker_id];
        let mut worker_guard = worker.lock().unwrap();

        // 先停止
        if let Some(mut child) = worker_guard.process.take() {
            #[cfg(target_os = "windows")]
            {
                let _ = child.kill();
                if let Ok(output) = Command::new("taskkill")
                    .args(&["/F", "/T", "/PID", &child.id().to_string()])
                    .output()
                {
                    if output.status.success() {
                        log::info!("Worker-{} 进程已终止", worker_id);
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = child.kill();
            }
        }

        worker_guard.set_state(WorkerState::Dead);
        std::thread::sleep(Duration::from_millis(500));

        // 再启动
        Self::start_worker(&mut worker_guard)?;
        Ok(format!("Worker-{} 已重启", worker_id))
    }

    /// 获取 Python 路径
    fn get_python_path() -> std::path::PathBuf {
        let base_dir = get_app_base_dir();
        base_dir.join("python313").join("python.exe")
    }

    /// 获取 AI Gateway 服务脚本路径
    fn get_ai_service_path() -> std::path::PathBuf {
        let base_dir = get_app_base_dir();
        base_dir.join("ai_service").join("main_gateway.py")
    }

    /// 健康检查单个 Worker
    pub fn health_check_worker(&self, worker_id: usize) -> bool {
        if worker_id >= self.workers.len() {
            return false;
        }

        let worker = &self.workers[worker_id];
        let mut worker_guard = worker.lock().unwrap();

        // 检查进程是否还在运行
        if let Some(ref mut child) = worker_guard.process {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // 进程已退出
                    log::warn!("[Gateway Pool] Worker-{} 进程已退出", worker_id);
                    worker_guard.set_state(WorkerState::Dead);
                    worker_guard.circuit_breaker_open = true;
                    worker_guard.circuit_breaker_opened_at = Some(Instant::now());
                    return false;
                }
                Ok(None) => {
                    // 进程仍在运行，继续检查 HTTP 健康状态
                }
                Err(e) => {
                    log::warn!("[Gateway Pool] 检查 Worker-{} 进程状态失败: {}", worker_id, e);
                    return false;
                }
            }
        } else {
            // 进程不存在
            worker_guard.set_state(WorkerState::Dead);
            return false;
        }

        // 检查 HTTP 健康状态
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());

        let health_url = worker_guard.health_url();
        match client.get(&health_url).send() {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    // 健康检查成功
                    worker_guard.last_health_check = Some(Instant::now());
                    worker_guard.last_success = Some(Instant::now());
                    let mut metrics = worker_guard.metrics.lock().unwrap();
                    metrics.consecutive_failures = 0;
                    drop(metrics);
                    worker_guard.set_state(WorkerState::Idle);
                    worker_guard.circuit_breaker_open = false;
                    worker_guard.circuit_breaker_opened_at = None;
                    log::debug!("[Gateway Pool] Worker-{} 健康检查通过", worker_id);
                    return true;
                } else {
                    log::warn!("[Gateway Pool] Worker-{} 健康检查失败，状态码: {}", worker_id, response.status());
                }
            }
            Err(e) => {
                log::warn!("[Gateway Pool] Worker-{} 健康检查请求失败: {}", worker_id, e);
            }
        }

        // 健康检查失败
        worker_guard.last_health_check = Some(Instant::now());
        let mut metrics = worker_guard.metrics.lock().unwrap();
        metrics.consecutive_failures += 1;
        let consecutive_failures = metrics.consecutive_failures;
        drop(metrics);
        
        // 如果连续失败 5 次，打开熔断器
        if consecutive_failures >= 5 {
            worker_guard.circuit_breaker_open = true;
            worker_guard.circuit_breaker_opened_at = Some(Instant::now());
            worker_guard.set_state(WorkerState::Unhealthy);
            log::warn!("[Gateway Pool] Worker-{} 连续失败 {} 次，打开熔断器", worker_id, consecutive_failures);
        } else {
                        worker_guard.set_state(WorkerState::Unhealthy);
        }

        false
    }

    /// 健康检查所有 Worker
    pub fn health_check_all(&self) {
        for (idx, worker) in self.workers.iter().enumerate() {
            let worker_guard = worker.lock().unwrap();
            // 只检查非 Dead 状态的 Worker
            if worker_guard.status() != WorkerState::Dead {
                drop(worker_guard);
                self.health_check_worker(idx);
            }
        }
    }

    /// 启动后台健康检查线程
    pub fn start_health_check_thread(&self) {
        let workers = self.workers.clone();
        let pool_size = self.pool_size;
        
        thread::spawn(move || {
            log::info!("[Gateway Pool] 健康检查线程已启动");
            let client = Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_else(|_| Client::new());

            loop {
                thread::sleep(Duration::from_secs(10)); // 每 10 秒检查一次

                for idx in 0..pool_size {
                    let worker = &workers[idx];
                    let mut worker_guard = worker.lock().unwrap();

                    // 跳过 Dead 状态的 Worker
                    if worker_guard.status() == WorkerState::Dead {
                        continue;
                    }

                    // 检查进程状态
                    if let Some(ref mut child) = worker_guard.process {
                        match child.try_wait() {
                            Ok(Some(_)) => {
                                log::warn!("[Gateway Pool] Worker-{} 进程已退出", idx);
                                worker_guard.set_state(WorkerState::Dead);
                                worker_guard.circuit_breaker_open = true;
                                worker_guard.circuit_breaker_opened_at = Some(Instant::now());
                                continue;
                            }
                            Ok(None) => {
                                // 进程仍在运行
                            }
                            Err(_) => {
                                continue;
                            }
                        }
                    } else {
                        worker_guard.set_state(WorkerState::Dead);
                        continue;
                    }

                    // 检查熔断器是否应该尝试恢复（进入半开状态）
                    if worker_guard.should_attempt_recovery() {
                        log::info!("[Gateway Pool] Worker-{} 熔断器半开，尝试恢复", idx);
                        worker_guard.half_open_testing = true;
                        let mut metrics = worker_guard.metrics.lock().unwrap();
                        metrics.consecutive_failures = 0;
                        drop(metrics);
                    }

                    // 如果熔断器打开，跳过健康检查
                    if worker_guard.circuit_breaker_open {
                        continue;
                    }

                    drop(worker_guard);

                    // 执行 HTTP 健康检查
                    let worker = &workers[idx];
                    let health_url = {
                        let wg = worker.lock().unwrap();
                        wg.health_url()
                    };

                    match client.get(&health_url).send() {
                        Ok(response) => {
                            let mut wg = worker.lock().unwrap();
                            if response.status() == StatusCode::OK {
                                wg.last_health_check = Some(Instant::now());
                                wg.record_success();
                            } else {
                                wg.last_health_check = Some(Instant::now());
                                wg.record_failure(false);
                            }
                        }
                        Err(e) => {
                            let mut wg = worker.lock().unwrap();
                            wg.last_health_check = Some(Instant::now());
                            wg.record_failure(false);
                        }
                    }
                }
            }
        });
    }

    /// 转发 HTTP 请求到可用的 Worker（带超时和重试）
    pub fn forward_request(&mut self, method: &str, path: &str, body: Option<&[u8]>, headers: Option<&[(&str, &str)]>) -> Result<(StatusCode, Vec<u8>), String> {
        let max_retries = 3;
        let timeout = Duration::from_secs(60); // 60 秒超时

        for attempt in 0..max_retries {
            // 选择可用的 Worker
            let worker = match self.select_worker() {
                Some(w) => w,
                None => {
                    if attempt < max_retries - 1 {
                        log::warn!("[Gateway Pool] 没有可用的 Worker，等待后重试 ({}/{})", attempt + 1, max_retries);
                        thread::sleep(Duration::from_millis(1000));
                        continue;
                    }
                    return Err("没有可用的 Gateway Worker".to_string());
                }
            };

            let (worker_id, api_url) = {
                let wg = worker.lock().unwrap();
                (wg.id, wg.api_url())
            };

            log::debug!("[Gateway Pool] 转发请求到 Worker-{}: {} {}", worker_id, method, path);

            // 标记 Worker 为忙碌（使用 BusyStreaming 状态）
            {
                let mut wg = worker.lock().unwrap();
                wg.set_state(WorkerState::BusyStreaming);
                let mut metrics = wg.metrics.lock().unwrap();
                metrics.active_requests += 1;
                drop(metrics);
                wg.total_requests += 1;
            }

            // 构建请求
            let client = Client::builder()
                .timeout(timeout)
                .build()
                .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

            let url = format!("{}{}", api_url, path);
            let mut request_builder = match method {
                "GET" => client.get(&url),
                "POST" => client.post(&url),
                "PUT" => client.put(&url),
                "DELETE" => client.delete(&url),
                _ => return Err(format!("不支持的 HTTP 方法: {}", method)),
            };

            // 添加请求头
            if let Some(headers_list) = headers {
                for (key, value) in headers_list {
                    request_builder = request_builder.header(*key, *value);
                }
            }

            // 添加请求体
            if let Some(body_data) = body {
                request_builder = request_builder.body(body_data.to_vec());
            }

            let start_time = Instant::now();
            let result = request_builder.send();

            // 恢复 Worker 状态
            {
                let mut wg = worker.lock().unwrap();
                let mut metrics = wg.metrics.lock().unwrap();
                metrics.active_requests = metrics.active_requests.saturating_sub(1);
                drop(metrics);
                wg.set_state(WorkerState::Idle);
            }

            match result {
                Ok(response) => {
                    let status = response.status();
                    let body_bytes = response.bytes()
                        .map_err(|e| format!("读取响应体失败: {}", e))?
                        .to_vec();

                    let elapsed = start_time.elapsed();
                    log::debug!("[Gateway Pool] Worker-{} 响应时间: {:?}, 状态码: {}", worker_id, elapsed, status);

                    // 记录成功
                    {
                        let mut wg = worker.lock().unwrap();
                        wg.record_success();
                    }

                    return Ok((status, body_bytes));
                }
                Err(e) => {
                    let elapsed = start_time.elapsed();
                    log::warn!("[Gateway Pool] Worker-{} 请求失败 (耗时: {:?}): {}", worker_id, elapsed, e);

                    // 记录失败
                    {
                        let mut wg = worker.lock().unwrap();
                        wg.total_errors += 1;
                        let is_timeout = elapsed >= timeout;
                        wg.record_failure(is_timeout);
                    }

                    // 如果是最后一次尝试，返回错误
                    if attempt >= max_retries - 1 {
                        return Err(format!("请求失败: {}", e));
                    }

                    // 等待后重试
                    thread::sleep(Duration::from_millis(500));
                }
            }
        }

        Err("所有重试都失败了".to_string())
    }

    /// 获取 Gateway 状态（调度核心）
    pub fn get_gateway_state(&self) -> GatewayState {
        let mut idle_count = 0;
        let mut busy_count = 0;
        let mut unhealthy_count = 0;
        let mut available_count = 0;

        for worker in &self.workers {
            let wg = worker.lock().unwrap();
            let state = wg.status();
            
            match state {
                WorkerState::Idle => {
                    idle_count += 1;
                    available_count += 1;
                },
                WorkerState::BusyStreaming | WorkerState::BusyBlocked => {
                    busy_count += 1;
                },
                WorkerState::Degraded if !wg.circuit_breaker_open => {
                    available_count += 1;
                },
                WorkerState::Unhealthy => {
                    unhealthy_count += 1;
                },
                _ => {},
            }
        }

        // 状态判定逻辑
        if idle_count > 0 {
            GatewayState::Healthy
        } else if busy_count > 0 && available_count > 0 {
            GatewayState::Busy
        } else if unhealthy_count > 0 && available_count > 0 {
            GatewayState::Degraded
        } else {
            GatewayState::Unavailable
        }
    }

    /// 分层健康检查（L0-L3）
    pub fn health_check_layered(&self, worker_id: usize, level: u8) -> bool {
        if worker_id >= self.workers.len() {
            return false;
        }

        let worker = &self.workers[worker_id];
        let mut worker_guard = worker.lock().unwrap();

        // L0: 进程是否存在
        if level == 0 {
            if let Some(ref mut child) = worker_guard.process {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        worker_guard.set_state(WorkerState::Dead);
                        return false;
                    }
                    Ok(None) => return true,
                    Err(_) => return false,
                }
            } else {
                worker_guard.set_state(WorkerState::Dead);
                return false;
            }
        }

        // L1: TCP 能否连接（简单端口检查）
        if level == 1 {
            // 这里可以添加 TCP 连接检查
            // 暂时跳过，直接进入 L2
        }

        // L2: /ping 秒回（或 /health）
        if level >= 2 {
            // Busy 时只允许 L0/L1，跳过 L2+
            if worker_guard.is_busy() {
                return true; // Busy 时认为健康，不进行 HTTP 检查
            }

            let client = Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_else(|_| Client::new());

            let health_url = worker_guard.health_url();
            match client.get(&health_url).send() {
                Ok(response) => {
                    if response.status() == StatusCode::OK {
                        worker_guard.last_health_check = Some(Instant::now());
                        worker_guard.last_success = Some(Instant::now());
                        let mut metrics = worker_guard.metrics.lock().unwrap();
                        metrics.consecutive_failures = 0;
                        metrics.last_heartbeat = Some(Instant::now());
                        drop(metrics);
                        worker_guard.set_state(WorkerState::Idle);
                        worker_guard.circuit_breaker_open = false;
                        worker_guard.circuit_breaker_opened_at = None;
                        return true;
                    }
                }
                Err(_) => {},
            }
        }

        // L3: 模型 warm / latency（未来扩展）
        if level >= 3 {
            // 可以添加模型预热检查
        }

        // 健康检查失败
        worker_guard.last_health_check = Some(Instant::now());
        let mut metrics = worker_guard.metrics.lock().unwrap();
        metrics.consecutive_failures += 1;
        drop(metrics);
        worker_guard.record_failure(false);

        false
    }
}

