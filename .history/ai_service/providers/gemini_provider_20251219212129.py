"""Google Gemini 模型提供商"""

from typing import List, Dict, Any, Optional
import os

try:
    import google.generativeai as genai
    HAS_GEMINI_LIB = True
except ImportError:
    HAS_GEMINI_LIB = False

from .base_provider import BaseProvider


class GeminiProvider(BaseProvider):
    """Google Gemini 模型提供商"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_key = config.get('api_key') or os.environ.get('GEMINI_API_KEY')
        self.model = config.get('model', 'gemini-pro')
        self.client = None
        
        if self.api_key and HAS_GEMINI_LIB:
            try:
                genai.configure(api_key=self.api_key)
                self.client = genai.GenerativeModel(self.model)
            except Exception as e:
                print(f"⚠️ Gemini 客户端初始化失败: {e}")
    
    def is_available(self) -> bool:
        """检查 Gemini 是否可用"""
        return HAS_GEMINI_LIB and self.api_key is not None and self.client is not None
    
    def chat(self, messages: List[Dict[str, str]], timeout: Optional[int] = None) -> str:
        """发送聊天消息并获取回复"""
        if not self.is_available():
            return "❌ Gemini 未配置或不可用，请检查 API 密钥"
        
        try:
            # Gemini API 使用不同的消息格式
            # 需要将对话历史转换为 Gemini 格式
            chat = self.client.start_chat(history=[])
            
            # 从配置获取参数
            temperature = self.config.get('temperature', 0.7)
            max_tokens = self.config.get('max_tokens', 2048)
            
            # 构建对话内容
            # Gemini 只处理最后一条用户消息，之前的消息作为历史
            user_message = None
            for msg in reversed(messages):
                role = msg.get('role', 'user')
                content = msg.get('text', msg.get('content', ''))
                
                if role == 'user' and user_message is None:
                    user_message = content
                elif role == 'assistant':
                    # 添加助手回复到历史
                    chat.history.append({
                        'role': 'model',
                        'parts': [content]
                    })
                elif role == 'user':
                    # 添加用户消息到历史
                    chat.history.insert(0, {
                        'role': 'user',
                        'parts': [content]
                    })
            
            if user_message is None:
                user_message = messages[-1].get('text', messages[-1].get('content', ''))
            
            # 调用 Gemini API
            generation_config = {
                'temperature': temperature,
                'max_output_tokens': max_tokens,
            }
            
            response = chat.send_message(
                user_message,
                generation_config=generation_config
            )
            
            return response.text
            
        except Exception as e:
            return f"❌ Gemini API 调用失败: {str(e)}"

