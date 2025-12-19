"""Anthropic Claude 模型提供商"""

from typing import List, Dict, Any, Optional
import os

try:
    import anthropic
    HAS_ANTHROPIC_LIB = True
except ImportError:
    HAS_ANTHROPIC_LIB = False

from .base_provider import BaseProvider


class ClaudeProvider(BaseProvider):
    """Anthropic Claude 模型提供商"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_key = config.get('api_key') or os.environ.get('ANTHROPIC_API_KEY')
        self.model = config.get('model', 'claude-3-5-sonnet-20241022')
        self.base_url = config.get('base_url')  # 支持自定义 base_url（如代理）
        self.client = None
        
        if self.api_key and HAS_ANTHROPIC_LIB:
            try:
                client_kwargs = {'api_key': self.api_key}
                if self.base_url:
                    client_kwargs['base_url'] = self.base_url
                self.client = anthropic.Anthropic(**client_kwargs)
            except Exception as e:
                print(f"⚠️ Claude 客户端初始化失败: {e}")
    
    def is_available(self) -> bool:
        """检查 Claude 是否可用"""
        return HAS_ANTHROPIC_LIB and self.api_key is not None and self.client is not None
    
    def chat(self, messages: List[Dict[str, str]], timeout: Optional[int] = None) -> str:
        """发送聊天消息并获取回复"""
        if not self.is_available():
            return "❌ Claude 未配置或不可用，请检查 API 密钥"
        
        try:
            # Claude API 使用 messages 格式，需要转换
            # Claude 要求第一条消息必须是 user 消息
            system_message = None
            chat_messages = []
            
            for msg in messages:
                role = msg.get('role', 'user')
                content = msg.get('text', msg.get('content', ''))
                
                if role == 'system':
                    system_message = content
                else:
                    # Claude 使用 'user' 和 'assistant' 角色
                    if role in ['user', 'assistant']:
                        chat_messages.append({
                            'role': role,
                            'content': content
                        })
            
            # 从配置获取参数
            temperature = self.config.get('temperature', 0.7)
            max_tokens = self.config.get('max_tokens', 4096)
            timeout = timeout or self.config.get('timeout', 60)
            
            # 调用 Claude API
            response = self.client.messages.create(
                model=self.model,
                messages=chat_messages,
                system=system_message,
                temperature=temperature,
                max_tokens=max_tokens,
                timeout=timeout
            )
            
            # Claude 返回的是 Message 对象，content 是列表
            if response.content and len(response.content) > 0:
                return response.content[0].text
            return ""
            
        except Exception as e:
            return f"❌ Claude API 调用失败: {str(e)}"

