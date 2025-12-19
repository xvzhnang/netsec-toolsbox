# -*- coding: utf-8 -*-
"""
Google Gemini 协议转换器
对应 One API 的 relay/adaptor/gemini/
"""
import os
import json
from typing import Dict, Any, Optional, List
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse


class GeminiConverter(ProtocolConverter):
    """Google Gemini 协议转换器"""
    
    # 支持 system instruction 的模型列表
    MODELS_SUPPORT_SYSTEM_INSTRUCTION = [
        "gemini-2.0-flash",
        "gemini-2.0-flash-exp",
        "gemini-2.0-flash-thinking-exp-01-21"
    ]
    
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        将 OpenAI 格式转换为 Gemini 格式
        对应 One API 的 ConvertRequest
        """
        gemini_request = {
            "contents": [],
            "safety_settings": [
                {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"},
                {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE"},
                {"category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_NONE"},
                {"category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE"},
                {"category": "HARM_CATEGORY_CIVIC_INTEGRITY", "threshold": "BLOCK_NONE"}
            ],
            "generation_config": {}
        }
        
        # 生成配置
        if request.temperature is not None:
            gemini_request["generation_config"]["temperature"] = request.temperature
        if request.top_p is not None:
            gemini_request["generation_config"]["topP"] = request.top_p
        if request.max_tokens is not None:
            gemini_request["generation_config"]["maxOutputTokens"] = request.max_tokens
        if request.stop is not None:
            stop_seqs = request.stop if isinstance(request.stop, list) else [request.stop]
            gemini_request["generation_config"]["stopSequences"] = stop_seqs
        
        # 处理 system instruction
        system_instruction = None
        should_add_dummy_model = False
        
        for msg in request.messages:
            role = msg.get("role", "")
            content = msg.get("content", "")
            
            # 处理 system message
            if role == "system":
                if self.model in self.MODELS_SUPPORT_SYSTEM_INSTRUCTION:
                    system_instruction = content if isinstance(content, str) else ""
                else:
                    # 转换为 user message
                    should_add_dummy_model = True
                    role = "user"
            
            # 转换 role
            if role == "assistant":
                role = "model"
            
            # 构建 parts
            parts = []
            if isinstance(content, str):
                parts.append({"text": content})
            elif isinstance(content, list):
                for part in content:
                    if isinstance(part, dict):
                        if part.get("type") == "text":
                            parts.append({"text": part.get("text", "")})
                        elif part.get("type") == "image_url":
                            # 图片处理（简化版）
                            image_url = part.get("image_url", {}).get("url", "")
                            parts.append({
                                "inlineData": {
                                    "mimeType": "image/jpeg",  # 简化，实际需要检测
                                    "data": image_url  # 简化，实际需要 base64
                                }
                            })
            
            if parts:
                gemini_request["contents"].append({
                    "role": role,
                    "parts": parts
                })
        
        # 设置 system instruction
        if system_instruction:
            gemini_request["system_instruction"] = {
                "parts": [{"text": system_instruction}]
            }
        
        # 如果需要添加 dummy model message
        if should_add_dummy_model and gemini_request["contents"]:
            gemini_request["contents"].append({
                "role": "model",
                "parts": [{"text": "Okay"}]
            })
        
        # 处理 tools
        if request.tools:
            functions = []
            for tool in request.tools:
                if isinstance(tool, dict) and "function" in tool:
                    functions.append(tool["function"])
            
            if functions:
                gemini_request["tools"] = [{
                    "function_declarations": functions
                }]
        
        return gemini_request
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        将 Gemini 格式转换为 OpenAI 格式
        对应 One API 的 responseGeminiChat2OpenAI
        """
        candidates = response_data.get("candidates", [])
        
        if not candidates:
            raise Exception("No candidates returned from Gemini API")
        
        choices = []
        for i, candidate in enumerate(candidates):
            content = candidate.get("content", {})
            parts = content.get("parts", [])
            
            # 提取文本内容
            text_parts = []
            tool_calls = []
            
            for part in parts:
                if "text" in part:
                    text_parts.append(part["text"])
                elif "functionCall" in part:
                    func_call = part["functionCall"]
                    tool_calls.append({
                        "id": f"call_{i}",
                        "type": "function",
                        "function": {
                            "name": func_call.get("name", ""),
                            "arguments": json.dumps(func_call.get("args", {}))
                        }
                    })
            
            message = {
                "role": "assistant",
                "content": "\n".join(text_parts) if text_parts else "",
            }
            
            if tool_calls:
                message["tool_calls"] = tool_calls
                message["content"] = None
            
            choices.append({
                "index": i,
                "message": message,
                "finish_reason": candidate.get("finishReason", "stop")
            })
        
        # 处理 usage（Gemini 通常不返回 usage，需要估算）
        usage = None
        
        return OpenAIChatResponse(
            id=f"chatcmpl-gemini-{hash(str(response_data))}",
            object="chat.completion",
            created=0,
            model=self.model,
            choices=choices,
            usage=usage
        )
    
    def get_request_headers(self) -> Dict[str, str]:
        """获取请求头"""
        api_key = self.config.get("api_key", "")
        if isinstance(api_key, str) and api_key.startswith("ENV:"):
            env_var = api_key[4:]
            api_key = os.environ.get(env_var, "")
        
        return {
            "Content-Type": "application/json",
            "x-goog-api-key": api_key
        }
    
    def get_request_url(self, base_url: str, endpoint: str) -> str:
        """构建请求 URL"""
        # Gemini 的 URL 格式特殊
        version = "v1"
        if "gemini-2.0" in self.model or "gemini-1.5" in self.model:
            version = "v1beta"
        
        # 替换占位符
        endpoint = endpoint.replace("{model}", self.model)
        endpoint = endpoint.replace("{version}", version)
        
        if endpoint.startswith("/"):
            return f"{base_url.rstrip('/')}{endpoint}"
        return f"{base_url.rstrip('/')}/{endpoint}"

