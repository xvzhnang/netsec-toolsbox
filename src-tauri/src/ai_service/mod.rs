pub mod legacy;
pub mod pool;
mod service_wrapper;

pub use pool::GatewayPool;
pub use service_wrapper::GatewayPoolService;

// 连接池状态管理
use crate::ai_service::pool::WorkerState;
use crate::service::circuit_breaker::CircuitBreakerState;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::State;

static GLOBAL_POOL: OnceLock<Arc<Mutex<GatewayPool>>> = OnceLock::new();

pub(crate) fn get_global_pool() -> Arc<Mutex<GatewayPool>> {
    GLOBAL_POOL
        .get_or_init(|| {
            let pool_size = 3;
            let base_port = 8765;
            let pool = GatewayPool::new(pool_size, base_port);
            Arc::new(Mutex::new(pool))
        })
        .clone()
}

/// AI 服务连接池状态
#[derive(Default)]
pub struct AIServicePoolState {
    _unused: (),
}

/// 初始化连接池（默认 3 个 Worker，端口从 8765 开始）
#[tauri::command]
pub fn init_gateway_pool(state: State<AIServicePoolState>) -> Result<String, String> {
    let _ = state;
    let pool = get_global_pool();
    let pool_guard = crate::utils::lock_or_recover(pool.as_ref(), "GatewayPool");
    match pool_guard.start_all() {
        Ok(results) => {
            log::info!("[Gateway Pool] 初始化成功: {:?}", results);
            pool_guard.start_health_check_thread();
            Ok("连接池已初始化".to_string())
        }
        Err(e) => Err(format!("初始化连接池失败: {}", e)),
    }
}

/// 启动连接池
#[tauri::command]
pub fn start_gateway_pool(state: State<AIServicePoolState>) -> Result<String, String> {
    init_gateway_pool(state).map(|_| "连接池已启动".to_string())
}

/// 停止连接池
#[tauri::command]
pub fn stop_gateway_pool(state: State<AIServicePoolState>) -> Result<String, String> {
    let _ = state;
    let pool = get_global_pool();
    let pool_guard = crate::utils::lock_or_recover(pool.as_ref(), "GatewayPool");
    match pool_guard.stop_all() {
        Ok(results) => {
            log::info!("[Gateway Pool] 停止成功: {:?}", results);
            Ok("连接池已停止".to_string())
        }
        Err(e) => Err(format!("停止连接池失败: {}", e)),
    }
}

/// 转发 HTTP 请求到连接池
#[tauri::command]
pub fn forward_ai_request(
    state: State<AIServicePoolState>,
    method: String,
    path: String,
    body: Option<Vec<u8>>,
    headers: Option<Vec<(String, String)>>,
) -> Result<(u16, Vec<u8>), String> {
    let _ = state;

    // 转换 headers 并转发请求
    let headers_opt = headers.as_ref().map(|h| {
        h.iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>()
    });

    let result = {
        let pool = get_global_pool();
        let mut pool_guard = crate::utils::lock_or_recover(pool.as_ref(), "GatewayPool");
        pool_guard.forward_request(&method, &path, body.as_deref(), headers_opt.as_deref())
    };

    match result {
        Ok((status, body_bytes)) => Ok((status.as_u16(), body_bytes)),
        Err(e) => Err(format!("转发请求失败: {}", e)),
    }
}

/// 诊断指定 Worker（用于排查问题，特别是 Worker-0）
#[tauri::command]
pub fn diagnose_worker(
    state: State<AIServicePoolState>,
    worker_id: usize,
) -> Result<String, String> {
    let _ = state;
    let pool = get_global_pool();
    let pool_guard = crate::utils::lock_or_recover(pool.as_ref(), "GatewayPool");
    Ok(pool_guard.diagnose_worker(worker_id))
}

/// 获取连接池状态
#[tauri::command]
pub fn get_gateway_pool_status(
    state: State<AIServicePoolState>,
) -> Result<Vec<serde_json::Value>, String> {
    let _ = state;
    let pool = get_global_pool();
    let pool_guard = crate::utils::lock_or_recover(pool.as_ref(), "GatewayPool");
    let workers = pool_guard.get_workers();
    let mut status_list = Vec::new();

    for worker in workers {
        let wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
        let status = match wg.status() {
            WorkerState::FailedPermanent => "FATAL".to_string(),
            WorkerState::Disabled => "DISABLED".to_string(),
            other => format!("{:?}", other),
        };
        status_list.push(serde_json::json!({
            "id": wg.id,
            "port": wg.port,
            "status": status,
            "active_requests": wg.active_requests(),
            "total_requests": wg.total_requests,
            "total_errors": wg.total_errors,
            "consecutive_failures": crate::utils::lock_or_recover(&wg.metrics, "GatewayWorker.metrics").consecutive_failures,
            "circuit_breaker_open": wg.circuit_breaker.state() == CircuitBreakerState::Open,
        }));
    }

    Ok(status_list)
}
