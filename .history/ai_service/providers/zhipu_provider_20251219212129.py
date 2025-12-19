"""智谱AI (GLM) 模型提供商"""

from typing import List, Dict, Any, Optional
import os

try:
    from zhipuai import ZhipuAI
    HAS_ZHIPU_LIB = True
except ImportError:
    HAS_ZHIPU_LIB = False

from .base_provider import BaseProvider


class ZhipuProvider(BaseProvider):
    """智谱AI (GLM) 模型提供商"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_key = config.get('api_key') or os.environ.get('ZHIPU_API_KEY')
        self.model = config.get('model', 'glm-4')
        self.client = None
        
        if self.api_key and HAS_ZHIPU_LIB:
            try:
                self.client = ZhipuAI(api_key=self.api_key)
            except Exception as e:
                print(f"⚠️ 智谱AI 客户端初始化失败: {e}")
    
    def is_available(self) -> bool:
        """检查智谱AI是否可用"""
        return HAS_ZHIPU_LIB and self.api_key is not None and self.client is not None
    
    def chat(self, messages: List[Dict[str, str]], timeout: Optional[int] = None) -> str:
        """发送聊天消息并获取回复"""
        if not self.is_available():
            return "❌ 智谱AI 未配置或不可用，请检查 API 密钥"
        
        try:
            # 转换消息格式（兼容 OpenAI 格式）
            chat_messages = []
            for msg in messages:
                role = msg.get('role', 'user')
                content = msg.get('text', msg.get('content', ''))
                
                # 智谱AI 使用 'user' 和 'assistant' 角色
                if role in ['user', 'assistant', 'system']:
                    chat_messages.append({
                        'role': role,
                        'content': content
                    })
            
            # 从配置获取参数
            temperature = self.config.get('temperature', 0.7)
            max_tokens = self.config.get('max_tokens', 2048)
            timeout = timeout or self.config.get('timeout', 60)
            
            # 调用智谱AI API
            response = self.client.chat.completions.create(
                model=self.model,
                messages=chat_messages,
                temperature=temperature,
                max_tokens=max_tokens,
                timeout=timeout
            )
            
            return response.choices[0].message.content
            
        except Exception as e:
            return f"❌ 智谱AI API 调用失败: {str(e)}"

