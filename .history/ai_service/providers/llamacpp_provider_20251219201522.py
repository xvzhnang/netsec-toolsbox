"""llama.cpp 本地模型提供商（通过 OpenAI 兼容接口）"""

from typing import List, Dict, Any, Optional
import os

try:
    from openai import OpenAI
    HAS_OPENAI_LIB = True
except ImportError:
    HAS_OPENAI_LIB = False

from .base_provider import BaseProvider


class LlamaCppProvider(BaseProvider):
    """llama.cpp 本地模型提供商（通过 OpenAI 兼容接口，如 llama-cpp-python）"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_url = config.get('api_url', 'http://localhost:8080/v1')
        self.model = config.get('model', 'local')
        self.timeout = config.get('timeout', 120)
        self.client = None
        
        if HAS_OPENAI_LIB:
            try:
                self.client = OpenAI(
                    base_url=self.api_url,
                    api_key='not-needed'  # llama.cpp 通常不需要 API key
                )
            except Exception as e:
                print(f"⚠️ llama.cpp 客户端初始化失败: {e}")
    
    def is_available(self) -> bool:
        """检查 llama.cpp 是否可用"""
        if not HAS_OPENAI_LIB or not self.client:
            return False
        
        try:
            import urllib.request
            req = urllib.request.Request(f"{self.api_url}/models")
            with urllib.request.urlopen(req, timeout=2) as response:
                return response.status == 200
        except:
            return False
    
    def chat(self, messages: List[Dict[str, str]], timeout: Optional[int] = None) -> str:
        """发送聊天消息并获取回复"""
        if not self.is_available():
            return "❌ llama.cpp 未配置或不可用，请确保服务正在运行"
        
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
            
            # 从配置获取参数
            temperature = self.config.get('temperature', 0.7)
            max_tokens = self.config.get('max_tokens', 2000)
            request_timeout = timeout or self.timeout
            
            # 调用 llama.cpp API
            response = self.client.chat.completions.create(
                model=self.model,
                messages=chat_messages,
                temperature=temperature,
                max_tokens=max_tokens,
                timeout=request_timeout
            )
            
            return response.choices[0].message.content
            
        except Exception as e:
            return f"❌ llama.cpp API 调用失败: {str(e)}"

