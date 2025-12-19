# -*- coding: utf-8 -*-
"""
AI 模型适配器基类
对应 One API 的 relay/adaptor/interface.go
"""
from abc import ABC, abstractmethod
from typing import Dict, Any, Optional, List
from dataclasses import dataclass


@dataclass
class OpenAIMessage:
    """OpenAI 消息格式"""
    role: str  # "user", "assistant", "system"
    content: str
    name: Optional[str] = None


@dataclass
class OpenAIChatRequest:
    """OpenAI Chat Completions 请求格式"""
    model: str
    messages: List[Dict[str, Any]]
    temperature: Optional[float] = None
    max_tokens: Optional[int] = None
    stream: bool = False
    top_p: Optional[float] = None
    frequency_penalty: Optional[float] = None
    presence_penalty: Optional[float] = None
    stop: Optional[List[str]] = None
    user: Optional[str] = None


@dataclass
class OpenAIChatResponse:
    """OpenAI Chat Completions 响应格式"""
    id: str
    object: str
    created: int
    model: str
    choices: List[Dict[str, Any]]
    usage: Optional[Dict[str, int]] = None


class ChatAdapter(ABC):
    """
    AI 模型适配器基类
    对应 One API 的 Adaptor 接口
    """
    
    def __init__(self, config: Dict[str, Any]):
        """
        初始化适配器
        
        Args:
            config: 模型配置（从 models.json 读取）
        """
        self.config = config
        self.model_id = config.get('id', 'unknown_model')
    
    @abstractmethod
    async def chat(
        self,
        request: OpenAIChatRequest,
        timeout: Optional[int] = None
    ) -> OpenAIChatResponse:
        """
        发送聊天消息并获取回复
        
        这个方法合并了 One API 的以下方法：
        - GetRequestURL: 构建目标 API URL
        - SetupRequestHeader: 设置认证头
        - ConvertRequest: 协议转换（如需要）
        - DoRequest: 发送 HTTP 请求
        - DoResponse: 响应转换（如需要）
        
        Args:
            request: OpenAI 格式的请求
            timeout: 超时时间（秒），None 表示使用默认值
        
        Returns:
            OpenAI 格式的响应
        """
        pass
    
    @abstractmethod
    def is_available(self) -> bool:
        """
        检查适配器是否可用（配置是否完整）
        
        Returns:
            True 如果适配器可用，False 否则
        """
        pass
    
    @property
    @abstractmethod
    def adapter_type(self) -> str:
        """
        返回适配器类型（如 'openai_compat', 'custom_http'）
        
        Returns:
            适配器类型字符串
        """
        pass
    
    def get_model_info(self) -> Dict[str, Any]:
        """
        获取模型信息（用于 /v1/models 接口）
        
        Returns:
            模型信息字典
        """
        return {
            "id": self.model_id,
            "object": "model",
            "created": 0,  # 可以记录加载时间
            "owned_by": self.adapter_type
        }

