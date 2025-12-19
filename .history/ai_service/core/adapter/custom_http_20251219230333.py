"""
Custom HTTP Adapter
适用于不兼容 OpenAI API 的模型，需要字段转换
"""

import json
import time
import uuid
import requests
from typing import Dict, Any, Optional, List
from concurrent.futures import ThreadPoolExecutor, TimeoutError as FutureTimeoutError

from .trait import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse


class CustomHTTPAdapter(ChatAdapter):
    """
    Custom HTTP Adapter
    
    适用于不兼容 OpenAI API 的模型，需要将 OpenAI 格式转换为自定义格式
    """
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.base_url = config.get('base_url', '').rstrip('/')
        self.api_key = self._resolve_api_key(config.get('api_key', ''))
        self.endpoint = config.get('endpoint', '/chat/completions')
        self.headers = config.get('headers', {})
        self.request_format = config.get('request_format', 'openai')  # 'openai' 或 'custom'
        self.response_path = config.get('response_path', 'choices[0].message.content')  # JSON 路径
        self.auth_type = config.get('auth_type', 'bearer')  # 'bearer', 'header', 'api_key'
        self.auth_header = config.get('auth_header', 'X-API-Key')
        
        # 构建请求头
        self.request_headers = {
            'Content-Type': 'application/json',
            **self.headers
        }
        
        if self.api_key:
            if self.auth_type == 'bearer':
                self.request_headers['Authorization'] = f'Bearer {self.api_key}'
            elif self.auth_type == 'header':
                self.request_headers[self.auth_header] = self.api_key
            elif self.auth_type == 'api_key':
                self.request_headers['api-key'] = self.api_key
    
    def _resolve_api_key(self, api_key: str) -> Optional[str]:
        """解析 API Key（支持环境变量）"""
        if not api_key:
            return None
        
        if api_key.startswith('ENV:'):
            import os
            env_var = api_key[4:].strip()
            return os.environ.get(env_var)
        
        return api_key
    
    def is_available(self) -> bool:
        """检查 Adapter 是否可用"""
        return self.base_url != '' and self.api_key is not None
    
    def _format_messages(self, messages: List[Dict[str, str]]) -> Any:
        """格式化消息为自定义格式"""
        if self.request_format == 'openai':
            # 保持 OpenAI 格式
            return messages
        elif self.request_format == 'prompt':
            # 拼接为单个 prompt
            prompt_parts = []
            for msg in messages:
                role = msg.get('role', 'user')
                content = msg.get('content', msg.get('text', ''))
                if role == 'system':
                    prompt_parts.append(f"System: {content}")
                elif role == 'user':
                    prompt_parts.append(f"User: {content}")
                elif role == 'assistant':
                    prompt_parts.append(f"Assistant: {content}")
            return "\n\n".join(prompt_parts)
        else:
            # 返回原始格式
            return messages
    
    def _extract_response(self, data: Dict[str, Any], path: str) -> str:
        """从响应中提取文本（支持 JSON 路径）"""
        parts = path.split('.')
        result = data
        
        for part in parts:
            if '[' in part:
                # 处理数组索引，如 'choices[0]'
                key, index_str = part.split('[')
                index = int(index_str.rstrip(']'))
                result = result.get(key, [])[index]
            else:
                result = result.get(part)
                if result is None:
                    return ""
        
        return str(result) if result else ""
    
    def chat(self, request: OpenAIChatRequest) -> OpenAIChatResponse:
        """发送聊天请求"""
        if not self.is_available():
            raise Exception(f"Adapter {self.model_id} 不可用，请检查配置")
        
        # 构建请求 URL
        url = f"{self.base_url}/{self.endpoint.lstrip('/')}"
        
        # 构建请求体
        if self.request_format == 'openai':
            body = {
                'model': request.model,
                'messages': self._format_messages(request.messages),
                'temperature': request.temperature or self.config.get('temperature', 0.7),
                'max_tokens': request.max_tokens or self.config.get('max_tokens', 2000),
            }
        else:
            # 自定义格式
            body_template = self.config.get('request_body', {})
            formatted_messages = self._format_messages(request.messages)
            # 替换模板变量
            body_str = json.dumps(body_template).replace('{{messages}}', json.dumps(formatted_messages))
            body = json.loads(body_str)
        
        # 发送请求（使用跨平台超时）
        request_timeout = request.timeout or self.config.get('timeout', 60)
        
        def _call_api():
            response = requests.post(
                url,
                headers=self.request_headers,
                json=body,
                timeout=request_timeout
            )
            response.raise_for_status()
            return response.json()
        
        with ThreadPoolExecutor(max_workers=1) as executor:
            future = executor.submit(_call_api)
            try:
                response_data = future.result(timeout=request_timeout + 5)
            except FutureTimeoutError:
                raise TimeoutError(f'AI 请求超时（{request_timeout}秒）')
            except Exception as e:
                raise Exception(f'API 调用失败: {str(e)}')
        
        # 从响应中提取文本
        content = self._extract_response(response_data, self.response_path)
        
        if not content:
            # 尝试常见格式
            if 'choices' in response_data and len(response_data['choices']) > 0:
                content = response_data['choices'][0].get('message', {}).get('content', '')
            elif 'text' in response_data:
                content = response_data['text']
            elif 'result' in response_data:
                content = str(response_data['result'])
            else:
                content = json.dumps(response_data)
        
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
                    "content": content
                },
                "finish_reason": "stop"
            }],
            usage=None
        )

