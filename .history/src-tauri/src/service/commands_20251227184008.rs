use crate::service::dto::{ServiceStatusDTO, ServiceStatusListDTO};
use crate::service::manager::ServiceManager;
use serde::Serialize;
use tauri::State;
use std::time::Duration;

/// ✅ 工程原则：UI 主线程永不阻塞
/// command 只是"信号入口"，快速返回，不等待
/// 
/// 获取所有服务状态
#[tauri::command]
pub fn get_all_services(
    manager: State<'_, ServiceManager>,
) -> Result<ServiceStatusListDTO, String> {
    // ✅ 关键修复：直接调用，无需锁 manager，因为 ServiceManager 内部处理并发
    
    // ✅ 关键修复：在后台线程启动监控，不阻塞 UI
    let manager_clone = manager.inner().clone();
    std::thread::spawn(move || {
        manager_clone.start_monitoring();
    });
    
    Ok(manager.get_all_status())
}

/// ✅ 工程原则：UI 主线程永不阻塞
/// 获取单个服务状态
#[tauri::command]
pub fn get_service_status(
    manager: State<'_, ServiceManager>,
    id: String,
) -> Result<Option<ServiceStatusDTO>, String> {
    // ✅ 关键修复：直接调用
    
    // ✅ 关键修复：在后台线程启动监控，不阻塞 UI
    let manager_clone = manager.inner().clone();
    std::thread::spawn(move || {
        manager_clone.start_monitoring();
    });
    
    Ok(manager.get_status(&id))
}

/// ✅ 工程原则：UI 主线程永不阻塞
/// command 只是"信号入口"，发送任务到后台线程
/// 
/// 启动服务
#[tauri::command]
pub fn start_service(
    manager: State<'_, ServiceManager>,
    id: String,
) -> Result<String, String> {
    // ✅ 关键修复：快速检查并发送任务，不等待启动完成
    let manager_clone = manager.inner().clone();
    let id_clone = id.clone();
    
    // 快速检查服务是否存在
    let exists = manager.get_status(&id).is_some();
    
    if !exists {
        return Err(format!("服务 {} 不存在", id));
    }
    
    // ✅ 关键修复：在后台线程执行启动，不阻塞 UI
    std::thread::spawn(move || {
        manager_clone.start_monitoring();
        if let Err(e) = manager_clone.start_service(&id_clone) {
            log::error!("[ServiceManager] 启动服务 {} 失败: {}", id_clone, e);
        }
    });
    
    Ok(format!("服务 {} 启动中...", id))
}

/// ✅ 工程原则：UI 主线程永不阻塞
/// 停止服务
#[tauri::command]
pub fn stop_service(
    manager: State<'_, ServiceManager>,
    id: String,
) -> Result<String, String> {
    // ✅ 关键修复：在后台线程执行停止，不阻塞 UI
    let manager_clone = manager.inner().clone();
    let id_clone = id.clone();
    
    std::thread::spawn(move || {
        manager_clone.start_monitoring();
        if let Err(e) = manager_clone.stop_service(&id_clone) {
            log::error!("[ServiceManager] 停止服务 {} 失败: {}", id_clone, e);
        }
    });
    
    Ok(format!("服务 {} 停止中...", id))
}

/// ✅ 工程原则：UI 主线程永不阻塞
/// 重启服务
#[tauri::command]
pub fn restart_service(
    manager: State<'_, ServiceManager>,
    id: String,
) -> Result<String, String> {
    // ✅ 关键修复：在后台线程执行重启，不阻塞 UI
    let manager_clone = manager.inner().clone();
    let id_clone = id.clone();
    
    std::thread::spawn(move || {
        manager_clone.start_monitoring();
        if let Err(e) = manager_clone.restart_service(&id_clone) {
            log::error!("[ServiceManager] 重启服务 {} 失败: {}", id_clone, e);
        }
    });
    
    Ok(format!("服务 {} 重启中...", id))
}

/// ✅ 工程原则：UI 主线程永不阻塞
/// 获取 Prometheus 格式的指标
#[tauri::command]
pub fn get_prometheus_metrics(manager: State<'_, ServiceManager>) -> Result<String, String> {
    // ✅ 关键修复：直接调用
    
    // ✅ 关键修复：在后台线程启动监控，不阻塞 UI
    let manager_clone = manager.inner().clone();
    std::thread::spawn(move || {
        manager_clone.start_monitoring();
    });
    
    Ok(manager.get_prometheus_metrics())
}

/// ✅ 工程原则：获取线程心跳状态（用于监控面板）
#[tauri::command]
pub fn get_thread_heartbeat_status(
    manager: State<'_, ServiceManager>,
) -> Result<std::collections::HashMap<String, crate::service::watchdog::ThreadStatus>, String> {
    let watchdog = manager.watchdog();
    let watchdog_guard = crate::utils::mutex_compat::lock_or_recover_parking(
        watchdog.as_ref(),
        "ServiceManager.watchdog",
    );
    
    if let Some(wd) = watchdog_guard.as_ref() {
        Ok(wd.get_thread_status())
    } else {
        Ok(std::collections::HashMap::<String, crate::service::watchdog::ThreadStatus>::new())
    }
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

/// ✅ 工程原则：UI 主线程永不阻塞
#[tauri::command]
pub fn get_service_metrics(
    manager: State<'_, ServiceManager>,
    id: String,
) -> Result<Option<ServiceMetricsDTO>, String> {
    // ✅ 关键修复：在后台线程启动监控，不阻塞 UI
    let manager_clone = manager.inner().clone();
    std::thread::spawn(move || {
        manager_clone.start_monitoring();
    });
    
    let metrics = manager.metrics();
    // 使用 std::sync::Mutex 的兼容方法
    let metrics_guard = crate::utils::lock_or_recover_std(
        metrics.as_ref(),
        "ServiceManager.metrics",
    );

    if let Some(metric) = metrics_guard.get_metrics(&id) {
        let success_rate: f64 = metric.success_rate();
        let failure_rate: f64 = metric.failure_rate();
        let health_check_success_rate: f64 = metric.health_check_success_rate();
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
