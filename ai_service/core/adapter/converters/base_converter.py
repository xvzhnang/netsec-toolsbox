# -*- coding: utf-8 -*-
"""
协议转换器基类
对应 One API 中各个适配器的 ConvertRequest 和 Response 转换函数
"""
from abc import ABC, abstractmethod
from typing import Dict, Any, Optional, List
from ..base_adapter import OpenAIChatRequest, OpenAIChatResponse


class ProtocolConverter(ABC):
    """协议转换器基类"""
    
    def __init__(self, config: Dict[str, Any]):
        """
        初始化转换器
        
        Args:
            config: 模型配置
        """
        self.config = config
        self.model_id = config.get('id', 'unknown')
        self.model = config.get('model', self.model_id)
    
    @abstractmethod
    def convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        将 OpenAI 格式的请求转换为目标协议格式
        
        Args:
            request: OpenAI 格式的请求
        
        Returns:
            目标协议格式的请求字典
        """
        pass
    
    @abstractmethod
    def convert_response(self, response_data: Dict[str, Any]) -> OpenAIChatResponse:
        """
        将目标协议格式的响应转换为 OpenAI 格式
        
        Args:
            response_data: 目标协议格式的响应字典
        
        Returns:
            OpenAI 格式的响应
        """
        pass
    
    @abstractmethod
    def get_request_headers(self) -> Dict[str, str]:
        """
        获取请求头（认证、版本等）
        
        Returns:
            请求头字典
        """
        pass
    
    def get_request_url(self, base_url: str, endpoint: str) -> str:
        """
        构建完整的请求 URL
        
        Args:
            base_url: 基础 URL
            endpoint: 端点路径
        
        Returns:
            完整的 URL
        """
        # 替换 endpoint 中的占位符
        endpoint = endpoint.replace('{model}', self.model)
        
        # 如果 endpoint 以 / 开头，直接拼接
        if endpoint.startswith('/'):
            return f"{base_url.rstrip('/')}{endpoint}"
        
        # 否则在 base_url 后添加 /
        return f"{base_url.rstrip('/')}/{endpoint.lstrip('/')}"
    
    def handle_error(self, response_data: Dict[str, Any], status_code: int) -> Optional[Exception]:
        """
        处理错误响应
        
        Args:
            response_data: 响应数据
            status_code: HTTP 状态码
        
        Returns:
            如果存在错误，返回 Exception；否则返回 None
        """
        # 默认实现：检查常见的错误字段
        if 'error' in response_data:
            error = response_data['error']
            if isinstance(error, dict):
                message = error.get('message', error.get('msg', 'Unknown error'))
                return Exception(f"{message}")
            elif isinstance(error, str):
                return Exception(error)
        
        if status_code >= 400:
            return Exception(f"HTTP {status_code}: {response_data.get('message', 'Unknown error')}")
        
        return None

