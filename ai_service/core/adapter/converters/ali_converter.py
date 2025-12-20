# -*- coding: utf-8 -*-
"""
阿里通义千问协议转换器
对应 One API 的 relay/adaptor/ali/
"""
import os
from typing import Dict, Any, Optional
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse


class AliConverter(ProtocolConverter):
    """阿里通义千问协议转换器"""
    
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        将 OpenAI 格式转换为阿里格式
        对应 One API 的 ConvertRequest
        """
        messages = []
        enable_search = False
        
        # 检查是否启用搜索（模型名以 -internet 结尾）
        ali_model = self.model
        if ali_model.endswith("-internet"):
            enable_search = True
            ali_model = ali_model[:-9]  # 移除 -internet 后缀
        
        for msg in request.messages:
            role = msg.get("role", "").lower()
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
        
        # 构建参数
        parameters = {
            "enable_search": enable_search,
            "incremental_output": request.stream,
            "result_format": "message"
        }
        
        if request.temperature is not None:
            parameters["temperature"] = request.temperature
        if request.top_p is not None:
            # 限制 top_p 最大值
            parameters["top_p"] = min(request.top_p, 0.9999)
        if request.top_k is not None:
            parameters["top_k"] = request.top_k
        if request.max_tokens is not None:
            parameters["max_tokens"] = request.max_tokens
        if request.seed is not None:
            parameters["seed"] = int(request.seed)
        if request.tools:
            parameters["tools"] = request.tools
        
        ali_request = {
            "model": ali_model,
            "input": {
                "messages": messages
            },
            "parameters": parameters
        }
        
        return ali_request
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        将阿里格式转换为 OpenAI 格式
        对应 One API 的 responseAli2OpenAI
        """
        # 检查错误
        if "code" in response_data and response_data["code"]:
            error_code = response_data.get("code", "unknown")
            error_msg = response_data.get("message", "Unknown error")
            raise Exception(f"Ali API error ({error_code}): {error_msg}")
        
        output = response_data.get("output", {})
        choices_data = output.get("choices", [])
        
        choices = []
        for i, choice in enumerate(choices_data):
            message = choice.get("message", {})
            choices.append({
                "index": i,
                "message": {
                    "role": message.get("role", "assistant"),
                    "content": message.get("content", "")
                },
                "finish_reason": choice.get("finish_reason", "stop")
            })
        
        # 处理 usage
        usage = None
        if "usage" in response_data:
            usage_data = response_data["usage"]
            usage = {
                "prompt_tokens": usage_data.get("input_tokens", 0),
                "completion_tokens": usage_data.get("output_tokens", 0),
                "total_tokens": usage_data.get("input_tokens", 0) + usage_data.get("output_tokens", 0)
            }
        
        return OpenAIChatResponse(
            id=response_data.get("request_id", "unknown"),
            object="chat.completion",
            created=0,
            model="qwen",
            choices=choices,
            usage=usage
        )
    
    def get_request_headers(self) -> Dict[str, str]:
        """获取请求头"""
        # 直接使用配置文件中的 api_key，不再支持环境变量
        api_key = self.config.get("api_key", "")
        if api_key == "not-needed":
            api_key = ""
        
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}"
        }
        
        # 流式请求需要特殊头
        if self.config.get("stream", False):
            headers["Accept"] = "text/event-stream"
            headers["X-DashScope-SSE"] = "enable"
        
        # 插件支持
        plugin = self.config.get("config", {}).get("plugin", "")
        if plugin:
            headers["X-DashScope-Plugin"] = plugin
        
        return headers

