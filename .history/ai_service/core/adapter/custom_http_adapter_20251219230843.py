"""
Custom HTTP Adapter
适用于不兼容 OpenAI API 的模型
需要字段转换和协议适配
"""

import json
import time
import uuid
import requests
from typing import Dict, Any, Optional
from concurrent.futures import ThreadPoolExecutor, TimeoutError as FutureTimeoutError

from .base_adapter import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse


class CustomHTTPAdapter(ChatAdapter):
    """
    Custom HTTP Adapter
    适用于不兼容 OpenAI API 的自定义 HTTP 接口
    """
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.base_url = config.get('base_url', '').rstrip('/')
        self.endpoint = config.get('endpoint', '/chat/completions')
        self.api_key = config.get('api_key', '')
        
        # 请求格式配置
        self.request_format = config.get('request_format', 'openai')  # 'openai' 或 'custom'
        self.request_template = config.get('request_template', {})  # 自定义请求模板
        
        # 响应格式配置
        self.response_path = config.get('response_path', 'choices[0].message.content')  # JSON 路径
        
        # 认证配置
        self.auth_type = config.get('auth_type', 'bearer')  # 'bearer', 'header', 'api_key'
        self.auth_header = config.get('auth_header', 'X-API-Key')
        
        # 自定义请求头
        self.custom_headers = config.get('headers', {})
    
    def is_available(self) -> bool:
        """检查适配器是否可用"""
        return bool(self.base_url)
    
    def chat(self, request: OpenAIChatRequest) -> OpenAIChatResponse:
        """发送聊天请求并获取回复"""
        if not self.is_available():
            raise Exception(f"Custom HTTP Adapter 不可用（模型: {self.model_id}）")
        
        try:
            # 构建请求 URL
            url = f"{self.base_url}{self.endpoint}"
            
            # 构建请求头
            headers = {
                'Content-Type': 'application/json',
                **self.custom_headers
            }
            
            # 添加认证
            if self.api_key:
                if self.auth_type == 'bearer':
                    headers['Authorization'] = f'Bearer {self.api_key}'
                elif self.auth_type == 'header':
                    headers[self.auth_header] = self.api_key
                elif self.auth_type == 'api_key':
                    headers['api-key'] = self.api_key
            
            # 构建请求体
            if self.request_format == 'openai':
                body = self._build_openai_request(request)
            else:
                body = self._build_custom_request(request)
            
            # 获取超时配置
            timeout = request.timeout or self.config.get('timeout', 60)
            
            # 发送请求（使用 ThreadPoolExecutor 实现跨平台超时）
            def _send_request():
                return requests.post(
                    url,
                    headers=headers,
                    json=body,
                    timeout=timeout
                )
            
            with ThreadPoolExecutor(max_workers=1) as executor:
                future = executor.submit(_send_request)
                try:
                    response = future.result(timeout=timeout + 5)
                    response.raise_for_status()
                except FutureTimeoutError:
                    raise Exception(f"请求超时（{timeout}秒）")
            
            # 解析响应
            response_data = response.json()
            return self._parse_response(response_data, request.model)
            
        except Exception as e:
            raise Exception(f"Custom HTTP API 调用失败: {str(e)}")
    
    def _build_openai_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """构建 OpenAI 格式的请求"""
        return {
            'model': request.model or self.model_id,
            'messages': request.messages,
            'temperature': request.temperature,
            'max_tokens': request.max_tokens,
            'top_p': request.top_p,
            'frequency_penalty': request.frequency_penalty,
            'presence_penalty': request.presence_penalty,
            'stop': request.stop
        }
    
    def _build_custom_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """构建自定义格式的请求"""
        # 使用模板，替换变量
        body = json.loads(json.dumps(self.request_template))
        
        # 替换消息变量
        if '{{messages}}' in json.dumps(body):
            body_str = json.dumps(body).replace('{{messages}}', json.dumps(request.messages))
            body = json.loads(body_str)
        
        # 替换其他变量
        replacements = {
            '{{model}}': request.model or self.model_id,
            '{{temperature}}': request.temperature,
            '{{max_tokens}}': request.max_tokens,
        }
        
        body_str = json.dumps(body)
        for key, value in replacements.items():
            if value is not None:
                body_str = body_str.replace(key, json.dumps(value))
        
        return json.loads(body_str)
    
    def _parse_response(self, response_data: Dict[str, Any], model: str) -> OpenAIChatResponse:
        """解析响应为 OpenAI 格式"""
        # 从响应中提取文本（支持 JSON 路径）
        content = self._extract_content(response_data, self.response_path)
        
        if content is None:
            # 尝试常见格式
            if 'choices' in response_data and len(response_data['choices']) > 0:
                content = response_data['choices'][0].get('message', {}).get('content', '')
            elif 'text' in response_data:
                content = response_data['text']
            elif 'result' in response_data:
                content = str(response_data['result'])
            else:
                content = json.dumps(response_data)
        
        # 构建 OpenAI 响应
        response_id = f"chatcmpl-{uuid.uuid4().hex[:8]}"
        
        choices = [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": str(content)
            },
            "finish_reason": "stop"
        }]
        
        return OpenAIChatResponse(
            id=response_id,
            created=int(time.time()),
            model=model or self.model_id,
            choices=choices
        )
    
    def _extract_content(self, data: Any, path: str) -> Optional[str]:
        """从 JSON 数据中提取指定路径的值"""
        parts = path.split('.')
        result = data
        
        for part in parts:
            if '[' in part:
                # 处理数组索引，如 'choices[0]'
                key, index_str = part.split('[')
                index = int(index_str.rstrip(']'))
                if isinstance(result, dict):
                    result = result.get(key, [])
                if isinstance(result, list) and index < len(result):
                    result = result[index]
                else:
                    return None
            else:
                if isinstance(result, dict):
                    result = result.get(part)
                else:
                    return None
                
                if result is None:
                    return None
        
        return str(result) if result is not None else None

