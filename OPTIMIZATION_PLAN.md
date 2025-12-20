# AI Gateway 高并发架构优化方案

## 核心问题分析

1. **Worker 重复启动**：`start_all()` 被多次调用，缺少初始化保护
2. **健康检查频率过高**：每15秒 HTTP 调用，阻塞且可能误触发熔断器
3. **/v1/models 阻塞**：10秒超时，Worker 自调用导致死锁
4. **内存持续上涨**：任务堆积、事件循环未清理、资源泄漏

## 优化方案

### 1. 一次性初始化的 GatewayPoolService

**方案**：使用 `once_cell::sync::OnceCell` 或 `std::sync::OnceLock`（Rust 1.70+）

```rust
use once_cell::sync::OnceCell;

static GATEWAY_POOL: OnceCell<Arc<Mutex<GatewayPool>>> = OnceCell::new();

impl GatewayPoolService {
    fn get_or_init_pool() -> Arc<Mutex<GatewayPool>> {
        GATEWAY_POOL.get_or_init(|| {
            let pool = GatewayPool::new(3, 8765);
            Arc::new(Mutex::new(pool))
        }).clone()
    }
}
```

### 2. Worker 生命周期与状态机

**状态转换流程**：
```
INIT -> READY -> IDLE -> (BUSY_STREAMING | BUSY_BLOCKED) -> (DEGRADED | UNHEALTHY) -> DEAD
         ↓         ↓
      (失败)   (健康检查失败)
         ↓         ↓
       DEAD    UNHEALTHY -> (Restart) -> INIT
```

**状态说明**：
- `INIT`: 进程启动中，等待 HTTP 服务就绪
- `READY`: 进程已启动，HTTP 服务可用（通过进程状态确认，无需 HTTP 调用）
- `IDLE`: 空闲，可接受请求
- `BUSY_STREAMING`: 正在处理流式请求
- `BUSY_BLOCKED`: 请求阻塞（超时检测）
- `DEGRADED`: 性能下降，但仍可用
- `UNHEALTHY`: 健康检查失败，需要重启
- `RESTARTING`: 正在重启
- `DEAD`: 进程死亡，永不复用

### 3. 无 HTTP 的健康检查方案

**方案**：进程状态检查 + 心跳文件机制

```rust
// 1. 进程状态检查（无阻塞）
fn check_process_health(worker: &GatewayWorker) -> bool {
    if let Some(ref child) = worker.process {
        match child.try_wait() {
            Ok(Some(_)) => false, // 进程已退出
            Ok(None) => true,     // 进程运行中
            Err(_) => false,
        }
    } else {
        false
    }
}

// 2. 心跳文件机制（Worker 定期写入心跳文件）
// Worker 端：每5秒写入心跳文件
// Pool 端：检查心跳文件时间戳，超过10秒认为不健康
```

### 4. 限频 + 缓存的模型列表策略

**方案**：
- 缓存时间：5分钟
- 限频间隔：30秒（避免频繁请求）
- 使用共享缓存，所有 Worker 共享同一份模型列表

```rust
struct ModelListCache {
    cached: Option<(Vec<String>, Instant)>,
    cache_ttl: Duration,           // 5分钟
    min_request_interval: Duration, // 30秒
    last_request: Option<Instant>,
}

// 获取模型列表（带缓存和限频）
fn get_models_with_cache(&self) -> Result<Vec<String>, String> {
    let mut cache = self.model_cache.lock().unwrap();
    
    // 1. 检查缓存
    if let Some(models) = cache.get_cached() {
        return Ok(models);
    }
    
    // 2. 检查限频
    if !cache.can_request() {
        // 返回过期缓存（如果有）
        return cache.get_cached()
            .ok_or_else(|| "模型列表不可用，请稍后重试".to_string());
    }
    
    // 3. 请求模型列表（只从 READY/IDLE 状态的 Worker 请求）
    // 4. 更新缓存
}
```

### 5. Rust/Tokio 避免自锁与任务堆积

**最佳实践**：

1. **避免长时间持有锁**：
```rust
// ❌ 错误：长时间持有锁
let worker = worker.lock().unwrap();
let result = expensive_operation(); // 阻塞操作
drop(worker);

// ✅ 正确：快速获取数据后释放锁
let (port, state) = {
    let wg = worker.lock().unwrap();
    (wg.port, wg.status())
};
expensive_operation(port, state); // 在锁外执行
```

2. **使用异步非阻塞操作**：
```rust
// 使用 Tokio 异步运行时
use tokio::time::{sleep, Duration};

async fn health_check_async(worker: Arc<Mutex<GatewayWorker>>) -> bool {
    // 快速检查进程状态
    let is_alive = {
        let wg = worker.lock().unwrap();
        wg.process.as_ref().and_then(|c| c.try_wait().ok()).is_none()
    };
    
    if !is_alive {
        return false;
    }
    
    // 异步检查心跳文件（非阻塞）
    tokio::fs::metadata(heartbeat_path)
        .await
        .map(|m| m.modified().unwrap().elapsed().unwrap() < Duration::from_secs(10))
        .unwrap_or(false)
}
```

3. **限制并发任务数**：
```rust
use tokio::sync::Semaphore;

let semaphore = Arc::new(Semaphore::new(10)); // 最多10个并发任务

for worker in workers {
    let permit = semaphore.clone().acquire_owned().await.unwrap();
    tokio::spawn(async move {
        // 执行健康检查
        drop(permit); // 释放许可
    });
}
```

4. **及时清理资源**：
```rust
// 使用 RAII 和 Drop trait
impl Drop for GatewayWorker {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
        }
    }
}
```

## 实施步骤

1. ✅ 更新状态机（添加 READY 状态）
2. ⏳ 实现一次性初始化（OnceCell）
3. ⏳ 实现无 HTTP 健康检查（进程状态 + 心跳文件）
4. ⏳ 优化模型列表缓存（限频 + 共享缓存）
5. ⏳ 重构为 Tokio 异步（避免阻塞）

