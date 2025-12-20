# -*- coding: utf-8 -*-
"""
Cohere 协议转换器
对应 One API 的 relay/adaptor/cohere/
"""
import os
from typing import Dict, Any, Optional
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse


class CohereConverter(ProtocolConverter):
    """Cohere 协议转换器"""
    
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        将 OpenAI 格式转换为 Cohere 格式
        对应 One API 的 ConvertRequest
        """
        # 提取消息
        chat_history = []
        message = ""
        
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
            
            if role == "user":
                # 最后一个 user 消息作为当前消息
                message = text_content
            else:
                # 其他消息作为历史记录
                cohere_role = "CHATBOT" if role == "assistant" else "SYSTEM" if role == "system" else "USER"
                chat_history.append({
                    "role": cohere_role,
                    "message": text_content
                })
        
        cohere_request = {
            "model": self.model or "command-r",
            "message": message,
            "chat_history": chat_history,
            "stream": request.stream,
        }
        
        # 移除 -internet 后缀（如果存在），并添加 web search connector
        if cohere_request["model"].endswith("-internet"):
            cohere_request["model"] = cohere_request["model"][:-9]  # 移除 -internet
            cohere_request["connectors"] = [{"id": "web-search"}]
        
        if request.temperature is not None:
            cohere_request["temperature"] = request.temperature
        if request.max_tokens is not None:
            cohere_request["max_tokens"] = request.max_tokens
        if request.top_p is not None:
            cohere_request["p"] = request.top_p
        if request.frequency_penalty is not None:
            cohere_request["frequency_penalty"] = request.frequency_penalty
        if request.presence_penalty is not None:
            cohere_request["presence_penalty"] = request.presence_penalty
        
        return cohere_request
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        将 Cohere 格式转换为 OpenAI 格式
        对应 One API 的 ResponseCohere2OpenAI
        """
        # 检查错误
        if "message" in response_data and "response_id" not in response_data:
            error_msg = response_data.get("message", "Unknown error")
            raise Exception(f"Cohere API error: {error_msg}")
        
        # Cohere 响应可能在 response 字段中，也可能直接在顶层
        response_obj = response_data.get("response", response_data)
        if not isinstance(response_obj, dict):
            response_obj = response_data
        
        # 提取响应内容
        text = response_obj.get("text", "")
        finish_reason = response_obj.get("finish_reason", "")
        
        # 如果 finish_reason 是字典，提取实际值
        if isinstance(finish_reason, dict):
            finish_reason = finish_reason.get("finish_reason", "")
        
        # 转换 finish_reason
        if finish_reason == "COMPLETE":
            finish_reason = "stop"
        
        # 提取 usage
        usage = None
        meta = response_data.get("meta", {})
        if meta and "tokens" in meta:
            tokens = meta["tokens"]
            usage = {
                "prompt_tokens": tokens.get("input_tokens", 0),
                "completion_tokens": tokens.get("output_tokens", 0),
                "total_tokens": tokens.get("input_tokens", 0) + tokens.get("output_tokens", 0)
            }
        
        # 获取 response_id
        response_id = response_obj.get("response_id", response_data.get("response_id", "unknown"))
        
        return OpenAIChatResponse(
            id=f"chatcmpl-{response_id}",
            object="chat.completion",
            created=0,
            model=self.model or "command-r",
            choices=[{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": text
                },
                "finish_reason": finish_reason
            }],
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

