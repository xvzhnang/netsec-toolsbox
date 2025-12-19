"""阿里云通义千问 (Qwen) 模型提供商"""

from typing import List, Dict, Any, Optional
import os

try:
    import dashscope
    from dashscope import Generation
    HAS_DASHSCOPE_LIB = True
except ImportError:
    HAS_DASHSCOPE_LIB = False

from .base_provider import BaseProvider


class QwenProvider(BaseProvider):
    """阿里云通义千问 (Qwen) 模型提供商"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_key = config.get('api_key') or os.environ.get('DASHSCOPE_API_KEY')
        self.model = config.get('model', 'qwen-turbo')
        self.client = None
        
        if self.api_key and HAS_DASHSCOPE_LIB:
            try:
                dashscope.api_key = self.api_key
                self.client = Generation
            except Exception as e:
                print(f"⚠️ 通义千问 客户端初始化失败: {e}")
    
    def is_available(self) -> bool:
        """检查通义千问是否可用"""
        return HAS_DASHSCOPE_LIB and self.api_key is not None and self.client is not None
    
    def chat(self, messages: List[Dict[str, str]], timeout: Optional[int] = None) -> str:
        """发送聊天消息并获取回复"""
        if not self.is_available():
            return "❌ 通义千问 未配置或不可用，请检查 API 密钥"
        
        try:
            # 转换消息格式
            # DashScope 使用 messages 格式，需要将对话历史转换为字符串
            system_message = None
            chat_history = []
            user_message = None
            
            for msg in messages:
                role = msg.get('role', 'user')
                content = msg.get('text', msg.get('content', ''))
                
                if role == 'system':
                    system_message = content
                elif role == 'user':
                    user_message = content
                    # 如果有历史对话，添加到历史
                    if chat_history:
                        chat_history.append({'role': 'user', 'content': content})
                    else:
                        chat_history = [{'role': 'user', 'content': content}]
                elif role == 'assistant':
                    chat_history.append({'role': 'assistant', 'content': content})
            
            # 如果没有用户消息，使用最后一条消息
            if user_message is None and messages:
                user_message = messages[-1].get('text', messages[-1].get('content', ''))
            
            # 从配置获取参数
            temperature = self.config.get('temperature', 0.7)
            max_tokens = self.config.get('max_tokens', 2000)
            
            # 调用通义千问 API
            response = self.client.call(
                model=self.model,
                messages=chat_history if len(chat_history) > 1 else [{'role': 'user', 'content': user_message}],
                system=system_message,
                temperature=temperature,
                max_tokens=max_tokens,
                result_format='message'
            )
            
            if response.status_code == 200:
                return response.output.choices[0].message.content
            else:
                return f"❌ 通义千问 API 调用失败: {response.message}"
            
        except Exception as e:
            return f"❌ 通义千问 API 调用失败: {str(e)}"

