"""AI 模型提供商基类"""

from abc import ABC, abstractmethod
from typing import List, Dict, Any


class BaseProvider(ABC):
    """AI 模型提供商基类"""
    
    def __init__(self, config: Dict[str, Any]):
        self.config = config
    
    @abstractmethod
    def chat(self, messages: List[Dict[str, str]]) -> str:
        """
        发送聊天消息并获取回复
        
        Args:
            messages: 消息列表，格式为 [{'role': 'user', 'content': '...'}, ...]
        
        Returns:
            模型回复文本
        """
        pass
    
    @abstractmethod
    def is_available(self) -> bool:
        """检查提供商是否可用"""
        pass

