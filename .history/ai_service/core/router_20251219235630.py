# -*- coding: utf-8 -*-
"""
请求路由器
对应 One API 的 middleware/distributor.go
负责根据 model_id 路由请求到对应的 Adapter
"""
import sys
import os

# 添加 ai_service 目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from typing import Optional
from core.registry import ModelRegistry
from core.adapter.base_adapter import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse


class Router:
    """请求路由器"""
    
    def __init__(self, registry: ModelRegistry):
        """
        初始化路由器
        
        Args:
            registry: 模型注册表实例
        """
        self.registry = registry
    
    async def route(
        self,
        model_id: str,
        request: OpenAIChatRequest
    ) -> OpenAIChatResponse:
        """
        路由请求到对应的适配器
        对应 One API 的 RelayTextHelper
        
        Args:
            model_id: 模型 ID（从请求中提取）
            request: OpenAI 格式的请求
        
        Returns:
            OpenAI 格式的响应
        
        Raises:
            ValueError: 如果模型未找到或未启用
            Exception: 如果适配器调用失败
        """
        # 查找适配器（对应 One API 的 CacheGetRandomSatisfiedChannel）
        adapter = self.registry.get_adapter(model_id)
        if not adapter:
            raise ValueError(f"模型 {model_id} 未找到或未启用")
        
        # 调用适配器（对应 One API 的 adaptor.DoRequest + DoResponse）
        response = await adapter.chat(request)
        return response

