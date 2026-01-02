/// ✅ 工程原则：锁必须可观测、可超时
/// 提供可超时的锁工具，防止死锁
/// 
/// # 核心原则
/// - 锁作用域 ≤ 20 行
/// - 拿锁后不允许：sleep、await、IO、channel recv
/// - 超时直接放弃，不死等
use parking_lot::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// 可超时的锁获取（防止死锁）
/// 
/// # 原则
/// - 锁作用域 ≤ 20 行
/// - 拿锁后不允许：sleep、await、IO、channel recv
/// - 超时直接放弃，不死等
pub fn try_lock_or_timeout<'a, T>(
    mutex: &'a Mutex<T>,
    name: &'a str,
    timeout: Duration,
) -> Option<MutexGuard<'a, T>> {
    let start = Instant::now();
    
    // 先尝试快速获取
    if let Some(guard) = mutex.try_lock() {
        return Some(guard);
    }
    
    // 如果快速获取失败，等待一段时间（但不超过超时时间）
    while start.elapsed() < timeout {
        if let Some(guard) = mutex.try_lock() {
            return Some(guard);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    
    log::warn!("[Lock Timeout] {} 锁获取超时（{}ms），放弃获取", name, timeout.as_millis());
    None
}

/// 兼容旧代码的锁获取（带恢复机制）
/// 
/// ⚠️ 注意：这是为了兼容现有代码，新代码应该使用 try_lock_or_timeout
pub fn lock_or_recover<'a, T: ?Sized>(mutex: &'a Mutex<T>, _name: &str) -> MutexGuard<'a, T> {
    // parking_lot::Mutex 不会 panic，所以这里直接 lock
    mutex.lock()
}

/// 快速尝试获取锁，失败立即返回
pub fn try_lock_or_skip<'a, T>(mutex: &'a Mutex<T>, name: &'a str) -> Option<MutexGuard<'a, T>> {
    if let Some(guard) = mutex.try_lock() {
        Some(guard)
    } else {
        log::debug!("[Lock Skip] {} 锁被占用，跳过操作", name);
        None
    }
}

