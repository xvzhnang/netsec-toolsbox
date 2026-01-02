/// ✅ 工程原则：兼容 std::sync::Mutex 和 parking_lot::Mutex
/// 提供统一的锁接口，逐步迁移到 parking_lot
use std::sync::Mutex as StdMutex;
use std::sync::MutexGuard as StdMutexGuard;
use parking_lot::Mutex as ParkingMutex;
use parking_lot::MutexGuard as ParkingMutexGuard;

/// 兼容 std::sync::Mutex 的锁获取（用于旧代码）
pub fn lock_or_recover_std<'a, T: ?Sized>(
    mutex: &'a StdMutex<T>,
    name: &str,
) -> StdMutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::error!("{} Mutex 被污染，尝试恢复", name);
            poisoned.into_inner()
        }
    }
}

/// 兼容 parking_lot::Mutex 的锁获取
pub fn lock_or_recover_parking<'a, T: ?Sized>(
    mutex: &'a ParkingMutex<T>,
    _name: &str,
) -> ParkingMutexGuard<'a, T> {
    // parking_lot::Mutex 不会 panic，所以这里直接 lock
    mutex.lock()
}
