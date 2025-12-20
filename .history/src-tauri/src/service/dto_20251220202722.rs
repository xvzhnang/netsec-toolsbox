/// 统一的服务状态 DTO（供前端使用）
use serde::{Serialize, Deserialize};
use crate::service::state::ServiceState;

/// 服务状态 DTO（前端只认这个格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusDTO {
    /// 服务 ID
    pub id: String,
    /// 服务名称
    pub name: String,
    /// 当前状态
    pub state: ServiceState,
    /// 状态消息（可选）
    pub message: Option<String>,
    /// 服务描述（可选）
    pub description: Option<String>,
    /// 是否健康
    pub is_healthy: bool,
    /// 是否可用（可接受请求）
    pub is_available: bool,
    /// 额外元数据（服务特定信息）
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl ServiceStatusDTO {
    /// 从 Service 创建 DTO
    pub fn from_service(service: &dyn crate::service::trait_def::Service) -> Self {
        Self {
            id: service.id().to_string(),
            name: service.name().to_string(),
            state: service.state(),
            message: service.message(),
            description: service.description().map(|s| s.to_string()),
            is_healthy: service.state().is_healthy(),
            is_available: service.state().is_available(),
            metadata: serde_json::json!({}),
        }
    }
}

/// 所有服务的状态列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusListDTO {
    pub services: Vec<ServiceStatusDTO>,
}

