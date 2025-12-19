# AI Gateway 实现指南

> 基于 One API 架构分析的轻量级实现指南

## 核心请求流程（简化版）

```
┌─────────────────────────────────────────────────────────────┐
│ UI (Vue)                                                     │
│  POST /v1/chat/completions                                   │
│  { "model": "gpt-3.5-turbo", "messages": [...] }            │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ API Handler (api/openai_handler.rs)                         │
│  1. 解析请求体 → OpenAIChatRequest                          │
│  2. 提取 model 字段                                         │
│  3. 调用 Router.route(model)                                │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ Router (core/router.rs)                                     │
│  1. 从 Registry 查找 model_id 对应的 Adapter                │
│  2. 返回 Adapter 实例                                        │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ Registry (core/registry.rs)                                 │
│  - 启动时加载 config/models.json                            │
│  - 为每个模型创建 Adapter 实例                              │
│  - 提供 get_adapter(model_id) 方法                          │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ Adapter (core/adapter/openai_compat.rs)                     │
│  1. 从配置获取 base_url, api_key                            │
│  2. 构建完整 URL: base_url + "/chat/completions"            │
│  3. 设置 Authorization: Bearer {api_key}                    │
│  4. 发送 HTTP 请求（直接转发，不转换）                      │
│  5. 返回响应（已经是 OpenAI 格式）                          │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ 目标 API (如 OpenAI)                                         │
│  - 处理请求，返回响应                                        │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ 响应返回给 UI（OpenAI 格式）                                 │
└─────────────────────────────────────────────────────────────┘
```

## 关键代码示例

### 1. Registry 实现（Python 版本）

```python
# core/registry.py
import json
import os
from typing import Dict, Optional
from .adapter.base_adapter import ChatAdapter
from .adapter.openai_compat_adapter import OpenAICompatAdapter
from .adapter.custom_http_adapter import CustomHTTPAdapter

class ModelRegistry:
    """模型注册表（对应 One API 的 Channel 管理）"""
    
    def __init__(self, config_path: str = "config/models.json"):
        self.config_path = config_path
        self.adapters: Dict[str, ChatAdapter] = {}
        self._load_models()
    
    def _load_models(self):
        """从配置文件加载模型"""
        if not os.path.exists(self.config_path):
            print(f"⚠️ 配置文件不存在: {self.config_path}")
            return
        
        with open(self.config_path, 'r', encoding='utf-8') as f:
            config = json.load(f)
        
        for model_config in config.get('models', []):
            model_id = model_config.get('id')
            if not model_id:
                continue
            
            adapter_type = model_config.get('adapter', 'openai_compat')
            enabled = model_config.get('enabled', True)
            
            if not enabled:
                continue
            
            try:
                adapter = self._create_adapter(adapter_type, model_config)
                if adapter and adapter.is_available():
                    self.adapters[model_id] = adapter
                    print(f"✅ 模型 {model_id} ({adapter_type}) 已加载")
            except Exception as e:
                print(f"❌ 初始化模型 {model_id} 失败: {e}")
    
    def _create_adapter(self, adapter_type: str, config: Dict) -> Optional[ChatAdapter]:
        """创建适配器实例（对应 One API 的 relay.GetAdaptor）"""
        if adapter_type == 'openai_compat':
            return OpenAICompatAdapter(config)
        elif adapter_type == 'custom_http':
            return CustomHTTPAdapter(config)
        # elif adapter_type == 'process':
        #     return ProcessAdapter(config)
        else:
            print(f"⚠️ 未知的适配器类型: {adapter_type}")
            return None
    
    def get_adapter(self, model_id: str) -> Optional[ChatAdapter]:
        """获取指定模型的适配器（对应 One API 的 CacheGetRandomSatisfiedChannel）"""
        return self.adapters.get(model_id)
    
    def list_models(self) -> Dict:
        """列出所有可用模型（用于 /v1/models 接口）"""
        models_info = [adapter.get_model_info() for adapter in self.adapters.values()]
        return {
            "object": "list",
            "data": models_info
        }
```

### 2. Router 实现（Python 版本）

```python
# core/router.py
from typing import Optional
from .registry import ModelRegistry
from .adapter.base_adapter import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse

class Router:
    """请求路由器（对应 One API 的 middleware.Distribute）"""
    
    def __init__(self, registry: ModelRegistry):
        self.registry = registry
    
    async def route(
        self,
        model_id: str,
        request: OpenAIChatRequest
    ) -> OpenAIChatResponse:
        """
        路由请求到对应的适配器
        
        Args:
            model_id: 模型 ID（从请求中提取）
            request: OpenAI 格式的请求
        
        Returns:
            OpenAI 格式的响应
        """
        # 查找适配器（对应 One API 的 CacheGetRandomSatisfiedChannel）
        adapter = self.registry.get_adapter(model_id)
        if not adapter:
            raise ValueError(f"模型 {model_id} 未找到或未启用")
        
        # 调用适配器（对应 One API 的 adaptor.DoRequest + DoResponse）
        response = await adapter.chat(request)
        return response
```

### 3. API Handler 实现（Python 版本）

```python
# api/openai_handler.py
from http.server import BaseHTTPRequestHandler
import json
from core.router import Router
from core.registry import ModelRegistry
from core.adapter.base_adapter import OpenAIChatRequest

class AIRequestHandler(BaseHTTPRequestHandler):
    """OpenAI-compatible API Handler（对应 One API 的 controller.Relay）"""
    
    def __init__(self, *args, router: Router, **kwargs):
        self.router = router
        super().__init__(*args, **kwargs)
    
    def do_POST(self):
        """处理 POST 请求"""
        if self.path == '/v1/chat/completions':
            self._handle_chat_completions()
        else:
            self._send_error(404, "Not Found")
    
    def do_GET(self):
        """处理 GET 请求"""
        if self.path == '/v1/models':
            self._handle_list_models()
        elif self.path == '/health':
            self._send_json_response({"status": "ok"})
        else:
            self._send_error(404, "Not Found")
    
    def _handle_chat_completions(self):
        """处理 /v1/chat/completions 请求"""
        try:
            # 解析请求体（对应 One API 的 getAndValidateTextRequest）
            content_length = int(self.headers['Content-Length'])
            request_body = self.rfile.read(content_length)
            request_data = json.loads(request_body.decode('utf-8'))
            
            # 提取 model 字段（对应 One API 的 meta.OriginModelName）
            model_id = request_data.get('model')
            if not model_id:
                self._send_error(400, "Missing 'model' field")
                return
            
            # 构建请求对象
            chat_request = OpenAIChatRequest(
                model=model_id,
                messages=request_data.get('messages', []),
                temperature=request_data.get('temperature'),
                max_tokens=request_data.get('max_tokens'),
                stream=request_data.get('stream', False)
            )
            
            # 路由到适配器（对应 One API 的 RelayTextHelper）
            import asyncio
            response = asyncio.run(self.router.route(model_id, chat_request))
            
            # 返回响应（已经是 OpenAI 格式）
            self._send_json_response(response.__dict__)
        
        except Exception as e:
            self._send_error(500, str(e))
    
    def _handle_list_models(self):
        """处理 /v1/models 请求"""
        models = self.router.registry.list_models()
        self._send_json_response(models)
    
    def _send_json_response(self, data: dict):
        """发送 JSON 响应"""
        response = json.dumps(data, ensure_ascii=False).encode('utf-8')
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(response)
    
    def _send_error(self, status_code: int, message: str):
        """发送错误响应（OpenAI 格式）"""
        error_response = {
            "error": {
                "message": message,
                "type": "invalid_request_error",
                "code": str(status_code)
            }
        }
        response = json.dumps(error_response, ensure_ascii=False).encode('utf-8')
        self.send_response(status_code)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(response)
```

### 4. OpenAI Compat Adapter 实现（Python 版本）

```python
# core/adapter/openai_compat_adapter.py
import os
import aiohttp
from typing import Optional
from .base_adapter import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse

class OpenAICompatAdapter(ChatAdapter):
    """
    OpenAI 兼容适配器（对应 One API 的 relay/adaptor/openai/）
    适用于：OpenAI, DeepSeek, Ollama, vLLM, LocalAI, LM Studio 等
    """
    
    def __init__(self, config: dict):
        super().__init__(config)
        self.api_key = config.get('api_key')
        # 支持环境变量：ENV:OPENAI_API_KEY
        if isinstance(self.api_key, str) and self.api_key.startswith('ENV:'):
            env_var = self.api_key[4:]
            self.api_key = os.environ.get(env_var)
        
        self.base_url = config.get('base_url')
        self.model = config.get('model', self.model_id)
    
    @property
    def adapter_type(self) -> str:
        return "openai_compat"
    
    def is_available(self) -> bool:
        """检查适配器是否可用（对应 One API 的适配器初始化检查）"""
        # 某些本地服务可能不需要 API Key
        if self.base_url and ('ollama' in self.base_url or 'lmstudio' in self.base_url):
            return self.base_url is not None
        return self.api_key is not None and self.base_url is not None
    
    async def chat(
        self,
        request: OpenAIChatRequest,
        timeout: Optional[int] = None
    ) -> OpenAIChatResponse:
        """
        发送聊天请求（合并了 One API 的 GetRequestURL + SetupRequestHeader + DoRequest + DoResponse）
        """
        # 构建 URL（对应 One API 的 GetRequestURL）
        url = f"{self.base_url.rstrip('/')}/chat/completions"
        
        # 构建请求体（对应 One API 的 ConvertRequest，这里是直通）
        request_body = {
            "model": self.model,  # 使用配置中的实际模型名
            "messages": [msg.__dict__ for msg in request.messages],
            "stream": request.stream,
        }
        if request.temperature is not None:
            request_body["temperature"] = request.temperature
        if request.max_tokens is not None:
            request_body["max_tokens"] = request.max_tokens
        
        # 设置请求头（对应 One API 的 SetupRequestHeader）
        headers = {
            'Content-Type': 'application/json',
            'Authorization': f'Bearer {self.api_key}'
        }
        
        # 发送请求（对应 One API 的 DoRequest）
        request_timeout = timeout or self.config.get('timeout', 60)
        async with aiohttp.ClientSession() as session:
            async with session.post(
                url,
                headers=headers,
                json=request_body,
                timeout=aiohttp.ClientTimeout(total=request_timeout)
            ) as response:
                if response.status != 200:
                    error_data = await response.json()
                    raise Exception(f"API 错误: {error_data.get('error', {}).get('message', 'Unknown error')}")
                
                # 解析响应（对应 One API 的 DoResponse，这里是直通）
                response_data = await response.json()
                return OpenAIChatResponse(
                    id=response_data.get('id'),
                    object=response_data.get('object'),
                    created=response_data.get('created'),
                    model=response_data.get('model'),
                    choices=response_data.get('choices', []),
                    usage=response_data.get('usage')
                )
```

## 与 One API 的关键差异总结

| 方面 | One API | 本地工具箱 |
|------|---------|-----------|
| **配置存储** | 数据库（Channel 表） | JSON 文件（models.json） |
| **模型查找** | 数据库查询 + Redis 缓存 | 内存 HashMap 查找 |
| **适配器创建** | switch 语句硬编码 | 配置驱动，动态创建 |
| **请求流程** | 多个中间件（TokenAuth、Distribute、配额） | 单一 Handler → Router → Adapter |
| **错误处理** | 重试机制（多个 Channel） | 简单错误返回 |
| **流式响应** | 完整的 SSE 处理 | 可简化或后续实现 |

## 实现优先级

1. **P0（必须）**：
   - ✅ `openai_compat` 适配器
   - ✅ Registry 和 Router
   - ✅ API Handler（`/v1/chat/completions`, `/v1/models`）

2. **P1（重要）**：
   - ⚠️ `custom_http` 适配器（如 Claude）
   - ⚠️ 流式响应支持（SSE）

3. **P2（可选）**：
   - ⚠️ `process` 适配器（本地 CLI）
   - ⚠️ 健康检查端点
   - ⚠️ 错误重试机制

