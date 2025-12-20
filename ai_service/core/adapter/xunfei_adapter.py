# -*- coding: utf-8 -*-
"""
讯飞星火 WebSocket 适配器
对应 One API 的 relay/adaptor/xunfei/
"""
import os
import json
import hmac
import hashlib
import base64
import time
import urllib.parse
from typing import Dict, Any, Optional
from datetime import datetime

from .websocket_adapter import WebSocketAdapter
from .base_adapter import OpenAIChatRequest, OpenAIStreamChunk


class XunfeiAdapter(WebSocketAdapter):
    """
    讯飞星火 WebSocket 适配器
    """
    
    # 域名映射（对应 One API 的 domain.go）
    DOMAIN_MAP = {
        "general": "general",
        "generalv2": "generalv2",
        "generalv3": "generalv3",
        "generalv3.5": "generalv3.5",
        "4.0Ultra": "4.0Ultra",
    }
    
    # 版本到域名的映射
    VERSION_DOMAIN_MAP = {
        "v1.1": "general",
        "v2.1": "generalv2",
        "v3.1": "generalv3",
        "v3.5": "generalv3.5",
        "v4.0": "4.0Ultra",
    }
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        
        # 直接使用配置文件中的值，不再支持环境变量
        # api_key, api_secret, app_id 已经在父类中从 config 读取
        # 如果值为 "not-needed" 或空字符串，则设为空字符串
        if self.api_key == 'not-needed':
            self.api_key = ''
        if self.api_secret == 'not-needed':
            self.api_secret = ''
        if self.app_id == 'not-needed':
            self.app_id = ''
        
        # 从配置或模型名称推断版本
        api_version = config.get('config', {}).get('api_version', 'v3.5')
        if not self.domain:
            self.domain = self.VERSION_DOMAIN_MAP.get(api_version, 'generalv3.5')
        
        # 基础 URL（如果没有配置，使用默认）
        if not self.ws_url or self.ws_url == 'ws://' or self.ws_url == 'wss://':
            self.ws_url = "wss://spark-api.xf-yun.com"
        
        # 构建基础路径
        version_path_map = {
            "v1.1": "/v1.1/chat",
            "v2.1": "/v2.1/chat",
            "v3.1": "/v3.1/chat",
            "v3.5": "/v3.5/chat",
            "v4.0": "/v4.0/chat",
        }
        self.endpoint = config.get('endpoint') or version_path_map.get(api_version, "/v3.5/chat")
    
    @property
    def adapter_type(self) -> str:
        return "websocket_xunfei"
    
    def _hmac_sha256_base64(self, data: str, key: str) -> str:
        """HMAC-SHA256 然后 Base64 编码"""
        mac = hmac.new(key.encode(), data.encode(), hashlib.sha256)
        return base64.b64encode(mac.digest()).decode()
    
    async def _build_auth_url(self) -> str:
        """
        构建讯飞星火认证 URL
        对应 One API 的 buildXunfeiAuthUrl
        """
        host = urllib.parse.urlparse(self.ws_url).netloc
        path = self.endpoint
        
        # 生成日期（RFC1123 格式）
        date = datetime.utcnow().strftime('%a, %d %b %Y %H:%M:%S GMT')
        
        # 构建签名字符串
        sign_string = f"host: {host}\ndate: {date}\nGET {path} HTTP/1.1"
        
        # 计算签名
        signature = self._hmac_sha256_base64(sign_string, self.api_secret)
        
        # 构建 authorization header
        authorization_origin = f'hmac username="{self.api_key}", algorithm="hmac-sha256", headers="host date request-line", signature="{signature}"'
        authorization = base64.b64encode(authorization_origin.encode()).decode()
        
        # 构建查询参数
        params = {
            "authorization": authorization,
            "date": date,
            "host": host
        }
        
        # 构建完整 URL
        query_string = urllib.parse.urlencode(params)
        auth_url = f"{self.ws_url}{path}?{query_string}"
        
        return auth_url
    
    async def _convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        转换 OpenAI 请求到讯飞星火格式
        对应 One API 的 requestOpenAI2Xunfei
        """
        # 转换消息
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
        
        # 构建讯飞请求
        xunfei_request = {
            "header": {
                "app_id": self.app_id
            },
            "parameter": {
                "chat": {
                    "domain": self.domain,
                    "temperature": request.temperature or 0.7,
                    "max_tokens": request.max_tokens or 2048
                }
            },
            "payload": {
                "message": {
                    "text": messages
                }
            }
        }
        
        # 支持 functions（如果是支持的版本）
        if self.domain in ["generalv3", "4.0Ultra"] and request.messages:
            # 如果有 tools，转换 functions
            # 这里简化处理，实际可以根据需求扩展
            pass
        
        return xunfei_request
    
    async def _parse_response(self, data: Dict[str, Any]) -> Optional[OpenAIStreamChunk]:
        """
        解析讯飞星火响应为 OpenAI 格式
        对应 One API 的 streamResponseXunfei2OpenAI
        """
        # 检查错误
        if "header" in data and data["header"].get("code") != 0:
            error_code = data["header"].get("code", "unknown")
            error_msg = data["header"].get("message", "Unknown error")
            raise Exception(f"讯飞星火 API 错误 ({error_code}): {error_msg}")
        
        # 提取响应内容
        payload = data.get("payload", {})
        choices = payload.get("choices", {})
        text_items = choices.get("text", [])
        
        if not text_items:
            # 如果没有文本内容，可能是中间状态，返回 None
            return None
        
        content = text_items[0].get("content", "")
        status = choices.get("status", 0)
        
        # 提取 usage
        usage = payload.get("usage", {})
        text_usage = usage.get("text", {})
        
        usage_dict = None
        if text_usage:
            usage_dict = {
                "prompt_tokens": text_usage.get("prompt_tokens", 0),
                "completion_tokens": text_usage.get("completion_tokens", 0),
                "total_tokens": text_usage.get("total_tokens", 0)
            }
        
        # 构建 OpenAI 格式的 chunk
        finish_reason = None
        if status == 2:  # 状态 2 表示完成
            finish_reason = "stop"
        
        return OpenAIStreamChunk(
            id=f"chatcmpl-{int(time.time())}",
            created=int(time.time()),
            model="SparkDesk",
            choices=[{
                "index": 0,
                "delta": {
                    "content": content
                },
                "finish_reason": finish_reason
            }],
            usage=usage_dict
        )
    
    async def _is_response_complete(self, data: Dict[str, Any]) -> bool:
        """
        判断讯飞星火响应是否完成
        """
        payload = data.get("payload", {})
        choices = payload.get("choices", {})
        status = choices.get("status", 0)
        
        # 状态 2 表示完成
        return status == 2

