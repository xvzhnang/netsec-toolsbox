mod pool;
mod legacy;

pub use pool::GatewayPool;

// 重新导出 legacy 模块的命令函数（供 lib.rs 使用）
pub use legacy::{
    AIServiceState,
    start_ai_service,
    stop_ai_service,
    check_ai_service_status,
    read_models_config,
    write_models_config,
};

// 连接池状态管理
use std::sync::{Arc, Mutex};
use tauri::State;

/// AI 服务连接池状态
#[derive(Default)]
pub struct AIServicePoolState {
    pool: Arc<Mutex<Option<GatewayPool>>>,
}

/// 初始化连接池（默认 3 个 Worker，端口从 8765 开始）
#[tauri::command]
pub fn init_gateway_pool(state: State<AIServicePoolState>) -> Result<String, String> {
    let mut pool_guard = state.pool.lock().unwrap();
    
    if pool_guard.is_some() {
        return Ok("连接池已存在".to_string());
    }

    let pool_size = 3;
    let base_port = 8765;
    let pool = GatewayPool::new(pool_size, base_port);
    
    // 启动所有 Worker
    match pool.start_all() {
        Ok(results) => {
            log::info!("[Gateway Pool] 初始化成功: {:?}", results);
            
            // 启动健康检查线程
            pool.start_health_check_thread();
            
            *pool_guard = Some(pool);
            Ok(format!("连接池已初始化，启动 {} 个 Worker", pool_size))
        }
        Err(e) => {
            Err(format!("初始化连接池失败: {}", e))
        }
    }
}

/// 启动连接池
#[tauri::command]
pub fn start_gateway_pool(state: State<AIServicePoolState>) -> Result<String, String> {
    let pool_guard = state.pool.lock().unwrap();
    
    if pool_guard.is_none() {
        drop(pool_guard);
        return init_gateway_pool(state);
    }

    let pool = pool_guard.as_ref().unwrap();
    let results = pool.start_all()?;
    log::info!("[Gateway Pool] 启动成功: {:?}", results);
    Ok("连接池已启动".to_string())
}

/// 停止连接池
#[tauri::command]
pub fn stop_gateway_pool(state: State<AIServicePoolState>) -> Result<String, String> {
    let mut pool_guard = state.pool.lock().unwrap();
    
    if let Some(pool) = pool_guard.as_ref() {
        match pool.stop_all() {
            Ok(results) => {
                log::info!("[Gateway Pool] 停止成功: {:?}", results);
                Ok("连接池已停止".to_string())
            }
            Err(e) => {
                Err(format!("停止连接池失败: {}", e))
            }
        }
    } else {
        Ok("连接池不存在".to_string())
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
    let mut pool_guard = state.pool.lock().unwrap();
    
    if pool_guard.is_none() {
        // 如果连接池不存在，先初始化
        drop(pool_guard);
        init_gateway_pool(state.clone())?;
        pool_guard = state.pool.lock().unwrap();
    }

    let pool = pool_guard.as_mut().ok_or("连接池未初始化")?;
    
    // 转换 headers
    let headers_opt = headers.as_ref().map(|h| {
        h.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>()
    });

    match pool.forward_request(
        &method,
        &path,
        body.as_deref(),
        headers_opt.as_deref(),
    ) {
        Ok((status, body_bytes)) => {
            Ok((status.as_u16(), body_bytes))
        }
        Err(e) => {
            Err(format!("转发请求失败: {}", e))
        }
    }
}

/// 获取连接池状态
#[tauri::command]
pub fn get_gateway_pool_status(state: State<AIServicePoolState>) -> Result<Vec<serde_json::Value>, String> {
    let pool_guard = state.pool.lock().unwrap();
    
    if let Some(pool) = pool_guard.as_ref() {
        let workers = pool.get_workers();
        let mut status_list = Vec::new();
        
        for worker in workers {
            let wg = worker.lock().unwrap();
            status_list.push(serde_json::json!({
                "id": wg.id,
                "port": wg.port,
                "status": format!("{:?}", wg.status),
                "active_requests": wg.active_requests,
                "total_requests": wg.total_requests,
                "total_errors": wg.total_errors,
                "consecutive_failures": wg.consecutive_failures,
                "circuit_breaker_open": wg.circuit_breaker_open,
            }));
        }
        
        Ok(status_list)
    } else {
        Ok(Vec::new())
    }
}

