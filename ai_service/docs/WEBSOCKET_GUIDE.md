# WebSocket 支持使用指南

## 概述

AI Gateway 支持 WebSocket 协议的模型（如讯飞星火），通过 WebSocket 连接实现实时流式通信。

## 支持情况

### 已实现的 WebSocket 适配器

- ✅ **讯飞星火 (Xunfei Spark)**：完全支持

### 适配器类型

- `websocket`：通用 WebSocket 适配器类型
- `websocket_xunfei`：讯飞星火专用适配器（自动识别）

## 讯飞星火配置

### 基本配置

```json
{
  "id": "xunfei-spark-max",
  "adapter": "websocket",
  "base_url": "wss://spark-api.xf-yun.com",
  "api_key": "ENV:XUNFEI_API_KEY",
  "enabled": true,
  "model": "Spark-Max",
  "endpoint": "/v3.5/chat",
  "request_format": "xunfei",
  "response_format": "openai",
  "temperature": 0.7,
  "max_tokens": 2000,
  "timeout": 60,
  "config": {
    "app_id": "ENV:XUNFEI_APP_ID",
    "api_secret": "ENV:XUNFEI_API_SECRET",
    "api_version": "v3.5",
    "domain": "generalv3.5"
  }
}
```

### 配置字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | ✅ | 模型唯一标识 |
| `adapter` | string | ✅ | 必须为 `"websocket"` |
| `base_url` | string | ✅ | WebSocket URL（wss:// 或 ws://） |
| `api_key` | string | ✅ | API Key（支持 `ENV:VAR_NAME`） |
| `endpoint` | string | ✅ | WebSocket 路径 |
| `request_format` | string | ✅ | 请求格式（`"xunfei"`） |
| `config.app_id` | string | ✅ | 应用 ID（支持环境变量） |
| `config.api_secret` | string | ✅ | API Secret（支持环境变量） |
| `config.api_version` | string | ❌ | API 版本（v1.1, v2.1, v3.1, v3.5, v4.0），默认 v3.5 |
| `config.domain` | string | ❌ | 模型域名，默认根据 api_version 推断 |

### API 版本和域名映射

| API 版本 | 默认域名 | 路径 |
|---------|---------|------|
| v1.1 | general | /v1.1/chat |
| v2.1 | generalv2 | /v2.1/chat |
| v3.1 | generalv3 | /v3.1/chat |
| v3.5 | generalv3.5 | /v3.5/chat |
| v4.0 | 4.0Ultra | /v4.0/chat |

### 示例配置

#### 讯飞星火 v3.5（推荐）

```json
{
  "id": "xunfei-spark-3.5",
  "adapter": "websocket",
  "base_url": "wss://spark-api.xf-yun.com",
  "api_key": "ENV:XUNFEI_API_KEY",
  "enabled": true,
  "model": "Spark-3.5",
  "endpoint": "/v3.5/chat",
  "request_format": "xunfei",
  "response_format": "openai",
  "config": {
    "app_id": "ENV:XUNFEI_APP_ID",
    "api_secret": "ENV:XUNFEI_API_SECRET",
    "api_version": "v3.5"
  }
}
```

#### 讯飞星火 v4.0 Ultra

```json
{
  "id": "xunfei-spark-ultra",
  "adapter": "websocket",
  "base_url": "wss://spark-api.xf-yun.com",
  "api_key": "ENV:XUNFEI_API_KEY",
  "enabled": true,
  "model": "Spark-Ultra",
  "endpoint": "/v4.0/chat",
  "request_format": "xunfei",
  "response_format": "openai",
  "config": {
    "app_id": "ENV:XUNFEI_APP_ID",
    "api_secret": "ENV:XUNFEI_API_SECRET",
    "api_version": "v4.0",
    "domain": "4.0Ultra"
  }
}
```

## 认证机制

讯飞星火使用 HMAC-SHA256 签名认证：

1. 生成当前 UTC 时间（RFC1123 格式）
2. 构建签名字符串：`host: {host}\ndate: {date}\nGET {path} HTTP/1.1`
3. 使用 API Secret 计算 HMAC-SHA256 签名
4. 将签名 Base64 编码
5. 构建 authorization header
6. 将 authorization 和参数添加到 WebSocket URL 查询字符串

认证 URL 会自动生成，无需手动配置。

## 协议转换

### 请求转换

OpenAI 格式 → 讯飞星火格式：

```json
{
  "header": {
    "app_id": "your-app-id"
  },
  "parameter": {
    "chat": {
      "domain": "generalv3.5",
      "temperature": 0.7,
      "max_tokens": 2048
    }
  },
  "payload": {
    "message": {
      "text": [
        {"role": "user", "content": "Hello"}
      ]
    }
  }
}
```

### 响应转换

讯飞星火格式 → OpenAI 格式：

- 流式响应自动转换为 SSE 格式
- 自动累积 usage 信息
- 自动处理完成状态

## 使用方式

### HTTP API（自动转换）

WebSocket 适配器通过 HTTP API 暴露，前端无需直接处理 WebSocket：

```bash
curl -X POST http://localhost:8765/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "xunfei-spark-max",
    "messages": [
      {"role": "user", "content": "Hello"}
    ],
    "stream": true
  }'
```

### 流式响应

WebSocket 适配器天然支持流式响应，设置 `stream: true` 即可：

```json
{
  "model": "xunfei-spark-max",
  "messages": [{"role": "user", "content": "Hello"}],
  "stream": true
}
```

响应格式与标准 SSE 流式响应相同。

## 依赖安装

确保已安装 `websockets` 库：

```bash
pip install websockets
```

或使用 requirements.txt：

```bash
pip install -r requirements.txt
```

## 错误处理

### 常见错误

1. **连接失败**
   - 检查 API Key、API Secret、App ID 是否正确
   - 检查网络连接
   - 检查 WebSocket URL 是否正确

2. **认证失败**
   - 检查 API Key 和 API Secret 是否正确
   - 检查时间同步（UTC 时间）

3. **域名错误**
   - 检查 `domain` 配置是否与 API 版本匹配
   - 检查 `endpoint` 路径是否正确

### 错误响应格式

错误会以 OpenAI 格式返回：

```json
{
  "error": {
    "message": "讯飞星火 API 错误 (10013): 认证失败",
    "type": "server_error",
    "code": "500"
  }
}
```

## 限制

1. **连接管理**：每次请求建立新的 WebSocket 连接
2. **并发**：多个请求会建立多个 WebSocket 连接
3. **超时**：默认超时 60 秒，可在配置中修改
4. **重试**：WebSocket 连接失败会触发重试机制

## 扩展其他 WebSocket 模型

如果需要支持其他 WebSocket 模型，可以：

1. 继承 `WebSocketAdapter` 基类
2. 实现以下方法：
   - `_build_auth_url()`: 构建认证 URL
   - `_convert_request()`: 转换请求格式
   - `_parse_response()`: 解析响应格式
   - `_is_response_complete()`: 判断响应完成

3. 在 `registry.py` 中注册新适配器

## 最佳实践

1. **环境变量**：使用环境变量存储敏感信息（API Key、Secret）
2. **版本选择**：根据需求选择合适的 API 版本
3. **超时设置**：根据模型响应时间设置合理的超时
4. **错误处理**：在客户端实现错误重试逻辑
5. **连接复用**：虽然当前实现每次请求新建连接，但可以优化为连接池

