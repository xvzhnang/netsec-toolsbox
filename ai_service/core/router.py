# -*- coding: utf-8 -*-
"""
请求路由器
对应 One API 的 middleware/distributor.go
负责根据 model_id 路由请求到对应的 Adapter
"""
import sys
import os
from typing import Optional

# 添加 ai_service 目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from core.registry import ModelRegistry
from core.adapter.base_adapter import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse
from core.retry import retry_with_backoff, RetryConfig, create_retry_config_from_dict


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
        request: OpenAIChatRequest,
        retry_config: Optional[RetryConfig] = None
    ) -> OpenAIChatResponse:
        """
        路由请求到对应的适配器（带重试机制）
        对应 One API 的 RelayTextHelper
        
        Args:
            model_id: 模型 ID（从请求中提取）
            request: OpenAI 格式的请求
            retry_config: 重试配置（None 表示使用默认配置或禁用重试）
        
        Returns:
            OpenAI 格式的响应
        
        Raises:
            ValueError: 如果模型未找到或未启用
            Exception: 如果适配器调用失败（所有重试后）
        """
        # 查找适配器（对应 One API 的 CacheGetRandomSatisfiedChannel）
        adapter = self.registry.get_adapter(model_id)
        if not adapter:
            raise ValueError(f"模型 {model_id} 未找到或未启用")
        
        # 从适配器配置获取重试设置
        adapter_config = adapter.config
        retry_enabled = adapter_config.get('retry', {}).get('enabled', True)
        
        if not retry_enabled:
            # 禁用重试，直接调用
            response = await adapter.chat(request)
            return response
        
        # 获取重试配置
        if retry_config is None:
            retry_dict = adapter_config.get('retry', {})
            if retry_dict:
                retry_config = create_retry_config_from_dict(retry_dict)
            else:
                # 使用默认配置
                retry_config = RetryConfig()
        
        # 重试回调（记录日志）
        def on_retry(error: Exception, attempt: int, delay: float):
            error_msg = str(error)[:100]  # 截断错误消息
            print(f"⚠️ 模型 {model_id} 请求失败，{delay:.2f}秒后重试（第 {attempt} 次）: {error_msg}", flush=True)
        
        # 使用重试机制调用适配器
        async def call_adapter(**kwargs):
            return await adapter.chat(request)
        
        response = await retry_with_backoff(
            call_adapter,
            config=retry_config,
            on_retry=on_retry
        )
        
        return response

