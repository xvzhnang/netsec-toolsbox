# -*- coding: utf-8 -*-
"""
智谱 AI (Zhipu) 协议转换器
对应 One API 的 relay/adaptor/zhipu/
"""
import os
import json
import time
import jwt
from typing import Dict, Any, Optional
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse


class ZhipuConverter(ProtocolConverter):
    """智谱 AI (Zhipu) 协议转换器"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self._token_cache = {}  # 简单的 token 缓存
    
    def _get_token(self) -> str:
        """
        获取 Zhipu JWT Token
        对应 One API 的 GetToken
        """
        api_key = self.config.get("api_key", "")
        if isinstance(api_key, str) and api_key.startswith("ENV:"):
            env_var = api_key[4:]
            api_key = os.environ.get(env_var, "")
        
        # 检查缓存
        if api_key in self._token_cache:
            token_data = self._token_cache[api_key]
            if time.time() < token_data["expires_at"]:
                return token_data["token"]
        
        # 生成新 token
        parts = api_key.split(".")
        if len(parts) != 2:
            raise ValueError("Invalid Zhipu API key format (should be id.secret)")
        
        api_key_id = parts[0]
        api_key_secret = parts[1]
        
        exp_seconds = 24 * 3600
        exp_millis = int((time.time() + exp_seconds) * 1000)
        
        payload = {
            "api_key": api_key_id,
            "exp": exp_millis,
            "timestamp": int(time.time() * 1000)
        }
        
        try:
            token = jwt.encode(payload, api_key_secret, algorithm="HS256", headers={
                "alg": "HS256",
                "sign_type": "SIGN"
            })
            
            # 缓存 token
            self._token_cache[api_key] = {
                "token": token,
                "expires_at": time.time() + exp_seconds - 3600  # 提前 1 小时过期
            }
            
            return token
        except Exception as e:
            raise Exception(f"Failed to generate Zhipu token: {e}")
    
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        将 OpenAI 格式转换为 Zhipu 格式
        对应 One API 的 ConvertRequest
        """
        messages = []
        for msg in request.messages:
            role = msg.get("role", "")
            content = msg.get("content", "")
            
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
            
            messages.append({
                "role": role,
                "content": text_content
            })
        
        zhipu_request = {
            "prompt": messages,
            "incremental": False
        }
        
        if request.temperature is not None:
            zhipu_request["temperature"] = request.temperature
        if request.top_p is not None:
            zhipu_request["top_p"] = request.top_p
        
        return zhipu_request
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        将 Zhipu 格式转换为 OpenAI 格式
        对应 One API 的 responseZhipu2OpenAI
        """
        if not response_data.get("success", False):
            error_msg = response_data.get("msg", "Unknown error")
            error_code = response_data.get("code", "unknown")
            raise Exception(f"Zhipu API error ({error_code}): {error_msg}")
        
        data = response_data.get("data", {})
        choices_data = data.get("choices", [])
        
        choices = []
        for i, choice in enumerate(choices_data):
            content = choice.get("content", "").strip('"')
            choices.append({
                "index": i,
                "message": {
                    "role": choice.get("role", "assistant"),
                    "content": content
                },
                "finish_reason": "stop" if i == len(choices_data) - 1 else None
            })
        
        # 处理 usage
        usage = None
        if "usage" in data:
            usage_data = data["usage"]
            usage = {
                "prompt_tokens": usage_data.get("prompt_tokens", 0),
                "completion_tokens": usage_data.get("completion_tokens", 0),
                "total_tokens": usage_data.get("total_tokens", 0)
            }
        
        return OpenAIChatResponse(
            id=data.get("task_id", "unknown"),
            object="chat.completion",
            created=0,
            model="chatglm",
            choices=choices,
            usage=usage
        )
    
    def get_request_headers(self) -> Dict[str, str]:
        """获取请求头"""
        token = self._get_token()
        return {
            "Content-Type": "application/json",
            "Authorization": token
        }

