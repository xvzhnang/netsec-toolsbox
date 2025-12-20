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

pub use state::ServiceState;
pub use trait_def::{Service, HealthStatus};
pub use manager::ServiceManager;
pub use dto::ServiceStatusDTO;
pub use commands::*;

