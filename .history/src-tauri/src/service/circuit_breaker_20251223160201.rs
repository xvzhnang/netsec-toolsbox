/// 统一熔断与限流策略
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// 关闭（正常）
    Closed,
    /// 打开（熔断）
    Open,
    /// 半开（测试恢复）
    HalfOpen,
}

/// 熔断器配置
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// 失败阈值（连续失败多少次后打开熔断器）
    pub failure_threshold: u32,
    /// 成功阈值（半开状态下成功多少次后关闭熔断器）
    pub success_threshold: u32,
    /// 超时时间（打开状态持续多久后进入半开）
    pub timeout: Duration,
    /// 时间窗口（统计失败率的时间窗口）
    pub time_window: Duration,
    /// 最小请求数（时间窗口内最少请求数才统计失败率）
    #[allow(dead_code)]
    pub min_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
            time_window: Duration::from_secs(60),
            min_requests: 10,
        }
    }
}

/// 熔断器
#[derive(Debug)]
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    config: CircuitBreakerConfig,
    /// 连续失败次数
    consecutive_failures: Arc<Mutex<u32>>,
    /// 半开状态下的成功次数
    half_open_successes: Arc<Mutex<u32>>,
    /// 熔断器打开时间
    opened_at: Arc<Mutex<Option<Instant>>>,
    /// 请求历史（用于统计失败率）
    request_history: Arc<Mutex<Vec<(Instant, bool)>>>, // (时间, 是否成功)
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitBreakerState::Closed)),
            config,
            consecutive_failures: Arc::new(Mutex::new(0)),
            half_open_successes: Arc::new(Mutex::new(0)),
            opened_at: Arc::new(Mutex::new(None)),
            request_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn opened_elapsed(&self) -> Option<Duration> {
        let opened_at =
            *crate::utils::lock_or_recover(self.opened_at.as_ref(), "CircuitBreaker.opened_at");
        opened_at.map(|t| t.elapsed())
    }

    pub fn reset(&self) {
        let mut state_guard =
            crate::utils::lock_or_recover(self.state.as_ref(), "CircuitBreaker.state");
        *state_guard = CircuitBreakerState::Closed;
        drop(state_guard);

        let mut failures = crate::utils::lock_or_recover(
            self.consecutive_failures.as_ref(),
            "CircuitBreaker.consecutive_failures",
        );
        *failures = 0;
        drop(failures);

        let mut successes = crate::utils::lock_or_recover(
            self.half_open_successes.as_ref(),
            "CircuitBreaker.half_open_successes",
        );
        *successes = 0;
        drop(successes);

        let mut opened_at =
            crate::utils::lock_or_recover(self.opened_at.as_ref(), "CircuitBreaker.opened_at");
        *opened_at = None;
    }

    pub fn force_open(&self) {
        let mut state_guard =
            crate::utils::lock_or_recover(self.state.as_ref(), "CircuitBreaker.state");
        *state_guard = CircuitBreakerState::Open;
        drop(state_guard);

        let mut failures = crate::utils::lock_or_recover(
            self.consecutive_failures.as_ref(),
            "CircuitBreaker.consecutive_failures",
        );
        *failures = self.config.failure_threshold;
        drop(failures);

        let mut successes = crate::utils::lock_or_recover(
            self.half_open_successes.as_ref(),
            "CircuitBreaker.half_open_successes",
        );
        *successes = 0;
        drop(successes);

        let mut opened_at =
            crate::utils::lock_or_recover(self.opened_at.as_ref(), "CircuitBreaker.opened_at");
        *opened_at = Some(Instant::now());
    }

    /// 检查是否允许请求通过
    pub fn can_execute(&self) -> bool {
        let state = *crate::utils::lock_or_recover(self.state.as_ref(), "CircuitBreaker.state");

        match state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // 检查是否超时，进入半开状态
                if let Some(opened_at) = *crate::utils::lock_or_recover(
                    self.opened_at.as_ref(),
                    "CircuitBreaker.opened_at",
                ) {
                    if opened_at.elapsed() >= self.config.timeout {
                        let mut state_guard = crate::utils::lock_or_recover(
                            self.state.as_ref(),
                            "CircuitBreaker.state",
                        );
                        *state_guard = CircuitBreakerState::HalfOpen;
                        let mut successes = crate::utils::lock_or_recover(
                            self.half_open_successes.as_ref(),
                            "CircuitBreaker.half_open_successes",
                        );
                        *successes = 0;
                        return true; // 半开状态允许一个请求测试
                    }
                }
                false
            }
            CircuitBreakerState::HalfOpen => true, // 半开状态允许请求测试
        }
    }

    /// 记录成功
    pub fn record_success(&self) {
        let mut state_guard =
            crate::utils::lock_or_recover(self.state.as_ref(), "CircuitBreaker.state");
        let mut failures = crate::utils::lock_or_recover(
            self.consecutive_failures.as_ref(),
            "CircuitBreaker.consecutive_failures",
        );
        let mut history = crate::utils::lock_or_recover(
            self.request_history.as_ref(),
            "CircuitBreaker.request_history",
        );

        // 记录到历史
        history.push((Instant::now(), true));
        self.cleanup_old_history(&mut history);

        match *state_guard {
            CircuitBreakerState::Closed => {
                // 重置连续失败次数
                *failures = 0;
            }
            CircuitBreakerState::HalfOpen => {
                // 半开状态下成功，增加成功计数
                let mut successes = crate::utils::lock_or_recover(
                    self.half_open_successes.as_ref(),
                    "CircuitBreaker.half_open_successes",
                );
                *successes += 1;

                // 如果成功次数达到阈值，关闭熔断器
                if *successes >= self.config.success_threshold {
                    *state_guard = CircuitBreakerState::Closed;
                    *failures = 0;
                    let mut opened_at = crate::utils::lock_or_recover(
                        self.opened_at.as_ref(),
                        "CircuitBreaker.opened_at",
                    );
                    *opened_at = None;
                }
            }
            CircuitBreakerState::Open => {
                // 打开状态下不应该有成功，但记录以防万一
            }
        }
    }

    /// 记录失败
    pub fn record_failure(&self) {
        let mut state_guard =
            crate::utils::lock_or_recover(self.state.as_ref(), "CircuitBreaker.state");
        let mut failures = crate::utils::lock_or_recover(
            self.consecutive_failures.as_ref(),
            "CircuitBreaker.consecutive_failures",
        );
        let mut history = crate::utils::lock_or_recover(
            self.request_history.as_ref(),
            "CircuitBreaker.request_history",
        );

        // 记录到历史
        history.push((Instant::now(), false));
        self.cleanup_old_history(&mut history);

        match *state_guard {
            CircuitBreakerState::Closed => {
                *failures += 1;

                // 检查是否达到失败阈值
                if *failures >= self.config.failure_threshold {
                    // 打开熔断器
                    *state_guard = CircuitBreakerState::Open;
                    let mut opened_at = crate::utils::lock_or_recover(
                        self.opened_at.as_ref(),
                        "CircuitBreaker.opened_at",
                    );
                    *opened_at = Some(Instant::now());
                }
            }
            CircuitBreakerState::HalfOpen => {
                // 半开状态下失败，立即打开熔断器
                *state_guard = CircuitBreakerState::Open;
                let mut opened_at = crate::utils::lock_or_recover(
                    self.opened_at.as_ref(),
                    "CircuitBreaker.opened_at",
                );
                *opened_at = Some(Instant::now());
                *failures = self.config.failure_threshold;
            }
            CircuitBreakerState::Open => {
                // 已经打开，更新打开时间
                let mut opened_at = crate::utils::lock_or_recover(
                    self.opened_at.as_ref(),
                    "CircuitBreaker.opened_at",
                );
                *opened_at = Some(Instant::now());
            }
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> CircuitBreakerState {
        *crate::utils::lock_or_recover(self.state.as_ref(), "CircuitBreaker.state")
    }

    /// 获取失败率（时间窗口内）
    #[allow(dead_code)]
    pub fn failure_rate(&self) -> f64 {
        let history = crate::utils::lock_or_recover(
            self.request_history.as_ref(),
            "CircuitBreaker.request_history",
        );
        let now = Instant::now();
        let window_start = now - self.config.time_window;

        let recent_requests: Vec<_> = history
            .iter()
            .filter(|(time, _)| *time >= window_start)
            .collect();

        if recent_requests.len() < self.config.min_requests as usize {
            return 0.0; // 请求数不足，返回 0
        }

        let failures = recent_requests
            .iter()
            .filter(|(_, success)| !*success)
            .count();
        failures as f64 / recent_requests.len() as f64
    }

    /// 清理过期历史记录
    fn cleanup_old_history(&self, history: &mut Vec<(Instant, bool)>) {
        let now = Instant::now();
        let cutoff = now - self.config.time_window * 2; // 保留 2 倍时间窗口的数据
        history.retain(|(time, _)| *time >= cutoff);
    }
}

/// 限流器（令牌桶算法）
#[derive(Debug)]
pub struct RateLimiter {
    /// 令牌桶容量
    capacity: u32,
    /// 当前令牌数
    tokens: Arc<Mutex<u32>>,
    /// 令牌生成速率（每秒）
    rate: f64,
    /// 上次更新时间
    last_update: Arc<Mutex<Instant>>,
}

impl RateLimiter {
    pub fn new(capacity: u32, rate_per_second: f64) -> Self {
        Self {
            capacity,
            tokens: Arc::new(Mutex::new(capacity)),
            rate: rate_per_second,
            last_update: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// 检查是否允许请求（消耗一个令牌）
    #[allow(dead_code)]
    pub fn allow(&self) -> bool {
        let mut tokens = crate::utils::lock_or_recover(self.tokens.as_ref(), "RateLimiter.tokens");
        let mut last_update =
            crate::utils::lock_or_recover(self.last_update.as_ref(), "RateLimiter.last_update");
        let now = Instant::now();

        // 计算应该生成的令牌数
        let elapsed = now.duration_since(*last_update);
        let tokens_to_add = (elapsed.as_secs_f64() * self.rate) as u32;

        if tokens_to_add > 0 {
            *tokens = (*tokens + tokens_to_add).min(self.capacity);
            *last_update = now;
        }

        // 检查是否有可用令牌
        if *tokens > 0 {
            *tokens -= 1;
            true
        } else {
            false
        }
    }

    /// 获取当前可用令牌数
    #[allow(dead_code)]
    pub fn available_tokens(&self) -> u32 {
        let mut tokens = crate::utils::lock_or_recover(self.tokens.as_ref(), "RateLimiter.tokens");
        let mut last_update =
            crate::utils::lock_or_recover(self.last_update.as_ref(), "RateLimiter.last_update");
        let now = Instant::now();

        // 更新令牌数
        let elapsed = now.duration_since(*last_update);
        let tokens_to_add = (elapsed.as_secs_f64() * self.rate) as u32;

        if tokens_to_add > 0 {
            *tokens = (*tokens + tokens_to_add).min(self.capacity);
            *last_update = now;
        }

        *tokens
    }
}
