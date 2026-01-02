/// ✅ 工程原则：为所有服务设计明确的状态机
/// 状态机 = 天然防死锁
use crate::service::state::ServiceState;

/// 服务状态转换规则
///
/// # 状态转换图
/// ```
/// Stopped -> Starting -> (Idle | Warmup) -> (Idle | Busy | Degraded)
///                                                      |
///                                                      v
/// Stopped <- Stopping <- (Unhealthy | Restarting) <- Degraded
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateTransition {
    /// 允许的状态转换
    Allowed,
    /// 不允许的状态转换
    Denied,
    /// 需要特殊处理的状态转换
    Conditional,
}

/// 状态机转换规则
pub struct StateMachine {
    // 状态转换规则表
    rules: Vec<(ServiceState, ServiceState, StateTransition)>,
}

impl StateMachine {
    pub fn new() -> Self {
        let mut rules = Vec::new();

        // ✅ 定义明确的状态转换规则

        // Stopped -> Starting (允许)
        rules.push((
            ServiceState::Stopped,
            ServiceState::Starting,
            StateTransition::Allowed,
        ));

        // Starting -> Idle/Warmup (允许)
        rules.push((
            ServiceState::Starting,
            ServiceState::Idle,
            StateTransition::Allowed,
        ));
        rules.push((
            ServiceState::Starting,
            ServiceState::Warmup,
            StateTransition::Allowed,
        ));
        rules.push((
            ServiceState::Starting,
            ServiceState::Failed,
            StateTransition::Allowed,
        ));

        // Warmup -> Idle (允许)
        rules.push((
            ServiceState::Warmup,
            ServiceState::Idle,
            StateTransition::Allowed,
        ));
        rules.push((
            ServiceState::Warmup,
            ServiceState::Failed,
            StateTransition::Allowed,
        ));

        // Idle <-> Busy (允许)
        rules.push((
            ServiceState::Idle,
            ServiceState::Busy,
            StateTransition::Allowed,
        ));
        rules.push((
            ServiceState::Busy,
            ServiceState::Idle,
            StateTransition::Allowed,
        ));

        // Idle/Busy -> Degraded (允许，降级)
        rules.push((
            ServiceState::Idle,
            ServiceState::Degraded,
            StateTransition::Allowed,
        ));
        rules.push((
            ServiceState::Busy,
            ServiceState::Degraded,
            StateTransition::Allowed,
        ));

        // Degraded -> Unhealthy (允许，进一步恶化)
        rules.push((
            ServiceState::Degraded,
            ServiceState::Unhealthy,
            StateTransition::Allowed,
        ));
        rules.push((
            ServiceState::Degraded,
            ServiceState::Failed,
            StateTransition::Allowed,
        ));

        // Unhealthy -> Restarting (允许，尝试恢复)
        rules.push((
            ServiceState::Unhealthy,
            ServiceState::Restarting,
            StateTransition::Allowed,
        ));
        rules.push((
            ServiceState::Unhealthy,
            ServiceState::Failed,
            StateTransition::Allowed,
        ));

        // Restarting -> Starting (允许，重启后重新启动)
        rules.push((
            ServiceState::Restarting,
            ServiceState::Starting,
            StateTransition::Allowed,
        ));

        rules.push((
            ServiceState::Failed,
            ServiceState::Starting,
            StateTransition::Allowed,
        ));

        // 任何状态 -> Stopping (允许，优雅关闭)
        for state in [
            ServiceState::Idle,
            ServiceState::Busy,
            ServiceState::Degraded,
            ServiceState::Unhealthy,
            ServiceState::Failed,
            ServiceState::Starting,
            ServiceState::Warmup,
            ServiceState::Restarting,
        ] {
            rules.push((state, ServiceState::Stopping, StateTransition::Allowed));
        }

        // Stopping -> Stopped (允许)
        rules.push((
            ServiceState::Stopping,
            ServiceState::Stopped,
            StateTransition::Allowed,
        ));

        // 任何状态 -> 自身 (允许，幂等)
        for state in [
            ServiceState::Stopped,
            ServiceState::Starting,
            ServiceState::Warmup,
            ServiceState::Idle,
            ServiceState::Busy,
            ServiceState::Degraded,
            ServiceState::Unhealthy,
            ServiceState::Failed,
            ServiceState::Restarting,
            ServiceState::Stopping,
        ] {
            rules.push((state, state, StateTransition::Allowed));
        }

        Self { rules }
    }

    /// 检查状态转换是否合法
    pub fn can_transit(&self, from: ServiceState, to: ServiceState) -> bool {
        // 查找转换规则
        for (rule_from, rule_to, transition) in &self.rules {
            if *rule_from == from && *rule_to == to {
                return matches!(transition, StateTransition::Allowed);
            }
        }

        // 如果没有找到规则，默认不允许（安全第一）
        false
    }

    /// 获取允许的下一个状态列表
    pub fn get_allowed_next_states(&self, from: ServiceState) -> Vec<ServiceState> {
        self.rules
            .iter()
            .filter(|(rule_from, _, transition)| {
                *rule_from == from && matches!(transition, StateTransition::Allowed)
            })
            .map(|(_, rule_to, _)| *rule_to)
            .collect()
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局状态机实例（单例）
static GLOBAL_STATE_MACHINE: std::sync::OnceLock<StateMachine> = std::sync::OnceLock::new();

/// 获取全局状态机
pub fn get_state_machine() -> &'static StateMachine {
    GLOBAL_STATE_MACHINE.get_or_init(|| StateMachine::new())
}
