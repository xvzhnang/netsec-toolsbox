# 统一服务管理架构优化总结

## 一、已实现的优化

### 1. 可配置的状态迁移规则 ✅

**文件**: `src-tauri/src/service/state_transition.rs`

- 实现了 `StateTransitionConfig`，支持从配置文件加载状态迁移规则
- 默认配置保持向后兼容
- 支持通配符规则（`*` → `stopped`，紧急停止）
- 提供 `SimpleState` 两级状态（`Active` / `Inactive` / `Error`）用于轻量级场景

**使用示例**:
```rust
let config = StateTransitionConfig::default();
if config.can_transit(ServiceState::Idle, ServiceState::Busy) {
    // 允许转换
}
```

### 2. 事件驱动架构 ✅

**文件**: `src-tauri/src/service/events.rs`

- 定义了 `ServiceEvent` 枚举（状态变化、健康检查、错误等）
- 实现了 `EventBus` 和 `EventListener` trait
- 支持事件订阅和发布
- 替代轮询式健康检查，响应更快

**事件类型**:
- `StateChanged`: 状态变化
- `HealthCheck`: 健康检查结果
- `Error`: 错误事件
- `Started` / `Stopped` / `Restarted`: 生命周期事件

### 3. 健康检查分级系统 ✅

**文件**: `src-tauri/src/service/health_check.rs`

- 实现了 4 级健康检查：
  - `ProcessCheck`: 进程检查（< 1ms）
  - `TcpCheck`: TCP 连接检查（< 10ms）
  - `HttpCheck`: HTTP 健康检查（< 100ms）
  - `FullCheck`: 完整功能检查（< 1000ms）

- 根据服务状态自动选择检查级别
- 支持自定义策略（轻量级、标准、深度）

**策略示例**:
```rust
let strategy = HealthCheckStrategy::lightweight(); // 高频轻量检查
let strategy = HealthCheckStrategy::standard();    // 标准检查
let strategy = HealthCheckStrategy::deep();        // 深度检查
```

### 4. 生命周期钩子系统 ✅

**文件**: `src-tauri/src/service/lifecycle.rs`

- 定义了 `LifecycleHooks` trait
- 支持钩子：
  - `on_before_start` / `on_after_start`
  - `on_before_stop` / `on_after_stop`
  - `on_error`
  - `on_before_restart` / `on_after_restart`
  - `on_before_health_check` / `on_after_health_check`

- 插件/新工具只需实现钩子即可

### 5. 服务分组和优先级 ✅

**更新**: `src-tauri/src/service/trait_def.rs`

- `Service` trait 新增方法：
  - `priority()`: 返回服务优先级（0-100）
  - `group()`: 返回服务分组（如 "core", "optional"）

- `GatewayPoolService` 已实现：
  - 优先级: 80（高优先级）
  - 分组: "core"（核心服务）

### 6. 状态扩展属性 ✅

**更新**: `src-tauri/src/service/dto.rs`

- `ServiceStatusDTO` 新增字段：
  - `progress: Option<u8>`: 进度（0-100）
  - `eta_seconds: Option<u64>`: 预计剩余时间（秒）

- 支持显示进度条、剩余时间等

## 二、待实现的优化

### 1. ServiceManager 集成事件总线

**计划**:
- 在 `ServiceManager` 中集成 `EventBus`
- 状态变化时自动发送事件
- 健康检查结果通过事件通知

### 2. 统一熔断与限流策略

**计划**:
- 在 `ServiceManager` 层统一实现熔断器
- 连续失败 N 次 → 熔断
- 熔断期间降级或使用备用 Worker

### 3. 服务分组管理

**计划**:
- 根据服务分组（core/optional）采用不同调度策略
- 核心服务优先启动和恢复
- 可选工具延迟启动

### 4. 统一日志与告警

**计划**:
- ServiceManager 统一采集日志
- 状态变化 → 前端更新
- 异常 → 告警/自动恢复

### 5. 前端事件推送

**计划**:
- WebSocket / SSE 推送状态变化
- 前端无需轮询
- 实时响应

### 6. 监控与指标

**计划**:
- 收集 Worker 状态、异常次数、响应时间
- ServiceManager 调度效率
- 接入 Prometheus / Grafana

## 三、架构优势

1. **可配置性**: 状态迁移规则可配置，无需改代码
2. **事件驱动**: 替代轮询，响应更快，资源消耗更低
3. **分级检查**: 根据场景选择合适检查级别，平衡性能和准确性
4. **生命周期钩子**: 插件化设计，易于扩展
5. **分组管理**: 核心服务和可选工具分别管理
6. **扩展属性**: 支持进度、ETA 等丰富信息

## 四、使用建议

### 对于新服务

1. 实现 `Service` trait
2. 可选实现 `LifecycleHooks` trait
3. 设置合适的 `priority()` 和 `group()`
4. 在状态变化时通过事件总线发送事件

### 对于配置

1. 创建 `state_transitions.toml` 定义状态迁移规则
2. 根据服务类型选择健康检查策略
3. 配置服务分组和优先级

### 对于前端

1. 使用统一的 `ServiceStatusDTO` 格式
2. 支持 `progress` 和 `eta_seconds` 显示进度
3. 通过 WebSocket 接收事件推送（待实现）

## 五、下一步计划

1. ✅ 可配置状态迁移规则
2. ✅ 事件驱动架构基础
3. ✅ 健康检查分级
4. ✅ 生命周期钩子
5. ✅ 服务分组和优先级
6. ✅ 状态扩展属性
7. ⏳ ServiceManager 集成事件总线
8. ⏳ 统一熔断与限流
9. ⏳ 前端事件推送
10. ⏳ 监控与指标

