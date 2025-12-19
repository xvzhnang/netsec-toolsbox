"""LocalAI 本地模型提供商（OpenAI 兼容）"""

from typing import List, Dict, Any, Optional
import os

try:
    from openai import OpenAI
    HAS_OPENAI_LIB = True
except ImportError:
    HAS_OPENAI_LIB = False

from .base_provider import BaseProvider


class LocalAIProvider(BaseProvider):
    """LocalAI 本地模型提供商（完全兼容 OpenAI API）"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_url = config.get('api_url', 'http://localhost:8080/v1')
        self.model = config.get('model', '')
        self.api_key = config.get('api_key', 'not-needed')  # LocalAI 通常不需要真实的 API key
        self.client = None
        
        if HAS_OPENAI_LIB:
            try:
                self.client = OpenAI(
                    base_url=self.api_url,
                    api_key=self.api_key
                )
            except AttributeError as e:
                # Windows 上 SIGALRM 错误（openai 库内部可能使用）
                if 'SIGALRM' in str(e):
                    print(f"⚠️ LocalAI 客户端初始化失败（Windows 不支持 SIGALRM）: {e}", flush=True)
                else:
                    print(f"⚠️ LocalAI 客户端初始化失败: {e}", flush=True)
            except Exception as e:
                print(f"⚠️ LocalAI 客户端初始化失败: {e}", flush=True)
    
    def is_available(self) -> bool:
        """检查 LocalAI 是否可用"""
        if not HAS_OPENAI_LIB or not self.client:
            return False
        
        try:
            import urllib.request
            req = urllib.request.Request(f"{self.api_url}/models")
            with urllib.request.urlopen(req, timeout=2) as response:
                return response.status == 200
        except:
            return False
    
    def get_available_models(self) -> Optional[List[str]]:
        """获取可用的模型列表"""
        if not self.is_available():
            return None
        
        try:
            models = self.client.models.list()
            return [model.id for model in models.data] if models.data else None
        except Exception as e:
            print(f"⚠️ 获取 LocalAI 模型列表失败: {e}", flush=True)
        return None
    
    def chat(self, messages: List[Dict[str, str]], timeout: Optional[int] = None) -> str:
        """发送聊天消息并获取回复（使用跨平台超时处理）"""
        if not self.is_available():
            return "❌ LocalAI 服务不可用，请确保 LocalAI 服务器正在运行"
        
        try:
            # 转换消息格式
            chat_messages = []
            for msg in messages:
                role = msg.get('role', 'user')
                content = msg.get('text', msg.get('content', ''))
                chat_messages.append({
                    'role': role,
                    'content': content
                })
            
            # 从配置获取参数
            temperature = self.config.get('temperature', 0.7)
            max_tokens = self.config.get('max_tokens', 2048)
            request_timeout = timeout or self.config.get('timeout', 120)
            
            # 使用 ThreadPoolExecutor 实现跨平台超时（避免 signal.SIGALRM 在 Windows 上的问题）
            from concurrent.futures import ThreadPoolExecutor, TimeoutError as FutureTimeoutError
            
            def _call_api():
                return self.client.chat.completions.create(
                    model=self.model,
                    messages=chat_messages,
                    temperature=temperature,
                    max_tokens=max_tokens,
                    timeout=request_timeout
                )
            
            with ThreadPoolExecutor(max_workers=1) as executor:
                future = executor.submit(_call_api)
                try:
                    response = future.result(timeout=request_timeout + 5)  # 额外 5 秒缓冲
                    return response.choices[0].message.content
                except FutureTimeoutError:
                    return f"❌ LocalAI API 请求超时（{request_timeout}秒）"
            
        except Exception as e:
            return f"❌ LocalAI API 调用失败: {str(e)}"

