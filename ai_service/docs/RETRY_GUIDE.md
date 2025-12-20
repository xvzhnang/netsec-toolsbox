# 错误重试机制使用指南

## 概述

AI Gateway 实现了智能错误重试机制，自动处理临时性错误（如网络错误、速率限制、服务器错误等），提高系统的可靠性和稳定性。

## 特性

### 1. 智能错误分类

系统会自动判断错误是否可重试：

**可重试错误**：
- 网络错误（连接失败、超时、DNS 解析失败等）
- 速率限制（429 Too Many Requests）
- 临时服务器错误（5xx）
- 超时错误

**不可重试错误**：
- 认证错误（401 Unauthorized, 403 Forbidden）
- 请求错误（400 Bad Request, 422 Validation Error）
- 资源不存在（404 Not Found, Model Not Found）

### 2. 指数退避策略

重试延迟使用指数退避算法，避免对服务器造成过大压力：

- **初始延迟**：1 秒
- **指数基数**：2（每次重试延迟翻倍）
- **最大延迟**：60 秒（防止延迟过长）
- **随机抖动**：±25%（避免大量请求同时重试）

**重试延迟示例**：
- 第 1 次重试：1 秒 ± 25%
- 第 2 次重试：2 秒 ± 25%
- 第 3 次重试：4 秒 ± 25%
- 第 4 次重试：8 秒 ± 25%（上限 60 秒）

### 3. 可配置重试参数

每个模型可以独立配置重试策略。

## 配置方式

### 模型级别配置

在 `models.json` 中为每个模型配置重试参数：

```json
{
  "id": "openai-gpt-3.5-turbo",
  "adapter": "openai_compat",
  "base_url": "https://api.openai.com/v1",
  "api_key": "ENV:OPENAI_API_KEY",
  "enabled": true,
  "model": "gpt-3.5-turbo",
  "retry": {
    "enabled": true,
    "max_retries": 3,
    "initial_delay": 1.0,
    "max_delay": 60.0,
    "exponential_base": 2.0,
    "jitter": true
  }
}
```

### 配置参数说明

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | boolean | `true` | 是否启用重试 |
| `max_retries` | integer | `3` | 最大重试次数（不包括首次请求） |
| `initial_delay` | float | `1.0` | 初始延迟（秒） |
| `max_delay` | float | `60.0` | 最大延迟（秒） |
| `exponential_base` | float | `2.0` | 指数退避基数 |
| `jitter` | boolean | `true` | 是否添加随机抖动 |

### 禁用重试

如果某个模型不需要重试，可以禁用：

```json
{
  "id": "local-model",
  "adapter": "process",
  "command": "llama-cli",
  "retry": {
    "enabled": false
  }
}
```

## 使用示例

### 默认配置

如果不配置 `retry` 字段，使用默认配置（启用，最多重试 3 次）。

### 自定义配置

```json
{
  "id": "unstable-api",
  "adapter": "openai_compat",
  "base_url": "https://unstable-api.com/v1",
  "api_key": "ENV:API_KEY",
  "retry": {
    "enabled": true,
    "max_retries": 5,
    "initial_delay": 2.0,
    "max_delay": 120.0,
    "exponential_base": 1.5,
    "jitter": true
  }
}
```

### 快速重试（适合本地服务）

```json
{
  "id": "local-ollama",
  "adapter": "openai_compat",
  "base_url": "http://localhost:11434/v1",
  "retry": {
    "enabled": true,
    "max_retries": 2,
    "initial_delay": 0.5,
    "max_delay": 5.0,
    "exponential_base": 2.0,
    "jitter": false
  }
}
```

### 慢速重试（适合速率限制严格的服务）

```json
{
  "id": "rate-limited-api",
  "adapter": "openai_compat",
  "base_url": "https://api.example.com/v1",
  "api_key": "ENV:API_KEY",
  "retry": {
    "enabled": true,
    "max_retries": 5,
    "initial_delay": 5.0,
    "max_delay": 300.0,
    "exponential_base": 2.0,
    "jitter": true
  }
}
```

## 重试日志

当发生重试时，系统会输出日志：

```
⚠️ 模型 openai-gpt-3.5-turbo 请求失败，1.23秒后重试（第 1 次）: Connection timeout
⚠️ 模型 openai-gpt-3.5-turbo 请求失败，2.45秒后重试（第 2 次）: Connection timeout
```

## 流式响应

**注意**：流式响应（`stream: true`）**暂不支持重试**，因为流式响应的重试逻辑复杂：

- 流式响应已经开始传输数据
- 重试需要重新建立连接
- 可能导致部分数据丢失

如果需要在流式响应中处理错误，建议：

1. 在客户端实现重试逻辑
2. 使用非流式请求（`stream: false`）
3. 在客户端缓存已接收的数据，然后从失败点继续

## 最佳实践

### 1. 根据服务特性调整配置

- **稳定服务**：`max_retries: 2-3`，`initial_delay: 1.0`
- **不稳定服务**：`max_retries: 5`，`initial_delay: 2.0`
- **本地服务**：`max_retries: 1-2`，`initial_delay: 0.5`

### 2. 速率限制处理

对于有速率限制的 API：
- 增加 `initial_delay`（如 5 秒）
- 增加 `max_delay`（如 300 秒）
- 启用 `jitter`（避免同时重试）

### 3. 超时设置

确保 `timeout` 配置合理，避免长时间等待：

```json
{
  "timeout": 30,
  "retry": {
    "max_retries": 3,
    "max_delay": 30.0
  }
}
```

### 4. 监控和告警

在生产环境中：
- 监控重试日志
- 如果某个模型频繁重试，检查网络或服务状态
- 设置告警阈值（如连续失败超过 10 次）

## 实现细节

### 错误分类逻辑

系统通过以下方式判断错误类型：

1. **错误消息关键词匹配**：检查错误消息中的关键词
2. **异常类型匹配**：检查 Python 异常类型
3. **HTTP 状态码**：如果错误包含 HTTP 状态码，直接判断

### 重试流程

```
1. 首次请求
   ↓ 成功
   返回结果
   ↓ 失败
   判断是否可重试
   ├─ 不可重试 → 立即抛出异常
   └─ 可重试 → 进入重试循环
   
2. 重试循环（最多 max_retries 次）
   ↓
   计算延迟时间（指数退避）
   ↓
   等待延迟
   ↓
   重试请求
   ├─ 成功 → 返回结果
   └─ 失败 → 判断是否可重试
       ├─ 不可重试 → 立即抛出异常
       └─ 可重试 → 继续循环
   
3. 所有重试失败
   ↓
   抛出最后一次的异常
```

## 限制

1. **流式响应**：暂不支持重试
2. **幂等性**：确保请求是幂等的（重试不会造成副作用）
3. **总延迟**：多次重试可能导致总延迟较长
4. **资源消耗**：重试会增加服务器负载

## 故障排查

### 频繁重试

如果某个模型频繁触发重试：

1. 检查网络连接
2. 检查 API Key 是否有效
3. 检查服务状态
4. 查看错误日志确定具体原因

### 重试无效

如果重试没有生效：

1. 检查 `retry.enabled` 是否为 `true`
2. 确认错误类型是否可重试
3. 查看日志确认重试逻辑是否触发

