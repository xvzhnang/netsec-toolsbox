/// ServiceManager 的 Tauri 命令
use tauri::State;
use crate::service::manager::ServiceManager;
use crate::service::dto::{ServiceStatusDTO, ServiceStatusListDTO};
use std::sync::Mutex;
use serde::Serialize;

/// 获取所有服务状态
#[tauri::command]
pub fn get_all_services(
    manager: State<'_, Mutex<ServiceManager>>,
) -> Result<ServiceStatusListDTO, String> {
    let manager_guard = manager.lock().unwrap();
    Ok(manager_guard.get_all_status())
}

/// 获取单个服务状态
#[tauri::command]
pub fn get_service_status(
    manager: State<'_, Mutex<ServiceManager>>,
    id: String,
) -> Result<Option<ServiceStatusDTO>, String> {
    let manager_guard = manager.lock().unwrap();
    Ok(manager_guard.get_status(&id))
}

/// 启动服务
#[tauri::command]
pub fn start_service(
    manager: State<'_, Mutex<ServiceManager>>,
    id: String,
) -> Result<String, String> {
    let manager_guard = manager.lock().unwrap();
    manager_guard.start_service(&id)
        .map(|_| format!("服务 {} 已启动", id))
        .map_err(|e| format!("启动失败: {}", e))
}

/// 停止服务
#[tauri::command]
pub fn stop_service(
    manager: State<'_, Mutex<ServiceManager>>,
    id: String,
) -> Result<String, String> {
    let manager_guard = manager.lock().unwrap();
    manager_guard.stop_service(&id)
        .map(|_| format!("服务 {} 已停止", id))
        .map_err(|e| format!("停止失败: {}", e))
}

/// 重启服务
#[tauri::command]
pub fn restart_service(
    manager: State<'_, Mutex<ServiceManager>>,
    id: String,
) -> Result<String, String> {
    let manager_guard = manager.lock().unwrap();
    manager_guard.restart_service(&id)
        .map(|_| format!("服务 {} 已重启", id))
        .map_err(|e| format!("重启失败: {}", e))
}

/// 获取 Prometheus 格式的指标
#[tauri::command]
pub fn get_prometheus_metrics(
    manager: State<'_, Mutex<ServiceManager>>,
) -> Result<String, String> {
    let manager_guard = manager.lock().unwrap();
    Ok(manager_guard.get_prometheus_metrics())
}

/// 获取服务指标
#[derive(Debug, Clone, Serialize)]
pub struct ServiceMetricsDTO {
    pub service_id: String,
    pub total_requests: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub avg_response_time_ms: f64,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub state_changes: u64,
    pub health_check_count: u64,
    pub health_check_failures: u64,
    pub health_check_success_rate: f64,
    pub start_count: u64,
    pub restart_count: u64,
}

#[tauri::command]
pub fn get_service_metrics(
    manager: State<'_, Mutex<ServiceManager>>,
    id: String,
) -> Result<Option<ServiceMetricsDTO>, String> {
    let manager_guard = manager.lock().unwrap();
    let metrics = manager_guard.metrics();
    let metrics_guard = metrics.lock().unwrap();
    
    if let Some(metric) = metrics_guard.get_metrics(&id) {
        let success_rate = metric.success_rate();
        let failure_rate = metric.failure_rate();
        let health_check_success_rate = metric.health_check_success_rate();
        Ok(Some(ServiceMetricsDTO {
            service_id: metric.service_id.clone(),
            total_requests: metric.total_requests,
            total_successes: metric.total_successes,
            total_failures: metric.total_failures,
            avg_response_time_ms: metric.avg_response_time_ms,
            success_rate,
            failure_rate,
            state_changes: metric.state_changes,
            health_check_count: metric.health_check_count,
            health_check_failures: metric.health_check_failures,
            health_check_success_rate,
            start_count: metric.start_count,
            restart_count: metric.restart_count,
        }))
    } else {
        Ok(None)
    }
}

