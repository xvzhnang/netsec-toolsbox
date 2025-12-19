"""Ollama 本地模型提供商"""

from typing import List, Dict, Any, Optional
import os
import json
import urllib.request
import urllib.error
from .base_provider import BaseProvider


class OllamaProvider(BaseProvider):
    """Ollama 本地模型提供商"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_url = config.get('api_url', 'http://localhost:11434')
        self.model = config.get('model', 'llama2')
        self.timeout = config.get('timeout', 60)
    
    def is_available(self) -> bool:
        """检查 Ollama 是否可用"""
        try:
            import urllib.request
            req = urllib.request.Request(f"{self.api_url}/api/tags")
            req.add_header('Content-Type', 'application/json')
            with urllib.request.urlopen(req, timeout=2) as response:
                return response.status == 200
        except:
            return False
    
    def chat(self, messages: List[Dict[str, str]]) -> str:
        """发送聊天消息并获取回复"""
        if not self.is_available():
            return "❌ Ollama 服务不可用，请确保 Ollama 正在运行"
        
        try:
            # 转换消息格式为 Ollama 格式
            # Ollama 使用 messages 数组，格式为 [{"role": "user", "content": "..."}]
            ollama_messages = []
            for msg in messages:
                role = msg.get('role', 'user')
                content = msg.get('text', msg.get('content', ''))
                ollama_messages.append({
                    'role': role,
                    'content': content
                })
            
            # 构建请求数据
            request_data = {
                'model': self.model,
                'messages': ollama_messages,
                'stream': False
            }
            
            # 发送请求
            req = urllib.request.Request(
                f"{self.api_url}/api/chat",
                data=json.dumps(request_data).encode('utf-8'),
                headers={'Content-Type': 'application/json'}
            )
            
            with urllib.request.urlopen(req, timeout=self.timeout) as response:
                result = json.loads(response.read().decode('utf-8'))
                
                if 'message' in result and 'content' in result['message']:
                    return result['message']['content']
                else:
                    return f"❌ Ollama API 返回格式错误: {result}"
                    
        except urllib.error.URLError as e:
            return f"❌ Ollama API 连接失败: {str(e)}"
        except json.JSONDecodeError as e:
            return f"❌ Ollama API 响应解析失败: {str(e)}"
        except Exception as e:
            return f"❌ Ollama API 调用失败: {str(e)}"

