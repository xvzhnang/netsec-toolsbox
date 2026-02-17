use crate::service::circuit_breaker::{CircuitBreaker, CircuitBreakerState, RateLimiter};
use crate::utils::get_app_base_dir;
use crate::utils::{
    find_free_port, is_port_free, kill_listening_pid_on_port, wait_port_free,
};
use parking_lot::Mutex;
use reqwest::{blocking::Client, StatusCode};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

static UNAVAILABLE_MODELS: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
static UNAVAILABLE_MODELS_LOGGER_STARTED: AtomicBool = AtomicBool::new(false);

fn extract_unavailable_model_id(log_line: &str) -> Option<String> {
    if let Some(start) = log_line.find("⚠️ 模型 ") {
        if let Some(end) = log_line[start..].find(" 不可用") {
            let model_part = &log_line[start + "⚠️ 模型 ".len()..start + end];
            return Some(model_part.trim().to_string());
        }
    }
    None
}

/// Worker 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WorkerState {
    /// 初始状态（进程启动中）
    Init = 0,
    /// 就绪状态（进程已启动，等待端口 bind）
    Ready = 1,
    /// 空闲（可以接受请求）
    Idle = 2,
    /// 忙碌（正在流式传输）
    BusyStreaming = 3,
    /// 忙碌（被阻塞，如模型加载中）
    BusyBlocked = 4,
    /// 降级（最近错误率高，降低权重）
    Degraded = 5,
    /// 不健康（心跳丢失或连续错误）
    Unhealthy = 6,
    /// 进程已退出
    Dead = 7,
    /// 僵尸进程（端口被占用但无法 kill）
    Zombie = 8,
    /// 永久失败（多次重启失败，不再尝试）
    FailedPermanent = 9,
    /// 已禁用（手动停止或配置禁用）
    Disabled = 10,
    /// 重启中
    Restarting = 11,
}

impl From<u8> for WorkerState {
    fn from(v: u8) -> Self {
        match v {
            0 => WorkerState::Init,
            1 => WorkerState::Ready,
            2 => WorkerState::Idle,
            3 => WorkerState::BusyStreaming,
            4 => WorkerState::BusyBlocked,
            5 => WorkerState::Degraded,
            6 => WorkerState::Unhealthy,
            7 => WorkerState::Dead,
            8 => WorkerState::Zombie,
            9 => WorkerState::FailedPermanent,
            10 => WorkerState::Disabled,
            11 => WorkerState::Restarting,
            _ => WorkerState::Dead,
        }
    }
}

impl WorkerState {
    pub fn can_accept_request(&self) -> bool {
        matches!(self, WorkerState::Idle | WorkerState::Degraded)
    }

    pub fn is_busy(&self) -> bool {
        matches!(self, WorkerState::BusyStreaming | WorkerState::BusyBlocked)
    }
}

/// 重启策略配置
#[derive(Debug, Clone)]
pub struct RestartPolicy {
    /// 重启冷却时间（基础值）
    pub cooldown: Duration,
    /// 最大重启次数（在时间窗口内）
    pub max_restarts: u32,
    /// 时间窗口
    pub window: Duration,
    /// 连续失败多少次后降级
    pub degrade_threshold: u32,
    /// 连续失败多少次后重启
    pub restart_threshold: u32,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            cooldown: Duration::from_secs(5),
            max_restarts: 5,
            window: Duration::from_secs(300),
            degrade_threshold: 3,
            restart_threshold: 5,
        }
    }
}

/// 健康检查信号
#[derive(Debug, Clone, Copy)]
pub enum HealthSignal {
    /// 进程退出
    ProcessExit,
    /// 心跳超时
    HeartbeatTimeout,
    /// 请求超时
    RequestTimeout,
    /// Panic 检测
    PanicDetected,
}

/// 重启预算（Token Bucket 变体）
#[derive(Debug)]
pub struct RestartBudget {
    /// 最近重启时间记录
    history: Vec<Instant>,
}

impl RestartBudget {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    pub fn allow_restart(&mut self, now: Instant) -> bool {
        // 清理过期记录（5分钟窗口）
        let window = Duration::from_secs(300);
        self.history.retain(|t| now.duration_since(*t) < window);

        // 限制：5分钟内最多5次
        self.history.len() < 5
    }

    pub fn record_restart(&mut self, now: Instant) {
        self.history.push(now);
    }

    pub fn restart_count(&self) -> u32 {
        self.history.len() as u32
    }
}

/// Worker 性能指标
#[derive(Debug, Default)]
pub struct WorkerMetrics {
    /// 总请求数
    pub total_requests: u64,
    /// 当前活跃请求数
    pub active_requests: u32,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 连续超时次数
    pub consecutive_timeouts: u32,
    /// 上次超时时间
    pub last_timeout_at: Option<Instant>,
    /// 最近 1 分钟失败率 (0.0 - 1.0)
    pub recent_fail_rate: f64,
    /// 退化分数 (0.0 - 1.0, 越高越差)
    pub degrade_score: f64,
    /// 上次心跳时间
    pub last_heartbeat: Option<Instant>,
    /// 是否检测到 panic
    pub panic_detected: bool,
}

/// 单个 Gateway Worker
#[derive(Debug)]
pub struct GatewayWorker {
    /// Worker ID (0, 1, 2...)
    pub id: usize,
    /// 端口号
    pub port: u16,
    /// 进程句柄
    pub process: Option<Child>,
    /// 状态（原子操作，用于快速读取）
    state: Arc<std::sync::atomic::AtomicU8>,
    /// 指标统计
    pub metrics: Arc<Mutex<WorkerMetrics>>,
    /// 熔断器
    pub circuit_breaker: CircuitBreaker,
    /// 限流器
    pub rate_limiter: RateLimiter,
    /// 启动时间
    pub started_at: Option<Instant>,
    /// 最后一次健康检查时间
    pub last_health_check: Option<Instant>,
    /// 最后一次成功请求时间
    pub last_success: Option<Instant>,
    /// 重启策略
    pub restart_policy: RestartPolicy,
    /// 待处理的重启信号
    pub pending_restart: Option<HealthSignal>,
    /// 下次允许重启的时间
    pub next_restart_at: Option<Instant>,
    /// 端口是否已绑定（通过 TCP 连接检查）
    pub port_bound: Arc<AtomicBool>,
    /// 模型是否已加载（通过 stderr 日志 [READY]）
    pub model_ready: Arc<AtomicBool>,
    /// 重启预算控制
    pub restart_budget: RestartBudget,
    /// 连续重启失败次数
    pub restart_failures: u32,
    /// 上次重启失败时间
    pub last_restart_failure: Option<Instant>,
    /// 半开状态探测中
    pub half_open_testing: bool,
    /// 总请求数（历史累计）
    pub total_requests: u64,
    /// 总错误数（历史累计）
    pub total_errors: u64,
}

impl GatewayWorker {
    pub fn new(id: usize, port: u16) -> Self {
        Self {
            id,
            port,
            process: None,
            state: Arc::new(std::sync::atomic::AtomicU8::new(WorkerState::Init as u8)),
            metrics: Arc::new(Mutex::new(WorkerMetrics::default())),
            circuit_breaker: CircuitBreaker::new(Default::default()),
            rate_limiter: RateLimiter::new(100, 10.0), // 100 capacity, 10 req/s
            started_at: None,
            last_health_check: None,
            last_success: None,
            restart_policy: RestartPolicy::default(),
            pending_restart: None,
            next_restart_at: None,
            port_bound: Arc::new(AtomicBool::new(false)),
            model_ready: Arc::new(AtomicBool::new(false)),
            restart_budget: RestartBudget::new(),
            restart_failures: 0,
            last_restart_failure: None,
            half_open_testing: false,
            total_requests: 0,
            total_errors: 0,
        }
    }

    pub fn active_requests(&self) -> u32 {
        self.metrics.lock().active_requests
    }

    pub fn status(&self) -> WorkerState {
        WorkerState::from(self.state.load(Ordering::Relaxed))
    }

    pub fn set_state(&self, state: WorkerState) {
        self.state.store(state as u8, Ordering::Relaxed);
    }

    pub fn api_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn health_url(&self) -> String {
        format!("http://127.0.0.1:{}/health", self.port)
    }

    pub fn mark_for_restart(&mut self, signal: HealthSignal) {
        if self.pending_restart.is_none() {
            self.pending_restart = Some(signal);
            log::warn!(
                "[Gateway Pool] Worker-{} 标记为待重启 (信号: {:?})",
                self.id,
                signal
            );
        }
    }

    pub fn restart_cooldown_remaining(&self, now: Instant) -> Duration {
        if let Some(last) = self.last_restart_failure {
            let elapsed = now.duration_since(last);
            let base_cooldown = self.restart_policy.cooldown;

            // 指数退避：每次失败冷却时间翻倍，最大 5 分钟
            let cooldown = base_cooldown.saturating_mul(1u32 << self.restart_failures.min(5));

            if elapsed < cooldown {
                return cooldown - elapsed;
            }
        }
        Duration::ZERO
    }

    pub fn should_mark_fatal_for_restart(&self, _now: Instant) -> bool {
        // 如果连续重启失败超过 10 次，标记为 Fatal
        if self.restart_failures >= 10 {
            return true;
        }

        // 如果最近 5 分钟内重启超过 10 次，标记为 Fatal
        if self.restart_budget.restart_count() >= 10 {
            return true;
        }

        false
    }

    pub fn record_restart_failure(&mut self, now: Instant) {
        self.restart_failures += 1;
        self.last_restart_failure = Some(now);
        self.restart_budget.record_restart(now);
    }

    pub fn reset_restart_failures(&mut self) {
        self.restart_failures = 0;
        self.last_restart_failure = None;
    }

    /// 记录失败并返回 (consecutive_failures, consecutive_timeouts)
    pub fn record_failure(&mut self, is_timeout: bool) -> (u32, u32) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "GatewayWorker.metrics");
        metrics.consecutive_failures += 1;
        if is_timeout {
            metrics.consecutive_timeouts += 1;
            metrics.last_timeout_at = Some(Instant::now());
        }

        // 更新最近失败率 (EMA)
        metrics.recent_fail_rate = metrics.recent_fail_rate * 0.8 + 1.0 * 0.2;

        self.circuit_breaker.record_failure();

        (metrics.consecutive_failures, metrics.consecutive_timeouts)
    }

    pub fn record_success(&self) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "GatewayWorker.metrics");
        metrics.consecutive_failures = 0;
        metrics.consecutive_timeouts = 0;
        metrics.recent_fail_rate *= 0.8; // Decay
        self.circuit_breaker.record_success();
    }

    pub fn should_attempt_recovery(&self) -> bool {
        self.circuit_breaker.state() == CircuitBreakerState::Open
            && self.circuit_breaker.can_execute()
    }

    pub fn is_healthy(&self) -> bool {
        !matches!(
            self.status(),
            WorkerState::Unhealthy
                | WorkerState::Dead
                | WorkerState::Zombie
                | WorkerState::FailedPermanent
                | WorkerState::Disabled
        )
    }
}

/// Gateway 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum GatewayState {
    Healthy,
    Degraded,
    Busy,
    Unavailable,
}

/// 模型列表缓存
#[derive(Debug, Clone)]
struct ModelListCache {
    models: Vec<String>,
    last_updated: Instant,
    last_request: Option<Instant>,
    cached: Option<(Vec<String>, Instant)>,
}

impl ModelListCache {
    fn new() -> Self {
        Self {
            models: Vec::new(),
            last_updated: Instant::now() - Duration::from_secs(3600), // 初始过期
            last_request: None,
            cached: None,
        }
    }

    fn is_valid(&self) -> bool {
        self.last_updated.elapsed() < Duration::from_secs(60) // 缓存 60 秒
    }

    fn can_request(&self) -> bool {
        match self.last_request {
            Some(last) => last.elapsed() >= Duration::from_secs(5), // 限频 5 秒
            None => true,
        }
    }

    fn get_cached(&self) -> Option<Vec<String>> {
        if self.is_valid() {
            Some(self.models.clone())
        } else {
            None
        }
    }

    fn update_cache(&mut self, models: Vec<String>) {
        self.models = models.clone();
        self.last_updated = Instant::now();
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
    /// 用户手动停止标志（true 表示用户手动停止，不应进行重试检查）
    user_stopped: Arc<AtomicBool>,
    startup_failed_reason: Arc<Mutex<Option<String>>>,
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
        // 关键修复：如果 Worker 处于 Dead 状态且没有 pending_restart，说明服务被停止，不进行重启
        if matches!(wg.status(), WorkerState::Dead) && wg.pending_restart.is_none() {
            return;
        }
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
            let startup_stuck = wg
                .started_at
                .map(|t| t.elapsed() > Duration::from_secs(30))
                .unwrap_or(false)
                && (!wg.port_bound.load(Ordering::Relaxed)
                    || !wg.model_ready.load(Ordering::Relaxed));

            if !startup_stuck {
                return;
            }
        }
        let now = Instant::now();
        wg.mark_for_restart(signal);
        let cooldown_remaining = wg.restart_cooldown_remaining(now);
        let delay = if wg.should_mark_fatal_for_restart(now) {
            let base = wg.restart_policy.cooldown;
            if cooldown_remaining > base {
                cooldown_remaining.saturating_add(jitter_duration(1500))
            } else {
                base.saturating_add(jitter_duration(1500))
            }
        } else if !wg.restart_budget.allow_restart(now) {
            let base = Duration::from_secs(180);
            if cooldown_remaining > base {
                cooldown_remaining.saturating_add(jitter_duration(1500))
            } else {
                base.saturating_add(jitter_duration(1500))
            }
        } else {
            let attempt = wg.restart_budget.restart_count();
            let base_delay = if attempt <= 1 {
                Duration::from_secs(10)
            } else if attempt == 2 {
                Duration::from_secs(30)
            } else {
                Duration::from_secs(120)
            };
            let delay = base_delay.saturating_add(jitter_duration(1500));
            if cooldown_remaining > delay {
                cooldown_remaining
            } else {
                delay
            }
        };
        wg.set_state(WorkerState::Restarting);
        wg.next_restart_at = Some(now + delay);
        (delay, wg.id)
    };

    thread::spawn(move || {
        // 关键修复：使用 catch_unwind 捕获 panic，避免重启线程崩溃
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
            // 关键修复：检查重启预算，防止无限递归重启
            let restart_count = wg.restart_budget.restart_count();
            const MAX_RESTART_ATTEMPTS: u32 = 10; // 限制最大重启次数，防止无限循环

            if restart_count >= MAX_RESTART_ATTEMPTS {
                log::error!(
                    "[Gateway Pool] Worker-{} 已达到最大重启次数 {}，停止自动重启",
                    worker_id,
                    MAX_RESTART_ATTEMPTS
                );
                wg.set_state(WorkerState::FailedPermanent);
                wg.pending_restart = None;
                wg.next_restart_at = None;
                return;
            }

            match GatewayPool::restart_worker_guard(&mut wg, worker_id) {
                Ok(_) => {
                    wg.pending_restart = None;
                    wg.next_restart_at = None;
                    wg.reset_restart_failures();
                }
                Err(_e) => {
                    let now = Instant::now();
                    wg.record_restart_failure(now);
                    if matches!(
                        wg.status(),
                        WorkerState::FailedPermanent | WorkerState::Disabled
                    ) {
                        wg.pending_restart = None;
                        wg.next_restart_at = None;
                        return;
                    }

                    // 关键修复：检查是否达到最大重启次数，防止无限递归
                    let new_restart_count = wg.restart_budget.restart_count();
                    if new_restart_count >= MAX_RESTART_ATTEMPTS {
                        log::error!(
                            "[Gateway Pool] Worker-{} 已达到最大重启次数 {}，停止自动重启",
                            worker_id,
                            MAX_RESTART_ATTEMPTS
                        );
                        wg.set_state(WorkerState::FailedPermanent);
                        wg.pending_restart = None;
                        wg.next_restart_at = None;
                    } else {
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
            }
        }));

        if let Err(panic_info) = result {
            log::error!(
                "[Gateway Pool] Worker-{} 重启线程 panic: {:?}",
                worker_id,
                panic_info
            );
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
            user_stopped: Arc::new(AtomicBool::new(false)),
            startup_failed_reason: Arc::new(Mutex::new(None)),
        }
    }

    pub fn mark_startup_failed(&self, reason: String) {
        let mut guard = crate::utils::lock_or_recover(
            self.startup_failed_reason.as_ref(),
            "GatewayPool.startup_failed_reason",
        );
        *guard = Some(reason);
    }

    pub fn clear_startup_failed(&self) {
        let mut guard = crate::utils::lock_or_recover(
            self.startup_failed_reason.as_ref(),
            "GatewayPool.startup_failed_reason",
        );
        *guard = None;
    }

    pub fn is_startup_failed(&self) -> bool {
        let guard = crate::utils::lock_or_recover(
            self.startup_failed_reason.as_ref(),
            "GatewayPool.startup_failed_reason",
        );
        guard.is_some()
    }

    pub fn startup_failed_reason(&self) -> Option<String> {
        let guard = crate::utils::lock_or_recover(
            self.startup_failed_reason.as_ref(),
            "GatewayPool.startup_failed_reason",
        );
        guard.clone()
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
            if worker_guard.circuit_breaker.state() == CircuitBreakerState::HalfOpen
                && (worker_guard.half_open_testing || metrics.active_requests > 0)
            {
                continue;
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
        // 关键优化：清除用户手动停止标志，允许正常重试检查
        self.user_stopped.store(false, Ordering::Relaxed);

        // 关键优化：先收集需要启动的 Worker，避免长时间持有锁
        let workers_to_start: Vec<Arc<Mutex<GatewayWorker>>> = {
            let mut workers_to_start = Vec::new();
            for worker in &self.workers {
                let worker_guard = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
                if matches!(
                    worker_guard.status(),
                    WorkerState::FailedPermanent | WorkerState::Disabled
                ) {
                    continue;
                }
                if worker_guard.process.is_none() {
                    workers_to_start.push(Arc::clone(worker));
                }
            }
            workers_to_start
        };

        // 关键优化：并行启动 Worker，大大减少总启动时间
        let results = std::sync::Mutex::new(Vec::new());

        thread::scope(|s| {
            for worker in workers_to_start {
                let results_clone = &results;
                s.spawn(move || {
                    let worker_id = {
                        let wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
                        wg.id
                    };

                    let mut worker_guard =
                        crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");

                    // 再次检查状态
                    if worker_guard.process.is_some() {
                        return;
                    }

                    match Self::start_worker(&mut worker_guard) {
                        Ok(msg) => {
                            if let Ok(mut r) = results_clone.lock() {
                                r.push(msg);
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Worker-{} 启动失败: {}", worker_id, e);
                            if let Ok(mut r) = results_clone.lock() {
                                r.push(error_msg.clone());
                            }
                            log::error!("[Gateway Pool] {}", error_msg);
                            if worker_guard.status() != WorkerState::FailedPermanent {
                                worker_guard.set_state(WorkerState::Dead);
                            }
                        }
                    }
                });
            }
        });

        if UNAVAILABLE_MODELS_LOGGER_STARTED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            thread::spawn(move || {
                // 关键修复：使用 catch_unwind 捕获 panic
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    thread::sleep(Duration::from_secs(5));
                    let set = UNAVAILABLE_MODELS.get_or_init(|| Mutex::new(BTreeSet::new()));
                    let guard = crate::utils::lock_or_recover(set, "UNAVAILABLE_MODELS");
                    if !guard.is_empty() {
                        let summary = guard.iter().cloned().collect::<Vec<_>>().join(", ");
                        log::warn!("[Gateway Pool] 启动时检测到不可用模型: {}", summary);
                    }
                }));
            });
        }

        let final_results = results.into_inner().unwrap_or_default();
        Ok(final_results)
    }

    /// 停止所有 Worker
    pub fn stop_all(&self) -> Result<Vec<String>, String> {
        // 关键优化：设置用户手动停止标志，避免停止后的重试检查
        self.user_stopped.store(true, Ordering::Relaxed);

        let mut results = Vec::new();

        for worker in &self.workers {
            let mut worker_guard = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
            let was_fatal = worker_guard.status() == WorkerState::FailedPermanent;
            let mut stop_ok = true;
            if let Some(mut child) = worker_guard.process.take() {
                let worker_port = worker_guard.port;
                kill_listening_pid_on_port(worker_port);
                let mut exited = false;
                let start = Instant::now();
                while start.elapsed() < Duration::from_millis(1200) {
                    match child.try_wait() {
                        Ok(Some(_)) => {
                            exited = true;
                            break;
                        }
                        Ok(None) => thread::sleep(Duration::from_millis(100)),
                        Err(_) => break,
                    }
                }

                #[cfg(target_os = "windows")]
                {
                    if !exited {
                        if let Err(e) = child.kill() {
                            log::warn!("终止 Worker-{} 失败: {}", worker_guard.id, e);
                        }

                        let mut cmd = Command::new("taskkill");
                        cmd.creation_flags(0x08000000);
                        let taskkill_output = cmd
                            .args(["/F", "/T", "/PID", &child.id().to_string()])
                            .output();
                        let taskkill_ok = match taskkill_output {
                            Ok(output) if output.status.success() => true,
                            Ok(output) => {
                                log::warn!(
                                    "Worker-{} taskkill 失败，状态码: {:?}",
                                    worker_guard.id,
                                    output.status.code()
                                );
                                false
                            }
                            Err(e) => {
                                log::warn!("Worker-{} taskkill 执行失败: {}", worker_guard.id, e);
                                false
                            }
                        };

                        if taskkill_ok {
                            exited = wait_port_free(worker_port, Duration::from_millis(1500));
                        } else if kill_listening_pid_on_port(worker_port) {
                            exited = wait_port_free(worker_port, Duration::from_millis(2000));
                        }
                    } else {
                        exited = wait_port_free(worker_port, Duration::from_millis(1500));
                    }

                    stop_ok = exited;
                }

                #[cfg(not(target_os = "windows"))]
                {
                    if let Err(e) = child.kill() {
                        log::warn!("终止 Worker-{} 失败: {}", worker_guard.id, e);
                    }
                    stop_ok = wait_port_free(worker_port, Duration::from_millis(1500));
                }

                if stop_ok {
                    results.push(format!("Worker-{} 已停止", worker_guard.id));
                } else {
                    worker_guard.circuit_breaker.force_open();
                    worker_guard.pending_restart = None;
                    worker_guard.next_restart_at = None;
                    worker_guard.set_state(WorkerState::Zombie);
                    results.push(format!(
                        "Worker-{} 停止失败，进入 Zombie（端口 {} 可能仍被占用）",
                        worker_guard.id, worker_port
                    ));
                }
            }

            // 关键修复：清除所有待重启任务，防止停止后继续重启
            worker_guard.pending_restart = None;
            worker_guard.next_restart_at = None;
            worker_guard.half_open_testing = false;
            if stop_ok {
                worker_guard.circuit_breaker.reset();
            } else {
                worker_guard.circuit_breaker.force_open();
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
            let mut bound = std::net::TcpListener::bind(("127.0.0.1", worker.port)).is_ok();
            if !bound {
                #[cfg(target_os = "windows")]
                {
                    if kill_listening_pid_on_port(worker.port) {
                        bound = std::net::TcpListener::bind(("127.0.0.1", worker.port)).is_ok();
                    }
                }
            }

            if !bound {
                let e = format!("端口 {} 被占用", worker.port);
                let start = worker.port.saturating_add(1);
                let end = worker.port.saturating_add(50);
                if let Some(new_port) = find_free_port(start, end) {
                    log::warn!(
                        "[Gateway Pool] [Worker-{}] 端口 {} 被占用，切换到 {}: {}",
                        worker.id,
                        worker.port,
                        new_port,
                        e,
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

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

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

        // 启动后台线程读取 stderr（检查 READY 状态，更新心跳，避免 HTTP 健康检查）
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
                                if line.contains("⚠️ 模型 ") && line.contains(" 不可用") {
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

                                // 关键优化：检查 [READY] 消息，等待端口 bind 成功后再转为 IDLE
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
                                            {
                                                let mut metrics = worker_metrics.lock();
                                                metrics.last_heartbeat = Some(Instant::now());
                                                log::info!(
                                                    "[Gateway Pool] [Worker-{}] READY + 端口可用，状态转为 IDLE",
                                                    worker_id
                                                );
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
                                    let mut metrics = worker_metrics.lock();
                                    metrics.panic_detected = true;
                                }

                                // 安全处理日志输出（避免 panic 传播）
                                if line.contains("[FATAL]")
                                    || line.contains("[EXIT]")
                                    || line.contains("[UNHANDLED]")
                                    || line.contains("[ERROR]")
                                {
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
                // 关键修复：使用 catch_unwind 捕获 panic，避免端口检查线程崩溃
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
                            let current_state =
                                WorkerState::from(worker_state.load(Ordering::Relaxed));
                            if matches!(current_state, WorkerState::Init | WorkerState::Ready)
                                && model_ready.load(Ordering::Relaxed)
                            {
                                worker_state.store(WorkerState::Idle as u8, Ordering::Relaxed);
                                {
                                    let mut metrics = worker_metrics.lock();
                                    metrics.last_heartbeat = Some(Instant::now());
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
                }));
            });
        }

        // 关键修复：安全锁住 Mutex，避免 poisoned 导致 panic
        {
            let mut metrics = worker.metrics.lock();
            metrics.consecutive_failures = 0;
            metrics.consecutive_timeouts = 0;
            metrics.last_timeout_at = None;
            metrics.panic_detected = false;
        }

        worker.circuit_breaker.reset();
        worker.half_open_testing = false;

        // 关键优化：大幅减少启动等待时间，避免阻塞 UI 线程
        // 进程已启动，快速检查后立即返回，让后台线程处理就绪检查
        let start = Instant::now();
        let max_wait = Duration::from_millis(200); // 从 800ms 减少到 200ms，快速返回
        let mut saw_ready = false;
        let mut check_count = 0;
        const MAX_CHECKS: u32 = 4; // 最多检查 4 次 (200ms / 50ms)

        loop {
            check_count += 1;

            // 快速检查进程是否立即退出
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
                    Ok(None) => {
                        // 进程仍在运行，继续
                    }
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

            // 快速检查状态，如果已经就绪则立即返回
            let current_state = worker.status();
            if current_state == WorkerState::Idle {
                saw_ready = true;
                break;
            }

            // 提前退出条件：超时或达到最大检查次数
            if start.elapsed() >= max_wait || check_count >= MAX_CHECKS {
                break;
            }

            thread::sleep(Duration::from_millis(50));
        }

        // 关键优化：如果未就绪，立即设置为 Ready 状态，允许后台继续初始化
        // 不等待完全就绪，避免阻塞启动流程，让健康检查线程处理后续状态转移
        if !saw_ready {
            let current_state = worker.status();
            if current_state == WorkerState::Init {
                worker.set_state(WorkerState::Ready);
                log::debug!(
                    "[Gateway Pool] Worker-{} 设置为 Ready 状态，后台继续初始化（快速启动模式）",
                    worker.id
                );
            }
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
        let mut zombie = false;
        if let Some(mut child) = worker.process.take() {
            let worker_port = worker.port;
            kill_listening_pid_on_port(worker_port);
            let mut exited = false;
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(1200) {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        exited = true;
                        break;
                    }
                    Ok(None) => thread::sleep(Duration::from_millis(100)),
                    Err(_) => break,
                }
            }

            #[cfg(target_os = "windows")]
            {
                if !exited {
                    if let Err(e) = child.kill() {
                        log::warn!("Worker-{} kill() 失败: {}", worker_id, e);
                    }
                    let taskkill_output = Command::new("taskkill")
                        .args(["/F", "/T", "/PID", &child.id().to_string()])
                        .output();

                    let taskkill_ok = match taskkill_output {
                        Ok(output) if output.status.success() => true,
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

                    if taskkill_ok {
                        exited = wait_port_free(worker_port, Duration::from_millis(1500));
                    } else if kill_listening_pid_on_port(worker_port) {
                        exited = wait_port_free(worker_port, Duration::from_millis(2000));
                    }
                } else {
                    exited = wait_port_free(worker_port, Duration::from_millis(1500));
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                if let Err(e) = child.kill() {
                    log::warn!("Worker-{} kill() 失败: {}", worker_id, e);
                }
                exited = wait_port_free(worker_port, Duration::from_millis(1500));
            }

            let start = Instant::now();
            let timeout = Duration::from_secs(3);
            while !exited && start.elapsed() < timeout {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        exited = true;
                        break;
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                    Err(_) => break,
                }
            }

            if !exited {
                worker.circuit_breaker.force_open();
                worker.set_state(WorkerState::Zombie);
                zombie = true;
                log::warn!(
                    "[Gateway Pool] Worker-{} 未能确认退出，进入 Zombie（端口 {} 可能仍被占用）",
                    worker_id,
                    worker_port
                );
            }
        }

        if zombie {
            #[cfg(target_os = "windows")]
            {
                let port = worker.port;
                let _ = kill_listening_pid_on_port(port);
                let _ = wait_port_free(port, Duration::from_millis(2000));
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
                    // 关键修复：只有在非用户停止时才调度重启
                    // 注意：这里无法直接访问 user_stopped，通过检查 pending_restart 来判断
                    let should_restart = worker_guard.pending_restart.is_some();
                    let worker_clone = Arc::clone(worker);
                    drop(worker_guard);
                    if should_restart {
                        schedule_restart_for_worker(worker_clone, HealthSignal::ProcessExit);
                    }
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
            let port = worker_guard.port;
            let worker_clone = Arc::clone(worker);
            if is_port_free(port) {
                worker_guard.set_state(WorkerState::Dead);
            } else {
                worker_guard.set_state(WorkerState::Zombie);
                #[cfg(target_os = "windows")]
                {
                    let _ = kill_listening_pid_on_port(port);
                    let _ = wait_port_free(port, Duration::from_millis(2000));
                }
            }
            worker_guard.circuit_breaker.force_open();
            // 关键修复：只有在非用户停止时才调度重启
            // 注意：这里无法直接访问 user_stopped，通过检查 pending_restart 来判断
            let should_restart = worker_guard.pending_restart.is_some();
            drop(worker_guard);
            if should_restart {
                schedule_restart_for_worker(worker_clone, HealthSignal::ProcessExit);
            }
            return false;
        }

        // 非 HTTP 健康检查：仅检查进程状态和心跳（非阻塞）
        let is_alive = true; // 已经在上面检查过了
        let now = Instant::now();
        let port_bound = worker_guard.port_bound.load(Ordering::Relaxed);
        let model_ready = worker_guard.model_ready.load(Ordering::Relaxed);
        let heartbeat_ok = {
            let mut metrics = crate::utils::lock_or_recover(
                worker_guard.metrics.as_ref(),
                "GatewayWorker.metrics",
            );
            if is_alive && port_bound && model_ready {
                metrics.last_heartbeat = Some(now);
                true
            } else if let Some(last_heartbeat) = metrics.last_heartbeat {
                last_heartbeat.elapsed() < Duration::from_secs(120)
            } else {
                is_alive
            }
        };

        worker_guard.last_health_check = Some(Instant::now());

        if is_alive && heartbeat_ok {
            // 进程运行中且心跳正常，更新状态和指标
            let current_state = worker_guard.status();
            if current_state == WorkerState::Ready && port_bound && model_ready {
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
        let user_stopped = Arc::clone(&self.user_stopped);

        thread::spawn(move || {
            // 关键优化：使用 catch_unwind 捕获 panic，避免健康检查线程崩溃导致应用退出
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                log::info!("[Gateway Pool] 健康检查线程已启动（进程状态 + 心跳，无 HTTP）");

                // 1. 必须：循环必须有"退出条件 + 心跳"
                let heartbeat = std::sync::Arc::new(crate::utils::Heartbeat::new());
                let heartbeat_clone = heartbeat.clone();

                loop {
                    // 2. 必须：更新心跳
                    heartbeat_clone.ping();

                    // 3. 必须：检查退出条件
                    if user_stopped.load(Ordering::Relaxed) {
                        // 用户手动停止时，延长检查间隔，减少资源占用
                        thread::sleep(Duration::from_secs(60));
                        continue;
                    }

                    // 4. 工程原则：避免 CPU 100%，必须有 sleep
                    // 关键优化：增加健康检查间隔到 30 秒，减少 CPU 占用和锁竞争
                    thread::sleep(Duration::from_secs(30));

                    // 5. 工程原则：心跳检查（Watchdog）
                    // 如果心跳超时（60秒），说明循环可能卡死，记录警告
                    if !heartbeat_clone.is_alive(60000) {
                        log::warn!("[Gateway Pool] 健康检查线程心跳超时，可能卡死");
                    }

                    for (idx, worker) in workers.iter().enumerate() {
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

                        if worker_guard.status() == WorkerState::Zombie {
                            let port = worker_guard.port;
                            #[cfg(target_os = "windows")]
                            {
                                let _ = kill_listening_pid_on_port(port);
                                let _ = wait_port_free(port, Duration::from_millis(2000));
                            }
                            worker_guard.circuit_breaker.force_open();
                            let worker_clone = Arc::clone(worker);
                            drop(worker_guard);
                            schedule_restart_for_worker(
                                worker_clone,
                                HealthSignal::HeartbeatTimeout,
                            );
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
                            // 关键修复：只有在非用户停止时才调度重启
                            if !user_stopped.load(Ordering::Relaxed) {
                                let worker_clone = Arc::clone(worker);
                                drop(worker_guard);
                                schedule_restart_for_worker(
                                    worker_clone,
                                    HealthSignal::PanicDetected,
                                );
                            }
                            continue;
                        }

                        // 检查进程状态
                        if let Some(ref mut child) = worker_guard.process {
                            match child.try_wait() {
                                Ok(Some(_)) => {
                                    log::warn!("[Gateway Pool] Worker-{} 进程已退出", idx);
                                    worker_guard.set_state(WorkerState::Dead);
                                    worker_guard.circuit_breaker.force_open();
                                    // 关键修复：只有在非用户停止时才调度重启
                                    if !user_stopped.load(Ordering::Relaxed) {
                                        let worker_clone = Arc::clone(worker);
                                        drop(worker_guard);
                                        schedule_restart_for_worker(
                                            worker_clone,
                                            HealthSignal::ProcessExit,
                                        );
                                    }
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
                            let port = worker_guard.port;
                            let worker_clone = Arc::clone(worker);
                            if is_port_free(port) {
                                worker_guard.set_state(WorkerState::Dead);
                            } else {
                                worker_guard.set_state(WorkerState::Zombie);
                                #[cfg(target_os = "windows")]
                                {
                                    let _ = kill_listening_pid_on_port(port);
                                    let _ = wait_port_free(port, Duration::from_millis(2000));
                                }
                            }
                            worker_guard.circuit_breaker.force_open();
                            // 关键修复：只有在非用户停止时才调度重启
                            if !user_stopped.load(Ordering::Relaxed) {
                                drop(worker_guard);
                                schedule_restart_for_worker(
                                    worker_clone,
                                    HealthSignal::ProcessExit,
                                );
                            }
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

                        let now = Instant::now();
                        let port_bound = worker_guard.port_bound.load(Ordering::Relaxed);
                        let model_ready = worker_guard.model_ready.load(Ordering::Relaxed);

                        if matches!(
                            worker_guard.status(),
                            WorkerState::Init | WorkerState::Ready
                        ) {
                            if let Some(started_at) = worker_guard.started_at {
                                if started_at.elapsed() > Duration::from_secs(30)
                                    && !(port_bound && model_ready)
                                {
                                    worker_guard.set_state(WorkerState::Unhealthy);
                                    worker_guard.circuit_breaker.force_open();
                                    if !user_stopped.load(Ordering::Relaxed) {
                                        let worker_clone = Arc::clone(worker);
                                        drop(worker_guard);
                                        schedule_restart_for_worker(
                                            worker_clone,
                                            HealthSignal::HeartbeatTimeout,
                                        );
                                    }
                                    continue;
                                }
                            }
                        }

                        let heartbeat_ok = {
                            let mut metrics = crate::utils::lock_or_recover(
                                worker_guard.metrics.as_ref(),
                                "GatewayWorker.metrics",
                            );
                            if is_alive && port_bound && model_ready {
                                metrics.last_heartbeat = Some(now);
                                true
                            } else if let Some(last_heartbeat) = metrics.last_heartbeat {
                                last_heartbeat.elapsed() < Duration::from_secs(120)
                            } else {
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
                            // 关键修复：只有在非用户停止时才调度重启
                            if !user_stopped.load(Ordering::Relaxed) {
                                let worker_clone = Arc::clone(worker);
                                drop(worker_guard);
                                schedule_restart_for_worker(
                                    worker_clone,
                                    HealthSignal::ProcessExit,
                                );
                            }
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

                        // 关键优化：进程运行中且心跳正常，更新状态和指标
                        let current_state = worker_guard.status();
                        if current_state == WorkerState::Ready && port_bound && model_ready {
                            worker_guard.set_state(WorkerState::Idle);
                        }

                        worker_guard.last_success = Some(Instant::now());
                        {
                            let mut metrics = crate::utils::lock_or_recover(
                                worker_guard.metrics.as_ref(),
                                "GatewayWorker.metrics",
                            );
                            metrics.consecutive_failures = 0;
                        }
                        worker_guard.record_success();
                        worker_guard.half_open_testing = false;
                        worker_guard.circuit_breaker.reset();

                        if idx == 0 {
                            log::debug!("[Gateway Pool] Worker-0 健康检查通过（进程状态 + 心跳）");
                        }
                    }
                }
            }));

            // 关键修复：如果健康检查线程 panic，记录但不影响主进程
            if let Err(panic_info) = result {
                log::error!("[Gateway Pool] 健康检查线程 panic: {:?}", panic_info);
                // 重置标志，允许重新启动健康检查线程
                HEALTH_CHECK_STARTED.store(false, Ordering::SeqCst);
            }
        });
    }

    /// 获取模型列表（带缓存和限频，避免频繁请求导致阻塞）
    pub fn get_models_cached(&self) -> Result<Vec<String>, String> {
        // 关键优化：如果服务被用户手动停止，直接返回错误，不进行重试检查
        if self.user_stopped.load(Ordering::Relaxed) {
            return Err("AI Gateway 服务已被用户手动停止，请先在设置页面启动服务".to_string());
        }

        if let Some(reason) = self.startup_failed_reason() {
            return Err(format!("AI Gateway 启动失败: {}", reason));
        }

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

        let models: Vec<String> = if let Some(data_array) = data
            .get("data")
            .and_then(|d: &serde_json::Value| d.as_array())
        {
            data_array
                .iter()
                .filter_map(|item: &serde_json::Value| {
                    item.get("id")
                        .and_then(|id: &serde_json::Value| id.as_str())
                        .map(|s: &str| s.to_string())
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
    /// 特殊处理：对 /v1/models 请求，使用缓存和限频，避免阻塞
    pub fn forward_request(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        headers: Option<&[(&str, &str)]>,
    ) -> Result<(StatusCode, Vec<u8>), String> {
        // 关键优化：如果服务被用户手动停止，直接返回错误，不进行重试检查
        if self.user_stopped.load(Ordering::Relaxed) {
            return Err("AI Gateway 服务已被用户手动停止，请先在设置页面启动服务".to_string());
        }

        if let Some(reason) = self.startup_failed_reason() {
            return Err(format!("AI Gateway 启动失败: {}", reason));
        }

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
            // 关键优化：在每次重试前检查用户是否手动停止了服务
            if self.user_stopped.load(Ordering::Relaxed) {
                return Err("AI Gateway 服务已被用户手动停止，请先在设置页面启动服务".to_string());
            }

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
                            // 关键优化：在重试前检查用户是否手动停止了服务
                            if self.user_stopped.load(Ordering::Relaxed) {
                                return Err(
                                    "AI Gateway 服务已被用户手动停止，请先在设置页面启动服务"
                                        .to_string(),
                                );
                            }
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
                    // 关键优化：如果服务被用户手动停止，直接返回错误，不进行重试，也不打印警告
                    if self.user_stopped.load(Ordering::Relaxed) {
                        return Err(
                            "AI Gateway 服务已被用户手动停止，请先在设置页面启动服务".to_string()
                        );
                    }

                    // 只有在非用户操作（异常关闭）时才进行重试检查和打印警告
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

                    // 关键优化：在重试前再次检查用户是否手动停止了服务
                    if self.user_stopped.load(Ordering::Relaxed) {
                        return Err(
                            "AI Gateway 服务已被用户手动停止，请先在设置页面启动服务".to_string()
                        );
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

    /// 获取所有 Worker 的详细信息（用于前端展示）
    pub fn get_workers_info(&self) -> Vec<serde_json::Value> {
        self.workers
            .iter()
            .map(|w| {
                let wg = crate::utils::lock_or_recover(w.as_ref(), "GatewayWorker");
                serde_json::json!({
                    "id": wg.id,
                    "port": wg.port,
                    "pid": wg.process.as_ref().map(|p| p.id()),
                    "status": format!("{:?}", wg.status()),
                    "is_healthy": wg.is_healthy(),
                    "active_requests": wg.metrics.lock().active_requests,
                    "total_requests": wg.metrics.lock().total_requests,
                    "recent_fail_rate": wg.metrics.lock().recent_fail_rate,
                })
            })
            .collect()
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
            if let Ok(response) = client.get(&health_url).send() {
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
