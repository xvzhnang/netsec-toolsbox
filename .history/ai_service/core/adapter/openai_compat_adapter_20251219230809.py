"""
OpenAI Compatible Adapter
适用于所有兼容 OpenAI API 的模型
"""

import os
import time
import uuid
from typing import Dict, Any, Optional
from concurrent.futures import ThreadPoolExecutor, TimeoutError as FutureTimeoutError

try:
    from openai import OpenAI
    HAS_OPENAI_LIB = True
except ImportError:
    HAS_OPENAI_LIB = False

from .base_adapter import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse


class OpenAICompatAdapter(ChatAdapter):
    """
    OpenAI Compatible Adapter
    适用于：OpenAI, DeepSeek, Ollama, vLLM, LocalAI, LM Studio 等
    """
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.base_url = config.get('base_url', '')
        self.api_key = config.get('api_key', '')
        
        # 支持环境变量：ENV:VAR_NAME
        if self.api_key.startswith('ENV:'):
            env_var = self.api_key[4:]
            self.api_key = os.environ.get(env_var, '')
        
        self.client = None
        self._init_client()
    
    def _init_client(self):
        """初始化 OpenAI 客户端"""
        if not HAS_OPENAI_LIB:
            return
        
        if not self.api_key and not self.base_url:
            return
        
        try:
            client_kwargs = {}
            if self.api_key:
                client_kwargs['api_key'] = self.api_key
            if self.base_url:
                client_kwargs['base_url'] = self.base_url.rstrip('/')
            
            self.client = OpenAI(**client_kwargs)
        except AttributeError as e:
            # Windows 上 SIGALRM 错误
            if 'SIGALRM' in str(e):
                print(f"⚠️ OpenAI Compat Adapter 初始化失败（Windows 不支持 SIGALRM）: {e}", flush=True)
            else:
                print(f"⚠️ OpenAI Compat Adapter 初始化失败: {e}", flush=True)
        except Exception as e:
            print(f"⚠️ OpenAI Compat Adapter 初始化失败: {e}", flush=True)
    
    def is_available(self) -> bool:
        """检查适配器是否可用"""
        return HAS_OPENAI_LIB and self.client is not None
    
    def chat(self, request: OpenAIChatRequest) -> OpenAIChatResponse:
        """发送聊天请求并获取回复"""
        if not self.is_available():
            raise Exception(f"OpenAI Compat Adapter 不可用（模型: {self.model_id}）")
        
        try:
            # 转换消息格式
            chat_messages = []
            for msg in request.messages:
                role = msg.get('role', 'user')
                content = msg.get('content', msg.get('text', ''))
                chat_messages.append({
                    'role': role,
                    'content': content
                })
            
            # 构建请求参数
            request_params = {
                'model': request.model or self.model_id,
                'messages': chat_messages,
            }
            
            # 添加可选参数
            if request.temperature is not None:
                request_params['temperature'] = request.temperature
            if request.max_tokens is not None:
                request_params['max_tokens'] = request.max_tokens
            if request.top_p is not None:
                request_params['top_p'] = request.top_p
            if request.frequency_penalty is not None:
                request_params['frequency_penalty'] = request.frequency_penalty
            if request.presence_penalty is not None:
                request_params['presence_penalty'] = request.presence_penalty
            if request.stop is not None:
                request_params['stop'] = request.stop
            
            # 获取超时配置
            timeout = request.timeout or self.config.get('timeout', 60)
            
            # 使用 ThreadPoolExecutor 实现跨平台超时
            def _call_api():
                return self.client.chat.completions.create(
                    **request_params,
                    timeout=timeout
                )
            
            with ThreadPoolExecutor(max_workers=1) as executor:
                future = executor.submit(_call_api)
                try:
                    response = future.result(timeout=timeout + 5)  # 额外 5 秒缓冲
                except FutureTimeoutError:
                    raise Exception(f"请求超时（{timeout}秒）")
            
            # 转换为 OpenAI 响应格式
            return self._convert_response(response)
            
        except Exception as e:
            raise Exception(f"OpenAI Compat API 调用失败: {str(e)}")
    
    def _convert_response(self, response) -> OpenAIChatResponse:
        """转换 OpenAI 响应为标准格式"""
        # 生成响应 ID
        response_id = f"chatcmpl-{uuid.uuid4().hex[:8]}"
        
        # 提取 choices
        choices = []
        if response.choices:
            for choice in response.choices:
                choices.append({
                    "index": choice.index if hasattr(choice, 'index') else 0,
                    "message": {
                        "role": choice.message.role if hasattr(choice.message, 'role') else "assistant",
                        "content": choice.message.content if hasattr(choice.message, 'content') else ""
                    },
                    "finish_reason": choice.finish_reason if hasattr(choice, 'finish_reason') else "stop"
                })
        
        # 提取 usage
        usage = None
        if hasattr(response, 'usage') and response.usage:
            usage = {
                "prompt_tokens": response.usage.prompt_tokens if hasattr(response.usage, 'prompt_tokens') else 0,
                "completion_tokens": response.usage.completion_tokens if hasattr(response.usage, 'completion_tokens') else 0,
                "total_tokens": response.usage.total_tokens if hasattr(response.usage, 'total_tokens') else 0
            }
        
        return OpenAIChatResponse(
            id=response_id,
            created=int(time.time()),
            model=response.model if hasattr(response, 'model') else self.model_id,
            choices=choices,
            usage=usage
        )
    
    def get_available_models(self) -> Optional[list[str]]:
        """获取可用的模型列表"""
        if not self.is_available():
            return None
        
        try:
            models = self.client.models.list()
            if models and hasattr(models, 'data'):
                return [model.id for model in models.data]
        except Exception as e:
            print(f"⚠️ 获取模型列表失败: {e}", flush=True)
        
        return None

