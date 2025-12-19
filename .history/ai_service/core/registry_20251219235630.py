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
if sys.platform == 'win32':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')

import sys
import os

# 添加 ai_service 目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from core.adapter.base_adapter import ChatAdapter
from core.adapter.openai_compat_adapter import OpenAICompatAdapter
# from .adapter.custom_http_adapter import CustomHTTPAdapter
# from .adapter.process_adapter import ProcessAdapter


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
        # 优先使用环境变量
        config_dir = os.environ.get('NETSEC_TOOLBOX_CONFIG_DIR')
        if config_dir:
            config_path = os.path.join(config_dir, 'models.json')
            if os.path.exists(config_path):
                return config_path
        
        # 尝试多个位置
        possible_paths = [
            os.path.join(os.path.dirname(__file__), '..', 'config', 'models.json'),
            os.path.join(os.path.dirname(__file__), '..', 'models.json'),
            'models.json',
        ]
        
        if sys.platform == 'win32':
            appdata = os.environ.get('APPDATA', '')
            if appdata:
                possible_paths.insert(0, os.path.join(appdata, 'netsec-toolbox', '.config', 'models.json'))
        
        for path in possible_paths:
            abs_path = os.path.abspath(path)
            if os.path.exists(abs_path):
                return abs_path
        
        # 如果都不存在，返回默认路径
        return os.path.join(os.path.dirname(__file__), '..', 'config', 'models.json')
    
    def _load_models(self):
        """从配置文件加载模型"""
        if not os.path.exists(self.config_path):
            print(f"⚠️ 配置文件不存在: {self.config_path}，尝试创建默认配置", flush=True)
            self._create_default_config()
            if not os.path.exists(self.config_path):
                print(f"❌ 无法创建默认配置文件，模型加载失败", flush=True)
                return
        
        try:
            with open(self.config_path, 'r', encoding='utf-8') as f:
                config = json.load(f)
            
            models = config.get('models', [])
            
            for model_config in models:
                model_id = model_config.get('id')
                if not model_id:
                    print(f"⚠️ 模型配置缺少 'id' 字段，跳过", flush=True)
                    continue
                
                adapter_type = model_config.get('adapter', 'openai_compat')
                enabled = model_config.get('enabled', True)
                
                if not enabled:
                    print(f"ℹ️ 模型 {model_id} 已禁用，跳过", flush=True)
                    continue
                
                try:
                    adapter = self._create_adapter(adapter_type, model_config)
                    if adapter and adapter.is_available():
                        self.adapters[model_id] = adapter
                        print(f"✅ 模型 {model_id} ({adapter_type}) 已加载", flush=True)
                    else:
                        print(f"⚠️ 模型 {model_id} ({adapter_type}) 不可用，跳过", flush=True)
                except Exception as e:
                    print(f"❌ 初始化模型 {model_id} 失败: {e}", file=sys.stderr, flush=True)
                    if config.get('debug', False):
                        traceback.print_exc(file=sys.stderr)
        
        except Exception as e:
            print(f"❌ 加载配置文件失败: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
    
    def _create_adapter(self, adapter_type: str, config: Dict) -> Optional[ChatAdapter]:
        """
        创建适配器实例
        对应 One API 的 relay.GetAdaptor
        """
        try:
            if adapter_type == 'openai_compat':
                return OpenAICompatAdapter(config)
            # elif adapter_type == 'custom_http':
            #     return CustomHTTPAdapter(config)
            # elif adapter_type == 'process':
            #     return ProcessAdapter(config)
            else:
                print(f"⚠️ 未知的适配器类型: {adapter_type}", flush=True)
                return None
        except Exception as e:
            print(f"❌ 创建适配器失败: {e}", file=sys.stderr, flush=True)
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
            print(f"✅ 已创建默认配置文件: {self.config_path}", flush=True)
        except Exception as e:
            print(f"⚠️ 创建默认配置文件失败: {e}", file=sys.stderr, flush=True)
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
        print("🔄 重新加载模型配置...", flush=True)
        self.adapters.clear()
        self._load_models()

