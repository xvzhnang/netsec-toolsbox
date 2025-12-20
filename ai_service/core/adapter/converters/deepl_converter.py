# -*- coding: utf-8 -*-
"""
DeepL 协议转换器（翻译服务）
对应 One API 的 relay/adaptor/deepl/
注意：DeepL 主要用于翻译，不是聊天模型
"""
import os
from typing import Dict, Any, Optional
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse


class DeeplConverter(ProtocolConverter):
    """DeepL 协议转换器（翻译服务）"""
    
    def _parse_lang_from_model(self, model_name: str) -> str:
        """
        从模型名称解析目标语言
        例如：deepl-en-US -> EN-US
        """
        # 移除前缀
        if model_name.startswith("deepl-"):
            lang = model_name[6:]  # 移除 "deepl-"
        else:
            lang = "EN-US"  # 默认英语
        
        # 转换为大写
        lang = lang.upper()
        
        # 常见语言代码映射
        lang_map = {
            "EN": "EN-US",
            "ZH": "ZH",
            "JA": "JA",
            "FR": "FR",
            "DE": "DE",
            "ES": "ES",
            "IT": "IT",
            "RU": "RU",
        }
        
        return lang_map.get(lang, lang)
    
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        将 OpenAI 格式转换为 DeepL 格式
        对应 One API 的 ConvertRequest
        """
        # DeepL 只翻译最后一个 user 消息
        text = ""
        if request.messages:
            last_msg = request.messages[-1]
            content = last_msg.get("content", "")
            
            if isinstance(content, str):
                text = content
            elif isinstance(content, list):
                text_parts = []
                for part in content:
                    if isinstance(part, dict) and part.get("type") == "text":
                        text_parts.append(part.get("text", ""))
                text = " ".join(text_parts)
            else:
                text = str(content)
        
        target_lang = self._parse_lang_from_model(self.model)
        
        deepl_request = {
            "target_lang": target_lang,
            "text": [text]  # DeepL 接受文本数组
        }
        
        return deepl_request
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        将 DeepL 格式转换为 OpenAI 格式
        对应 One API 的 ResponseDeepL2OpenAI
        """
        # 检查错误
        if "message" in response_data and "translations" not in response_data:
            error_msg = response_data.get("message", "Unknown error")
            raise Exception(f"DeepL API error: {error_msg}")
        
        # 提取翻译结果
        translations = response_data.get("translations", [])
        content = translations[0].get("text", "") if translations else ""
        
        return OpenAIChatResponse(
            id="deepl-translation",
            object="chat.completion",
            created=0,
            model=self.model,
            choices=[{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            usage=None
        )
    
    def get_request_headers(self) -> Dict[str, str]:
        """获取请求头"""
        # 直接使用配置文件中的 api_key，不再支持环境变量
        api_key = self.config.get("api_key", "")
        if api_key == "not-needed":
            api_key = ""
        
        return {
            "Content-Type": "application/x-www-form-urlencoded",  # DeepL 使用表单格式
            "Authorization": f"DeepL-Auth-Key {api_key}"
        }
    
    def get_request_url(self, base_url: str, endpoint: str) -> str:
        """构建请求 URL"""
        if endpoint.startswith("/"):
            return f"{base_url.rstrip('/')}{endpoint}"
        return f"{base_url.rstrip('/')}/{endpoint}"

