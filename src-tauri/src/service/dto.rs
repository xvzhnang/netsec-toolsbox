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
    /// 进度（0-100，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
    /// 预计剩余时间（秒，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<u64>,
}

impl ServiceStatusDTO {
    /// 从 Service 创建 DTO
    pub fn from_service(service: &dyn crate::service::trait_def::Service) -> Self {
        let state = service.state();
        Self {
            id: service.id().to_string(),
            name: service.name().to_string(),
            state,
            message: service.message(),
            description: service.description().map(|s| s.to_string()),
            is_healthy: state.is_healthy(),
            is_available: state.is_available(),
            metadata: serde_json::json!({
                "state_display": state.to_string(),
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            }),
            progress: None,
            eta_seconds: None,
        }
    }
    
    /// 从 Service 创建 DTO（带自定义元数据）
    pub fn from_service_with_metadata(
        service: &dyn crate::service::trait_def::Service,
        metadata: serde_json::Value,
    ) -> Self {
        let mut dto = Self::from_service(service);
        // 合并自定义元数据
        if let Some(map) = dto.metadata.as_object_mut() {
            if let Some(custom) = metadata.as_object() {
                for (k, v) in custom {
                    map.insert(k.clone(), v.clone());
                }
            }
        } else {
            dto.metadata = metadata;
        }
        dto
    }
}

/// 所有服务的状态列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusListDTO {
    pub services: Vec<ServiceStatusDTO>,
}

