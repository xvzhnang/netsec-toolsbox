use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex, OnceLock,
};
use std::thread;
use std::time::{Duration, Instant};

use crate::service::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
use crate::utils::get_app_base_dir;

static UNAVAILABLE_MODELS: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
static UNAVAILABLE_MODELS_LOGGER_STARTED: AtomicBool = AtomicBool::new(false);

fn extract_unavailable_model_id(line: &str) -> Option<String> {
    let prefix = "⚠️ 模型 ";
    let unavailable = " 不可用";
    let start = line.find(prefix)? + prefix.len();
    let rest = line.get(start..)?;
    let end = rest
        .find(" (")
        .or_else(|| rest.find(unavailable))
        .unwrap_or(rest.len());
    let model = rest[..end].trim();
    if model.is_empty() {
        None
    } else {
        Some(model.to_string())
    }
}

/// Gateway Worker 状态（优化后的状态机）
/// 状态转换：INIT -> READY -> (IDLE | BUSY) -> (DEGRADED | UNHEALTHY) -> DEAD
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum WorkerState {
    /// 启动中（进程启动，等待就绪）
    Init = 0,
    /// 就绪（进程已启动，HTTP 服务可用，但尚未确认健康）
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
    FailedPermanent = 9,
    Disabled = 10,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HealthSignal {
    HeartbeatTimeout,
    RequestTimeout,
    ProcessExit,
    PanicDetected,
}

#[derive(Debug, Clone, Copy)]
struct RestartPolicy {
    max_retries: u32,
    cooldown: Duration,
    degrade_threshold: u32,
    restart_threshold: u32,
}

#[derive(Debug, Clone)]
struct RestartBudget {
    window: Duration,
    max_restarts: u32,
    history: Vec<Instant>,
}

impl RestartBudget {
    fn new(window: Duration, max_restarts: u32) -> Self {
        Self {
            window,
            max_restarts,
            history: Vec::new(),
        }
    }

    fn allow_restart(&mut self, now: Instant) -> bool {
        self.history
            .retain(|t| now.duration_since(*t) < self.window);
        if self.history.len() >= self.max_restarts as usize {
            return false;
        }
        self.history.push(now);
        true
    }

    fn restart_count(&self) -> u32 {
        self.history.len() as u32
    }
}

fn find_free_port(start: u16, end: u16) -> Option<u16> {
    if start > end {
        return None;
    }
    (start..=end).find(|p| std::net::TcpListener::bind(("127.0.0.1", *p)).is_ok())
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
    pub consecutive_timeouts: u32,
    pub last_timeout_at: Option<Instant>,
    pub panic_detected: bool,
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
            consecutive_timeouts: 0,
            last_timeout_at: None,
            panic_detected: false,
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
#[allow(dead_code)]
pub type WorkerStatus = WorkerState;

impl From<u8> for WorkerState {
    fn from(value: u8) -> Self {
        match value {
            0 => WorkerState::Init,
            1 => WorkerState::Ready,
            2 => WorkerState::Idle,
            3 => WorkerState::BusyStreaming,
            4 => WorkerState::BusyBlocked,
            5 => WorkerState::Degraded,
            6 => WorkerState::Unhealthy,
            7 => WorkerState::Restarting,
            8 => WorkerState::Dead,
            9 => WorkerState::FailedPermanent,
            10 => WorkerState::Disabled,
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
        matches!(
            self,
            WorkerState::Idle
                | WorkerState::Degraded
                | WorkerState::BusyStreaming
                | WorkerState::BusyBlocked
        )
    }

    /// 检查是否忙碌
    pub fn is_busy(&self) -> bool {
        matches!(self, WorkerState::BusyStreaming | WorkerState::BusyBlocked)
    }

    /// 检查是否已就绪（进程启动且服务可用）
    pub fn is_ready(&self) -> bool {
        matches!(
            self,
            WorkerState::Ready
                | WorkerState::Idle
                | WorkerState::BusyStreaming
                | WorkerState::BusyBlocked
                | WorkerState::Degraded
        )
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
    /// 最近一次启动时间（用于启动宽限/超时判定）
    pub started_at: Option<Instant>,
    /// 总请求数
    pub total_requests: u64,
    /// 总错误数
    pub total_errors: u64,
    pub circuit_breaker: CircuitBreaker,
    /// 半开熔断状态（只接 1 个请求测试）
    pub half_open_testing: bool,
    pub port_bound: Arc<AtomicBool>,
    pub model_ready: Arc<AtomicBool>,
    /// Trace ID（当前请求的追踪 ID）
    #[allow(dead_code)]
    pub current_trace_id: Option<String>,
    restart_budget: RestartBudget,
    next_restart_at: Option<Instant>,
    pending_restart: Option<HealthSignal>,
    restart_policy: RestartPolicy,
    restart_failures: u32,
    last_restart_failure: Option<Instant>,
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
        crate::utils::lock_or_recover(self.metrics.as_ref(), "GatewayWorker.metrics")
            .active_requests
    }

    /// 记录成功请求
    pub fn record_success(&mut self) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "GatewayWorker.metrics");
        metrics.consecutive_failures = 0;
        metrics.consecutive_timeouts = 0;
        metrics.last_timeout_at = None;
        metrics.last_heartbeat = Some(Instant::now());
        self.last_success = Some(Instant::now());
        // 使用默认延迟 0，因为健康检查不需要延迟信息
        metrics.record_request(true, 0);
        self.circuit_breaker.record_success();
    }

    /// 记录失败请求
    pub fn record_failure(&mut self, is_timeout: bool) -> (u32, u32) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "GatewayWorker.metrics");
        metrics.consecutive_failures += 1;
        if is_timeout {
            let now = Instant::now();
            let next = if let Some(last) = metrics.last_timeout_at {
                if now.duration_since(last) <= Duration::from_secs(120) {
                    metrics.consecutive_timeouts.saturating_add(1)
                } else {
                    1
                }
            } else {
                1
            };
            metrics.consecutive_timeouts = next;
            metrics.last_timeout_at = Some(now);
        }
        self.total_errors += 1;
        // 使用默认延迟 0
        metrics.record_request(false, 0);
        self.circuit_breaker.record_failure();
        (metrics.consecutive_failures, metrics.consecutive_timeouts)
    }

    fn mark_for_restart(&mut self, signal: HealthSignal) {
        if matches!(
            self.status(),
            WorkerState::FailedPermanent | WorkerState::Disabled
        ) {
            return;
        }
        self.pending_restart = Some(signal);
    }

    fn record_restart_failure(&mut self, now: Instant) {
        self.restart_failures = self.restart_failures.saturating_add(1);
        self.last_restart_failure = Some(now);
    }

    fn reset_restart_failures(&mut self) {
        self.restart_failures = 0;
        self.last_restart_failure = None;
    }

    fn restart_cooldown_remaining(&self, now: Instant) -> Duration {
        match self.last_restart_failure {
            Some(last) => {
                let elapsed = now.duration_since(last);
                if elapsed >= self.restart_policy.cooldown {
                    Duration::from_secs(0)
                } else {
                    self.restart_policy.cooldown - elapsed
                }
            }
            None => Duration::from_secs(0),
        }
    }

    fn should_mark_fatal_for_restart(&self, now: Instant) -> bool {
        if self.restart_failures < self.restart_policy.max_retries {
            return false;
        }
        match self.last_restart_failure {
            Some(last) => now.duration_since(last) < self.restart_policy.cooldown,
            None => false,
        }
    }
}

/// 实现 Drop trait，确保 Worker 资源被正确清理
impl Drop for GatewayWorker {
    fn drop(&mut self) {
        // 清理进程句柄，避免资源泄漏
        if let Some(mut child) = self.process.take() {
            log::debug!("[Gateway Worker] Worker-{} 正在清理进程资源", self.id);

            // 尝试终止进程
            #[cfg(target_os = "windows")]
            {
                let _ = child.kill();
                // Windows 上使用 taskkill 确保进程终止
                if let Ok(output) = std::process::Command::new("taskkill")
                    .args(&["/F", "/T", "/PID", &child.id().to_string()])
                    .output()
                {
                    if output.status.success() {
                        log::debug!("Worker-{} 进程已通过 taskkill 终止", self.id);
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = child.kill();
            }

            // 等待进程退出（非阻塞）
            let _ = child.try_wait();
        }
    }
}

impl GatewayWorker {
    pub fn new(id: usize, port: u16) -> Self {
        let circuit_breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 1,
            timeout: Duration::from_secs(30),
            time_window: Duration::from_secs(60),
            min_requests: 10,
        });
        Self {
            id,
            port,
            process: None,
            state: Arc::new(AtomicU8::new(WorkerState::Dead as u8)),
            metrics: Arc::new(Mutex::new(WorkerMetrics::default())),
            capability: Arc::new(Mutex::new(WorkerCapability::default())),
            last_health_check: None,
            last_success: None,
            started_at: None,
            total_requests: 0,
            total_errors: 0,
            circuit_breaker,
            half_open_testing: false,
            port_bound: Arc::new(AtomicBool::new(false)),
            model_ready: Arc::new(AtomicBool::new(false)),
            current_trace_id: None,
            restart_budget: RestartBudget::new(Duration::from_secs(300), 2),
            next_restart_at: None,
            pending_restart: None,
            restart_policy: RestartPolicy {
                max_retries: 3,
                cooldown: Duration::from_secs(60),
                degrade_threshold: 5,
                restart_threshold: 3,
            },
            restart_failures: 0,
            last_restart_failure: None,
        }
    }

    /// 检查是否应该尝试恢复（半开熔断）
    pub fn should_attempt_recovery(&self) -> bool {
        if matches!(
            self.status(),
            WorkerState::FailedPermanent | WorkerState::Disabled
        ) {
            return false;
        }
        if self.circuit_breaker.state() != CircuitBreakerState::Open {
            return false;
        }
        let allowed = self.circuit_breaker.can_execute();
        allowed && self.circuit_breaker.state() == CircuitBreakerState::HalfOpen
    }

    /// 检查是否健康（可用于调度）
    pub fn is_healthy(&self) -> bool {
        let state = self.status();
        let metrics = crate::utils::lock_or_recover(self.metrics.as_ref(), "GatewayWorker.metrics");

        state != WorkerState::Dead
            && state != WorkerState::FailedPermanent
            && state != WorkerState::Disabled
            && self.circuit_breaker.can_execute()
            && !metrics.should_skip()
            && (state.is_available() || state.is_busy())
    }

    /// 状态转换：Init -> Idle
    pub fn transition_to_idle(&self) {
        self.set_state(WorkerState::Idle);
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "GatewayWorker.metrics");
        metrics.last_heartbeat = Some(Instant::now());
    }

    /// 状态转换：Idle -> BusyStreaming
    #[allow(dead_code)]
    pub fn transition_to_busy_streaming(&self, _trace_id: String) {
        self.set_state(WorkerState::BusyStreaming);
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "GatewayWorker.metrics");
        metrics.active_requests += 1;
        metrics.last_token_at = Some(Instant::now());
        // 注意：current_trace_id 需要是 Arc<Mutex<Option<String>>> 或使用其他方式
        // 暂时简化，后续优化
    }

    /// 状态转换：BusyStreaming -> Idle（正常完成）
    #[allow(dead_code)]
    pub fn transition_to_idle_from_busy(&mut self, success: bool, latency_ms: u64) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "GatewayWorker.metrics");
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
    #[allow(dead_code)]
    pub fn check_blocked(&self, token_timeout: Duration) -> bool {
        let state = self.status();
        if state == WorkerState::BusyStreaming {
            let metrics =
                crate::utils::lock_or_recover(self.metrics.as_ref(), "GatewayWorker.metrics");
            if metrics.is_blocked(token_timeout) {
                self.set_state(WorkerState::BusyBlocked);
                return true;
            }
        }
        false
    }

    /// 状态转换：进入 Degraded
    #[allow(dead_code)]
    pub fn transition_to_degraded(&self) {
        let state = self.status();
        if state != WorkerState::Dead
            && state != WorkerState::Unhealthy
            && state != WorkerState::FailedPermanent
            && state != WorkerState::Disabled
        {
            self.set_state(WorkerState::Degraded);
        }
    }

    /// 状态转换：进入 Unhealthy
    #[allow(dead_code)]
    pub fn transition_to_unhealthy(&mut self) {
        if matches!(
            self.status(),
            WorkerState::FailedPermanent | WorkerState::Disabled
        ) {
            return;
        }
        self.set_state(WorkerState::Unhealthy);
        self.half_open_testing = false;
        self.circuit_breaker.force_open();
    }

    /// 状态转换：进入 Restarting
    #[allow(dead_code)]
    pub fn transition_to_restarting(&self) {
        self.set_state(WorkerState::Restarting);
    }

    /// 状态转换：进入 Dead
    #[allow(dead_code)]
    pub fn transition_to_dead(&self) {
        self.set_state(WorkerState::Dead);
        // Dead Worker 永不复用，但保留在池中用于监控
    }

    /// 获取健康检查 URL
    #[allow(dead_code)]
    pub fn health_url(&self) -> String {
        format!("http://127.0.0.1:{}/health", self.port)
    }

    /// 获取 API URL
    pub fn api_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

/// 模型列表缓存（限频 + 缓存）
#[derive(Debug, Clone)]
struct ModelListCache {
    /// 缓存的模型列表和缓存时间
    cached: Option<(Vec<String>, Instant)>,
    /// 缓存有效期（秒）
    cache_ttl: Duration,
    /// 最后请求时间（用于限频）
    last_request: Option<Instant>,
    /// 最小请求间隔（秒）
    min_request_interval: Duration,
}

impl ModelListCache {
    fn new() -> Self {
        Self {
            cached: None,
            cache_ttl: Duration::from_secs(300), // 5 分钟缓存
            last_request: None,
            min_request_interval: Duration::from_secs(30), // 30 秒限频（避免频繁请求）
        }
    }

    /// 获取缓存的模型列表（如果有效）
    fn get_cached(&self) -> Option<Vec<String>> {
        if let Some((models, cached_at)) = &self.cached {
            if cached_at.elapsed() < self.cache_ttl {
                return Some(models.clone());
            }
        }
        None
    }

    /// 检查是否可以请求（限频）
    fn can_request(&self) -> bool {
        if let Some(last) = self.last_request {
            last.elapsed() >= self.min_request_interval
        } else {
            true
        }
    }

    /// 更新缓存
    fn update_cache(&mut self, models: Vec<String>) {
        self.cached = Some((models, Instant::now()));
        self.last_request = Some(Instant::now());
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
    /// 模型列表缓存（限频 + 缓存）
    model_cache: Arc<Mutex<ModelListCache>>,
}

fn jitter_duration(max_ms: u64) -> Duration {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let ms = if max_ms == 0 { 0 } else { nanos % (max_ms + 1) };
    Duration::from_millis(ms)
}

fn backoff_with_jitter(base: Duration, max: Duration, attempt: u32, jitter_ms: u64) -> Duration {
    let exp = attempt.saturating_sub(1);
    let multiplier = 1u32.checked_shl(exp).unwrap_or(u32::MAX);
    let backoff = base.saturating_mul(multiplier).min(max);
    backoff.saturating_add(jitter_duration(jitter_ms))
}

fn schedule_restart_for_worker(worker: Arc<Mutex<GatewayWorker>>, signal: HealthSignal) {
    let (delay, worker_id) = {
        let mut wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
        if matches!(
            wg.status(),
            WorkerState::FailedPermanent | WorkerState::Disabled
        ) {
            return;
        }
        if matches!(wg.status(), WorkerState::Restarting) && wg.next_restart_at.is_some() {
            return;
        }
        if matches!(wg.status(), WorkerState::Init | WorkerState::Ready)
            && !matches!(
                signal,
                HealthSignal::ProcessExit | HealthSignal::PanicDetected
            )
        {
            return;
        }
        let now = Instant::now();
        if wg.should_mark_fatal_for_restart(now) {
            wg.circuit_breaker.force_open();
            wg.set_state(WorkerState::Disabled);
            wg.pending_restart = None;
            wg.next_restart_at = None;
            return;
        }
        wg.mark_for_restart(signal);
        if !wg.restart_budget.allow_restart(now) {
            wg.circuit_breaker.force_open();
            wg.set_state(WorkerState::Disabled);
            wg.pending_restart = None;
            wg.next_restart_at = None;
            return;
        }
        let attempt = wg.restart_budget.restart_count();
        let base_delay = if attempt <= 1 {
            Duration::from_secs(10)
        } else if attempt == 2 {
            Duration::from_secs(30)
        } else {
            Duration::from_secs(120)
        };
        let delay = base_delay.saturating_add(jitter_duration(1500));
        let cooldown_remaining = wg.restart_cooldown_remaining(now);
        let delay = if cooldown_remaining > delay {
            cooldown_remaining
        } else {
            delay
        };
        wg.set_state(WorkerState::Restarting);
        wg.next_restart_at = Some(now + delay);
        (delay, wg.id)
    };

    thread::spawn(move || {
        thread::sleep(delay);
        let mut wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
        if wg.status() != WorkerState::Restarting {
            return;
        }
        if let Some(until) = wg.next_restart_at {
            if Instant::now() < until {
                return;
            }
        }
        match GatewayPool::restart_worker_guard(&mut wg, worker_id) {
            Ok(_) => {
                wg.pending_restart = None;
                wg.next_restart_at = None;
                wg.reset_restart_failures();
            }
            Err(e) => {
                let now = Instant::now();
                wg.record_restart_failure(now);
                if wg.should_mark_fatal_for_restart(now) {
                    wg.circuit_breaker.force_open();
                    wg.set_state(WorkerState::Disabled);
                    wg.pending_restart = None;
                    wg.next_restart_at = None;
                    log::error!(
                        "[Gateway Pool] [Worker-{}] DISABLED: 重启失败次数过多，禁用该 Worker: {}",
                        worker_id,
                        e
                    );
                    return;
                }
                if matches!(
                    wg.status(),
                    WorkerState::FailedPermanent | WorkerState::Disabled
                ) {
                    wg.pending_restart = None;
                    wg.next_restart_at = None;
                    return;
                }
                wg.circuit_breaker.force_open();
                wg.set_state(WorkerState::Unhealthy);
                wg.next_restart_at = None;
                if let Some(sig) = wg.pending_restart {
                    let worker_clone = Arc::clone(&worker);
                    drop(wg);
                    schedule_restart_for_worker(worker_clone, sig);
                }
            }
        }
    });
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
            model_cache: Arc::new(Mutex::new(ModelListCache::new())),
        }
    }

    /// 获取所有 Worker
    pub fn get_workers(&self) -> &Vec<Arc<Mutex<GatewayWorker>>> {
        &self.workers
    }

    /// 选择一个可用的 Worker（优化调度：最少活跃请求 + 退化系数）
    /// 特殊处理：跳过有问题的 Worker-0（如果它处于 Unhealthy 状态）
    pub fn select_worker(&mut self, client_id: Option<&str>) -> Option<Arc<Mutex<GatewayWorker>>> {
        // 边界检查，防止数组越界
        if self.pool_size == 0 || self.workers.is_empty() {
            log::warn!("[Gateway Pool] Worker 池为空，无法选择 Worker");
            return None;
        }

        // 粘性会话：如果有 client_id，优先选择同一个 worker
        if let Some(cid) = client_id {
            let hash = cid.len() % self.pool_size;
            // 确保 hash 在有效范围内（虽然取模已经保证，但双重检查更安全）
            if hash >= self.workers.len() {
                log::warn!("[Gateway Pool] Worker 索引 {} 超出范围", hash);
                // 回退到轮询算法
            } else {
                let worker = &self.workers[hash];
                let mut worker_guard =
                    crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
                let state = worker_guard.status();

                if worker_guard.id == 0
                    && matches!(
                        state,
                        WorkerState::Unhealthy
                            | WorkerState::FailedPermanent
                            | WorkerState::Disabled
                    )
                {
                    log::debug!("[Gateway Pool] 跳过 Worker-0（处于 Unhealthy 状态）");
                } else if state.can_accept_request() && worker_guard.is_healthy() {
                    let (recent_fail_rate, active_requests) = {
                        let metrics = crate::utils::lock_or_recover(
                            worker_guard.metrics.as_ref(),
                            "GatewayWorker.metrics",
                        );
                        (metrics.recent_fail_rate, metrics.active_requests)
                    };
                    if recent_fail_rate < 0.3 {
                        if worker_guard.circuit_breaker.state() == CircuitBreakerState::HalfOpen {
                            if !worker_guard.half_open_testing && active_requests == 0 {
                                worker_guard.half_open_testing = true;
                                return Some(Arc::clone(worker));
                            }
                        } else {
                            return Some(Arc::clone(worker));
                        }
                    }
                }
            }
        }

        // 最少活跃请求算法（考虑退化系数）
        let mut best_worker: Option<Arc<Mutex<GatewayWorker>>> = None;
        let mut best_score = f64::MAX;

        for worker in &self.workers {
            let worker_guard = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
            let state = worker_guard.status();

            // 特殊检查：如果 Worker-0 处于 Unhealthy 状态，明确跳过
            if worker_guard.id == 0 && state == WorkerState::Unhealthy {
                log::debug!("[Gateway Pool] 跳过 Worker-0（处于 Unhealthy 状态，等待恢复）");
                continue;
            }

            // 只选择可以接受请求的状态
            if !state.can_accept_request() {
                continue;
            }

            // 跳过不健康的 worker
            if !worker_guard.is_healthy() {
                continue;
            }

            // 跳过最近失败率过高的 worker
            let metrics = crate::utils::lock_or_recover(
                worker_guard.metrics.as_ref(),
                "GatewayWorker.metrics",
            );
            if metrics.recent_fail_rate > 0.3 {
                continue;
            }
            if worker_guard.circuit_breaker.state() == CircuitBreakerState::HalfOpen {
                if worker_guard.half_open_testing || metrics.active_requests > 0 {
                    continue;
                }
            }

            let mut score = 1.0 + metrics.active_requests as f64;
            score *= 1.0 + metrics.degrade_score;
            if state == WorkerState::Degraded {
                score *= 5.0;
            }

            if score < best_score {
                best_score = score;
                best_worker = Some(Arc::clone(worker));
            }
        }

        if let Some(worker) = best_worker.as_ref() {
            let mut worker_guard = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
            if worker_guard.circuit_breaker.state() == CircuitBreakerState::HalfOpen
                && !worker_guard.half_open_testing
            {
                let active_requests = crate::utils::lock_or_recover(
                    worker_guard.metrics.as_ref(),
                    "GatewayWorker.metrics",
                )
                .active_requests;
                if active_requests == 0 {
                    worker_guard.half_open_testing = true;
                }
            }
        }

        // 如果找到合适的 worker，更新轮询索引
        if best_worker.is_some() {
            self.current_index = (self.current_index + 1) % self.pool_size;
        }

        best_worker
    }

    /// 软队列等待（等待可用 worker，最多等待指定时间）
    #[allow(dead_code)]
    pub fn select_worker_with_queue(
        &mut self,
        client_id: Option<&str>,
        max_wait_ms: u64,
    ) -> Option<Arc<Mutex<GatewayWorker>>> {
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
            // 关键修复：安全锁定 Mutex，避免 poisoned 导致 panic
            let mut worker_guard = match worker.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    log::error!("[Gateway Pool] Worker Mutex 被污染，尝试恢复");
                    poisoned.into_inner()
                }
            };
            if matches!(
                worker_guard.status(),
                WorkerState::FailedPermanent | WorkerState::Disabled
            ) {
                results.push(format!(
                    "Worker-{} 已标记为不可自动管理，跳过启动",
                    worker_guard.id
                ));
                continue;
            }
            if worker_guard.process.is_none() {
                match Self::start_worker(&mut worker_guard) {
                    Ok(msg) => {
                        let msg_clone = msg.clone();
                        results.push(msg);
                        log::info!(
                            "[Gateway Pool] Worker-{} 启动成功: {}",
                            worker_guard.id,
                            msg_clone
                        );
                    }
                    Err(e) => {
                        let error_msg = format!("Worker-{} 启动失败: {}", worker_guard.id, e);
                        results.push(error_msg.clone());
                        log::error!("[Gateway Pool] {}", error_msg);
                        if worker_guard.status() != WorkerState::FailedPermanent {
                            worker_guard.set_state(WorkerState::Dead);
                        }
                    }
                }
            }
        }

        if UNAVAILABLE_MODELS_LOGGER_STARTED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(5));
                let set = UNAVAILABLE_MODELS.get_or_init(|| Mutex::new(BTreeSet::new()));
                let guard = crate::utils::lock_or_recover(set, "UNAVAILABLE_MODELS");
                if !guard.is_empty() {
                    let summary = guard.iter().cloned().collect::<Vec<_>>().join(", ");
                    log::warn!("[Gateway Pool] 启动时检测到不可用模型: {}", summary);
                }
            });
        }

        Ok(results)
    }

    /// 停止所有 Worker
    pub fn stop_all(&self) -> Result<Vec<String>, String> {
        let mut results = Vec::new();

        for worker in &self.workers {
            let mut worker_guard = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
            let was_fatal = worker_guard.status() == WorkerState::FailedPermanent;
            let mut stop_ok = true;
            if let Some(mut child) = worker_guard.process.take() {
                #[cfg(target_os = "windows")]
                {
                    if let Err(e) = child.kill() {
                        log::warn!("终止 Worker-{} 失败: {}", worker_guard.id, e);
                    }

                    let taskkill_output = Command::new("taskkill")
                        .args(&["/F", "/T", "/PID", &child.id().to_string()])
                        .output();
                    match taskkill_output {
                        Ok(output) if output.status.success() => {
                            log::info!("Worker-{} 进程已终止", worker_guard.id);
                        }
                        Ok(output) => {
                            log::warn!(
                                "Worker-{} taskkill 失败，状态码: {:?}",
                                worker_guard.id,
                                output.status.code()
                            );
                            stop_ok = false;
                        }
                        Err(e) => {
                            log::warn!("Worker-{} taskkill 执行失败: {}", worker_guard.id, e);
                            stop_ok = false;
                        }
                    };
                }

                #[cfg(not(target_os = "windows"))]
                {
                    if let Err(e) = child.kill() {
                        log::warn!("终止 Worker-{} 失败: {}", worker_guard.id, e);
                    }
                }

                if stop_ok {
                    results.push(format!("Worker-{} 已停止", worker_guard.id));
                } else {
                    worker_guard.process = Some(child);
                    worker_guard.circuit_breaker.force_open();
                    worker_guard.pending_restart = None;
                    worker_guard.next_restart_at = None;
                    worker_guard.set_state(WorkerState::Disabled);
                    results.push(format!("Worker-{} 停止失败，已隔离", worker_guard.id));
                }
            }

            worker_guard.pending_restart = None;
            worker_guard.next_restart_at = None;
            worker_guard.half_open_testing = false;
            if worker_guard.status() == WorkerState::Disabled {
                worker_guard.circuit_breaker.force_open();
            } else {
                worker_guard.circuit_breaker.reset();
            }
            worker_guard.started_at = None;
            worker_guard.last_health_check = None;
            worker_guard.last_success = None;
            worker_guard.reset_restart_failures();
            worker_guard.restart_budget.history.clear();

            {
                let mut metrics = crate::utils::lock_or_recover(
                    worker_guard.metrics.as_ref(),
                    "GatewayWorker.metrics",
                );
                *metrics = WorkerMetrics::default();
            }

            if !was_fatal && stop_ok {
                worker_guard.set_state(WorkerState::Dead);
            }
        }

        Ok(results)
    }

    /// 启动单个 Worker（检查是否已启动，避免重复启动）
    fn start_worker(worker: &mut GatewayWorker) -> Result<String, String> {
        // 检查 Worker 是否已启动
        if let Some(ref mut child) = worker.process {
            // 检查进程是否仍在运行
            match child.try_wait() {
                Ok(Some(_)) => {
                    // 进程已退出，需要重启
                    log::warn!("[Gateway Pool] Worker-{} 进程已退出，需要重启", worker.id);
                }
                Ok(None) => {
                    // 进程仍在运行，跳过启动
                    log::info!("[Gateway Pool] Worker-{} 已在运行，跳过重复启动", worker.id);
                    return Ok(format!("Worker-{} 已在运行", worker.id));
                }
                Err(_) => {
                    // 无法检查状态，尝试重启
                    log::warn!("[Gateway Pool] Worker-{} 状态检查失败，尝试重启", worker.id);
                }
            }
        }

        {
            let bind_result = std::net::TcpListener::bind(("127.0.0.1", worker.port));
            match bind_result {
                Ok(listener) => drop(listener),
                Err(e) => {
                    let start = worker.port.saturating_add(1);
                    let end = worker.port.saturating_add(50);
                    if let Some(new_port) = find_free_port(start, end) {
                        log::warn!(
                            "[Gateway Pool] [Worker-{}] 端口 {} 被占用，切换到 {}: {}",
                            worker.id,
                            worker.port,
                            new_port,
                            e
                        );
                        worker.port = new_port;
                    } else {
                        worker.circuit_breaker.force_open();
                        worker.pending_restart = None;
                        worker.next_restart_at = None;
                        worker.set_state(WorkerState::FailedPermanent);
                        log::error!(
                            "[Gateway Pool] [Worker-{}] FATAL: 端口 {} 已被占用且无可用端口，禁用该 Worker: {}",
                            worker.id,
                            worker.port,
                            e
                        );
                        return Err(format!(
                            "端口 {} 已被占用，拒绝启动 Worker-{}: {}",
                            worker.port, worker.id, e
                        ));
                    }
                }
            }
        }
        let python_path = Self::get_python_path();
        let service_path = Self::get_ai_service_path();
        let base_dir = get_app_base_dir();

        if !python_path.exists() {
            return Err(format!(
                "Python 可执行文件不存在: {}",
                python_path.display()
            ));
        }

        if !service_path.exists() {
            return Err(format!(
                "AI Gateway 服务脚本不存在: {}",
                service_path.display()
            ));
        }

        log::info!(
            "[Gateway Pool] 启动 Worker-{} 在端口 {}",
            worker.id,
            worker.port
        );

        worker.started_at = Some(Instant::now());
        worker.port_bound.store(false, Ordering::Relaxed);
        worker.model_ready.store(false, Ordering::Relaxed);

        let config_path = base_dir
            .join("ai_service")
            .join("config")
            .join("models.json");

        let mut cmd = Command::new(&python_path);
        cmd.arg(&service_path)
            .arg("--port")
            .arg(worker.port.to_string());

        if config_path.exists() {
            cmd.arg("--config").arg(config_path.to_str().unwrap_or(""));
        }

        let mut child = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动 Worker-{} 失败: {}", worker.id, e))?;

        // 启动后台线程读取 stderr（检测 READY 状态，更新心跳，避免 HTTP 健康检查）
        // 关键修复：使用 panic::catch_unwind 捕获所有 panic，避免读取线程崩溃导致主进程退出
        if let Some(stderr) = child.stderr.take() {
            let worker_id = worker.id;
            let worker_port = worker.port;
            let worker_state = Arc::clone(&worker.state);
            let worker_metrics = Arc::clone(&worker.metrics);
            let port_bound = Arc::clone(&worker.port_bound);
            let model_ready = Arc::clone(&worker.model_ready);
            let stderr_reader = BufReader::new(stderr);

            thread::spawn(move || {
                // 关键修复：捕获所有可能的 panic，避免读取线程崩溃导致主进程退出
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    log::info!(
                        "[Gateway Pool] [Worker-{}] 开始读取 stderr 输出...",
                        worker_id
                    );

                    for line in stderr_reader.lines() {
                        // 安全处理每一行，避免单个错误导致整个线程退出
                        match line {
                            Ok(line) => {
                                if line.contains("⚠️ 模型 ") && line.contains("不可用") {
                                    if let Some(model_id) = extract_unavailable_model_id(&line) {
                                        let set = UNAVAILABLE_MODELS
                                            .get_or_init(|| Mutex::new(BTreeSet::new()));
                                        let mut guard = crate::utils::lock_or_recover(
                                            set,
                                            "UNAVAILABLE_MODELS",
                                        );
                                        guard.insert(model_id);
                                        continue;
                                    }
                                }

                                // 关键优化：检测 [READY] 消息，等待端口 bind 成功后再转为 IDLE
                                if line.contains("[READY]") {
                                    model_ready.store(true, Ordering::Relaxed);
                                    let current_state =
                                        WorkerState::from(worker_state.load(Ordering::Relaxed));
                                    if current_state == WorkerState::Ready
                                        || current_state == WorkerState::Init
                                    {
                                        if port_bound.load(Ordering::Relaxed) {
                                            worker_state
                                                .store(WorkerState::Idle as u8, Ordering::Relaxed);
                                            match worker_metrics.lock() {
                                                Ok(mut metrics) => {
                                                    metrics.last_heartbeat = Some(Instant::now());
                                                    log::info!(
                                                        "[Gateway Pool] [Worker-{}] READY + 端口可用，状态转为 IDLE",
                                                        worker_id
                                                    );
                                                }
                                                Err(poisoned) => {
                                                    log::error!(
                                                        "[Gateway Pool] [Worker-{}] metrics Mutex 被污染，尝试恢复",
                                                        worker_id
                                                    );
                                                    let mut metrics = poisoned.into_inner();
                                                    metrics.last_heartbeat = Some(Instant::now());
                                                }
                                            }
                                        } else {
                                            worker_state
                                                .store(WorkerState::Ready as u8, Ordering::Relaxed);
                                            log::info!(
                                                "[Gateway Pool] [Worker-{}] 检测到 READY，等待端口 {} bind 后注册",
                                                worker_id,
                                                worker_port
                                            );
                                        }
                                    }
                                }

                                if line.contains("[UNHANDLED]")
                                    || line.contains("Traceback (most recent call last)")
                                    || line.contains("panic")
                                {
                                    match worker_metrics.lock() {
                                        Ok(mut metrics) => {
                                            metrics.panic_detected = true;
                                        }
                                        Err(poisoned) => {
                                            let mut metrics = poisoned.into_inner();
                                            metrics.panic_detected = true;
                                        }
                                    }
                                }

                                // 安全处理日志输出（避免 panic 传播）
                                if line.contains("[FATAL]")
                                    || line.contains("[EXIT]")
                                    || line.contains("[UNHANDLED]")
                                {
                                    log::error!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                                } else if line.contains("[ERROR]") {
                                    log::error!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                                } else if line.contains("[WARN]") {
                                    log::warn!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                                } else if line.contains("[INIT]")
                                    || line.contains("[READY]")
                                    || line.contains("[SERVER]")
                                    || line.contains("[MAIN]")
                                {
                                    log::info!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                                } else if line.contains("[REQUEST-")
                                    || line.contains("[HANDLER]")
                                    || line.contains("[STEP-")
                                {
                                    // 静默处理详细日志（减少日志输出）
                                } else if !line.trim().is_empty() {
                                    log::info!("[Gateway Pool] [Worker-{}] {}", worker_id, line);
                                }
                            }
                            Err(e) => {
                                // 关键修复：stderr 关闭通常是正常的（进程退出），不是错误
                                // 不要 panic，只是退出循环
                                log::debug!("[Gateway Pool] [Worker-{}] stderr 读取结束: {} (通常是进程退出或管道关闭)", worker_id, e);
                                break;
                            }
                        }
                    }
                    log::debug!(
                        "[Gateway Pool] [Worker-{}] stderr 读取线程正常结束",
                        worker_id
                    );
                }));

                // 关键修复：如果线程 panic，记录但不影响主进程
                if let Err(panic_info) = result {
                    log::error!(
                        "[Gateway Pool] [Worker-{}] stderr 读取线程 panic: {:?}",
                        worker_id,
                        panic_info
                    );
                    // 标记 Worker 为异常状态，但不 panic
                    worker_state.store(WorkerState::Unhealthy as u8, Ordering::Relaxed);
                }
            });
        }

        // 关键修复：先将 child 保存到 worker，避免被 drop 导致进程终止
        // 在 Windows 上，如果 Child 被 drop，子进程会被立即终止
        worker.process = Some(child);

        worker.set_state(WorkerState::Init);
        worker.last_health_check = Some(Instant::now());

        {
            let worker_id = worker.id;
            let worker_port = worker.port;
            let worker_state = Arc::clone(&worker.state);
            let worker_metrics = Arc::clone(&worker.metrics);
            let port_bound = Arc::clone(&worker.port_bound);
            let model_ready = Arc::clone(&worker.model_ready);
            thread::spawn(move || {
                let start = Instant::now();
                let timeout = Duration::from_secs(5);
                let connect_timeout = Duration::from_millis(200);

                let addr = std::net::SocketAddr::from((
                    std::net::Ipv4Addr::new(127, 0, 0, 1),
                    worker_port,
                ));

                while start.elapsed() < timeout {
                    if std::net::TcpStream::connect_timeout(&addr, connect_timeout).is_ok() {
                        port_bound.store(true, Ordering::Relaxed);
                        let current_state = WorkerState::from(worker_state.load(Ordering::Relaxed));
                        if matches!(current_state, WorkerState::Init | WorkerState::Ready)
                            && model_ready.load(Ordering::Relaxed)
                        {
                            worker_state.store(WorkerState::Idle as u8, Ordering::Relaxed);
                            match worker_metrics.lock() {
                                Ok(mut metrics) => {
                                    metrics.last_heartbeat = Some(Instant::now());
                                }
                                Err(poisoned) => {
                                    let mut metrics = poisoned.into_inner();
                                    metrics.last_heartbeat = Some(Instant::now());
                                }
                            }
                            log::info!(
                                "[Gateway Pool] [Worker-{}] 端口 {} bind 成功，注册到 Pool",
                                worker_id,
                                worker_port
                            );
                        }
                        return;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            });
        }

        // 关键修复：安全锁定 Mutex，避免 poisoned 导致 panic
        match worker.metrics.lock() {
            Ok(mut metrics) => {
                metrics.consecutive_failures = 0;
                metrics.consecutive_timeouts = 0;
                metrics.last_timeout_at = None;
                metrics.panic_detected = false;
            }
            Err(poisoned) => {
                log::error!(
                    "[Gateway Pool] Worker-{} metrics Mutex 被污染，尝试恢复",
                    worker.id
                );
                let mut metrics = poisoned.into_inner();
                metrics.consecutive_failures = 0;
                metrics.consecutive_timeouts = 0;
                metrics.last_timeout_at = None;
                metrics.panic_detected = false;
            }
        }

        worker.circuit_breaker.reset();
        worker.half_open_testing = false;

        let start = Instant::now();
        let max_wait = Duration::from_millis(1500);
        let mut saw_ready = false;

        loop {
            if let Some(ref mut child) = worker.process {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        log::warn!(
                            "[Gateway Pool] Worker-{} 进程在启动阶段退出 (状态: {:?})",
                            worker.id,
                            status
                        );
                        worker.set_state(WorkerState::Dead);
                        return Err(format!(
                            "Worker-{} 进程在启动阶段退出，退出状态: {:?}",
                            worker.id, status
                        ));
                    }
                    Ok(None) => {}
                    Err(e) => {
                        log::warn!(
                            "[Gateway Pool] 检查 Worker-{} 状态失败: {}，假设进程还在运行",
                            worker.id,
                            e
                        );
                    }
                }
            } else {
                log::error!("[Gateway Pool] Worker-{} child 进程句柄丢失", worker.id);
                worker.set_state(WorkerState::Dead);
                return Err(format!("Worker-{} child 进程句柄丢失", worker.id));
            }

            let current_state = worker.status();
            if current_state == WorkerState::Idle {
                saw_ready = true;
                break;
            }

            if start.elapsed() >= max_wait {
                break;
            }

            thread::sleep(Duration::from_millis(50));
        }

        if !saw_ready && worker.status() == WorkerState::Init {
            worker.set_state(WorkerState::Ready);
        }

        worker.reset_restart_failures();
        Ok(format!("Worker-{} 已启动在端口 {}", worker.id, worker.port))
    }

    /// 重启单个 Worker
    #[allow(dead_code)]
    pub fn restart_worker(&self, worker_id: usize) -> Result<String, String> {
        if worker_id >= self.workers.len() {
            return Err(format!("Worker ID {} 不存在", worker_id));
        }

        let worker = &self.workers[worker_id];
        let mut worker_guard = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
        if matches!(
            worker_guard.status(),
            WorkerState::FailedPermanent | WorkerState::Disabled
        ) {
            return Err(format!(
                "Worker-{} 已标记为不可自动管理，拒绝重启",
                worker_id
            ));
        }
        Self::restart_worker_guard(&mut worker_guard, worker_id)?;
        Ok(format!("Worker-{} 已重启", worker_id))
    }

    fn restart_worker_guard(worker: &mut GatewayWorker, worker_id: usize) -> Result<(), String> {
        if let Some(mut child) = worker.process.take() {
            #[cfg(target_os = "windows")]
            {
                if let Err(e) = child.kill() {
                    log::warn!("Worker-{} kill() 失败: {}", worker_id, e);
                }
                let taskkill_output = Command::new("taskkill")
                    .args(&["/F", "/T", "/PID", &child.id().to_string()])
                    .output();

                let taskkill_ok = match taskkill_output {
                    Ok(output) if output.status.success() => {
                        log::info!("Worker-{} 进程已通过 taskkill 终止", worker_id);
                        true
                    }
                    Ok(output) => {
                        log::warn!(
                            "Worker-{} taskkill 失败，状态码: {:?}",
                            worker_id,
                            output.status.code()
                        );
                        false
                    }
                    Err(e) => {
                        log::warn!("Worker-{} taskkill 执行失败: {}", worker_id, e);
                        false
                    }
                };

                if !taskkill_ok {
                    worker.process = Some(child);
                    worker.circuit_breaker.force_open();
                    worker.pending_restart = None;
                    worker.next_restart_at = None;
                    worker.set_state(WorkerState::Disabled);
                    return Err(format!(
                        "Worker-{} taskkill 失败，进入 Disabled（隔离）",
                        worker_id
                    ));
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                if let Err(e) = child.kill() {
                    log::warn!("Worker-{} kill() 失败: {}", worker_id, e);
                }
            }

            let start = Instant::now();
            let timeout = Duration::from_secs(3);
            let mut exited = false;
            while start.elapsed() < timeout {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        log::debug!("Worker-{} 进程已退出，状态: {:?}", worker_id, status);
                        exited = true;
                        break;
                    }
                    Ok(None) => {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        log::warn!("Worker-{} 等待进程退出时出错: {}", worker_id, e);
                        break;
                    }
                }
            }

            if !exited {
                worker.process = Some(child);
                worker.circuit_breaker.force_open();
                worker.pending_restart = None;
                worker.next_restart_at = None;
                worker.set_state(WorkerState::Disabled);
                return Err(format!(
                    "Worker-{} 未能确认退出，进入 Disabled（隔离）",
                    worker_id
                ));
            }
        }

        worker.set_state(WorkerState::Dead);
        std::thread::sleep(Duration::from_millis(500));
        Self::start_worker(worker)?;
        Ok(())
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
    /// 诊断 Worker 状态（用于排查问题）
    pub fn diagnose_worker(&self, worker_id: usize) -> String {
        if worker_id >= self.workers.len() {
            return format!("Worker-{} 不存在", worker_id);
        }

        let worker = &self.workers[worker_id];
        let worker_guard = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");

        let mut diagnostics = Vec::new();
        diagnostics.push(format!("Worker-{} 诊断信息:", worker_id));
        diagnostics.push(format!("  状态: {:?}", worker_guard.status()));
        diagnostics.push(format!("  端口: {}", worker_guard.port));
        diagnostics.push(format!("  进程存在: {}", worker_guard.process.is_some()));

        // 注意：try_wait() 需要可变引用，但诊断函数中 worker_guard 是不可变的
        // 所以这里只检查进程是否存在，不检查具体状态
        if worker_guard.process.is_some() {
            diagnostics.push("  进程状态: 存在（具体状态需要可变引用才能检查）".to_string());
        }

        diagnostics.push(format!(
            "  熔断器打开: {}",
            worker_guard.circuit_breaker.state() == CircuitBreakerState::Open
        ));
        if let Some(elapsed) = worker_guard.circuit_breaker.opened_elapsed() {
            diagnostics.push(format!("  熔断器打开时间: {:?} 前", elapsed));
        }

        let metrics =
            crate::utils::lock_or_recover(worker_guard.metrics.as_ref(), "GatewayWorker.metrics");
        diagnostics.push(format!("  连续失败次数: {}", metrics.consecutive_failures));
        diagnostics.push(format!(
            "  最近失败率: {:.2}%",
            metrics.recent_fail_rate * 100.0
        ));
        diagnostics.push(format!("  退化系数: {:.2}", metrics.degrade_score));
        drop(metrics);

        if let Some(last_check) = worker_guard.last_health_check {
            diagnostics.push(format!("  最后健康检查: {:?} 前", last_check.elapsed()));
        }

        diagnostics.join("\n")
    }

    #[allow(dead_code)]
    pub fn health_check_worker(&self, worker_id: usize) -> bool {
        // 边界检查，防止数组越界 panic
        if worker_id >= self.workers.len() {
            log::warn!(
                "[Gateway Pool] Worker ID {} 超出范围 (总共 {} 个 Worker)",
                worker_id,
                self.workers.len()
            );
            return false;
        }

        let worker = &self.workers[worker_id];
        let mut worker_guard = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
        if matches!(
            worker_guard.status(),
            WorkerState::FailedPermanent | WorkerState::Disabled
        ) {
            return false;
        }

        // Worker-0 特殊诊断：如果连续失败，输出详细诊断信息
        if worker_id == 0 {
            let metrics = crate::utils::lock_or_recover(
                worker_guard.metrics.as_ref(),
                "GatewayWorker.metrics",
            );
            let consecutive_failures = metrics.consecutive_failures;
            drop(metrics);

            if consecutive_failures > 0 && consecutive_failures % 3 == 0 {
                log::warn!("[Gateway Pool] Worker-0 诊断:\n{}", self.diagnose_worker(0));
            }
        }

        // 检查进程是否还在运行
        if let Some(ref mut child) = worker_guard.process {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // 进程已退出
                    log::warn!("[Gateway Pool] Worker-{} 进程已退出", worker_id);
                    worker_guard.set_state(WorkerState::Dead);
                    worker_guard.circuit_breaker.force_open();
                    let worker_clone = Arc::clone(worker);
                    drop(worker_guard);
                    schedule_restart_for_worker(worker_clone, HealthSignal::ProcessExit);
                    return false;
                }
                Ok(None) => {
                    // 进程仍在运行，继续检查 HTTP 健康状态
                }
                Err(e) => {
                    log::warn!(
                        "[Gateway Pool] 检查 Worker-{} 进程状态失败: {}",
                        worker_id,
                        e
                    );
                    return false;
                }
            }
        } else {
            // 进程不存在
            worker_guard.set_state(WorkerState::Dead);
            return false;
        }

        // 无 HTTP 健康检查：仅检查进程状态和心跳（非阻塞）
        let is_alive = true; // 已经在上面检查过了
        let heartbeat_ok = {
            let metrics = crate::utils::lock_or_recover(
                worker_guard.metrics.as_ref(),
                "GatewayWorker.metrics",
            );
            if let Some(last_heartbeat) = metrics.last_heartbeat {
                last_heartbeat.elapsed() < Duration::from_secs(60) // 心跳在60秒内有效
            } else {
                // 如果没有心跳记录，但进程运行中，认为正常（可能是刚启动）
                true
            }
        };

        worker_guard.last_health_check = Some(Instant::now());

        if is_alive && heartbeat_ok {
            // 进程运行中且心跳正常，更新状态和指标
            let current_state = worker_guard.status();
            if current_state == WorkerState::Ready {
                worker_guard.set_state(WorkerState::Idle);
            }

            worker_guard.last_success = Some(Instant::now());
            let mut metrics = crate::utils::lock_or_recover(
                worker_guard.metrics.as_ref(),
                "GatewayWorker.metrics",
            );
            metrics.consecutive_failures = 0;
            drop(metrics);
            worker_guard.record_success();
            worker_guard.half_open_testing = false;
            worker_guard.circuit_breaker.reset();

            if worker_id == 0 {
                log::debug!("[Gateway Pool] Worker-0 健康检查通过（进程状态 + 心跳）");
            }
            return true;
        } else if !heartbeat_ok {
            let (failures, _timeouts) = worker_guard.record_failure(false);
            let degrade_at = worker_guard.restart_policy.degrade_threshold;
            let restart_at =
                degrade_at.saturating_add(worker_guard.restart_policy.restart_threshold);
            if failures >= degrade_at {
                let mut metrics = crate::utils::lock_or_recover(
                    worker_guard.metrics.as_ref(),
                    "GatewayWorker.metrics",
                );
                metrics.degrade_score = 0.6;
                drop(metrics);
                worker_guard.set_state(WorkerState::Degraded);
            }
            if failures >= restart_at {
                let worker_clone = Arc::clone(worker);
                drop(worker_guard);
                schedule_restart_for_worker(worker_clone, HealthSignal::HeartbeatTimeout);
            }
            return false;
        }
        false
    }

    /// 健康检查所有 Worker
    #[allow(dead_code)]
    pub fn health_check_all(&self) {
        for (idx, worker) in self.workers.iter().enumerate() {
            let worker_guard = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
            if !matches!(
                worker_guard.status(),
                WorkerState::Dead
                    | WorkerState::FailedPermanent
                    | WorkerState::Disabled
                    | WorkerState::Restarting
            ) {
                drop(worker_guard);
                self.health_check_worker(idx);
            }
        }
    }

    /// 启动后台健康检查线程（使用静态变量确保只启动一次）
    pub fn start_health_check_thread(&self) {
        static HEALTH_CHECK_STARTED: AtomicBool = AtomicBool::new(false);

        // 使用 compare_and_swap 确保只启动一次
        if HEALTH_CHECK_STARTED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            log::info!("[Gateway Pool] 健康检查线程已在运行，跳过重复启动");
            return;
        }

        let workers = self.workers.clone();
        let pool_size = self.pool_size;

        thread::spawn(move || {
            let client = match Client::builder().timeout(Duration::from_secs(5)).build() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("[Gateway Pool] 创建健康检查 HTTP 客户端失败: {}", e);
                    return;
                }
            };

            log::info!("[Gateway Pool] 健康检查线程已启动（HTTP /health + 进程状态 + 心跳）");

            loop {
                thread::sleep(Duration::from_secs(10));

                for idx in 0..pool_size {
                    let worker = &workers[idx];
                    let mut worker_guard =
                        crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");

                    // 跳过 Dead / FailedPermanent / Restarting 状态的 Worker
                    if matches!(
                        worker_guard.status(),
                        WorkerState::Dead
                            | WorkerState::FailedPermanent
                            | WorkerState::Disabled
                            | WorkerState::Restarting
                    ) {
                        continue;
                    }

                    let panic_detected = {
                        let metrics = crate::utils::lock_or_recover(
                            worker_guard.metrics.as_ref(),
                            "GatewayWorker.metrics",
                        );
                        metrics.panic_detected
                    };
                    if panic_detected {
                        let worker_clone = Arc::clone(worker);
                        drop(worker_guard);
                        schedule_restart_for_worker(worker_clone, HealthSignal::PanicDetected);
                        continue;
                    }

                    // 检查进程状态
                    if let Some(ref mut child) = worker_guard.process {
                        match child.try_wait() {
                            Ok(Some(_)) => {
                                log::warn!("[Gateway Pool] Worker-{} 进程已退出", idx);
                                worker_guard.set_state(WorkerState::Dead);
                                worker_guard.circuit_breaker.force_open();
                                let worker_clone = Arc::clone(worker);
                                drop(worker_guard);
                                schedule_restart_for_worker(
                                    worker_clone,
                                    HealthSignal::ProcessExit,
                                );
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
                        worker_guard.half_open_testing = false;
                        let mut metrics = crate::utils::lock_or_recover(
                            worker_guard.metrics.as_ref(),
                            "GatewayWorker.metrics",
                        );
                        metrics.consecutive_failures = 0;
                        drop(metrics);
                    }

                    let is_alive = if let Some(ref mut child) = worker_guard.process {
                        match child.try_wait() {
                            Ok(None) => true,     // 进程运行中
                            Ok(Some(_)) => false, // 进程已退出
                            Err(_) => false,
                        }
                    } else {
                        false
                    };

                    let heartbeat_ok = {
                        let metrics = crate::utils::lock_or_recover(
                            worker_guard.metrics.as_ref(),
                            "GatewayWorker.metrics",
                        );
                        if let Some(last_heartbeat) = metrics.last_heartbeat {
                            last_heartbeat.elapsed() < Duration::from_secs(60) // 心跳在60秒内有效
                        } else {
                            // 如果没有心跳记录，但进程运行中，认为正常（可能是刚启动）
                            is_alive
                        }
                    };

                    worker_guard.last_health_check = Some(Instant::now());

                    if !is_alive {
                        // 进程已退出，标记为 Dead
                        log::warn!("[Gateway Pool] Worker-{} 进程已退出", idx);
                        worker_guard.set_state(WorkerState::Dead);
                        worker_guard.circuit_breaker.force_open();
                        let _ = worker_guard.record_failure(false);
                        let worker_clone = Arc::clone(worker);
                        drop(worker_guard);
                        schedule_restart_for_worker(worker_clone, HealthSignal::ProcessExit);
                        continue;
                    }

                    if !heartbeat_ok {
                        let (failures, _timeouts) = worker_guard.record_failure(false);
                        let degrade_at = worker_guard.restart_policy.degrade_threshold;
                        let restart_at = degrade_at
                            .saturating_add(worker_guard.restart_policy.restart_threshold);
                        if failures >= degrade_at {
                            let mut metrics = crate::utils::lock_or_recover(
                                worker_guard.metrics.as_ref(),
                                "GatewayWorker.metrics",
                            );
                            metrics.degrade_score = 0.6;
                            drop(metrics);
                            worker_guard.set_state(WorkerState::Degraded);
                        }
                        if failures >= restart_at {
                            let worker_clone = Arc::clone(worker);
                            drop(worker_guard);
                            schedule_restart_for_worker(
                                worker_clone,
                                HealthSignal::HeartbeatTimeout,
                            );
                            continue;
                        }
                        continue;
                    }

                    let current_state = worker_guard.status();
                    let health_url = worker_guard.health_url();
                    let started_at = worker_guard.started_at;
                    let last_heartbeat = crate::utils::lock_or_recover(
                        worker_guard.metrics.as_ref(),
                        "GatewayWorker.metrics",
                    )
                    .last_heartbeat;
                    drop(worker_guard);

                    if matches!(current_state, WorkerState::Init | WorkerState::Ready) {
                        if let Some(started_at) = started_at {
                            if started_at.elapsed() < Duration::from_secs(30) {
                                continue;
                            }
                            if started_at.elapsed() > Duration::from_secs(90)
                                && last_heartbeat.is_none()
                            {
                                let worker_guard =
                                    crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
                                if matches!(
                                    worker_guard.status(),
                                    WorkerState::Init | WorkerState::Ready
                                ) {
                                    let mut metrics = crate::utils::lock_or_recover(
                                        worker_guard.metrics.as_ref(),
                                        "GatewayWorker.metrics",
                                    );
                                    metrics.degrade_score = 0.6;
                                    drop(metrics);
                                    worker_guard.set_state(WorkerState::Degraded);
                                }
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }

                    let (health_ok, is_timeout) = match client.get(&health_url).send() {
                        Ok(resp) => (resp.status().is_success(), false),
                        Err(e) => (false, e.is_timeout()),
                    };

                    let mut worker_guard =
                        crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
                    if matches!(
                        worker_guard.status(),
                        WorkerState::Dead
                            | WorkerState::FailedPermanent
                            | WorkerState::Disabled
                            | WorkerState::Restarting
                    ) {
                        continue;
                    }

                    worker_guard.last_health_check = Some(Instant::now());

                    if health_ok {
                        let current_state = worker_guard.status();
                        if matches!(
                            current_state,
                            WorkerState::Ready | WorkerState::Unhealthy | WorkerState::Degraded
                        ) {
                            worker_guard.set_state(WorkerState::Idle);
                        }
                        {
                            let mut metrics = crate::utils::lock_or_recover(
                                worker_guard.metrics.as_ref(),
                                "GatewayWorker.metrics",
                            );
                            metrics.last_heartbeat = Some(Instant::now());
                        }
                        worker_guard.record_success();
                        worker_guard.half_open_testing = false;
                        worker_guard.circuit_breaker.reset();
                    } else {
                        let (failures, _timeouts) = worker_guard.record_failure(is_timeout);
                        let degrade_at = worker_guard.restart_policy.degrade_threshold;
                        let restart_at = degrade_at
                            .saturating_add(worker_guard.restart_policy.restart_threshold);
                        if failures >= degrade_at {
                            let mut metrics = crate::utils::lock_or_recover(
                                worker_guard.metrics.as_ref(),
                                "GatewayWorker.metrics",
                            );
                            metrics.degrade_score = 0.6;
                            drop(metrics);
                            worker_guard.set_state(WorkerState::Degraded);
                        }
                        if failures >= restart_at {
                            let worker_clone = Arc::clone(worker);
                            drop(worker_guard);
                            let signal = if is_timeout {
                                HealthSignal::RequestTimeout
                            } else {
                                HealthSignal::HeartbeatTimeout
                            };
                            schedule_restart_for_worker(worker_clone, signal);
                            continue;
                        }
                    }
                }
            }
        });
    }

    /// 获取模型列表（带缓存和限频，避免频繁请求导致阻塞）
    pub fn get_models_cached(&self) -> Result<Vec<String>, String> {
        // 1. 先检查缓存
        {
            let cache_guard =
                crate::utils::lock_or_recover(self.model_cache.as_ref(), "GatewayPool.model_cache");
            if let Some(models) = cache_guard.get_cached() {
                return Ok(models);
            }

            // 2. 检查限频
            if !cache_guard.can_request() {
                // 如果缓存过期但还在限频期内，返回空列表（前端可以显示"加载中"）
                return Ok(vec![]);
            }
        }

        // 3. 选择一个健康的 Worker（跳过 Worker-0 如果它处于 Unhealthy 状态）
        // 注意：由于 self 是不可变的，我们需要通过其他方式选择 Worker
        // 这里我们遍历所有 Worker，找到第一个健康的
        let worker = self
            .workers
            .iter()
            .find_map(|w| {
                let wg = crate::utils::lock_or_recover(w.as_ref(), "GatewayWorker");
                if wg.is_healthy() && wg.status().can_accept_request() {
                    if wg.id == 0
                        && (wg.status() == WorkerState::Unhealthy
                            || wg.status() == WorkerState::FailedPermanent)
                    {
                        return None;
                    }
                    Some(Arc::clone(w))
                } else {
                    None
                }
            })
            .ok_or("没有可用的 Worker")?;

        let (_worker_id, port) = {
            let wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
            (wg.id, wg.port)
        };

        // 4. 请求模型列表（使用短超时，避免阻塞）
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let url = format!("http://127.0.0.1:{}/v1/models", port);
        let response = client
            .get(&url)
            .send()
            .map_err(|e| format!("请求失败: {}", e))?;

        if response.status() != StatusCode::OK {
            return Err(format!("HTTP 状态码: {}", response.status()));
        }

        let data: serde_json::Value = response
            .json()
            .map_err(|e| format!("解析 JSON 失败: {}", e))?;

        let models: Vec<String> =
            if let Some(data_array) = data.get("data").and_then(|d| d.as_array()) {
                data_array
                    .iter()
                    .filter_map(|item| {
                        item.get("id")
                            .and_then(|id| id.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            } else {
                vec![]
            };

        // 5. 更新缓存
        {
            let mut cache_guard =
                crate::utils::lock_or_recover(self.model_cache.as_ref(), "GatewayPool.model_cache");
            cache_guard.update_cache(models.clone());
        }

        Ok(models)
    }

    /// 转发 HTTP 请求到可用的 Worker（带超时和重试）
    /// 特殊处理：对于 /v1/models 请求，使用缓存和限频，避免阻塞
    pub fn forward_request(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        headers: Option<&[(&str, &str)]>,
    ) -> Result<(StatusCode, Vec<u8>), String> {
        // 特殊处理：/v1/models 请求使用缓存和限频
        if method == "GET" && path == "/v1/models" {
            match self.get_models_cached() {
                Ok(models) => {
                    // 构建 OpenAI 格式的响应
                    let response_data = serde_json::json!({
                        "object": "list",
                        "data": models.iter().map(|id| {
                            serde_json::json!({
                                "id": id,
                                "object": "model",
                                "created": 0,
                                "owned_by": "system"
                            })
                        }).collect::<Vec<_>>()
                    });
                    let body_bytes = serde_json::to_vec(&response_data)
                        .map_err(|e| format!("序列化 JSON 失败: {}", e))?;
                    return Ok((StatusCode::OK, body_bytes));
                }
                Err(e) => {
                    // 如果缓存获取失败，降级到直接转发（但跳过 Worker-0）
                    log::warn!("[Gateway Pool] 获取模型列表缓存失败: {}，降级到直接转发", e);
                }
            }
        }

        let effective_body: Option<Vec<u8>> = match body {
            Some(body_data)
                if method != "GET" && path.starts_with("/v1/") && path != "/v1/models" =>
            {
                let mut replaced = None;
                if let Ok(mut json_value) = serde_json::from_slice::<serde_json::Value>(body_data) {
                    let requested_model = json_value
                        .get("model")
                        .and_then(|m| m.as_str())
                        .map(|s| s.to_string());

                    if let Some(requested_model) = requested_model {
                        if requested_model != "deepseek-chat" {
                            if let Ok(models) = self.get_models_cached() {
                                if !models.is_empty()
                                    && models.iter().any(|m| m == "deepseek-chat")
                                    && !models.iter().any(|m| m == &requested_model)
                                {
                                    if let Some(obj) = json_value.as_object_mut() {
                                        obj.insert(
                                            "model".to_string(),
                                            serde_json::Value::String("deepseek-chat".to_string()),
                                        );
                                        if let Ok(bytes) = serde_json::to_vec(&json_value) {
                                            replaced = Some(bytes);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                replaced.or_else(|| Some(body_data.to_vec()))
            }
            Some(body_data) => Some(body_data.to_vec()),
            None => None,
        };

        let max_retries = 3;
        // 对于 /v1/models 请求，使用更短的超时时间（10秒），避免 Worker-0 阻塞
        let timeout = if path == "/v1/models" {
            Duration::from_secs(10)
        } else {
            Duration::from_secs(60) // 其他请求使用 60 秒超时
        };

        for attempt in 0..max_retries {
            // 选择可用的 Worker（对于 /v1/models 请求，明确跳过 Worker-0 如果它处于 Unhealthy 状态）
            let worker = match self.select_worker(None) {
                Some(w) => {
                    let wg = crate::utils::lock_or_recover(w.as_ref(), "GatewayWorker");
                    if path == "/v1/models"
                        && wg.id == 0
                        && (wg.status() == WorkerState::Unhealthy
                            || wg.status() == WorkerState::FailedPermanent)
                    {
                        log::warn!("[Gateway Pool] /v1/models 请求跳过 Worker-0（Unhealthy 状态），尝试其他 Worker");
                        drop(wg);
                        // 继续循环，尝试选择其他 Worker
                        if attempt < max_retries - 1 {
                            thread::sleep(Duration::from_millis(500));
                            continue;
                        } else {
                            return Err(
                                "Worker-0 处于 Unhealthy 状态，且没有其他可用的 Worker".to_string()
                            );
                        }
                    } else {
                        drop(wg);
                        w
                    }
                }
                None => {
                    if attempt < max_retries - 1 {
                        log::warn!(
                            "[Gateway Pool] 没有可用的 Worker，等待后重试 ({}/{})",
                            attempt + 1,
                            max_retries
                        );
                        thread::sleep(Duration::from_millis(1000));
                        continue;
                    }
                    return Err("没有可用的 Gateway Worker".to_string());
                }
            };

            let (worker_id, api_url) = {
                let wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
                (wg.id, wg.api_url())
            };

            log::debug!(
                "[Gateway Pool] 转发请求到 Worker-{}: {} {}",
                worker_id,
                method,
                path
            );

            // 标记 Worker 为忙碌（使用 BusyStreaming 状态）
            {
                let mut wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
                wg.set_state(WorkerState::BusyStreaming);
                let mut metrics =
                    crate::utils::lock_or_recover(wg.metrics.as_ref(), "GatewayWorker.metrics");
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
            if let Some(body_data) = &effective_body {
                request_builder = request_builder.body(body_data.clone());
            }

            let start_time = Instant::now();
            let result = request_builder.send();

            // 恢复 Worker 状态（优化：减少锁持有时间，避免自锁）
            // 先处理响应，再更新状态（避免长时间持有锁）

            // 先处理响应，再更新状态（避免长时间持有锁导致任务堆积）
            let (response_result, elapsed) = match result {
                Ok(response) => {
                    let status = response.status();
                    let body_bytes = response
                        .bytes()
                        .map_err(|e| format!("读取响应体失败: {}", e))?
                        .to_vec();
                    let elapsed = start_time.elapsed();
                    (Ok((status, body_bytes)), elapsed)
                }
                Err(e) => {
                    let elapsed = start_time.elapsed();
                    (Err(e), elapsed)
                }
            };

            // 恢复 Worker 状态（快速更新，避免阻塞）
            {
                let mut wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
                let mut metrics =
                    crate::utils::lock_or_recover(wg.metrics.as_ref(), "GatewayWorker.metrics");
                metrics.active_requests = metrics.active_requests.saturating_sub(1);

                let _consecutive_failures_before = metrics.consecutive_failures;

                match &response_result {
                    Ok(_) => {
                        // 请求成功
                        metrics.consecutive_failures = 0;
                        metrics.consecutive_timeouts = 0;
                        metrics.last_timeout_at = None;
                        drop(metrics);
                        wg.record_success();
                        wg.set_state(WorkerState::Idle);
                    }
                    Err(_) => {
                        // 请求失败
                        metrics.consecutive_failures += 1;
                        let consecutive_failures = metrics.consecutive_failures;
                        drop(metrics);
                        let (_failures, timeouts) = wg.record_failure(elapsed >= timeout);

                        if consecutive_failures >= 5 {
                            wg.set_state(WorkerState::Degraded);
                        }
                        if timeouts >= 5 {
                            let worker_clone = Arc::clone(&worker);
                            drop(wg);
                            schedule_restart_for_worker(worker_clone, HealthSignal::RequestTimeout);
                        }
                    }
                }
            }

            match response_result {
                Ok((status, body_bytes)) => {
                    log::debug!(
                        "[Gateway Pool] Worker-{} 响应时间: {:?}, 状态码: {}",
                        worker_id,
                        elapsed,
                        status
                    );
                    return Ok((status, body_bytes));
                }
                Err(e) => {
                    log::warn!(
                        "[Gateway Pool] Worker-{} 请求失败 (耗时: {:?}): {}",
                        worker_id,
                        elapsed,
                        e
                    );

                    // 记录失败（已在上面处理，这里只需要更新 total_errors）
                    {
                        let mut wg =
                            crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
                        wg.total_errors += 1;
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
        let mut available_count = 0;

        for worker in &self.workers {
            let wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
            let state = wg.status();

            match state {
                WorkerState::Idle => {
                    idle_count += 1;
                    available_count += 1;
                }
                WorkerState::BusyStreaming | WorkerState::BusyBlocked => {
                    busy_count += 1;
                }
                WorkerState::Degraded
                    if wg.circuit_breaker.state() != CircuitBreakerState::Open =>
                {
                    available_count += 1;
                }
                _ => {}
            }
        }

        // 状态判定逻辑
        if idle_count > 0 {
            GatewayState::Healthy
        } else if busy_count > 0 {
            GatewayState::Busy
        } else if available_count > 0 {
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
        let mut worker_guard = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");

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
            if worker_guard.status().is_busy() {
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
                        let mut metrics = crate::utils::lock_or_recover(
                            worker_guard.metrics.as_ref(),
                            "GatewayWorker.metrics",
                        );
                        metrics.consecutive_failures = 0;
                        metrics.last_heartbeat = Some(Instant::now());
                        drop(metrics);
                        worker_guard.set_state(WorkerState::Idle);
                        worker_guard.record_success();
                        worker_guard.half_open_testing = false;
                        worker_guard.circuit_breaker.reset();
                        return true;
                    }
                }
                Err(_) => {}
            }
        }

        // L3: 模型 warm / latency（未来扩展）
        if level >= 3 {
            // 可以添加模型预热检查
        }

        // 健康检查失败
        worker_guard.last_health_check = Some(Instant::now());
        let (failures, _timeouts) = worker_guard.record_failure(false);
        let degrade_at = worker_guard.restart_policy.degrade_threshold;
        let restart_at = degrade_at.saturating_add(worker_guard.restart_policy.restart_threshold);
        if failures >= degrade_at {
            let mut metrics = crate::utils::lock_or_recover(
                worker_guard.metrics.as_ref(),
                "GatewayWorker.metrics",
            );
            metrics.degrade_score = 0.6;
            drop(metrics);
            worker_guard.set_state(WorkerState::Degraded);
        }
        if failures >= restart_at {
            let worker_clone = Arc::clone(worker);
            drop(worker_guard);
            schedule_restart_for_worker(worker_clone, HealthSignal::HeartbeatTimeout);
        }

        false
    }
}
