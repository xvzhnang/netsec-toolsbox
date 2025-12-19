"""
Adapter 抽象接口
所有模型适配器必须实现此接口
"""

from abc import ABC, abstractmethod
from typing import Dict, Any, Optional, AsyncIterator
from dataclasses import dataclass


@dataclass
class OpenAIChatRequest:
    """OpenAI Chat Completions 请求格式"""
    model: str
    messages: list[Dict[str, str]]
    temperature: Optional[float] = None
    max_tokens: Optional[int] = None
    stream: bool = False
    timeout: Optional[int] = None
    # 其他 OpenAI 参数
    top_p: Optional[float] = None
    frequency_penalty: Optional[float] = None
    presence_penalty: Optional[float] = None
    stop: Optional[list[str]] = None


@dataclass
class OpenAIChatResponse:
    """OpenAI Chat Completions 响应格式"""
    id: str
    object: str = "chat.completion"
    created: int = 0
    model: str = ""
    choices: list[Dict[str, Any]] = None
    usage: Optional[Dict[str, int]] = None
    
    def __post_init__(self):
        if self.choices is None:
            self.choices = []
        if self.usage is None:
            self.usage = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    
    def to_dict(self) -> Dict[str, Any]:
        """转换为字典格式"""
        return {
            "id": self.id,
            "object": self.object,
            "created": self.created,
            "model": self.model,
            "choices": self.choices,
            "usage": self.usage
        }


@dataclass
class OpenAIErrorResponse:
    """OpenAI 错误响应格式"""
    error: Dict[str, Any]
    
    def to_dict(self) -> Dict[str, Any]:
        return {"error": self.error}


class ChatAdapter(ABC):
    """
    Chat Adapter 抽象基类
    所有模型适配器必须实现此接口
    """
    
    def __init__(self, config: Dict[str, Any]):
        """
        初始化适配器
        
        Args:
            config: 适配器配置字典
        """
        self.config = config
        self.model_id = config.get('id', '')
        self.adapter_type = config.get('adapter', '')
    
    @abstractmethod
    def is_available(self) -> bool:
        """
        检查适配器是否可用
        
        Returns:
            True 如果适配器可用，False 否则
        """
        pass
    
    @abstractmethod
    def chat(self, request: OpenAIChatRequest) -> OpenAIChatResponse:
        """
        发送聊天请求并获取回复（同步版本）
        
        Args:
            request: OpenAI 格式的聊天请求
            
        Returns:
            OpenAI 格式的聊天响应
            
        Raises:
            Exception: 如果请求失败
        """
        pass
    
    def chat_stream(self, request: OpenAIChatRequest) -> AsyncIterator[OpenAIChatResponse]:
        """
        流式聊天请求（可选实现）
        
        如果底层不支持流式，可以降级为：
        1. 先完整生成
        2. 再拆分为 SSE chunk 返回
        
        Args:
            request: OpenAI 格式的聊天请求
            
        Yields:
            OpenAI 格式的聊天响应片段
        """
        # 默认实现：非流式，完整生成后返回
        response = self.chat(request)
        yield response
    
    def get_available_models(self) -> Optional[list[str]]:
        """
        获取可用的模型列表（可选方法）
        
        Returns:
            模型列表，如果适配器不支持则返回 None
        """
        return None

