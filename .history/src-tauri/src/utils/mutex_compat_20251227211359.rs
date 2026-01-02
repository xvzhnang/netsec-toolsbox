use parking_lot::Mutex as ParkingMutex;
use parking_lot::MutexGuard as ParkingMutexGuard;
/// ✅ 工程原则：兼容 std::sync::Mutex 和 parking_lot::Mutex
/// 提供统一的锁接口，逐步迁移到 parking_lot

/// 兼容 parking_lot::Mutex 的锁获取
pub fn lock_or_recover_parking<'a, T: ?Sized>(
    mutex: &'a ParkingMutex<T>,
    _name: &str,
) -> ParkingMutexGuard<'a, T> {
    // parking_lot::Mutex 不会 panic，所以这里直接 lock
    mutex.lock()
}
