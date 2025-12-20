# -*- coding: utf-8 -*-
"""
Moonshot (Kimi) 协议转换器
Moonshot 使用 OpenAI 兼容 API，只需要处理 endpoint
对应 One API 的 relay/adaptor/openai + moonshot/main.go
"""
import os
from typing import Dict, Any, Optional
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse


class MoonshotConverter(ProtocolConverter):
    """Moonshot (Kimi) 协议转换器 - OpenAI 兼容"""
    
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        Moonshot 使用 OpenAI 兼容格式，直接返回
        """
        # 构建 OpenAI 格式请求
        request_dict = {
            "model": self.model,
            "messages": request.messages,
            "stream": request.stream
        }
        
        if request.temperature is not None:
            request_dict["temperature"] = request.temperature
        if request.max_tokens is not None:
            request_dict["max_tokens"] = request.max_tokens
        if request.top_p is not None:
            request_dict["top_p"] = request.top_p
        if request.frequency_penalty is not None:
            request_dict["frequency_penalty"] = request.frequency_penalty
        if request.presence_penalty is not None:
            request_dict["presence_penalty"] = request.presence_penalty
        if request.stop is not None:
            request_dict["stop"] = request.stop
        if request.user is not None:
            request_dict["user"] = request.user
        
        return request_dict
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        Moonshot 返回 OpenAI 兼容格式，直接转换
        """
        # 检查错误
        if "error" in response_data:
            error = response_data["error"]
            error_msg = error.get("message", "Unknown error")
            error_type = error.get("type", "unknown")
            raise Exception(f"Moonshot API error ({error_type}): {error_msg}")
        
        choices = []
        for i, choice in enumerate(response_data.get("choices", [])):
            choices.append({
                "index": i,
                "message": choice.get("message", {}),
                "finish_reason": choice.get("finish_reason")
            })
        
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
            object=response_data.get("object", "chat.completion"),
            created=response_data.get("created", 0),
            model=response_data.get("model", self.model),
            choices=choices,
            usage=usage
        )
    
    def get_request_headers(self) -> Dict[str, str]:
        """获取请求头"""
        # 直接使用配置文件中的 api_key，不再支持环境变量
        api_key = self.config.get("api_key", "")
        if api_key == "not-needed":
            api_key = ""
        
        return {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}"
        }
    
    def get_request_url(self, base_url: str, endpoint: str) -> str:
        """构建请求 URL"""
        # Moonshot 的标准 endpoint 是 /v1/chat/completions
        if endpoint.startswith("/"):
            return f"{base_url.rstrip('/')}{endpoint}"
        return f"{base_url.rstrip('/')}/{endpoint}"

