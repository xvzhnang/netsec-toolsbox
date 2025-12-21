use crate::service::state::ServiceState;
use serde::{Deserialize, Serialize};
/// 可配置的状态迁移规则
use std::collections::HashMap;

/// 状态迁移规则配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionConfig {
    /// 状态迁移表：from_state -> [to_states]
    pub transitions: HashMap<String, Vec<String>>,
}

impl Default for StateTransitionConfig {
    fn default() -> Self {
        let mut transitions = HashMap::new();

        // 默认迁移规则（保持向后兼容）
        transitions.insert("stopped".to_string(), vec!["starting".to_string()]);
        transitions.insert(
            "starting".to_string(),
            vec![
                "warmup".to_string(),
                "idle".to_string(),
                "unhealthy".to_string(),
            ],
        );
        transitions.insert(
            "warmup".to_string(),
            vec!["idle".to_string(), "unhealthy".to_string()],
        );
        transitions.insert(
            "idle".to_string(),
            vec![
                "busy".to_string(),
                "degraded".to_string(),
                "unhealthy".to_string(),
                "stopping".to_string(),
            ],
        );
        transitions.insert(
            "busy".to_string(),
            vec![
                "idle".to_string(),
                "degraded".to_string(),
                "unhealthy".to_string(),
            ],
        );
        transitions.insert(
            "degraded".to_string(),
            vec![
                "idle".to_string(),
                "busy".to_string(),
                "unhealthy".to_string(),
                "stopping".to_string(),
            ],
        );
        transitions.insert(
            "unhealthy".to_string(),
            vec!["restarting".to_string(), "stopped".to_string()],
        );
        transitions.insert(
            "restarting".to_string(),
            vec!["starting".to_string(), "stopped".to_string()],
        );
        transitions.insert("stopping".to_string(), vec!["stopped".to_string()]);

        // 任何状态都可以转到 stopped（紧急停止）
        transitions.insert("*".to_string(), vec!["stopped".to_string()]);

        Self { transitions }
    }
}

impl StateTransitionConfig {
    /// 从配置文件加载（未来支持）
    #[allow(dead_code)]
    pub fn from_file(_path: &str) -> Result<Self, String> {
        // TODO: 实现从文件加载
        Ok(Self::default())
    }

    /// 检查状态转换是否合法
    pub fn can_transit(&self, from: ServiceState, to: ServiceState) -> bool {
        // 相同状态允许（幂等）
        if from == to {
            return true;
        }

        let from_str = state_to_string(from);
        let to_str = state_to_string(to);

        // 检查显式规则
        if let Some(allowed_states) = self.transitions.get(&from_str) {
            if allowed_states.contains(&to_str) {
                return true;
            }
        }

        // 检查通配符规则（紧急停止）
        if let Some(allowed_states) = self.transitions.get("*") {
            if allowed_states.contains(&to_str) {
                return true;
            }
        }

        false
    }

    /// 获取所有允许的下一状态
    pub fn get_allowed_transitions(&self, from: ServiceState) -> Vec<ServiceState> {
        let from_str = state_to_string(from);
        let mut allowed = Vec::new();

        if let Some(states) = self.transitions.get(&from_str) {
            for state_str in states {
                if let Some(state) = string_to_state(state_str) {
                    allowed.push(state);
                }
            }
        }

        // 添加通配符规则（紧急停止）
        if let Some(states) = self.transitions.get("*") {
            for state_str in states {
                if let Some(state) = string_to_state(state_str) {
                    if !allowed.contains(&state) {
                        allowed.push(state);
                    }
                }
            }
        }

        allowed
    }
}

/// 状态到字符串转换
fn state_to_string(state: ServiceState) -> String {
    match state {
        ServiceState::Stopped => "stopped",
        ServiceState::Starting => "starting",
        ServiceState::Warmup => "warmup",
        ServiceState::Idle => "idle",
        ServiceState::Busy => "busy",
        ServiceState::Degraded => "degraded",
        ServiceState::Unhealthy => "unhealthy",
        ServiceState::Restarting => "restarting",
        ServiceState::Stopping => "stopping",
    }
    .to_string()
}

/// 字符串到状态转换
fn string_to_state(s: &str) -> Option<ServiceState> {
    match s {
        "stopped" => Some(ServiceState::Stopped),
        "starting" => Some(ServiceState::Starting),
        "warmup" => Some(ServiceState::Warmup),
        "idle" => Some(ServiceState::Idle),
        "busy" => Some(ServiceState::Busy),
        "degraded" => Some(ServiceState::Degraded),
        "unhealthy" => Some(ServiceState::Unhealthy),
        "restarting" => Some(ServiceState::Restarting),
        "stopping" => Some(ServiceState::Stopping),
        _ => None,
    }
}

/// 两级状态（简化版，用于轻量级场景）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimpleState {
    /// 活跃（可接受请求）
    Active,
    /// 非活跃（停止或启动中）
    Inactive,
    /// 错误（需要恢复）
    Error,
}

impl From<ServiceState> for SimpleState {
    fn from(state: ServiceState) -> Self {
        match state {
            ServiceState::Idle | ServiceState::Busy | ServiceState::Degraded => SimpleState::Active,
            ServiceState::Stopped
            | ServiceState::Starting
            | ServiceState::Warmup
            | ServiceState::Stopping => SimpleState::Inactive,
            ServiceState::Unhealthy | ServiceState::Restarting => SimpleState::Error,
        }
    }
}
