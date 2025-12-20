/// 健康检查分级系统
use crate::service::state::ServiceState;
use crate::service::trait_def::HealthStatus;

/// 健康检查级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCheckLevel {
    /// L0: 进程是否存在（最快，< 1ms）
    ProcessCheck,
    /// L1: TCP 连接检查（快，< 10ms）
    TcpCheck,
    /// L2: HTTP 健康检查（中等，< 100ms）
    HttpCheck,
    /// L3: 完整功能检查（慢，< 1000ms）
    FullCheck,
}

impl HealthCheckLevel {
    /// 获取检查超时时间
    pub fn timeout(&self) -> std::time::Duration {
        match self {
            HealthCheckLevel::ProcessCheck => std::time::Duration::from_millis(10),
            HealthCheckLevel::TcpCheck => std::time::Duration::from_millis(50),
            HealthCheckLevel::HttpCheck => std::time::Duration::from_millis(200),
            HealthCheckLevel::FullCheck => std::time::Duration::from_millis(2000),
        }
    }
    
    /// 根据服务状态选择检查级别
    pub fn for_state(state: ServiceState) -> Self {
        match state {
            ServiceState::Stopped | ServiceState::Starting => HealthCheckLevel::ProcessCheck,
            ServiceState::Idle | ServiceState::Warmup => HealthCheckLevel::HttpCheck,
            ServiceState::Busy => HealthCheckLevel::TcpCheck, // Busy 时只做轻量检查
            ServiceState::Degraded => HealthCheckLevel::HttpCheck,
            ServiceState::Unhealthy | ServiceState::Restarting => HealthCheckLevel::FullCheck,
            ServiceState::Stopping => HealthCheckLevel::ProcessCheck,
        }
    }
}

/// 健康检查策略
#[derive(Debug, Clone)]
pub struct HealthCheckStrategy {
    /// 默认检查级别
    pub default_level: HealthCheckLevel,
    /// 检查间隔（秒）
    pub interval_secs: u64,
    /// 失败阈值（连续失败多少次后标记为不健康）
    pub failure_threshold: u32,
    /// 是否在 Busy 状态时跳过检查
    pub skip_on_busy: bool,
}

impl Default for HealthCheckStrategy {
    fn default() -> Self {
        Self {
            default_level: HealthCheckLevel::HttpCheck,
            interval_secs: 5,
            failure_threshold: 3,
            skip_on_busy: true,
        }
    }
}

impl HealthCheckStrategy {
    /// 轻量级策略（高频检查，低开销）
    pub fn lightweight() -> Self {
        Self {
            default_level: HealthCheckLevel::TcpCheck,
            interval_secs: 2,
            failure_threshold: 5,
            skip_on_busy: true,
        }
    }
    
    /// 标准策略
    pub fn standard() -> Self {
        Self::default()
    }
    
    /// 深度策略（低频检查，深度检测）
    pub fn deep() -> Self {
        Self {
            default_level: HealthCheckLevel::FullCheck,
            interval_secs: 30,
            failure_threshold: 2,
            skip_on_busy: false,
        }
    }
}

/// 健康检查结果（带详细信息）
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub level: HealthCheckLevel,
    pub latency_ms: u64,
    pub message: Option<String>,
    pub timestamp: std::time::Instant,
}

impl HealthCheckResult {
    pub fn new(status: HealthStatus, level: HealthCheckLevel, latency_ms: u64) -> Self {
        Self {
            status,
            level,
            latency_ms,
            message: None,
            timestamp: std::time::Instant::now(),
        }
    }
    
    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

