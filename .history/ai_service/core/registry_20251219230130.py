"""
Model Registry
管理所有注册的模型和对应的 Adapter
"""

import os
import json
import sys
from typing import Dict, Optional, List
from pathlib import Path

from .adapter.trait import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse
from .adapter.openai_compat import OpenAICompatAdapter
from .adapter.custom_http import CustomHTTPAdapter
from .adapter.process import ProcessAdapter


class ModelRegistry:
    """
    模型注册表
    
    负责加载配置、初始化 Adapter、管理模型生命周期
    """
    
    def __init__(self, config_path: Optional[str] = None):
        """
        初始化注册表
        
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
        
        # Windows 平台
        if sys.platform == 'win32':
            appdata = os.environ.get('APPDATA', '')
            if appdata:
                config_path = os.path.join(appdata, 'netsec-toolbox', '.config', 'models.json')
                if os.path.exists(config_path):
                    return config_path
        
        # 默认路径
        script_dir = os.path.dirname(os.path.abspath(__file__))
        config_path = os.path.join(script_dir, '..', 'config', 'models.json')
        if os.path.exists(config_path):
            return config_path
        
        # 如果都不存在，返回默认路径（将创建新文件）
        return config_path
    
    def _load_models(self):
        """从配置文件加载模型"""
        if not os.path.exists(self.config_path):
            print(f"⚠️ 配置文件不存在: {self.config_path}，将使用空配置", flush=True)
            return
        
        try:
            with open(self.config_path, 'r', encoding='utf-8') as f:
                config = json.load(f)
            
            models = config.get('models', [])
            
            for model_config in models:
                model_id = model_config.get('id')
                if not model_id:
                    print(f"⚠️ 跳过无效模型配置（缺少 id）: {model_config}", flush=True)
                    continue
                
                adapter_type = model_config.get('adapter', 'openai_compat')
                
                try:
                    adapter = self._create_adapter(adapter_type, model_config)
                    if adapter and adapter.is_available():
                        self.adapters[model_id] = adapter
                        print(f"✅ 模型 {model_id} 已注册（adapter: {adapter_type}）", flush=True)
                    else:
                        print(f"⚠️ 模型 {model_id} 不可用，跳过注册", flush=True)
                except Exception as e:
                    print(f"⚠️ 初始化模型 {model_id} 失败: {e}", flush=True)
        
        except Exception as e:
            print(f"❌ 加载配置文件失败: {e}", flush=True)
            import traceback
            traceback.print_exc(file=sys.stderr)
    
    def _create_adapter(self, adapter_type: str, config: Dict) -> Optional[ChatAdapter]:
        """
        创建 Adapter 实例
        
        Args:
            adapter_type: Adapter 类型（openai_compat, custom_http, process）
            config: 模型配置
            
        Returns:
            Adapter 实例，如果创建失败则返回 None
        """
        try:
            if adapter_type == 'openai_compat':
                return OpenAICompatAdapter(config)
            elif adapter_type == 'custom_http':
                return CustomHTTPAdapter(config)
            elif adapter_type == 'process':
                return ProcessAdapter(config)
            else:
                print(f"⚠️ 未知的 Adapter 类型: {adapter_type}", flush=True)
                return None
        except AttributeError as e:
            # Windows 上 SIGALRM 错误
            if 'SIGALRM' in str(e):
                print(f"⚠️ Windows 不支持 SIGALRM，跳过 Adapter 初始化: {e}", flush=True)
            else:
                print(f"⚠️ 创建 Adapter 失败: {e}", flush=True)
            return None
        except Exception as e:
            print(f"⚠️ 创建 Adapter 失败: {e}", flush=True)
            return None
    
    def get_adapter(self, model_id: str) -> Optional[ChatAdapter]:
        """
        获取指定模型的 Adapter
        
        Args:
            model_id: 模型 ID
            
        Returns:
            Adapter 实例，如果不存在则返回 None
        """
        return self.adapters.get(model_id)
    
    def list_models(self) -> List[str]:
        """
        列出所有可用的模型 ID
        
        Returns:
            模型 ID 列表
        """
        return list(self.adapters.keys())
    
    def get_model_info(self, model_id: str) -> Optional[Dict]:
        """
        获取模型信息
        
        Args:
            model_id: 模型 ID
            
        Returns:
            模型信息字典，如果不存在则返回 None
        """
        adapter = self.adapters.get(model_id)
        if not adapter:
            return None
        
        return {
            "id": model_id,
            "object": "model",
            "created": 0,
            "owned_by": adapter.adapter_type
        }
    
    def reload(self):
        """重新加载配置"""
        self.adapters.clear()
        self._load_models()

