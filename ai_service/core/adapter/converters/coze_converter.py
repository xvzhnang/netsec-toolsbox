# -*- coding: utf-8 -*-
"""
Coze 协议转换器
对应 One API 的 relay/adaptor/coze/
"""
import os
from typing import Dict, Any, Optional
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse


class CozeConverter(ProtocolConverter):
    """Coze 协议转换器"""
    
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        将 OpenAI 格式转换为 Coze 格式
        对应 One API 的 ConvertRequest
        """
        # Coze 使用 bot_id，需要从 model 中提取
        bot_id = self.model
        if bot_id.startswith("bot-"):
            bot_id = bot_id[4:]
        
        # 提取消息：最后一个作为 query，其他作为 chat_history
        chat_history = []
        query = ""
        
        for i, msg in enumerate(request.messages):
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
            
            if i == len(request.messages) - 1:
                # 最后一个消息作为 query
                query = text_content
            else:
                # 其他消息作为历史记录
                chat_history.append({
                    "role": role,
                    "content": text_content
                })
        
        coze_request = {
            "bot_id": bot_id,
            "query": query,
            "chat_history": chat_history,
            "stream": request.stream,
        }
        
        if request.user is not None:
            coze_request["user"] = request.user
        
        return coze_request
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        将 Coze 格式转换为 OpenAI 格式
        对应 One API 的 ResponseCoze2OpenAI
        """
        # Coze 响应格式：messages 数组，需要找到 type 为 "answer" 的消息
        messages = response_data.get("messages", [])
        content = ""
        
        for msg in messages:
            if msg.get("type") == "answer":
                content = msg.get("content", "")
                break
        
        # 转换 finish_reason
        finish_reason = "stop"
        for msg in messages:
            if msg.get("type") == "answer":
                stop_reason = msg.get("stop_reason")
                if stop_reason == "end_turn" or stop_reason == "stop_sequence":
                    finish_reason = "stop"
                elif stop_reason == "max_tokens":
                    finish_reason = "length"
                break
        
        return OpenAIChatResponse(
            id=response_data.get("conversation_id", "unknown"),
            object="chat.completion",
            created=0,
            model=self.model,
            choices=[{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": finish_reason
            }],
            usage=None  # Coze 可能不返回 usage
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

