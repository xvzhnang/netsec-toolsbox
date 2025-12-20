# -*- coding: utf-8 -*-
"""
模型注册表
对应 One API 的 model/channel.go + relay/adaptor.go
负责从配置文件加载模型，管理 Adapter 实例
"""
import json
import os
import sys
import io
import traceback
from typing import Dict, Optional, List, Any
from pathlib import Path

# Windows 编码修复
# 注意：不要在这里重新包装 stdout/stderr，因为可能会与管道重定向冲突
# 编码问题由调用方（main_gateway.py）处理

# 添加 ai_service 目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from core.adapter.base_adapter import ChatAdapter
from core.adapter.openai_compat_adapter import OpenAICompatAdapter
from core.adapter.custom_http_adapter import CustomHTTPAdapter
from core.adapter.process_adapter import ProcessAdapter
from core.adapter.xunfei_adapter import XunfeiAdapter


def safe_print(*args, **kwargs):
    """安全打印函数，在 stdout/stderr 不可用时跳过"""
    try:
        # 如果指定了 file 参数，使用指定的流，否则使用 stdout
        file = kwargs.pop('file', sys.stdout)
        if file and hasattr(file, 'closed') and not file.closed:
            print(*args, file=file, flush=True, **kwargs)
        elif file and not hasattr(file, 'closed'):
            # 某些流可能没有 closed 属性
            print(*args, file=file, flush=True, **kwargs)
    except (ValueError, OSError, AttributeError):
        # 流已关闭或不可用，跳过输出
        pass


class ModelRegistry:
    """模型注册表"""
    
    def __init__(self, config_path: Optional[str] = None):
        """
        初始化模型注册表
        
        Args:
            config_path: 配置文件路径，如果为 None 则自动查找
        """
        self.config_path = config_path or self._find_config_path()
        self.adapters: Dict[str, ChatAdapter] = {}
        self._load_models()
    
    def _find_config_path(self) -> str:
        """查找配置文件路径"""
        # 获取 ai_service 目录
        ai_service_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        
        # 默认配置文件路径：ai_service/config/models.json
        default_config_path = os.path.join(ai_service_dir, 'config', 'models.json')
        
        # 如果默认路径存在，直接返回
        if os.path.exists(default_config_path):
            return default_config_path
        
        # 尝试其他位置
        possible_paths = [
            default_config_path,
            os.path.join(ai_service_dir, 'models.json'),
            os.path.join(os.path.dirname(ai_service_dir), 'ai_service', 'config', 'models.json'),
            'models.json',
        ]
        
        for path in possible_paths:
            abs_path = os.path.abspath(path)
            if os.path.exists(abs_path):
                return abs_path
        
        # 如果都不存在，返回默认路径（用于创建默认配置）
        return default_config_path
    
    def _load_models(self):
        """从配置文件加载模型"""
        if not os.path.exists(self.config_path):
            safe_print(f"⚠️ 配置文件不存在: {self.config_path}，尝试创建默认配置")
            self._create_default_config()
            if not os.path.exists(self.config_path):
                safe_print(f"❌ 无法创建默认配置文件，模型加载失败")
                return
        
        try:
            with open(self.config_path, 'r', encoding='utf-8') as f:
                config = json.load(f)
            
            models = config.get('models', [])
            
            for model_config in models:
                # 跳过注释字段（以 _ 开头的字段）
                if isinstance(model_config, dict) and any(key.startswith('_') for key in model_config.keys()):
                    if not model_config.get('id'):
                        # 这是一个注释条目，跳过
                        continue
                
                # 确保是字典类型
                if not isinstance(model_config, dict):
                    continue
                
                model_id = model_config.get('id')
                if not model_id:
                    # 可能是注释字段，跳过
                    continue
                
                adapter_type = model_config.get('adapter', 'openai_compat')
                enabled = model_config.get('enabled', True)
                
                if not enabled:
                    safe_print(f"ℹ️ 模型 {model_id} 已禁用，跳过")
                    continue
                
                try:
                    adapter = self._create_adapter(adapter_type, model_config)
                    if adapter and adapter.is_available():
                        self.adapters[model_id] = adapter
                        # 输出到 stderr 以便被 Rust 后端捕获
                        print(f"✅ 模型 {model_id} ({adapter_type}) 已加载", file=sys.stderr, flush=True)
                    else:
                        # 输出详细信息帮助调试
                        reason = []
                        if not adapter:
                            reason.append("适配器创建失败")
                        elif not adapter.is_available():
                            reason.append("适配器不可用")
                            # 检查具体原因
                            if hasattr(adapter, 'api_key') and not adapter.api_key:
                                reason.append("缺少 API Key")
                            if hasattr(adapter, 'base_url') and not adapter.base_url:
                                reason.append("缺少 Base URL")
                        # 输出到 stderr 以便被 Rust 后端捕获
                        print(f"⚠️ 模型 {model_id} ({adapter_type}) 不可用，跳过。原因: {', '.join(reason) if reason else '未知'}", file=sys.stderr, flush=True)
                except Exception as e:
                    print(f"❌ 初始化模型 {model_id} 失败: {e}", file=sys.stderr, flush=True)
                    if config.get('debug', False):
                        traceback.print_exc(file=sys.stderr)
        
        except Exception as e:
            safe_print(f"❌ 加载配置文件失败: {e}", file=sys.stderr)
            if config.get('debug', False):
                traceback.print_exc(file=sys.stderr)
    
    def _create_adapter(self, adapter_type: str, config: Dict) -> Optional[ChatAdapter]:
        """
        创建适配器实例
        对应 One API 的 relay.GetAdaptor
        """
        try:
            if adapter_type == 'openai_compat':
                return OpenAICompatAdapter(config)
            elif adapter_type == 'custom_http':
                return CustomHTTPAdapter(config)
            elif adapter_type == 'process':
                return ProcessAdapter(config)
            elif adapter_type == 'websocket' or adapter_type == 'websocket_xunfei':
                # 根据 request_format 选择具体的 WebSocket 适配器
                request_format = config.get('request_format', '')
                if request_format == 'xunfei':
                    return XunfeiAdapter(config)
                else:
                    # 默认使用讯飞适配器（向后兼容）
                    return XunfeiAdapter(config)
            else:
                safe_print(f"⚠️ 未知的适配器类型: {adapter_type}")
                return None
        except Exception as e:
            safe_print(f"❌ 创建适配器失败: {e}", file=sys.stderr)
            traceback.print_exc(file=sys.stderr)
            return None
    
    def _create_default_config(self):
        """创建默认配置文件"""
        default_config = {
            "models": [
                {
                    "id": "gpt-3.5-turbo",
                    "adapter": "openai_compat",
                    "base_url": "https://api.openai.com/v1",
                    "api_key": "ENV:OPENAI_API_KEY",
                    "enabled": True,
                    "model": "gpt-3.5-turbo",
                    "temperature": 0.7,
                    "max_tokens": 2000,
                    "timeout": 60
                },
                {
                    "id": "deepseek-chat",
                    "adapter": "openai_compat",
                    "base_url": "https://api.deepseek.com/v1",
                    "api_key": "ENV:DEEPSEEK_API_KEY",
                    "enabled": True,
                    "model": "deepseek-chat",
                    "temperature": 0.7,
                    "max_tokens": 2000,
                    "timeout": 60
                }
            ]
        }
        
        # 确保目录存在
        os.makedirs(os.path.dirname(self.config_path), exist_ok=True)
        
        try:
            with open(self.config_path, 'w', encoding='utf-8') as f:
                json.dump(default_config, f, indent=2, ensure_ascii=False)
            safe_print(f"✅ 已创建默认配置文件: {self.config_path}")
        except Exception as e:
            safe_print(f"⚠️ 创建默认配置文件失败: {e}", file=sys.stderr)
            traceback.print_exc(file=sys.stderr)
    
    def get_adapter(self, model_id: str) -> Optional[ChatAdapter]:
        """
        获取指定模型的适配器
        对应 One API 的 CacheGetRandomSatisfiedChannel（简化版，无负载均衡）
        """
        return self.adapters.get(model_id)
    
    def list_models(self) -> Dict[str, Any]:
        """
        列出所有可用模型（OpenAI 兼容格式）
        用于 /v1/models 接口
        """
        models_info = [adapter.get_model_info() for adapter in self.adapters.values()]
        return {
            "object": "list",
            "data": models_info
        }
    
    def reload(self):
        """重新加载配置"""
        try:
            if sys.stdout and not sys.stdout.closed:
                print("🔄 重新加载模型配置...", flush=True)
        except (ValueError, OSError, AttributeError):
            pass
        self.adapters.clear()
        self._load_models()

