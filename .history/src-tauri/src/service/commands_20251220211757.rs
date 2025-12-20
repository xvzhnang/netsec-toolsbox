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

