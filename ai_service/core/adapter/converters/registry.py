# -*- coding: utf-8 -*-
"""
协议转换器注册表
根据 request_format 字段获取对应的转换器
"""
from typing import Dict, Any, Optional
from .base_converter import ProtocolConverter
from .anthropic_converter import AnthropicConverter
from .gemini_converter import GeminiConverter
from .zhipu_converter import ZhipuConverter
from .baidu_converter import BaiduConverter
from .ali_converter import AliConverter


def get_converter(request_format: str, config: Dict[str, Any]) -> Optional[ProtocolConverter]:
    """
    根据 request_format 获取对应的转换器
    
    Args:
        request_format: 请求格式标识（如 'anthropic', 'gemini', 'zhipu' 等）
        config: 模型配置
    
    Returns:
        协议转换器实例，如果格式不支持则返回 None
    """
    converters = {
        "anthropic": AnthropicConverter,
        "gemini": GeminiConverter,
        "zhipu": ZhipuConverter,
        "baidu": BaiduConverter,
        "ali": AliConverter,
        "alibailian": AliConverter,  # 阿里百炼使用相同的转换器
    }
    
    converter_class = converters.get(request_format.lower())
    if converter_class:
        return converter_class(config)
    
    return None

