# One API 架构分析与轻量级 AI Gateway 设计

> 本文档基于对 `one-api-main` 代码库的深入分析，提取核心架构思想，设计适合本地工具箱的轻量级 AI Gateway。

---

## 第一阶段：阅读与解构 One API

### 1️⃣ 核心模块职责拆解

| 模块路径 | 职责 | 是否保留 | 说明 |
|---------|------|---------|------|
| **OpenAI API Handler** | | | |
| `router/relay.go` | 注册 `/v1/chat/completions` 等 OpenAI 兼容路由 | ✅ **必须保留** | UI 只对接这个接口 |
| `controller/relay.go` | 处理中继请求，调用适配器 | ✅ **必须保留** | 核心请求处理逻辑 |
| `relay/controller/text.go` | 文本请求的具体处理（参数校验、配额、适配器调用） | ⚠️ **简化保留** | 保留核心流程，删除配额逻辑 |
| **模型路由** | | | |
| `middleware/distributor.go` | 根据模型名、用户分组选择 Channel | ✅ **必须保留** | 核心路由逻辑，但需简化（无用户分组） |
| `model/channel.go` | Channel 数据模型（Key、BaseURL、Models 列表） | ✅ **必须保留** | 但简化为配置驱动 |
| `model/ability.go` | Channel 与 Model 的关联关系 | ⚠️ **简化保留** | 改为配置中的 `models` 字段 |
| **Provider 适配** | | | |
| `relay/adaptor/interface.go` | Adaptor 接口定义 | ✅ **必须保留** | 这是核心抽象 |
| `relay/adaptor.go` | 根据 API Type 获取适配器实例 | ✅ **必须保留** | 但改为配置驱动 |
| `relay/adaptor/openai/` | OpenAI 兼容适配器实现 | ✅ **必须保留** | 作为参考实现 |
| `relay/adaptor/anthropic/` | Claude 适配器实现（协议转换） | ✅ **必须保留** | 展示非 OpenAI 协议的适配 |
| `relay/channeltype/define.go` | Channel Type 枚举 | ⚠️ **简化保留** | 改为配置中的 `adapter` 字段 |
| `relay/apitype/define.go` | API Type 枚举 | ⚠️ **简化保留** | 与 adapter 合并 |
| **Key / BaseURL 管理** | | | |
| `model/channel.go` | Channel.Key、Channel.BaseURL | ✅ **必须保留** | 改为配置字段 |
| `middleware/distributor.go` | 从 Channel 提取 Key 设置到请求头 | ✅ **必须保留** | 核心逻辑 |
| **SaaS 特有（删除）** | | | |
| `model/user.go` | 用户系统 | ❌ **删除** | 本地工具箱不需要 |
| `model/token.go` | Token 管理（用户令牌、额度） | ❌ **删除** | 本地工具箱不需要 |
| `controller/auth/` | 登录注册（GitHub、飞书、微信） | ❌ **删除** | 本地工具箱不需要 |
| `relay/billing/` | 计费、配额计算 | ❌ **删除** | 本地工具箱不需要 |
| `model/log.go` | 请求日志审计 | ❌ **删除** | 可选，本地可简化 |
| `controller/group.go` | 用户分组管理 | ❌ **删除** | 本地工具箱不需要 |
| `controller/redemption.go` | 兑换码系统 | ❌ **删除** | 本地工具箱不需要 |
| `monitor/` | 渠道监控、自动禁用 | ⚠️ **可选保留** | 可简化为健康检查 |
| `web/` | Web 管理后台 | ❌ **删除** | 本地工具箱用配置文件 |

### 2️⃣ Provider / Channel 设计思想

#### One API 的抽象层次

```
Channel (数据库记录)
  ├── Type (channeltype.OpenAI / channeltype.Anthropic / ...)
  ├── Key (API Key)
  ├── BaseURL (可选，覆盖默认)
  ├── Models (支持的模型列表，逗号分隔)
  └── Config (JSON，存储额外配置如 APIVersion、Region)
       ↓
APIType (relay/apitype.OpenAI / apitype.Anthropic / ...)
       ↓
Adaptor (relay/adaptor.Adaptor 接口实现)
  ├── Init(meta *meta.Meta)
  ├── GetRequestURL(meta *meta.Meta) (string, error)
  ├── SetupRequestHeader(...)
  ├── ConvertRequest(...) (any, error)
  ├── DoRequest(...) (*http.Response, error)
  └── DoResponse(...) (*model.Usage, *model.ErrorWithStatusCode)
```

#### 关键设计点

1. **Channel Type → API Type → Adaptor 映射**
   - `channeltype.OpenAI` → `apitype.OpenAI` → `openai.Adaptor`
   - `channeltype.Anthropic` → `apitype.Anthropic` → `anthropic.Adaptor`
   - 通过 `channeltype.ToAPIType()` 和 `relay.GetAdaptor()` 完成映射

2. **最小可复用 Provider 抽象**

```go
// One API 的 Adaptor 接口（relay/adaptor/interface.go）
type Adaptor interface {
    Init(meta *meta.Meta)                                    // 初始化，接收 Channel 配置
    GetRequestURL(meta *meta.Meta) (string, error)          // 构建目标 API URL
    SetupRequestHeader(c *gin.Context, req *http.Request, meta *meta.Meta) error  // 设置认证头
    ConvertRequest(c *gin.Context, relayMode int, request *model.GeneralOpenAIRequest) (any, error)  // 协议转换
    ConvertImageRequest(request *model.ImageRequest) (any, error)
    DoRequest(c *gin.Context, meta *meta.Meta, requestBody io.Reader) (*http.Response, error)  // 发送请求
    DoResponse(c *gin.Context, resp *http.Response, meta *meta.Meta) (usage *model.Usage, err *model.ErrorWithStatusCode)  // 响应转换
    GetModelList() []string                                  // 返回支持的模型列表
    GetChannelName() string                                  // 返回适配器名称
}
```

3. **字段分类**

**OpenAI 协议必需字段**（`model.GeneralOpenAIRequest`）：
- `model`: 模型名称
- `messages`: 消息列表
- `stream`: 是否流式
- `temperature`, `max_tokens`, `top_p` 等参数

**One API 扩展字段**（`meta.Meta`）：
- `ChannelId`: Channel ID（用于日志、监控）
- `TokenId`: Token ID（用于配额扣减）
- `UserId`: 用户 ID（用于权限、分组）
- `Group`: 用户分组（用于路由、倍率）
- `ModelMapping`: 模型映射（用户请求的模型 → 实际使用的模型）
- `ForcedSystemPrompt`: 强制系统提示词
- `PromptTokens`: 预计算的 Token 数（用于配额）

**对于本地工具箱，只需要**：
- `model`: 模型名称
- `base_url`: API 基础 URL
- `api_key`: API Key
- `adapter`: 适配器类型（openai_compat / custom_http / process）

### 3️⃣ 请求生命周期（核心流程）

```
HTTP Request (POST /v1/chat/completions)
    ↓
[Middleware: TokenAuth]
    ├── 解析 Authorization: Bearer <token>
    ├── 验证 Token（查询数据库）
    └── 设置 UserId、Group 到 Context
    ↓
[Middleware: Distribute]
    ├── 从请求体提取 model 字段
    ├── 根据 UserGroup + Model 查询可用 Channel
    │   └── CacheGetRandomSatisfiedChannel(group, model, false)
    ├── 设置 Channel 信息到 Context
    │   ├── ChannelId
    │   ├── ChannelType
    │   ├── BaseURL
    │   ├── APIKey (从 Channel.Key)
    │   └── ModelMapping
    ↓
[Controller: Relay]
    ├── 调用 relayHelper(c, relayMode)
    └── 如果是文本请求 → RelayTextHelper(c)
    ↓
[RelayTextHelper]
    ├── 解析请求体 → GeneralOpenAIRequest
    ├── 模型映射：OriginModelName → ActualModelName
    ├── 预扣配额（preConsumeQuota）
    ├── 获取 Adaptor：relay.GetAdaptor(meta.APIType)
    ├── 初始化：adaptor.Init(meta)
    ├── 转换请求：adaptor.ConvertRequest(...)
    ├── 发送请求：adaptor.DoRequest(...)
    ├── 处理响应：adaptor.DoResponse(...)
    └── 后扣配额（postConsumeQuota）
    ↓
[Adaptor.DoRequest]
    ├── GetRequestURL(meta) → 构建完整 URL
    ├── SetupRequestHeader(...) → 设置认证头
    ├── 发送 HTTP 请求到目标 API
    └── 返回 *http.Response
    ↓
[Adaptor.DoResponse]
    ├── 如果是 Stream → StreamHandler
    │   ├── 逐行解析 SSE 流
    │   ├── 转换为 OpenAI 格式
    │   └── 流式返回给客户端
    └── 如果是非 Stream → Handler
        ├── 解析 JSON 响应
        ├── 转换为 OpenAI 格式
        └── 返回 Usage 信息
    ↓
HTTP Response (OpenAI 兼容格式)
```

**对于本地工具箱，必须保留的步骤**：
1. ✅ HTTP Request 接收
2. ✅ 参数校验（model、messages）
3. ✅ 模型解析（从请求中提取 model）
4. ✅ Provider 选择（根据 model 从配置中查找）
5. ✅ Key / Endpoint 选择（从配置中读取）
6. ✅ 请求转发（调用 Adaptor）
7. ✅ 响应标准化（转换为 OpenAI 格式）

**可以删除的步骤**：
- ❌ Token 验证（本地工具箱不需要）
- ❌ 配额预扣/后扣（本地工具箱不需要）
- ❌ 用户分组路由（本地工具箱不需要）
- ❌ 模型映射（可选，可简化）
- ❌ 强制系统提示词（可选）

---

## 第二阶段：迁移到本地工具箱

### 1️⃣ 必须删掉的内容

| 功能 | One API 位置 | 删除原因 |
|------|-------------|---------|
| 用户系统 | `model/user.go`, `controller/user.go` | 本地工具箱单用户，不需要 |
| Token 管理 | `model/token.go`, `controller/token.go` | 本地工具箱不需要 API Key 分发 |
| 配额/计费 | `relay/billing/`, `controller/billing.go` | 本地工具箱不需要额度管理 |
| 日志审计 | `model/log.go`, `controller/log.go` | 可选，可简化为控制台日志 |
| Web 管理后台 | `web/`, `router/dashboard.go` | 本地工具箱用配置文件 |
| 多租户 | `model/group.go`, `controller/group.go` | 本地工具箱不需要 |
| 兑换码 | `controller/redemption.go` | 本地工具箱不需要 |

### 2️⃣ 必须保留的思想

1. **统一 OpenAI-compatible 接口**
   - UI 永远只调用 `POST /v1/chat/completions`
   - UI 永远只接收 OpenAI 格式的响应
   - 所有协议差异在 Gateway 内部处理

2. **Provider / Adapter 解耦**
   - 每个模型厂商实现一个 Adapter
   - Adapter 负责：请求转换、响应转换、错误处理
   - 新增模型 = 新增 Adapter 实现，不改核心代码

3. **配置驱动模型注册**
   - 模型信息存储在 `models.json`
   - 启动时加载配置，注册所有模型
   - 运行时根据 `model` 字段查找对应的 Adapter

4. **模型 → Provider → Endpoint 映射**
   - 请求中的 `model` 字段 → 查找配置 → 获取 `adapter` + `base_url` + `api_key`
   - 通过 `adapter` 类型获取 Adapter 实例
   - Adapter 使用 `base_url` 和 `api_key` 发送请求

### 3️⃣ 简化架构设计

```
ai_gateway/
├── api/
│   └── openai_handler.rs / openai_handler.py
│       └── 职责：接收 HTTP 请求，解析 OpenAI 格式，调用 Router
├── core/
│   ├── adapter/
│   │   ├── trait.rs / base_adapter.py
│   │   │   └── 职责：定义 Adapter 接口（类似 One API 的 Adaptor）
│   │   ├── openai_compat.rs / openai_compat_adapter.py
│   │   │   └── 职责：OpenAI 兼容协议的直通适配器
│   │   ├── custom_http.rs / custom_http_adapter.py
│   │   │   └── 职责：自定义 HTTP 协议的适配器（需要请求/响应转换）
│   │   └── process.rs / process_adapter.py
│   │       └── 职责：本地 CLI 工具的适配器（stdin/stdout）
│   ├── registry.rs / registry.py
│   │   └── 职责：从 models.json 加载配置，管理 Adapter 实例
│   └── router.rs / router.py
│       └── 职责：根据 model 字段查找对应的 Adapter
└── config/
    └── models.json
        └── 职责：模型配置（替代 One API 的 Channel 数据库表）
```

#### 与 One API 的对应关系

| 本地工具箱 | One API | 说明 |
|-----------|---------|------|
| `api/openai_handler` | `router/relay.go` + `controller/relay.go` | 接收请求，调用 Router |
| `core/router` | `middleware/distributor.go` | 根据 model 查找 Adapter |
| `core/registry` | `model/channel.go` + `relay/adaptor.go` | 管理模型配置和 Adapter 实例 |
| `core/adapter/trait` | `relay/adaptor/interface.go` | Adapter 接口定义 |
| `core/adapter/openai_compat` | `relay/adaptor/openai/` | OpenAI 兼容适配器 |
| `core/adapter/custom_http` | `relay/adaptor/anthropic/` 等 | 非 OpenAI 协议的适配器 |
| `config/models.json` | `model/channel` (数据库表) | 模型配置存储 |

#### 每一层的职责

**API Layer (`api/openai_handler`)**
- 接收 `POST /v1/chat/completions` 请求
- 解析请求体为 `OpenAIChatRequest`
- 调用 `Router.route(model)` 获取 Adapter
- 调用 `Adapter.chat(request)` 获取响应
- 返回 OpenAI 格式的响应

**Router Layer (`core/router`)**
- 接收 `model` 字符串（如 `"gpt-3.5-turbo"`）
- 从 `Registry` 查找对应的 Adapter
- 返回 Adapter 实例

**Registry Layer (`core/registry`)**
- 启动时读取 `config/models.json`
- 为每个模型配置创建对应的 Adapter 实例
- 提供 `get_adapter(model_id: str) -> Adapter` 方法
- 提供 `list_models() -> List[ModelInfo]` 方法（用于 `/v1/models`）

**Adapter Layer (`core/adapter/*`)**
- `openai_compat`: 直接转发请求，不做转换
- `custom_http`: 转换请求格式，转换响应格式
- `process`: 调用本地 CLI，处理 stdin/stdout

---

## 第三阶段：实现指导

### 1️⃣ Adapter 接口定义

#### Rust 版本

```rust
// core/adapter/trait.rs
use async_trait::async_trait;
use serde_json::Value;

pub type Result<T> = std::result::Result<T, AdapterError>;

#[derive(Debug)]
pub struct AdapterError {
    pub message: String,
    pub code: String,
    pub status_code: u16,
}

pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    // ... 其他 OpenAI 字段
}

pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[async_trait]
pub trait ChatAdapter: Send + Sync {
    /// 初始化适配器（从配置加载）
    fn init(config: &Value) -> Result<Self>
    where
        Self: Sized;
    
    /// 检查适配器是否可用（配置是否完整）
    fn is_available(&self) -> bool;
    
    /// 发送聊天请求
    async fn chat(
        &self,
        request: OpenAIChatRequest,
        timeout: Option<u64>,
    ) -> Result<OpenAIChatResponse>;
    
    /// 获取适配器类型（用于日志、调试）
    fn adapter_type(&self) -> &'static str;
    
    /// 获取模型信息（用于 /v1/models）
    fn get_model_info(&self) -> ModelInfo;
}
```

#### Python 版本

```python
# core/adapter/base_adapter.py
from abc import ABC, abstractmethod
from typing import Dict, Any, Optional, List
from dataclasses import dataclass

@dataclass
class OpenAIChatRequest:
    model: str
    messages: List[Dict[str, Any]]
    temperature: Optional[float] = None
    max_tokens: Optional[int] = None
    stream: bool = False

@dataclass
class OpenAIChatResponse:
    id: str
    object: str
    created: int
    model: str
    choices: List[Dict[str, Any]]
    usage: Optional[Dict[str, int]] = None

class ChatAdapter(ABC):
    """AI 模型适配器基类（对应 One API 的 Adaptor 接口）"""
    
    def __init__(self, config: Dict[str, Any]):
        self.config = config
        self.model_id = config.get('id', 'unknown_model')
    
    @abstractmethod
    async def chat(
        self,
        request: OpenAIChatRequest,
        timeout: Optional[int] = None
    ) -> OpenAIChatResponse:
        """
        发送聊天消息并获取回复
        
        Args:
            request: OpenAI 格式的请求
            timeout: 超时时间（秒）
        
        Returns:
            OpenAI 格式的响应
        """
        pass
    
    @abstractmethod
    def is_available(self) -> bool:
        """检查适配器是否可用（配置是否完整）"""
        pass
    
    @property
    @abstractmethod
    def adapter_type(self) -> str:
        """返回适配器类型（如 'openai_compat', 'custom_http'）"""
        pass
    
    def get_model_info(self) -> Dict[str, Any]:
        """获取模型信息（用于 /v1/models 接口）"""
        return {
            "id": self.model_id,
            "object": "model",
            "created": 0,
            "owned_by": self.adapter_type
        }
```

#### 与 One API Provider 的对照说明

| One API | 本地工具箱 | 说明 |
|---------|-----------|------|
| `Adaptor.Init(meta *meta.Meta)` | `ChatAdapter.__init__(config)` | 初始化，接收配置 |
| `Adaptor.GetRequestURL(meta)` | 在 `chat()` 内部实现 | 构建目标 API URL |
| `Adaptor.SetupRequestHeader(...)` | 在 `chat()` 内部实现 | 设置认证头 |
| `Adaptor.ConvertRequest(...)` | 在 `chat()` 内部实现 | 协议转换（如需要） |
| `Adaptor.DoRequest(...)` | 在 `chat()` 内部实现 | 发送 HTTP 请求 |
| `Adaptor.DoResponse(...)` | 在 `chat()` 内部实现 | 响应转换 |
| `Adaptor.GetModelList()` | `get_model_info()` | 返回模型信息 |

**关键差异**：
- One API 将流程拆分为多个方法（便于中间件插入配额逻辑）
- 本地工具箱合并为一个 `chat()` 方法（更简洁）

### 2️⃣ 配置格式设计

#### 轻量级 `models.json`

```json
{
  "models": [
    {
      "id": "gpt-3.5-turbo",
      "adapter": "openai_compat",
      "base_url": "https://api.openai.com/v1",
      "api_key": "ENV:OPENAI_API_KEY",
      "enabled": true,
      "model": "gpt-3.5-turbo",
      "temperature": 0.7,
      "max_tokens": 2000,
      "timeout": 60
    },
    {
      "id": "deepseek-chat",
      "adapter": "openai_compat",
      "base_url": "https://api.deepseek.com/v1",
      "api_key": "ENV:DEEPSEEK_API_KEY",
      "enabled": true,
      "model": "deepseek-chat",
      "temperature": 0.7,
      "max_tokens": 2000,
      "timeout": 60
    },
    {
      "id": "claude-3-5-sonnet",
      "adapter": "custom_http",
      "base_url": "https://api.anthropic.com",
      "api_key": "ENV:ANTHROPIC_API_KEY",
      "enabled": true,
      "model": "claude-3-5-sonnet-20241022",
      "endpoint": "/v1/messages",
      "request_format": "anthropic",
      "response_format": "openai",
      "timeout": 60
    },
    {
      "id": "local-llama",
      "adapter": "process",
      "command": "llama.cpp",
      "args": ["-m", "/path/to/model.gguf", "--ctx-size", "4096"],
      "enabled": false,
      "timeout": 120
    }
  ]
}
```

#### 与 One API Channel 的对应关系

| 本地工具箱字段 | One API Channel 字段 | 说明 |
|--------------|-------------------|------|
| `id` | `Channel.Id` | 模型唯一标识（用于路由） |
| `adapter` | `Channel.Type` → `APIType` | 适配器类型（替代 Type 枚举） |
| `base_url` | `Channel.BaseURL` | API 基础 URL |
| `api_key` | `Channel.Key` | API Key（支持 `ENV:VAR_NAME` 环境变量） |
| `enabled` | `Channel.Status` | 是否启用 |
| `model` | `Channel.Models` (第一个) | 实际使用的模型名称 |
| `temperature`, `max_tokens` | 无（One API 从请求中获取） | 默认参数（可选） |
| `timeout` | 无 | 请求超时时间 |

**关键简化**：
- ❌ 删除 `Channel.Weight`（负载均衡权重）
- ❌ 删除 `Channel.Group`（渠道分组）
- ❌ 删除 `Channel.Priority`（优先级）
- ❌ 删除 `Channel.ModelMapping`（模型映射，可选）
- ❌ 删除 `Channel.Config`（JSON 配置，合并到顶层字段）
- ❌ 删除 `Channel.Balance`（余额）
- ❌ 删除 `Channel.UsedQuota`（使用量）

### 3️⃣ UI 对接说明

#### UI 只需要做什么

1. **调用 OpenAI-compatible API**
   ```typescript
   // src/utils/aiService.ts
   export async function sendAIChat(
     model: string,  // 模型 ID，如 "gpt-3.5-turbo"
     messages: OpenAIMessage[],
     options?: { temperature?: number; max_tokens?: number }
   ): Promise<OpenAIChatResponse> {
     const response = await fetch('http://localhost:8765/v1/chat/completions', {
       method: 'POST',
       headers: {
         'Content-Type': 'application/json',
       },
       body: JSON.stringify({
         model,  // 只需要传递模型 ID
         messages,
         ...options,
       }),
     });
     return response.json();
   }
   ```

2. **获取可用模型列表**
   ```typescript
   export async function getAvailableModels(): Promise<string[]> {
     const response = await fetch('http://localhost:8765/v1/models');
     const data = await response.json();
     return data.data.map((m: any) => m.id);  // 返回模型 ID 列表
   }
   ```

3. **处理错误响应**
   ```typescript
   if (!response.ok) {
     const error = await response.json();
     throw new Error(error.error?.message || 'Request failed');
   }
   ```

#### UI 完全不用知道

- ❌ **Provider**：UI 不需要知道模型来自 OpenAI 还是 DeepSeek
- ❌ **Channel**：UI 不需要知道 Channel ID、Channel Type
- ❌ **Adapter**：UI 不需要知道使用了哪个适配器
- ❌ **BaseURL**：UI 不需要知道实际的 API 地址
- ❌ **API Key**：UI 不需要知道 API Key（由 Gateway 管理）

**UI 的视角**：
- 只有一个统一的 API：`POST /v1/chat/completions`
- 只需要传递 `model` ID（如 `"gpt-3.5-turbo"`）
- 只需要接收 OpenAI 格式的响应

---

## 总结：架构对比

| 维度 | One API | 本地工具箱 |
|------|---------|-----------|
| **接口** | OpenAI-compatible | ✅ 相同 |
| **模型注册** | 数据库（Channel 表） | ✅ 配置文件（models.json） |
| **适配器抽象** | Adaptor 接口 | ✅ 相同思想 |
| **路由逻辑** | 数据库查询 + 缓存 | ✅ 内存查找（配置加载） |
| **用户系统** | ✅ 完整（用户、Token、分组） | ❌ 无 |
| **配额管理** | ✅ 完整（预扣、后扣、倍率） | ❌ 无 |
| **日志审计** | ✅ 数据库记录 | ⚠️ 可选（控制台日志） |
| **Web 管理后台** | ✅ 完整 | ❌ 无（配置文件） |
| **复杂度** | 高（SaaS 级） | 低（本地工具级） |

**核心思想提取**：
1. ✅ **统一接口**：UI 永远只使用 OpenAI-compatible API
2. ✅ **适配器模式**：每个模型厂商一个适配器，解耦协议差异
3. ✅ **配置驱动**：新增模型 = 新增配置，不改代码
4. ✅ **请求生命周期**：接收 → 路由 → 适配 → 转发 → 转换 → 返回

**简化点**：
1. ❌ 删除用户系统、Token 管理
2. ❌ 删除配额、计费逻辑
3. ❌ 删除 Web 管理后台
4. ✅ 保留核心路由和适配器逻辑
5. ✅ 保留 OpenAI-compatible 接口

---

## 下一步实现建议

1. **先实现 `openai_compat` 适配器**（最简单，直接转发）
2. **实现 `Registry` 和 `Router`**（配置加载和模型查找）
3. **实现 `openai_handler`**（HTTP 接口）
4. **测试端到端流程**（UI → Gateway → OpenAI API）
5. **再实现 `custom_http` 适配器**（如 Claude，需要协议转换）
6. **最后实现 `process` 适配器**（本地 CLI 工具）

