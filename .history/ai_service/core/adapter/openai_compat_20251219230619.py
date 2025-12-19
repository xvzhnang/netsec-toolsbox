"""
OpenAI Compatible Adapter
适用于所有兼容 OpenAI API 的模型（OpenAI, DeepSeek, Ollama, vLLM, LocalAI, LM Studio 等）
"""

import os
import time
import uuid
from typing import Dict, Any, Optional, List
from concurrent.futures import ThreadPoolExecutor, TimeoutError as FutureTimeoutError

try:
    from openai import OpenAI
    HAS_OPENAI_LIB = True
except ImportError:
    HAS_OPENAI_LIB = False

from .trait import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse


class OpenAICompatAdapter(ChatAdapter):
    """
    OpenAI Compatible Adapter
    
    适用于所有兼容 OpenAI API 的模型，直接转发请求不做任何转换
    """
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.base_url = config.get('base_url', 'https://api.openai.com/v1')
        self.api_key = self._resolve_api_key(config.get('api_key', ''))
        self.client = None
        
        if self.api_key and HAS_OPENAI_LIB:
            try:
                self.client = OpenAI(
                    api_key=self.api_key,
                    base_url=self.base_url.rstrip('/')
                )
            except AttributeError as e:
                # Windows 上 SIGALRM 错误（openai 库内部可能使用）
                if 'SIGALRM' in str(e):
                    print(f"⚠️ OpenAI Compat Adapter 客户端初始化失败（Windows 不支持 SIGALRM）: {e}", flush=True)
                else:
                    print(f"⚠️ OpenAI Compat Adapter 客户端初始化失败: {e}", flush=True)
            except Exception as e:
                print(f"⚠️ OpenAI Compat Adapter 客户端初始化失败: {e}", flush=True)
    
    def _resolve_api_key(self, api_key: str) -> Optional[str]:
        """
        解析 API Key（支持环境变量）
        
        Args:
            api_key: API Key 字符串，格式为 "ENV:VAR_NAME" 或直接是 key
            
        Returns:
            解析后的 API Key
        """
        if not api_key:
            return None
        
        if api_key.startswith('ENV:'):
            env_var = api_key[4:].strip()
            return os.environ.get(env_var)
        
        return api_key
    
    def is_available(self) -> bool:
        """检查 Adapter 是否可用"""
        return HAS_OPENAI_LIB and self.api_key is not None and self.client is not None
    
    def get_available_models(self) -> Optional[List[str]]:
        """获取可用的模型列表"""
        if not self.is_available():
            return None
        
        try:
            models = self.client.models.list()
            return [model.id for model in models.data] if models.data else None
        except Exception as e:
            print(f"⚠️ 获取模型列表失败: {e}", flush=True)
            return None
    
    def chat(self, request: OpenAIChatRequest) -> OpenAIChatResponse:
        """
        发送聊天请求（使用跨平台超时处理）
        
        Args:
            request: OpenAI 格式的聊天请求
            
        Returns:
            OpenAI 格式的聊天响应
        """
        if not self.is_available():
            raise Exception(f"Adapter {self.model_id} 不可用，请检查配置")
        
        # 转换消息格式
        chat_messages = []
        for msg in request.messages:
            role = msg.get('role', 'user')
            content = msg.get('content', msg.get('text', ''))
            chat_messages.append({
                'role': role,
                'content': content
            })
        
        # 从配置获取参数
        temperature = request.temperature or self.config.get('temperature', 0.7)
        max_tokens = request.max_tokens or self.config.get('max_tokens', 2000)
        request_timeout = request.timeout or self.config.get('timeout', 60)
        
        # 使用 ThreadPoolExecutor 实现跨平台超时（避免 signal.SIGALRM 在 Windows 上的问题）
        def _call_api():
            return self.client.chat.completions.create(
                model=request.model,
                messages=chat_messages,
                temperature=temperature,
                max_tokens=max_tokens,
                timeout=request_timeout
            )
        
        with ThreadPoolExecutor(max_workers=1) as executor:
            future = executor.submit(_call_api)
            try:
                response = future.result(timeout=request_timeout + 5)  # 额外 5 秒缓冲
            except FutureTimeoutError:
                raise TimeoutError(f'AI 请求超时（{request_timeout}秒）')
            except Exception as e:
                raise Exception(f'API 调用失败: {str(e)}')
        
        # 转换为 OpenAI 响应格式
        return OpenAIChatResponse(
            id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
            object="chat.completion",
            created=int(time.time()),
            model=request.model,
            choices=[{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": response.choices[0].message.content
                },
                "finish_reason": response.choices[0].finish_reason or "stop"
            }],
            usage={
                "prompt_tokens": response.usage.prompt_tokens if response.usage else 0,
                "completion_tokens": response.usage.completion_tokens if response.usage else 0,
                "total_tokens": response.usage.total_tokens if response.usage else 0
            } if response.usage else None
        )

