# -*- coding: utf-8 -*-
"""
百度文心一言协议转换器
对应 One API 的 relay/adaptor/baidu/
"""
import os
import json
import time
from typing import Dict, Any, Optional
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse

try:
    import aiohttp
    HAS_AIOHTTP = True
except ImportError:
    HAS_AIOHTTP = False


class BaiduConverter(ProtocolConverter):
    """百度文心一言协议转换器"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self._access_token_cache = {}  # access_token 缓存
    
    async def _get_access_token(self) -> str:
        """
        获取百度 Access Token
        对应 One API 的 GetAccessToken
        """
        api_key = self.config.get("api_key", "")
        if isinstance(api_key, str) and api_key.startswith("ENV:"):
            env_var = api_key[4:]
            api_key = os.environ.get(env_var, "")
        
        # 检查缓存
        if api_key in self._access_token_cache:
            token_data = self._access_token_cache[api_key]
            if time.time() < token_data["expires_at"]:
                return token_data["access_token"]
        
        # 获取新 token
        parts = api_key.split("|")
        if len(parts) != 2:
            raise ValueError("Invalid Baidu API key format (should be client_id|client_secret)")
        
        client_id = parts[0]
        client_secret = parts[1]
        
        token_url = f"https://aip.baidubce.com/oauth/2.0/token?grant_type=client_credentials&client_id={client_id}&client_secret={client_secret}"
        
        if not HAS_AIOHTTP:
            raise Exception("aiohttp is required for Baidu converter")
        
        async with aiohttp.ClientSession() as session:
            async with session.post(token_url) as resp:
                if resp.status != 200:
                    raise Exception(f"Failed to get Baidu access token: HTTP {resp.status}")
                
                token_data = await resp.json()
                
                if "error" in token_data:
                    error = token_data.get("error", "")
                    error_desc = token_data.get("error_description", "")
                    raise Exception(f"Baidu token error: {error}: {error_desc}")
                
                access_token = token_data.get("access_token", "")
                if not access_token:
                    raise Exception("Empty access token from Baidu")
                
                expires_in = token_data.get("expires_in", 2592000)  # 默认 30 天
                
                # 缓存 token
                self._access_token_cache[api_key] = {
                    "access_token": access_token,
                    "expires_at": time.time() + expires_in - 3600  # 提前 1 小时过期
                }
                
                return access_token
    
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        将 OpenAI 格式转换为百度格式
        对应 One API 的 ConvertRequest
        """
        baidu_request = {
            "messages": [],
            "stream": request.stream
        }
        
        system_prompt = ""
        
        for msg in request.messages:
            role = msg.get("role", "")
            content = msg.get("content", "")
            
            if role == "system":
                system_prompt = content if isinstance(content, str) else ""
                continue
            
            # 提取文本内容
            if isinstance(content, str):
                text_content = content
            elif isinstance(content, list):
                text_parts = []
                for part in content:
                    if isinstance(part, dict) and part.get("type") == "text":
                        text_parts.append(part.get("text", ""))
                text_content = " ".join(text_parts)
            else:
                text_content = str(content)
            
            baidu_request["messages"].append({
                "role": role,
                "content": text_content
            })
        
        if system_prompt:
            baidu_request["system"] = system_prompt
        
        # 参数转换
        if request.temperature is not None:
            baidu_request["temperature"] = request.temperature
        if request.top_p is not None:
            baidu_request["top_p"] = request.top_p
        if request.frequency_penalty is not None:
            baidu_request["penalty_score"] = request.frequency_penalty
        if request.max_tokens is not None:
            baidu_request["max_output_tokens"] = request.max_tokens
        if request.user is not None:
            baidu_request["user_id"] = request.user
        
        return baidu_request
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        将百度格式转换为 OpenAI 格式
        对应 One API 的 responseBaidu2OpenAI
        """
        # 检查错误
        if "error_msg" in response_data or "error_code" in response_data:
            error_code = response_data.get("error_code", "unknown")
            error_msg = response_data.get("error_msg", "Unknown error")
            raise Exception(f"Baidu API error ({error_code}): {error_msg}")
        
        result = response_data.get("result", "")
        
        choices = [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": result
            },
            "finish_reason": "stop"
        }]
        
        # 处理 usage
        usage = None
        if "usage" in response_data:
            usage_data = response_data["usage"]
            usage = {
                "prompt_tokens": usage_data.get("prompt_tokens", 0),
                "completion_tokens": usage_data.get("completion_tokens", 0),
                "total_tokens": usage_data.get("total_tokens", 0)
            }
        
        return OpenAIChatResponse(
            id=response_data.get("id", "unknown"),
            object="chat.completion",
            created=response_data.get("created", 0),
            model="ernie-bot",
            choices=choices,
            usage=usage
        )
    
    def get_request_headers(self) -> Dict[str, str]:
        """获取请求头"""
        return {
            "Content-Type": "application/json"
        }
    
    async def get_request_url(self, base_url: str, endpoint: str) -> str:
        """
        构建请求 URL（包含 access_token）
        注意：这是异步方法，因为需要获取 access_token
        """
        access_token = await self._get_access_token()
        
        # 构建完整 URL
        if endpoint.startswith("/"):
            url = f"{base_url.rstrip('/')}{endpoint}"
        else:
            url = f"{base_url.rstrip('/')}/{endpoint}"
        
        # 添加 access_token 参数
        separator = "&" if "?" in url else "?"
        url = f"{url}{separator}access_token={access_token}"
        
        return url

