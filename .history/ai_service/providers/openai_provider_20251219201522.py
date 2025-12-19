"""OpenAI 模型提供商"""

from typing import List, Dict, Any, Optional
import os

try:
    from openai import OpenAI
    HAS_OPENAI_LIB = True
except ImportError:
    HAS_OPENAI_LIB = False

from .base_provider import BaseProvider


class OpenAIProvider(BaseProvider):
    """OpenAI 模型提供商"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_key = config.get('api_key') or os.environ.get('OPENAI_API_KEY')
        self.model = config.get('model', 'gpt-3.5-turbo')
        self.base_url = config.get('base_url')  # 支持自定义 base_url（如代理）
        self.client = None
        
        if self.api_key and HAS_OPENAI_LIB:
            try:
                client_kwargs = {'api_key': self.api_key}
                if self.base_url:
                    client_kwargs['base_url'] = self.base_url
                self.client = OpenAI(**client_kwargs)
            except Exception as e:
                print(f"⚠️ OpenAI 客户端初始化失败: {e}")
    
    def is_available(self) -> bool:
        """检查 OpenAI 是否可用"""
        return HAS_OPENAI_LIB and self.api_key is not None and self.client is not None
    
    def chat(self, messages: List[Dict[str, str]], timeout: Optional[int] = None) -> str:
        """发送聊天消息并获取回复"""
        if not self.is_available():
            return "❌ OpenAI 未配置或不可用，请检查 API 密钥"
        
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
            
            # 从配置获取参数（支持外置配置）
            temperature = self.config.get('temperature', 0.7)
            max_tokens = self.config.get('max_tokens', 2000)
            timeout = timeout or self.config.get('timeout', 60)
            
            # 调用 OpenAI API
            response = self.client.chat.completions.create(
                model=self.model,
                messages=chat_messages,
                temperature=temperature,
                max_tokens=max_tokens,
                timeout=timeout
            )
            
            return response.choices[0].message.content
            
        except Exception as e:
            return f"❌ OpenAI API 调用失败: {str(e)}"

