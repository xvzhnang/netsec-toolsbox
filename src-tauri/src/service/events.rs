use crate::service::state::ServiceState;
/// 事件驱动架构（替代轮询）
use serde::{Deserialize, Serialize};

/// 服务事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceEvent {
    /// 状态变化事件
    StateChanged {
        service_id: String,
        from: ServiceState,
        to: ServiceState,
        timestamp: u64,
    },
    /// 健康检查事件
    HealthCheck {
        service_id: String,
        status: HealthCheckResult,
        timestamp: u64,
    },
    /// 错误事件
    Error {
        service_id: String,
        error: String,
        timestamp: u64,
    },
    /// 启动事件
    Started { service_id: String, timestamp: u64 },
    /// 停止事件
    Stopped { service_id: String, timestamp: u64 },
    /// 重启事件
    Restarted { service_id: String, timestamp: u64 },
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckResult {
    Healthy,
    Degraded,
    Unhealthy,
}

/// 事件监听器 trait
pub trait EventListener: Send + Sync {
    fn on_event(&self, event: &ServiceEvent);
}

/// 事件总线（简化版，使用回调）
pub struct EventBus {
    listeners: Vec<Box<dyn EventListener>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn subscribe(&mut self, listener: Box<dyn EventListener>) {
        self.listeners.push(listener);
    }

    pub fn emit(&self, event: &ServiceEvent) {
        for listener in &self.listeners {
            listener.on_event(event);
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取当前时间戳（毫秒）
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
