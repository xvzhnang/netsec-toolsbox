# Custom HTTP 适配器实现说明

## 概述

Custom HTTP 适配器支持所有需要协议转换的 AI 模型，基于 One API 的实现，将非 OpenAI 兼容的 API 转换为 OpenAI 兼容格式。

## 架构设计

```
CustomHTTPAdapter
    ↓
ProtocolConverter (协议转换器)
    ├── AnthropicConverter (Claude)
    ├── GeminiConverter (Google Gemini)
    ├── ZhipuConverter (智谱 AI)
    ├── BaiduConverter (百度文心一言)
    └── AliConverter (阿里通义千问)
```

## 目录结构

```
ai_service/core/adapter/
├── custom_http_adapter.py      # Custom HTTP 适配器主类
└── converters/
    ├── __init__.py
    ├── base_converter.py        # 协议转换器基类
    ├── anthropic_converter.py   # Anthropic Claude 转换器
    ├── gemini_converter.py      # Google Gemini 转换器
    ├── zhipu_converter.py       # 智谱 AI 转换器
    ├── baidu_converter.py       # 百度文心一言转换器
    ├── ali_converter.py         # 阿里通义千问转换器
    └── registry.py              # 转换器注册表
```

## 已实现的转换器

### 1. Anthropic (Claude)

**对应 One API**: `relay/adaptor/anthropic/`

**特性**:
- 支持 system message 单独处理
- 支持 tool use 转换
- 支持流式响应（待实现）
- 特殊模型支持（claude-3-5-sonnet）

**请求头**:
- `x-api-key`: API Key
- `anthropic-version`: API 版本
- `anthropic-beta`: Beta 特性标识

**配置示例**:
```json
{
  "id": "claude-3-5-sonnet",
  "adapter": "custom_http",
  "base_url": "https://api.anthropic.com",
  "api_key": "ENV:ANTHROPIC_API_KEY",
  "model": "claude-3-5-sonnet-20241022",
  "endpoint": "/v1/messages",
  "request_format": "anthropic",
  "response_format": "openai"
}
```

### 2. Google Gemini

**对应 One API**: `relay/adaptor/gemini/`

**特性**:
- 支持 system instruction（部分模型）
- assistant role → model role 转换
- 支持多模态（图片）
- 自动添加 dummy model message（旧模型）

**请求头**:
- `x-goog-api-key`: API Key

**URL 格式**:
- 标准: `/{version}/models/{model}:generateContent`
- 流式: `/{version}/models/{model}:streamGenerateContent?alt=sse`

**配置示例**:
```json
{
  "id": "gemini-pro",
  "adapter": "custom_http",
  "base_url": "https://generativelanguage.googleapis.com",
  "api_key": "ENV:GEMINI_API_KEY",
  "model": "gemini-pro",
  "endpoint": "/v1beta/models/{model}:generateContent",
  "request_format": "gemini",
  "response_format": "openai"
}
```

### 3. 智谱 AI (Zhipu)

**对应 One API**: `relay/adaptor/zhipu/`

**特性**:
- JWT Token 生成和缓存
- 简单的消息格式转换
- 支持 v3 和 v4 API

**请求头**:
- `Authorization`: JWT Token（自动生成）

**Token 生成**:
- 使用 `PyJWT` 库生成 JWT Token
- Token 有效期 24 小时，自动缓存

**配置示例**:
```json
{
  "id": "zhipu-glm-4",
  "adapter": "custom_http",
  "base_url": "https://open.bigmodel.cn/api/paas/v4",
  "api_key": "ENV:ZHIPU_API_KEY",
  "model": "glm-4",
  "endpoint": "/chat/completions",
  "request_format": "zhipu",
  "response_format": "openai"
}
```

### 4. 百度文心一言

**对应 One API**: `relay/adaptor/baidu/`

**特性**:
- Access Token 获取和缓存
- 异步获取 Access Token（需要先获取 token 才能构建 URL）
- 支持多种模型变体

**请求头**:
- `Content-Type`: application/json
- Access Token 通过 URL 参数传递

**Access Token 获取**:
- 使用 `client_id|client_secret` 格式的 API Key
- 从 `https://aip.baidubce.com/oauth/2.0/token` 获取
- Token 自动缓存，提前 1 小时刷新

**配置示例**:
```json
{
  "id": "baidu-ernie-4.0",
  "adapter": "custom_http",
  "base_url": "https://aip.baidubce.com",
  "api_key": "ENV:BAIDU_API_KEY",
  "model": "ERNIE-4.0-8K",
  "endpoint": "/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/completions",
  "request_format": "baidu",
  "response_format": "openai"
}
```

### 5. 阿里通义千问

**对应 One API**: `relay/adaptor/ali/`

**特性**:
- 支持搜索功能（模型名以 `-internet` 结尾）
- 支持流式响应（SSE）
- 支持插件（通过 header）

**请求头**:
- `Authorization`: Bearer Token
- `X-DashScope-SSE`: enable（流式请求）
- `X-DashScope-Plugin`: 插件名称（可选）

**配置示例**:
```json
{
  "id": "ali-qwen-max",
  "adapter": "custom_http",
  "base_url": "https://dashscope.aliyuncs.com",
  "api_key": "ENV:ALI_API_KEY",
  "model": "qwen-max",
  "endpoint": "/api/v1/services/aigc/text-generation/generation",
  "request_format": "ali",
  "response_format": "openai",
  "config": {
    "plugin": "ENV:ALI_PLUGIN"
  }
}
```

## 使用流程

1. **配置模型**: 在 `config/models.json` 中添加模型配置，设置 `adapter: "custom_http"` 和 `request_format`
2. **设置环境变量**: 配置对应的 API Key 环境变量
3. **启用模型**: 将模型的 `enabled` 设置为 `true`
4. **调用**: 通过 OpenAI-compatible API 调用，Gateway 自动进行协议转换

## 扩展新的转换器

要添加新的协议转换器：

1. **创建转换器类**: 继承 `ProtocolConverter`
2. **实现必要方法**:
   - `convert_request()`: 请求转换
   - `convert_response()`: 响应转换
   - `get_request_headers()`: 请求头
   - `get_request_url()`: URL 构建（可选）
3. **注册转换器**: 在 `converters/registry.py` 中添加映射

示例：

```python
# converters/my_converter.py
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse

class MyConverter(ProtocolConverter):
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        # 实现请求转换
        pass
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        # 实现响应转换
        pass
    
    def get_request_headers(self) -> Dict[str, str]:
        # 返回请求头
        pass

# converters/registry.py
from .my_converter import MyConverter

def get_converter(request_format: str, config: Dict[str, Any]):
    converters = {
        # ...
        "my_format": MyConverter,
    }
    # ...
```

## 待实现功能

- [ ] 流式响应支持（SSE）
- [ ] 更多转换器（腾讯、Moonshot、Minimax 等）
- [ ] WebSocket 支持（讯飞星火）
- [ ] 错误重试机制
- [ ] 请求/响应日志

## 依赖

- `aiohttp`: 异步 HTTP 客户端
- `PyJWT`: JWT Token 生成（Zhipu）

## 参考

- [One API 项目](https://github.com/songquanpeng/one-api)
- [模型配置指南](./MODELS_CONFIG_GUIDE.md)
- [架构分析文档](../ONE_API_ARCHITECTURE_ANALYSIS.md)

