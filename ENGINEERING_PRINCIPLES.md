# ✅ 工程总原则：避免死锁/无限循环

## 核心 4 条原则

1. **UI 主线程永不阻塞**
2. **锁必须可观测、可超时**
3. **循环必须有"退出条件 + 心跳"**
4. **所有后台任务都必须可取消/可熔断**

---

## 一、UI 主线程"零阻塞"铁律

### ❌ 错误示例

```rust
#[tauri::command]
fn start_service(manager: State<Mutex<ServiceManager>>) {
    let guard = manager.lock().unwrap(); // ❌ 可能阻塞
    guard.start_service("ai-gateway");   // ❌ 等待启动完成
}
```

### ✅ 正确实现

```rust
#[tauri::command]
fn start_service(manager: State<Mutex<ServiceManager>>, id: String) -> Result<String, String> {
    // ✅ 快速检查，立即返回
    let manager_arc = manager.inner();
    let id_clone = id.clone();
    
    // ✅ 在后台线程执行，不阻塞 UI
    std::thread::spawn(move || {
        if let Some(guard) = crate::utils::lock::try_lock_or_timeout(
            &*manager_arc,
            "ServiceManager (start_service)",
            Duration::from_secs(2),
        ) {
            guard.start_service(&id_clone);
        }
    });
    
    Ok(format!("服务 {} 启动中...", id))
}
```

### 原则

- `#[tauri::command]` 只能做三件事：
  1. 参数校验
  2. 发送任务到后台线程
  3. 立刻返回

---

## 二、锁必须可超时

### ✅ 使用 parking_lot::Mutex

```rust
use parking_lot::Mutex;

// ✅ 可超时锁获取
let guard = crate::utils::lock::try_lock_or_timeout(
    &mutex,
    "锁名称",
    Duration::from_millis(100), // 100ms 超时
).ok_or_else(|| "获取锁超时".to_string())?;
```

### 锁使用规范

1. **锁作用域 ≤ 20 行**
2. **拿锁后不允许**：
   - `sleep`
   - `await`
   - IO 操作
   - `channel.recv()`（无 timeout）
3. **超时直接放弃，不死等**

### ✅ 推荐写法

```rust
// ✅ 快速获取数据，立即释放锁
let data = {
    let guard = mutex.lock();
    guard.clone()
}; // 🔓 立刻释放

// 在锁外处理数据
process(data);
```

---

## 三、循环必须有"退出条件 + 心跳"

### ❌ 错误示例

```rust
loop {
    check_status(); // ❌ 没有退出条件，没有 sleep
}
```

### ✅ 正确实现

```rust
// ✅ 使用控制标志
let shutdown = Arc::new(AtomicBool::new(false));
let heartbeat = Arc::new(Heartbeat::new());

loop {
    // ✅ 必须：检查退出条件
    if shutdown.load(Ordering::Relaxed) {
        break;
    }
    
    // ✅ 必须：更新心跳
    heartbeat.ping();
    
    // 业务逻辑
    do_work();
    
    // ✅ 必须：避免 CPU 100%
    thread::sleep(Duration::from_millis(100));
    
    // ✅ 可选：心跳检查（Watchdog）
    if !heartbeat.is_alive(5000) {
        warn!("线程心跳超时，可能卡死");
    }
}
```

### ✅ 更好的实现（channel 驱动）

```rust
loop {
    match rx.recv_timeout(Duration::from_secs(1)) {
        Ok(msg) => handle(msg),
        Err(RecvTimeoutError::Timeout) => {
            heartbeat.ping(); // 心跳
        },
        Err(RecvTimeoutError::Disconnected) => break,
    }
}
```

---

## 四、所有后台任务都必须可取消/可熔断

### ✅ 实现方式

1. **使用 AtomicBool 控制标志**
2. **定期检查标志**
3. **支持优雅关闭**

```rust
pub struct ServiceController {
    shutdown: Arc<AtomicBool>,
    heartbeat: Arc<Heartbeat>,
}

impl ServiceController {
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
    
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
}
```

---

## 五、Watchdog（兜底神器）

### ✅ 实现

每个核心线程都上 watchdog：

```rust
let heartbeat = Arc::new(Heartbeat::new());

// 子线程
loop {
    heartbeat.ping();
    // ... 业务逻辑
}

// 监控线程
if !heartbeat.is_alive(5000) {
    error!("worker stuck, restarting");
}
```

---

## 六、快速自检清单

- [ ] `#[tauri::command]` 里有没有 `loop`
- [ ] 有没有 `.lock()` 后 `await`
- [ ] 有没有 `join()` 在主线程
- [ ] 有没有 `while true`（无退出条件）
- [ ] 有没有两个 `Mutex` 同时 `lock`（锁顺序）
- [ ] 有没有 `channel.recv()` 无 `timeout`
- [ ] 有没有后台线程无法 `stop`

---

## 七、项目中的具体实现

### 1. 可超时锁工具

**位置**: `src-tauri/src/utils/lock.rs`

- `try_lock_or_timeout()`: 可超时锁获取
- `try_lock_or_skip()`: 快速尝试，失败跳过
- `lock_or_recover()`: 兼容旧代码

### 2. 心跳监控工具

**位置**: `src-tauri/src/utils/heartbeat.rs`

- `Heartbeat`: 心跳监控器
- `LoopController`: 带心跳的循环控制

### 3. Tauri Command 修复

**位置**: `src-tauri/src/service/commands.rs`

- 所有 command 都使用可超时锁
- 所有耗时操作都在后台线程执行
- 快速返回，不等待完成

### 4. 监控循环修复

**位置**: 
- `src-tauri/src/service/manager.rs` (ServiceManager 监控)
- `src-tauri/src/ai_service/pool.rs` (健康检查循环)

- 添加心跳机制
- 使用可超时锁检查退出条件
- 定期 sleep，避免 CPU 100%

---

## 八、依赖变更

### 新增依赖

```toml
parking_lot = "0.12"  # 可超时锁
```

### 替换的依赖

- `std::sync::Mutex` → `parking_lot::Mutex`
- 所有锁获取都使用 `try_lock_or_timeout()`

---

## 九、测试建议

1. **压力测试**：快速连续调用 command，确保 UI 不卡顿
2. **死锁测试**：多线程同时访问，确保不会死锁
3. **心跳测试**：模拟线程卡死，验证 Watchdog 能检测
4. **退出测试**：验证所有后台线程都能正常退出

---

## 十、后续优化方向

1. **Channel 驱动**：将更多循环改为 channel 驱动
2. **状态机**：为所有服务设计明确的状态机
3. **监控面板**：可视化显示所有线程的心跳状态
4. **自动恢复**：Watchdog 检测到卡死后自动重启

