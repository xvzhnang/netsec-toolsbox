pub mod circuit_breaker;
pub mod commands;
pub mod dto;
pub mod events;
pub mod health_check;
pub mod helpers;
pub mod lifecycle;
pub mod manager;
pub mod metrics;
pub mod sse_handler;
/// 统一的服务管理架构
///
/// 核心思想：
/// - 所有服务（AI Gateway、Wiki、工具等）都实现 Service trait
/// - ServiceManager 统一管理、监控、恢复
/// - 前端只认统一的状态格式
pub mod state;
pub mod state_transition;
pub mod trait_def;
pub mod websocket;

// 核心类型导出（供外部使用）
pub use commands::*;
pub use manager::ServiceManager;

// 其他类型按需导出（避免未使用警告）
// pub use helpers::*;
// pub use state_transition::{StateTransitionConfig, SimpleState};
// pub use events::{ServiceEvent, EventBus, EventListener, current_timestamp};
// pub use health_check::{HealthCheckLevel, HealthCheckStrategy, HealthCheckResult as HealthCheckResultDetail};
// pub use lifecycle::{LifecycleHooks, DefaultLifecycleHooks};
// pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState, RateLimiter};
// pub use metrics::{MetricsCollector, ServiceMetrics};
// pub use websocket::{WebSocketEventListener, SSEEventStream};
// pub use sse_handler::SSEEventListener;
