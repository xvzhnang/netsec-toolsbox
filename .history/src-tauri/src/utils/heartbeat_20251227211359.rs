/// ✅ 工程原则：所有循环都必须"可退出" + "心跳"
/// Watchdog 机制：监控线程健康，防止死循环
use std::sync::atomic::{AtomicU64, Ordering};

/// 心跳监控器（Watchdog）
///
/// # 使用场景
/// - 监控后台线程是否卡死
/// - 检测死循环
/// - 自动恢复机制
pub struct Heartbeat {
    last_ping: AtomicU64, // 使用时间戳（毫秒）存储
}

impl Heartbeat {
    pub fn new() -> Self {
        Self {
            last_ping: AtomicU64::new(0),
        }
    }

    /// 更新心跳（子线程调用）
    pub fn ping(&self) {
        // ✅ 修复：使用 SystemTime 而不是 Instant 来计算时间戳
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.last_ping.store(now, Ordering::Relaxed);
    }

    /// 检查心跳是否超时（监控线程调用）
    ///
    /// # 参数
    /// - `timeout_ms`: 超时时间（毫秒）
    ///
    /// # 返回
    /// - `true`: 心跳正常
    /// - `false`: 心跳超时，可能卡死
    pub fn is_alive(&self, timeout_ms: u64) -> bool {
        let last = self.last_ping.load(Ordering::Relaxed);
        if last == 0 {
            // 从未 ping 过，认为正常（可能是刚启动）
            return true;
        }

        // ✅ 修复：使用 SystemTime 而不是 Instant 来计算时间戳
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let elapsed = now.saturating_sub(last);
        elapsed < timeout_ms
    }

    /// 获取最后心跳时间（毫秒）
    pub fn last_ping_ms(&self) -> u64 {
        self.last_ping.load(Ordering::Relaxed)
    }
}

impl Default for Heartbeat {
    fn default() -> Self {
        Self::new()
    }
}
