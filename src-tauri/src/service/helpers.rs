/// Service 相关的辅助函数
use crate::service::state::ServiceState;
use crate::ai_service::pool::WorkerState;

/// 将 WorkerState 转换为 ServiceState（用于 GatewayPoolService）
pub fn worker_state_to_service_state(worker_state: WorkerState) -> ServiceState {
    match worker_state {
        WorkerState::Init => ServiceState::Starting,
        WorkerState::Ready => ServiceState::Starting, // READY 状态视为 Starting（服务就绪中）
        WorkerState::Idle => ServiceState::Idle,
        WorkerState::BusyStreaming | WorkerState::BusyBlocked => ServiceState::Busy,
        WorkerState::Degraded => ServiceState::Degraded,
        WorkerState::Unhealthy | WorkerState::Dead => ServiceState::Unhealthy,
        WorkerState::Restarting => ServiceState::Restarting,
    }
}

/// 检查服务状态是否应该触发自动恢复
pub fn should_auto_recover(state: ServiceState) -> bool {
    matches!(state, ServiceState::Unhealthy | ServiceState::Stopped)
}

/// 检查服务状态是否应该触发告警
pub fn should_alert(state: ServiceState) -> bool {
    matches!(state, ServiceState::Unhealthy | ServiceState::Restarting)
}

