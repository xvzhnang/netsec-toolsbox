/// ✅ 工程原则：Watchdog 自动恢复机制
/// 监控所有线程的心跳，检测到卡死后自动重启
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use parking_lot::Mutex;
use crate::utils::Heartbeat;

/// 线程监控信息
#[derive(Clone)]
pub struct ThreadMonitor {
    pub name: String,
    pub heartbeat: Arc<Heartbeat>,
    pub last_restart: Option<u64>, // 使用时间戳（毫秒）而不是 Instant
    pub restart_count: u32,
    pub max_restarts: u32,
}

/// Watchdog 管理器
pub struct Watchdog {
    threads: Arc<Mutex<HashMap<String, ThreadMonitor>>>,
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

impl Watchdog {
    pub fn new() -> Self {
        Self {
            threads: Arc::new(Mutex::new(HashMap::new())),
            shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 注册线程监控
    pub fn register_thread(&self, name: String, heartbeat: Arc<Heartbeat>) {
        let mut threads = self.threads.lock();
        threads.insert(name.clone(), ThreadMonitor {
            name: name.clone(),
            heartbeat,
            last_restart: None, // 使用时间戳（毫秒）
            restart_count: 0,
            max_restarts: 5, // 最多自动重启 5 次
        });
        log::info!("[Watchdog] 注册线程监控: {}", name);
    }

    /// 启动 Watchdog 监控循环
    pub fn start(&self) {
        if self.shutdown.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let threads = Arc::clone(&self.threads);
        let shutdown = Arc::clone(&self.shutdown);

        thread::spawn(move || {
            log::info!("[Watchdog] Watchdog 监控线程已启动");

            loop {
                // ✅ 检查退出条件
                if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    log::info!("[Watchdog] Watchdog 监控线程已停止");
                    break;
                }

                // 检查所有线程的心跳
                let mut threads_to_restart = Vec::new();
                {
                    let threads_guard = threads.lock();
                    for (name, monitor) in threads_guard.iter() {
                        // 检查心跳超时（60秒）
                        if !monitor.heartbeat.is_alive(60000) {
                            // 检查是否超过最大重启次数
                            if monitor.restart_count < monitor.max_restarts {
                                threads_to_restart.push(name.clone());
                                log::warn!(
                                    "[Watchdog] 线程 {} 心跳超时，准备自动重启（第 {} 次）",
                                    name,
                                    monitor.restart_count + 1
                                );
                            } else {
                                log::error!(
                                    "[Watchdog] 线程 {} 心跳超时，但已达到最大重启次数 {}，停止自动重启",
                                    name,
                                    monitor.max_restarts
                                );
                            }
                        }
                    }
                }

                // 触发重启（这里需要具体的重启逻辑，由调用方实现）
                for name in threads_to_restart {
                    log::warn!("[Watchdog] 触发线程 {} 的自动重启", name);
                    // 更新重启时间戳
                    {
                        let mut threads_guard = threads.lock();
                        if let Some(monitor) = threads_guard.get_mut(&name) {
                            monitor.restart_count += 1;
                            monitor.last_restart = Some(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64
                            );
                        }
                    }
                    // 注意：实际的重启逻辑需要由具体的服务管理器实现
                    // 这里只是记录和通知
                }

                // ✅ 避免 CPU 100%
                thread::sleep(Duration::from_secs(30));
            }
        });
    }

    /// 停止 Watchdog
    pub fn stop(&self) {
        self.shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
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

