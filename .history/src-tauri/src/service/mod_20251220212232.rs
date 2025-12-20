/// 统一的服务管理架构
/// 
/// 核心思想：
/// - 所有服务（AI Gateway、Wiki、工具等）都实现 Service trait
/// - ServiceManager 统一管理、监控、恢复
/// - 前端只认统一的状态格式

pub mod state;
pub mod trait_def;
pub mod manager;
pub mod dto;
pub mod commands;
pub mod helpers;
pub mod state_transition;
pub mod events;
pub mod health_check;
pub mod lifecycle;
pub mod circuit_breaker;
pub mod metrics;
pub mod websocket;

pub use state::ServiceState;
pub use trait_def::HealthStatus;
pub use trait_def::{Service, ServiceHandle};
pub use manager::ServiceManager;
pub use dto::ServiceStatusDTO;
pub use commands::*;
pub use helpers::*;
pub use state_transition::{StateTransitionConfig, SimpleState};
pub use events::{ServiceEvent, EventBus, EventListener, current_timestamp};
pub use health_check::{HealthCheckLevel, HealthCheckStrategy, HealthCheckResult as HealthCheckResultDetail};
pub use lifecycle::{LifecycleHooks, DefaultLifecycleHooks};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState, RateLimiter};
pub use metrics::{MetricsCollector, ServiceMetrics};
pub use websocket::{WebSocketEventListener, SSEEventStream};
pub use sse_handler::SSEEventListener;

