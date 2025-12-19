"""LM Studio 本地模型提供商（使用 OpenAI 兼容接口）"""

from typing import List, Dict, Any, Optional
import os

try:
    from openai import OpenAI
    HAS_OPENAI_LIB = True
except ImportError:
    HAS_OPENAI_LIB = False

from .base_provider import BaseProvider


class LMStudioProvider(BaseProvider):
    """LM Studio 本地模型提供商（使用 OpenAI 兼容接口）"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_url = config.get('api_url', 'http://localhost:1234/v1')
        self.model = config.get('model', 'local-model')
        self.timeout = config.get('timeout', 120)  # 本地模型可能需要更长时间
        self.client = None
        
        if HAS_OPENAI_LIB:
            try:
                self.client = OpenAI(
                    base_url=self.api_url,
                    api_key='lm-studio'  # LM Studio 不需要真实的 API key
                )
            except Exception as e:
                print(f"⚠️ LM Studio 客户端初始化失败: {e}")
    
    def is_available(self) -> bool:
        """检查 LM Studio 是否可用"""
        if not HAS_OPENAI_LIB or not self.client:
            return False
        
        try:
            # 尝试列出模型（简单检查）
            import urllib.request
            req = urllib.request.Request(f"{self.api_url}/models")
            with urllib.request.urlopen(req, timeout=2) as response:
                return response.status == 200
        except:
            return False
    
    def chat(self, messages: List[Dict[str, str]]) -> str:
        """发送聊天消息并获取回复"""
        if not self.is_available():
            return "❌ LM Studio 未配置或不可用，请确保 LM Studio 正在运行"
        
        try:
            # 转换消息格式
            chat_messages = []
            for msg in messages:
                role = msg.get('role', 'user')
                content = msg.get('text', msg.get('content', ''))
                chat_messages.append({
                    'role': role,
                    'content': content
                })
            
            # 调用 LM Studio API（使用 OpenAI 兼容接口）
            response = self.client.chat.completions.create(
                model=self.model,
                messages=chat_messages,
                temperature=0.7,
                max_tokens=2000,
                timeout=self.timeout
            )
            
            return response.choices[0].message.content
            
        except Exception as e:
            return f"❌ LM Studio API 调用失败: {str(e)}"

