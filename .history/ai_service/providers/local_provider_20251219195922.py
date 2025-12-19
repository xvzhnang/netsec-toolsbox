"""本地模型提供商（占位符）"""

from typing import List, Dict, Any
from .base_provider import BaseProvider


class LocalProvider(BaseProvider):
    """本地模型提供商（如 Ollama、LM Studio 等）"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.model = config.get('model', 'local')
        self.api_url = config.get('api_url', 'http://localhost:11434/api/generate')
    
    def is_available(self) -> bool:
        """检查本地模型是否可用"""
        # TODO: 实现实际的可用性检查
        return False
    
    def chat(self, messages: List[Dict[str, str]]) -> str:
        """发送聊天消息并获取回复"""
        # TODO: 实现本地模型调用（如 Ollama）
        return "❌ 本地模型功能开发中，敬请期待"

