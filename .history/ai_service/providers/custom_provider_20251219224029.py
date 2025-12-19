"""自定义 AI 模型提供商（支持用户自行添加）"""

from typing import List, Dict, Any, Optional
import os
import requests
import json

try:
    from openai import OpenAI
    HAS_OPENAI_LIB = True
except ImportError:
    HAS_OPENAI_LIB = False

from .base_provider import BaseProvider


class CustomProvider(BaseProvider):
    """自定义 AI 模型提供商
    
    支持两种模式：
    1. OpenAI 兼容模式：只需配置 base_url 和 api_key
    2. 自定义 HTTP 模式：完全自定义请求格式
    """
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.provider_name = config.get('name', 'custom')
        self.api_key = config.get('api_key') or os.environ.get('CUSTOM_API_KEY')
        self.model = config.get('model', '')
        self.base_url = config.get('base_url', '')
        self.api_url = config.get('api_url', '')  # 自定义 API URL
        self.mode = config.get('mode', 'openai')  # 'openai' 或 'custom'
        self.client = None
        
        # OpenAI 兼容模式
        if self.mode == 'openai' and self.base_url and self.api_key and HAS_OPENAI_LIB:
            try:
                self.client = OpenAI(
                    api_key=self.api_key,
                    base_url=self.base_url.rstrip('/')
                )
            except AttributeError as e:
                # Windows 上 SIGALRM 错误（openai 库内部可能使用）
                if 'SIGALRM' in str(e):
                    print(f"⚠️ 自定义提供商 {self.provider_name} 客户端初始化失败（Windows 不支持 SIGALRM）: {e}", flush=True)
                else:
                    print(f"⚠️ 自定义提供商 {self.provider_name} 客户端初始化失败: {e}", flush=True)
            except Exception as e:
                print(f"⚠️ 自定义提供商 {self.provider_name} 客户端初始化失败: {e}", flush=True)
        
        # 自定义 HTTP 模式配置
        self.custom_endpoint = config.get('endpoint', '/chat/completions')
        self.custom_headers = config.get('headers', {})
        self.custom_request_format = config.get('request_format', 'openai')  # 'openai' 或 'custom'
        self.custom_response_path = config.get('response_path', 'choices[0].message.content')  # JSON 路径
    
    def is_available(self) -> bool:
        """检查自定义提供商是否可用"""
        if self.mode == 'openai':
            return HAS_OPENAI_LIB and self.api_key is not None and self.base_url and self.client is not None
        else:
            return self.api_url is not None and self.api_key is not None
    
    def _extract_json_path(self, data: Any, path: str) -> Any:
        """从 JSON 数据中提取指定路径的值
        
        支持路径格式：
        - 'choices[0].message.content'
        - 'data.result.text'
        """
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
                    return None
        
        return result
    
    def _format_messages_openai(self, messages: List[Dict[str, str]]) -> List[Dict[str, str]]:
        """将消息格式化为 OpenAI 格式"""
        chat_messages = []
        for msg in messages:
            role = msg.get('role', 'user')
            content = msg.get('text', msg.get('content', ''))
            chat_messages.append({
                'role': role,
                'content': content
            })
        return chat_messages
    
    def _format_messages_custom(self, messages: List[Dict[str, str]]) -> Any:
        """将消息格式化为自定义格式（默认返回原始格式）"""
        # 用户可以重写此方法或通过配置指定格式
        format_type = self.config.get('message_format', 'openai')
        if format_type == 'openai':
            return self._format_messages_openai(messages)
        elif format_type == 'list':
            # 简单列表格式
            return [msg.get('text', msg.get('content', '')) for msg in messages]
        else:
            # 返回原始格式
            return messages
    
    def chat(self, messages: List[Dict[str, str]], timeout: Optional[int] = None) -> str:
        """发送聊天消息并获取回复（使用跨平台超时处理）"""
        if not self.is_available():
            return f"❌ 自定义提供商 {self.provider_name} 未配置或不可用"
        
        try:
            request_timeout = timeout or self.config.get('timeout', 60)
            
            # OpenAI 兼容模式
            if self.mode == 'openai' and self.client:
                chat_messages = self._format_messages_openai(messages)
                temperature = self.config.get('temperature', 0.7)
                max_tokens = self.config.get('max_tokens', 2000)
                
                # 使用 ThreadPoolExecutor 实现跨平台超时（避免 signal.SIGALRM 在 Windows 上的问题）
                from concurrent.futures import ThreadPoolExecutor, TimeoutError as FutureTimeoutError
                
                def _call_api():
                    return self.client.chat.completions.create(
                        model=self.model,
                        messages=chat_messages,
                        temperature=temperature,
                        max_tokens=max_tokens,
                        timeout=request_timeout
                    )
                
                with ThreadPoolExecutor(max_workers=1) as executor:
                    future = executor.submit(_call_api)
                    try:
                        response = future.result(timeout=request_timeout + 5)  # 额外 5 秒缓冲
                        return response.choices[0].message.content
                    except FutureTimeoutError:
                        return f"❌ 自定义提供商 {self.provider_name} API 请求超时（{request_timeout}秒）"
            
            # 自定义 HTTP 模式
            else:
                # 构建请求 URL
                url = f"{self.api_url.rstrip('/')}/{self.custom_endpoint.lstrip('/')}"
                
                # 构建请求头
                headers = {
                    'Content-Type': 'application/json',
                    **self.custom_headers
                }
                
                if self.api_key:
                    # 支持不同的认证方式
                    auth_type = self.config.get('auth_type', 'bearer')  # 'bearer', 'header', 'api_key'
                    if auth_type == 'bearer':
                        headers['Authorization'] = f'Bearer {self.api_key}'
                    elif auth_type == 'header':
                        auth_header = self.config.get('auth_header', 'X-API-Key')
                        headers[auth_header] = self.api_key
                    elif auth_type == 'api_key':
                        headers['api-key'] = self.api_key
                
                # 构建请求体
                if self.custom_request_format == 'openai':
                    # OpenAI 格式
                    body = {
                        'model': self.model,
                        'messages': self._format_messages_openai(messages),
                        'temperature': self.config.get('temperature', 0.7),
                        'max_tokens': self.config.get('max_tokens', 2000),
                    }
                else:
                    # 完全自定义格式
                    body_template = self.config.get('request_body', {})
                    # 替换模板变量
                    body = json.loads(json.dumps(body_template).replace('{{messages}}', json.dumps(messages)))
                
                # 发送请求（requests 库的 timeout 参数是跨平台的）
                response = requests.post(
                    url,
                    headers=headers,
                    json=body,
                    timeout=request_timeout
                )
                response.raise_for_status()
                
                # 解析响应
                response_data = response.json()
                
                # 从响应中提取文本（支持 JSON 路径）
                if self.custom_response_path:
                    result = self._extract_json_path(response_data, self.custom_response_path)
                    if result:
                        return str(result)
                
                # 如果路径提取失败，尝试常见格式
                if 'choices' in response_data and len(response_data['choices']) > 0:
                    return response_data['choices'][0].get('message', {}).get('content', '')
                elif 'text' in response_data:
                    return response_data['text']
                elif 'result' in response_data:
                    return str(response_data['result'])
                else:
                    return json.dumps(response_data)
            
        except Exception as e:
            return f"❌ 自定义提供商 {self.provider_name} API 调用失败: {str(e)}"

