# -*- coding: utf-8 -*-
"""
Anthropic Claude 协议转换器
对应 One API 的 relay/adaptor/anthropic/
"""
import os
import json
from typing import Dict, Any, Optional
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse


class AnthropicConverter(ProtocolConverter):
    """Anthropic Claude 协议转换器"""
    
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        将 OpenAI 格式转换为 Anthropic 格式
        对应 One API 的 ConvertRequest
        """
        claude_request = {
            "model": self.model,
            "max_tokens": request.max_tokens or 4096,
            "stream": request.stream,
        }
        
        # 参数转换
        if request.temperature is not None:
            claude_request["temperature"] = request.temperature
        if request.top_p is not None:
            claude_request["top_p"] = request.top_p
        if request.top_k is not None:
            claude_request["top_k"] = request.top_k
        if request.stop is not None:
            claude_request["stop_sequences"] = request.stop if isinstance(request.stop, list) else [request.stop]
        
        # 处理 system message
        system_prompt = ""
        messages = []
        
        for msg in request.messages:
            role = msg.get("role", "")
            content = msg.get("content", "")
            
            if role == "system":
                system_prompt = content if isinstance(content, str) else ""
                continue
            
            # 构建 Claude message
            claude_message = {
                "role": role if role != "assistant" else "assistant",
                "content": []
            }
            
            # 处理文本内容
            if isinstance(content, str):
                claude_message["content"].append({
                    "type": "text",
                    "text": content
                })
            elif isinstance(content, list):
                # 处理多模态内容
                for part in content:
                    if isinstance(part, dict):
                        if part.get("type") == "text":
                            claude_message["content"].append({
                                "type": "text",
                                "text": part.get("text", "")
                            })
                        elif part.get("type") == "image_url":
                            # 图片处理（简化版，实际需要 base64 转换）
                            image_url = part.get("image_url", {}).get("url", "")
                            claude_message["content"].append({
                                "type": "image",
                                "source": {
                                    "type": "url",
                                    "url": image_url
                                }
                            })
            
            messages.append(claude_message)
        
        claude_request["messages"] = messages
        if system_prompt:
            claude_request["system"] = system_prompt
        
        # 处理 tools（简化版）
        if request.tools:
            claude_tools = []
            for tool in request.tools:
                if isinstance(tool, dict) and "function" in tool:
                    func = tool["function"]
                    params = func.get("parameters", {})
                    if isinstance(params, dict):
                        claude_tools.append({
                            "name": func.get("name", ""),
                            "description": func.get("description", ""),
                            "input_schema": {
                                "type": params.get("type", "object"),
                                "properties": params.get("properties", {}),
                                "required": params.get("required", [])
                            }
                        })
            if claude_tools:
                claude_request["tools"] = claude_tools
                claude_request["tool_choice"] = {"type": "auto"}
        
        return claude_request
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        将 Anthropic 格式转换为 OpenAI 格式
        对应 One API 的 ResponseClaude2OpenAI
        """
        # 提取响应文本
        content_list = response_data.get("content", [])
        response_text = ""
        tool_calls = []
        
        for content_item in content_list:
            if isinstance(content_item, dict):
                content_type = content_item.get("type", "")
                if content_type == "text":
                    response_text += content_item.get("text", "")
                elif content_type == "tool_use":
                    # 处理 tool use
                    tool_calls.append({
                        "id": content_item.get("id", ""),
                        "type": "function",
                        "function": {
                            "name": content_item.get("name", ""),
                            "arguments": json.dumps(content_item.get("input", {}))
                        }
                    })
        
        # 构建 OpenAI 格式的响应
        choices = [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": response_text,
                "tool_calls": tool_calls if tool_calls else None
            },
            "finish_reason": self._convert_stop_reason(response_data.get("stop_reason"))
        }]
        
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
            id=f"chatcmpl-{response_data.get('id', 'unknown')}",
            object="chat.completion",
            created=0,  # Anthropic 不返回 created 时间
            model=response_data.get("model", self.model),
            choices=choices,
            usage=usage
        )
    
    def _convert_stop_reason(self, reason: Optional[str]) -> Optional[str]:
        """转换停止原因"""
        if not reason:
            return None
        
        mapping = {
            "end_turn": "stop",
            "stop_sequence": "stop",
            "max_tokens": "length",
            "tool_use": "tool_calls"
        }
        return mapping.get(reason, reason)
    
    def get_request_headers(self) -> Dict[str, str]:
        """获取请求头"""
        headers = {
            "Content-Type": "application/json",
            "x-api-key": self.config.get("api_key", ""),
            "anthropic-version": "2023-06-01",
            "anthropic-beta": "messages-2023-12-15"
        }
        
        # 支持环境变量
        api_key = self.config.get("api_key", "")
        if isinstance(api_key, str) and api_key.startswith("ENV:"):
            env_var = api_key[4:]
            api_key = os.environ.get(env_var, "")
        
        headers["x-api-key"] = api_key
        
        # 特殊模型支持
        if "claude-3-5-sonnet" in self.model:
            headers["anthropic-beta"] = "max-tokens-3-5-sonnet-2024-07-15"
        
        return headers
    
    def handle_error(self, response_data: Dict[str, Any], status_code: int) -> Optional[Exception]:
        """处理错误响应"""
        if "error" in response_data:
            error = response_data["error"]
            if isinstance(error, dict):
                error_type = error.get("type", "unknown")
                error_msg = error.get("message", "Unknown error")
                return Exception(f"Anthropic API error ({error_type}): {error_msg}")
        
        return super().handle_error(response_data, status_code)

