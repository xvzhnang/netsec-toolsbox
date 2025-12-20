# -*- coding: utf-8 -*-
"""
腾讯混元协议转换器
对应 One API 的 relay/adaptor/tencent/
需要实现 TC3-HMAC-SHA256 签名算法
"""
import os
import json
import hmac
import hashlib
import time
from typing import Dict, Any, Optional
from .base_converter import ProtocolConverter
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse


class TencentConverter(ProtocolConverter):
    """腾讯混元协议转换器"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.action = "ChatCompletions"
        self.version = "2023-09-01"
        self.region = config.get("config", {}).get("region", "ap-beijing")
        
        # 解析 API Key: app_id|secret_id|secret_key
        # 直接使用配置文件中的 api_key，不再支持环境变量
        api_key = config.get("api_key", "")
        if api_key == "not-needed":
            api_key = ""
        
        if "|" in api_key:
            parts = api_key.split("|")
            if len(parts) == 3:
                self.app_id = parts[0]
                self.secret_id = parts[1]
                self.secret_key = parts[2]
            else:
                raise ValueError("Invalid Tencent API key format (should be app_id|secret_id|secret_key)")
        else:
            # 从 config 中读取
            config_obj = config.get("config", {})
            self.app_id = config_obj.get("app_id", "")
            self.secret_id = config_obj.get("secret_id", "") or api_key
            self.secret_key = config_obj.get("secret_key", "")
    
    def _sha256hex(self, s: str) -> str:
        """计算 SHA256 十六进制"""
        return hashlib.sha256(s.encode()).hexdigest()
    
    def _get_signature(self, request_data: Dict[str, Any]) -> str:
        """
        生成腾讯云 TC3-HMAC-SHA256 签名
        对应 One API 的 GetSign
        """
        host = "hunyuan.tencentcloudapi.com"
        http_request_method = "POST"
        canonical_uri = "/"
        canonical_query_string = ""
        
        # 构建规范请求头
        timestamp = int(time.time())
        canonical_headers = f"content-type:application/json\nhost:{host}\nx-tc-action:{self.action.lower()}\n"
        signed_headers = "content-type;host;x-tc-action"
        
        # 计算 payload hash
        payload = json.dumps(request_data, ensure_ascii=False, separators=(',', ':'))
        hashed_request_payload = self._sha256hex(payload)
        
        # 构建规范请求
        canonical_request = f"{http_request_method}\n{canonical_uri}\n{canonical_query_string}\n{canonical_headers}\n{signed_headers}\n{hashed_request_payload}"
        
        # 构建待签名字符串
        algorithm = "TC3-HMAC-SHA256"
        date = time.strftime("%Y-%m-%d", time.gmtime(timestamp))
        credential_scope = f"{date}/hunyuan/tc3_request"
        hashed_canonical_request = self._sha256hex(canonical_request)
        string_to_sign = f"{algorithm}\n{timestamp}\n{credential_scope}\n{hashed_canonical_request}"
        
        # 计算签名
        # 注意：HMAC-SHA256 需要先计算 bytes，然后转换为 hex
        secret_date_bytes = hmac.new(
            ("TC3" + self.secret_key).encode(),
            date.encode(),
            hashlib.sha256
        ).digest()
        
        secret_service_bytes = hmac.new(
            secret_date_bytes,
            "hunyuan".encode(),
            hashlib.sha256
        ).digest()
        
        secret_signing_bytes = hmac.new(
            secret_service_bytes,
            "tc3_request".encode(),
            hashlib.sha256
        ).digest()
        
        signature_bytes = hmac.new(
            secret_signing_bytes,
            string_to_sign.encode(),
            hashlib.sha256
        ).digest()
        
        signature = signature_bytes.hex()
        
        # 构建 Authorization
        authorization = f"{algorithm} Credential={self.secret_id}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
        
        return authorization
    
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        将 OpenAI 格式转换为腾讯混元格式
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
                "Role": role.capitalize(),  # 腾讯要求首字母大写
                "Content": text_content
            })
        
        tencent_request = {
            "Model": self.model,
            "Messages": messages,
            "Stream": request.stream
        }
        
        if request.temperature is not None:
            tencent_request["Temperature"] = request.temperature
        if request.top_p is not None:
            tencent_request["TopP"] = request.top_p
        
        return tencent_request
    
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        将腾讯混元格式转换为 OpenAI 格式
        对应 One API 的 responseTencent2OpenAI
        """
        # 处理响应包装
        if "Response" in response_data:
            response_data = response_data["Response"]
        
        # 检查错误
        if "Error" in response_data and response_data["Error"].get("Code"):
            error = response_data["Error"]
            raise Exception(f"Tencent API error ({error.get('Code', 'unknown')}): {error.get('Message', 'Unknown error')}")
        
        choices_data = response_data.get("Choices", [])
        
        choices = []
        for i, choice in enumerate(choices_data):
            # 腾讯响应中，Message 包含完整消息，Delta 包含增量消息（流式）
            message = choice.get("Message", {})
            if not message:
                message = choice.get("Delta", {})
            
            choices.append({
                "index": i,
                "message": {
                    "role": message.get("Role", "assistant").lower(),
                    "content": message.get("Content", "")
                },
                "finish_reason": choice.get("FinishReason", "stop")
            })
        
        # 处理 usage
        usage = None
        if "Usage" in response_data:
            usage_data = response_data["Usage"]
            usage = {
                "prompt_tokens": usage_data.get("PromptTokens", 0),
                "completion_tokens": usage_data.get("CompletionTokens", 0),
                "total_tokens": usage_data.get("TotalTokens", 0)
            }
        
        return OpenAIChatResponse(
            id=response_data.get("ReqID", response_data.get("Id", "unknown")),
            object="chat.completion",
            created=response_data.get("Created", int(time.time())),
            model="hunyuan",
            choices=choices,
            usage=usage
        )
    
    def get_request_headers(self) -> Dict[str, str]:
        """获取请求头（包含签名）"""
        # 注意：签名需要在请求体构建后才能计算，这里返回基础头
        # 实际的签名会在 CustomHTTPAdapter 中计算
        timestamp = str(int(time.time()))
        
        return {
            "Content-Type": "application/json",
            "X-TC-Action": self.action,
            "X-TC-Version": self.version,
            "X-TC-Timestamp": timestamp,
            "X-TC-Region": self.region
        }
    
    def get_signature_for_request(self, request_data: Dict[str, Any]) -> str:
        """获取请求的签名（需要在请求体构建后调用）"""
        return self._get_signature(request_data)

