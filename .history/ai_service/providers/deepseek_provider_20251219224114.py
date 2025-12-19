"""DeepSeek 模型提供商"""

from typing import List, Dict, Any, Optional
import os

try:
    from openai import OpenAI
    HAS_OPENAI_LIB = True
except ImportError:
    HAS_OPENAI_LIB = False

from .base_provider import BaseProvider


class DeepSeekProvider(BaseProvider):
    """DeepSeek 模型提供商（使用 OpenAI 兼容接口）"""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_key = config.get('api_key') or os.environ.get('DEEPSEEK_API_KEY')
        self.model = config.get('model', 'deepseek-chat')
        self.base_url = config.get('base_url', 'https://api.deepseek.com/v1')
        self.client = None
        
        if self.api_key and HAS_OPENAI_LIB:
            try:
                self.client = OpenAI(
                    api_key=self.api_key,
                    base_url=self.base_url
                )
            except AttributeError as e:
                # Windows 上 SIGALRM 错误（openai 库内部可能使用）
                if 'SIGALRM' in str(e):
                    print(f"⚠️ DeepSeek 客户端初始化失败（Windows 不支持 SIGALRM）: {e}", flush=True)
                else:
                    print(f"⚠️ DeepSeek 客户端初始化失败: {e}", flush=True)
            except Exception as e:
                print(f"⚠️ DeepSeek 客户端初始化失败: {e}", flush=True)
    
    def is_available(self) -> bool:
        """检查 DeepSeek 是否可用"""
        return HAS_OPENAI_LIB and self.api_key is not None and self.client is not None
    
    def get_available_models(self) -> Optional[List[str]]:
        """获取可用的模型列表"""
        if not self.is_available():
            return None
        
        try:
            models = self.client.models.list()
            # DeepSeek 模型通常以 deepseek- 开头
            deepseek_models = [
                model.id for model in models.data 
                if 'deepseek' in model.id.lower()
            ]
            return sorted(deepseek_models) if deepseek_models else None
        except Exception as e:
            print(f"⚠️ 获取 DeepSeek 模型列表失败: {e}", flush=True)
            # 如果获取失败，返回一些常见的模型列表
            return [
                'deepseek-chat',
                'deepseek-coder'
            ]
    
    def chat(self, messages: List[Dict[str, str]], timeout: Optional[int] = None) -> str:
        """发送聊天消息并获取回复（使用跨平台超时处理）"""
        if not self.is_available():
            return "❌ DeepSeek 未配置或不可用，请检查 API 密钥"
        
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
                    timeout=request_timeout
                )
            
            with ThreadPoolExecutor(max_workers=1) as executor:
                future = executor.submit(_call_api)
                try:
                    response = future.result(timeout=request_timeout + 5)  # 额外 5 秒缓冲
                    return response.choices[0].message.content
                except FutureTimeoutError:
                    return f"❌ DeepSeek API 请求超时（{request_timeout}秒）"
            
        except Exception as e:
            return f"❌ DeepSeek API 调用失败: {str(e)}"

