"""
Adapter 抽象接口
所有 Adapter 必须实现此接口，将不同协议的模型统一为 OpenAI-compatible 格式
"""

from abc import ABC, abstractmethod
from typing import Dict, Any, Optional, List
from dataclasses import dataclass


@dataclass
class OpenAIChatRequest:
    """OpenAI Chat Completions 请求格式"""
    model: str
    messages: List[Dict[str, str]]
    temperature: Optional[float] = 0.7
    max_tokens: Optional[int] = None
    stream: Optional[bool] = False
    timeout: Optional[int] = None
    # 其他 OpenAI 参数
    top_p: Optional[float] = None
    frequency_penalty: Optional[float] = None
    presence_penalty: Optional[float] = None
    stop: Optional[List[str]] = None


@dataclass
class OpenAIChatResponse:
    """OpenAI Chat Completions 响应格式"""
    id: str
    object: str = "chat.completion"
    created: int = 0
    model: str = ""
    choices: List[Dict[str, Any]] = None
    usage: Optional[Dict[str, int]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """转换为字典格式"""
        result = {
            "id": self.id,
            "object": self.object,
            "created": self.created,
            "model": self.model,
            "choices": self.choices or [],
        }
        if self.usage:
            result["usage"] = self.usage
        return result


@dataclass
class OpenAIErrorResponse:
    """OpenAI 错误响应格式"""
    error: Dict[str, Any]
    
    def to_dict(self) -> Dict[str, Any]:
        return {"error": self.error}


class ChatAdapter(ABC):
    """
    Chat Adapter 抽象基类
    
    所有模型适配器必须实现此接口，将不同协议的模型统一为 OpenAI-compatible 格式
    """
    
    def __init__(self, config: Dict[str, Any]):
        """
        初始化 Adapter
        
        Args:
            config: 模型配置字典
        """
        self.config = config
        self.model_id = config.get('id', '')
        self.adapter_type = config.get('adapter', '')
    
    @abstractmethod
    def chat(self, request: OpenAIChatRequest) -> OpenAIChatResponse:
        """
        发送聊天请求并返回 OpenAI-compatible 响应
        
        Args:
            request: OpenAI 格式的聊天请求
            
        Returns:
            OpenAI 格式的聊天响应
            
        Raises:
            Exception: 当请求失败时抛出异常
        """
        pass
    
    @abstractmethod
    def is_available(self) -> bool:
        """
        检查 Adapter 是否可用
        
        Returns:
            True 如果可用，False 否则
        """
        pass
    
    def get_available_models(self) -> Optional[List[str]]:
        """
        获取可用的模型列表（可选方法）
        
        Returns:
            模型列表，如果不支持则返回 None
        """
        return None
    
    def get_model_id(self) -> str:
        """获取模型 ID"""
        return self.model_id

