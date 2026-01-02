pub mod legacy;
pub mod pool;
mod service_wrapper;

pub use pool::GatewayPool;
pub use service_wrapper::GatewayPoolService;

// 连接池状态管理
use parking_lot::Mutex;
use std::sync::{Arc, OnceLock};
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
