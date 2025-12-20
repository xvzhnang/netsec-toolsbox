# -*- coding: utf-8 -*-
"""
Custom HTTP 适配器
支持需要协议转换的模型（如 Claude, Gemini, Zhipu, Baidu, Ali 等）
对应 One API 的各个非 OpenAI 兼容适配器
"""
import os
from typing import Dict, Any, Optional

try:
    import aiohttp
    HAS_AIOHTTP = True
except ImportError:
    HAS_AIOHTTP = False

from .base_adapter import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse
from .converters.registry import get_converter


class CustomHTTPAdapter(ChatAdapter):
    """
    Custom HTTP 适配器
    支持通过协议转换器处理非 OpenAI 兼容的 API
    """
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        # 直接使用配置文件中的 api_key，不再支持环境变量
        self.api_key = config.get('api_key')
        # 如果 api_key 是 "not-needed"、空字符串或 "ENV:" 开头的环境变量占位符，则设为 None
        if self.api_key == 'not-needed' or self.api_key == '' or (isinstance(self.api_key, str) and self.api_key.startswith('ENV:')):
            self.api_key = None
        
        self.base_url = config.get('base_url')
        self.endpoint = config.get('endpoint', '/chat/completions')
        self.request_format = config.get('request_format', 'openai')
        self.response_format = config.get('response_format', 'openai')
        self.model = config.get('model', self.model_id)
        
        # 获取协议转换器
        self.converter = get_converter(self.request_format, config)
        if not self.converter:
            raise ValueError(f"Unsupported request format: {self.request_format}")
    
    @property
    def adapter_type(self) -> str:
        return "custom_http"
    
    def is_available(self) -> bool:
        """检查适配器是否可用"""
        if not HAS_AIOHTTP:
            return False
        
        if not self.converter:
            return False
        
        # 某些转换器可能需要特殊检查（如百度需要 access_token）
        return self.api_key is not None and self.base_url is not None
    
    async def chat(
        self,
        request: OpenAIChatRequest,
        timeout: Optional[int] = None
    ) -> OpenAIChatResponse:
        """
        发送聊天请求
        使用协议转换器进行请求/响应转换
        """
        if not self.is_available():
            raise ValueError(f"模型 {self.model_id} 未配置或不可用")
        
        # 使用转换器转换请求
        converted_request = self.converter.convert_request(request)
        
        # 构建 URL
        # 特殊处理：百度需要异步获取 access_token
        if self.request_format == 'baidu':
            # 百度需要先获取 access_token，然后构建 URL
            access_token = await self.converter._get_access_token()
            url = f"{self.base_url.rstrip('/')}/{self.endpoint.lstrip('/')}"
            separator = "&" if "?" in url else "?"
            url = f"{url}{separator}access_token={access_token}"
        elif hasattr(self.converter, 'get_request_url'):
            url = self.converter.get_request_url(self.base_url, self.endpoint)
        else:
            url = f"{self.base_url.rstrip('/')}/{self.endpoint.lstrip('/')}"
        
        # 获取请求头
        headers = self.converter.get_request_headers()
        
        # 特殊处理：腾讯需要签名（签名依赖于请求体）
        if self.request_format == 'tencent' and hasattr(self.converter, 'get_signature_for_request'):
            authorization = self.converter.get_signature_for_request(converted_request)
            headers["Authorization"] = authorization
        
        # 特殊处理：DeepL 使用表单格式
        is_form_data = self.request_format == 'deepl'
        if is_form_data:
            import urllib.parse
            # 将 JSON 转换为表单格式
            form_data = {}
            if isinstance(converted_request, dict):
                text_array = converted_request.get("text", [])
                if text_array:
                    form_data["text"] = text_array
                if "target_lang" in converted_request:
                    form_data["target_lang"] = converted_request["target_lang"]
            converted_request = urllib.parse.urlencode(form_data, doseq=True)
        
        # 发送请求
        request_timeout = timeout or self.config.get('timeout', 60)
        
        try:
            async with aiohttp.ClientSession() as session:
                # DeepL 使用表单格式，其他使用 JSON
                if is_form_data:
                    async with session.post(
                        url,
                        headers=headers,
                        data=converted_request,  # 表单数据
                        timeout=aiohttp.ClientTimeout(total=request_timeout)
                    ) as response:
                        response_text = await response.text()
                        try:
                            response_data = await response.json()
                        except:
                            # 如果不是 JSON，构建简单的响应结构
                            response_data = {"translations": [{"text": response_text}]}
                        
                        # 处理错误
                        error = self.converter.handle_error(response_data, response.status)
                        if error:
                            raise error
                        
                        if response.status != 200:
                            error_msg = response_data.get('message', f'HTTP {response.status}') if isinstance(response_data, dict) else f'HTTP {response.status}'
                            raise Exception(f"API 错误: {error_msg}")
                        
                        # 使用转换器转换响应
                        openai_response = self.converter.convert_response(response_data)
                        return openai_response
                else:
                    async with session.post(
                        url,
                        headers=headers,
                        json=converted_request,  # JSON 数据
                        timeout=aiohttp.ClientTimeout(total=request_timeout)
                    ) as response:
                        response_data = await response.json()
                    
                    # 处理错误
                    error = self.converter.handle_error(response_data, response.status)
                    if error:
                        raise error
                    
                    if response.status != 200:
                        error_msg = response_data.get('error', {}).get('message', f'HTTP {response.status}')
                        raise Exception(f"API 错误: {error_msg}")
                    
                    # 使用转换器转换响应
                    openai_response = self.converter.convert_response(response_data)
                    return openai_response
        
        except aiohttp.ClientError as e:
            raise Exception(f"网络请求失败: {str(e)}")
        except Exception as e:
            raise Exception(f"请求处理失败: {str(e)}")

