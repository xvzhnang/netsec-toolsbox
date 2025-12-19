"""DeepSeek 模型提供商"""

from typing import List, Dict, Any
import os

try:
    from openai import OpenAI
    HAS_OPENAI_LIB = True
except ImportError:
    HAS_OPENAI_LIB = False

from .base_provider import BaseProvider


class DeepSeekProvider(BaseProvider):
    """DeepSeek 模型提供商（使用 OpenAI 兼容接口）"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_key = config.get('api_key') or os.environ.get('DEEPSEEK_API_KEY')
        self.model = config.get('model', 'deepseek-chat')
        self.base_url = config.get('base_url', 'https://api.deepseek.com/v1')
        self.client = None
        
        if self.api_key and HAS_OPENAI_LIB:
            try:
                self.client = OpenAI(
                    api_key=self.api_key,
                    base_url=self.base_url
                )
            except Exception as e:
                print(f"⚠️ DeepSeek 客户端初始化失败: {e}")
    
    def is_available(self) -> bool:
        """检查 DeepSeek 是否可用"""
        return HAS_OPENAI_LIB and self.api_key is not None and self.client is not None
    
    def chat(self, messages: List[Dict[str, str]]) -> str:
        """发送聊天消息并获取回复"""
        if not self.is_available():
            return "❌ DeepSeek 未配置或不可用，请检查 API 密钥"
        
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
            
            # 调用 DeepSeek API
            response = self.client.chat.completions.create(
                model=self.model,
                messages=chat_messages,
                temperature=0.7,
                max_tokens=2000
            )
            
            return response.choices[0].message.content
            
        except Exception as e:
            return f"❌ DeepSeek API 调用失败: {str(e)}"

