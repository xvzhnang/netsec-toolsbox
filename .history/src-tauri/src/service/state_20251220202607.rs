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
    /// 检查状态转换是否合法
    pub fn can_transit(from: ServiceState, to: ServiceState) -> bool {
        match (from, to) {
            // 停止状态可以转到启动
            (ServiceState::Stopped, ServiceState::Starting) => true,
            
            // 启动后可以转到预热或空闲
            (ServiceState::Starting, ServiceState::Warmup) => true,
            (ServiceState::Starting, ServiceState::Idle) => true,
            (ServiceState::Starting, ServiceState::Unhealthy) => true,
            
            // 预热后可以转到空闲或不健康
            (ServiceState::Warmup, ServiceState::Idle) => true,
            (ServiceState::Warmup, ServiceState::Unhealthy) => true,
            
            // 空闲可以转到忙碌、降级或不健康
            (ServiceState::Idle, ServiceState::Busy) => true,
            (ServiceState::Idle, ServiceState::Degraded) => true,
            (ServiceState::Idle, ServiceState::Unhealthy) => true,
            (ServiceState::Idle, ServiceState::Stopping) => true,
            
            // 忙碌可以转回空闲、降级或不健康
            (ServiceState::Busy, ServiceState::Idle) => true,
            (ServiceState::Busy, ServiceState::Degraded) => true,
            (ServiceState::Busy, ServiceState::Unhealthy) => true,
            
            // 降级可以转到空闲、忙碌或不健康
            (ServiceState::Degraded, ServiceState::Idle) => true,
            (ServiceState::Degraded, ServiceState::Busy) => true,
            (ServiceState::Degraded, ServiceState::Unhealthy) => true,
            (ServiceState::Degraded, ServiceState::Stopping) => true,
            
            // 不健康可以转到重启
            (ServiceState::Unhealthy, ServiceState::Restarting) => true,
            (ServiceState::Unhealthy, ServiceState::Stopped) => true,
            
            // 重启后转到启动
            (ServiceState::Restarting, ServiceState::Starting) => true,
            (ServiceState::Restarting, ServiceState::Stopped) => true,
            
            // 停止中可以转到停止
            (ServiceState::Stopping, ServiceState::Stopped) => true,
            
            // 任何状态都可以转到停止（紧急停止）
            (_, ServiceState::Stopped) => true,
            
            // 相同状态允许（幂等）
            (a, b) if a == b => true,
            
            // 其他转换不合法
            _ => false,
        }
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

