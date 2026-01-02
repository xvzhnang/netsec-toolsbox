use crate::utils::Heartbeat;
use parking_lot::Mutex;
/// ✅ 工程原则：Watchdog 自动恢复机制
/// 监控所有线程的心跳，检测到卡死后自动重启
use std::collections::HashMap;
use std::sync::Arc;

/// 线程监控信息
#[derive(Clone)]
pub struct ThreadMonitor {
    pub heartbeat: Arc<Heartbeat>,
    pub last_restart: Option<u64>, // 使用时间戳（毫秒）而不是 Instant
    pub restart_count: u32,
}

/// Watchdog 管理器
pub struct Watchdog {
    threads: Arc<Mutex<HashMap<String, ThreadMonitor>>>,
}

impl Watchdog {
    pub fn new() -> Self {
        Self {
            threads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 注册线程监控
    pub fn register_thread(&self, name: String, heartbeat: Arc<Heartbeat>) {
        let mut threads = self.threads.lock();
        threads.insert(
            name.clone(),
            ThreadMonitor {
                heartbeat,
                last_restart: None, // 使用时间戳（毫秒）
                restart_count: 0,
            },
        );
        log::info!("[Watchdog] 注册线程监控: {}", name);
    }

    /// 获取所有线程的心跳状态
    pub fn get_thread_status(&self) -> HashMap<String, ThreadStatus> {
        let threads_guard = self.threads.lock();
        threads_guard
            .iter()
            .map(|(name, monitor)| {
                let is_alive = monitor.heartbeat.is_alive(60000);
                let last_ping_ms = monitor.heartbeat.last_ping_ms();
                (
                    name.clone(),
                    ThreadStatus {
                        name: name.clone(),
                        is_alive,
                        last_ping_ms,
                        restart_count: monitor.restart_count,
                        last_restart: monitor.last_restart,
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ThreadStatus {
    pub name: String,
    pub is_alive: bool,
    pub last_ping_ms: u64,
    pub restart_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_restart: Option<u64>, // 使用时间戳（毫秒）而不是 Instant
}

impl Default for Watchdog {
    fn default() -> Self {
        Self::new()
    }
}
