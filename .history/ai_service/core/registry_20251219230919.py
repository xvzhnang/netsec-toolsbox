"""
模型注册表
负责加载配置、初始化适配器、管理模型列表
"""

import json
import os
import sys
import traceback
from typing import Dict, Optional, List
from pathlib import Path

from .adapter.base_adapter import ChatAdapter
from .adapter.openai_compat_adapter import OpenAICompatAdapter
from .adapter.custom_http_adapter import CustomHTTPAdapter


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
            if os.path.exists(path):
                return path
        
        # 如果都不存在，返回默认路径
        return possible_paths[0]
    
    def _load_models(self):
        """从配置文件加载模型"""
        if not os.path.exists(self.config_path):
            print(f"⚠️ 配置文件不存在: {self.config_path}，使用默认配置", flush=True)
            self._create_default_config()
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
                    print(f"⚠️ 初始化模型 {model_id} 失败: {e}", flush=True)
        
        except Exception as e:
            print(f"❌ 加载配置文件失败: {e}", flush=True)
            traceback.print_exc(file=sys.stderr)
    
    def _create_adapter(self, adapter_type: str, config: Dict) -> Optional[ChatAdapter]:
        """创建适配器实例"""
        try:
            if adapter_type == 'openai_compat':
                return OpenAICompatAdapter(config)
            elif adapter_type == 'custom_http':
                return CustomHTTPAdapter(config)
            else:
                print(f"⚠️ 未知的适配器类型: {adapter_type}", flush=True)
                return None
        except AttributeError as e:
            # Windows 上 SIGALRM 错误
            if 'SIGALRM' in str(e):
                print(f"⚠️ Windows 不支持 SIGALRM，跳过适配器初始化: {e}", flush=True)
                return None
            raise
        except Exception as e:
            print(f"⚠️ 创建适配器失败: {e}", flush=True)
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
                    "enabled": True
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
            print(f"⚠️ 创建默认配置文件失败: {e}", flush=True)
    
    def get_adapter(self, model_id: str) -> Optional[ChatAdapter]:
        """获取指定模型的适配器"""
        return self.adapters.get(model_id)
    
    def list_models(self) -> List[Dict[str, Any]]:
        """列出所有可用模型"""
        models = []
        for model_id, adapter in self.adapters.items():
            models.append({
                "id": model_id,
                "object": "model",
                "created": 0,
                "owned_by": adapter.adapter_type
            })
        return models
    
    def reload(self):
        """重新加载配置"""
        self.adapters.clear()
        self._load_models()
