/// 统一的服务状态机（不绑定任何业务）
use serde::{Serialize, Deserialize};

/// 服务状态枚举（统一所有服务的状态）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    /// 已停止
    Stopped,
    /// 启动中
    Starting,
    /// 预热中（加载模型/索引等）
    Warmup,
    /// 空闲，可接受请求
    Idle,
    /// 忙碌，正在处理请求
    Busy,
    /// 降级，性能下降但可用
    Degraded,
    /// 不健康，需要恢复
    Unhealthy,
    /// 正在重启
    Restarting,
    /// 正在停止
    Stopping,
}

impl ServiceState {
    /// 检查状态转换是否合法（使用默认配置）
    pub fn can_transit(from: ServiceState, to: ServiceState) -> bool {
        // 使用默认配置进行校验（保持向后兼容）
        let config = crate::service::state_transition::StateTransitionConfig::default();
        config.can_transit(from, to)
    }
    
    /// 检查状态转换是否合法（使用自定义配置）
    pub fn can_transit_with_config(from: ServiceState, to: ServiceState, config: &crate::service::state_transition::StateTransitionConfig) -> bool {
        config.can_transit(from, to)
    }
    
    /// 检查是否可用（可接受请求）
    pub fn is_available(&self) -> bool {
        matches!(self, ServiceState::Idle | ServiceState::Degraded | ServiceState::Busy)
    }
    
    /// 检查是否健康（可用于调度）
    pub fn is_healthy(&self) -> bool {
        matches!(self, ServiceState::Idle | ServiceState::Busy | ServiceState::Degraded)
    }
    
    /// 检查是否忙碌
    pub fn is_busy(&self) -> bool {
        matches!(self, ServiceState::Busy)
    }
    
    /// 检查是否处于错误状态
    pub fn is_error(&self) -> bool {
        matches!(self, ServiceState::Unhealthy | ServiceState::Stopped)
    }
}

impl Default for ServiceState {
    fn default() -> Self {
        ServiceState::Stopped
    }
}

impl std::fmt::Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ServiceState::Stopped => "stopped",
            ServiceState::Starting => "starting",
            ServiceState::Warmup => "warmup",
            ServiceState::Idle => "idle",
            ServiceState::Busy => "busy",
            ServiceState::Degraded => "degraded",
            ServiceState::Unhealthy => "unhealthy",
            ServiceState::Restarting => "restarting",
            ServiceState::Stopping => "stopping",
        };
        write!(f, "{}", s)
    }
}

