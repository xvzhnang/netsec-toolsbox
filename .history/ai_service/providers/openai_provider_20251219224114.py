"""OpenAI 模型提供商"""

from typing import List, Dict, Any, Optional
import os

try:
    from openai import OpenAI
    HAS_OPENAI_LIB = True
except ImportError:
    HAS_OPENAI_LIB = False

from .base_provider import BaseProvider


class OpenAIProvider(BaseProvider):
    """OpenAI 模型提供商"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_key = config.get('api_key') or os.environ.get('OPENAI_API_KEY')
        self.model = config.get('model', 'gpt-3.5-turbo')
        self.base_url = config.get('base_url')  # 支持自定义 base_url（如代理）
        self.client = None
        
        if self.api_key and HAS_OPENAI_LIB:
            try:
                client_kwargs = {'api_key': self.api_key}
                if self.base_url:
                    client_kwargs['base_url'] = self.base_url
                self.client = OpenAI(**client_kwargs)
            except AttributeError as e:
                # Windows 上 SIGALRM 错误（openai 库内部可能使用）
                if 'SIGALRM' in str(e):
                    print(f"⚠️ OpenAI 客户端初始化失败（Windows 不支持 SIGALRM）: {e}", flush=True)
                else:
                    print(f"⚠️ OpenAI 客户端初始化失败: {e}", flush=True)
            except Exception as e:
                print(f"⚠️ OpenAI 客户端初始化失败: {e}", flush=True)
    
    def is_available(self) -> bool:
        """检查 OpenAI 是否可用"""
        return HAS_OPENAI_LIB and self.api_key is not None and self.client is not None
    
    def get_available_models(self) -> Optional[List[str]]:
        """获取可用的模型列表"""
        if not self.is_available():
            return None
        
        try:
            models = self.client.models.list()
            # 过滤出 chat 模型（通常以 gpt- 开头）
            chat_models = [
                model.id for model in models.data 
                if 'gpt' in model.id.lower() or 'o1' in model.id.lower()
            ]
            return sorted(chat_models) if chat_models else None
        except Exception as e:
            print(f"⚠️ 获取 OpenAI 模型列表失败: {e}", flush=True)
            # 如果获取失败，返回一些常见的模型列表
            return [
                'gpt-4o',
                'gpt-4o-mini',
                'gpt-4-turbo',
                'gpt-4',
                'gpt-3.5-turbo',
                'o1-preview',
                'o1-mini'
            ]
    
    def chat(self, messages: List[Dict[str, str]], timeout: Optional[int] = None) -> str:
        """发送聊天消息并获取回复（使用跨平台超时处理）"""
        if not self.is_available():
            return "❌ OpenAI 未配置或不可用，请检查 API 密钥"
        
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
            
            # 从配置获取参数（支持外置配置）
            temperature = self.config.get('temperature', 0.7)
            max_tokens = self.config.get('max_tokens', 2000)
            request_timeout = timeout or self.config.get('timeout', 60)
            
            # 使用 ThreadPoolExecutor 实现跨平台超时（避免 signal.SIGALRM 在 Windows 上的问题）
            from concurrent.futures import ThreadPoolExecutor, TimeoutError as FutureTimeoutError
            
            def _call_api():
                return self.client.chat.completions.create(
                    model=self.model,
                    messages=chat_messages,
                    temperature=temperature,
                    max_tokens=max_tokens,
                    timeout=request_timeout  # 仍然传递 timeout 给 openai 库，但外层用 ThreadPoolExecutor 做额外保护
                )
            
            with ThreadPoolExecutor(max_workers=1) as executor:
                future = executor.submit(_call_api)
                try:
                    response = future.result(timeout=request_timeout + 5)  # 额外 5 秒缓冲
                    return response.choices[0].message.content
                except FutureTimeoutError:
                    return f"❌ OpenAI API 请求超时（{request_timeout}秒）"
            
        except Exception as e:
            return f"❌ OpenAI API 调用失败: {str(e)}"

