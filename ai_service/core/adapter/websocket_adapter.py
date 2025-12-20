# -*- coding: utf-8 -*-
"""
WebSocket 适配器基类
用于支持 WebSocket 协议的模型（如讯飞星火）
"""
import json
import time
from typing import Dict, Any, Optional, AsyncIterator

try:
    import websockets
    import asyncio
    HAS_WEBSOCKETS = True
except ImportError:
    HAS_WEBSOCKETS = False

from .base_adapter import (
    ChatAdapter,
    OpenAIChatRequest,
    OpenAIChatResponse,
    OpenAIStreamChunk
)


class WebSocketAdapter(ChatAdapter):
    """
    WebSocket 适配器基类
    用于支持 WebSocket 协议的模型
    """
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.ws_url = config.get('base_url', '').replace('http://', 'ws://').replace('https://', 'wss://')
        api_key = config.get('api_key', '')
        # 如果 api_key 是 "ENV:" 开头的环境变量占位符，则设为空字符串
        if isinstance(api_key, str) and api_key.startswith('ENV:'):
            self.api_key = ''
        else:
            self.api_key = api_key
        api_secret = config.get('config', {}).get('api_secret', '')
        # 如果 api_secret 是 "ENV:" 开头的环境变量占位符，则设为空字符串
        if isinstance(api_secret, str) and api_secret.startswith('ENV:'):
            self.api_secret = ''
        else:
            self.api_secret = api_secret
        self.app_id = config.get('config', {}).get('app_id', '')
        self.domain = config.get('config', {}).get('domain', '')
    
    @property
    def adapter_type(self) -> str:
        return "websocket"
    
    def is_available(self) -> bool:
        """检查适配器是否可用"""
        if not HAS_WEBSOCKETS:
            return False
        return self.ws_url and self.api_key and self.api_secret and self.app_id
    
    async def _build_auth_url(self) -> str:
        """
        构建认证 URL（需要子类实现）
        
        Returns:
            认证后的 WebSocket URL
        """
        raise NotImplementedError("子类必须实现 _build_auth_url 方法")
    
    async def _convert_request(self, request: OpenAIChatRequest) -> Dict[str, Any]:
        """
        转换 OpenAI 请求到 WebSocket 协议格式（需要子类实现）
        
        Args:
            request: OpenAI 格式的请求
        
        Returns:
            WebSocket 协议格式的请求字典
        """
        raise NotImplementedError("子类必须实现 _convert_request 方法")
    
    async def _parse_response(self, data: Dict[str, Any]) -> Optional[OpenAIStreamChunk]:
        """
        解析 WebSocket 响应为 OpenAI 格式（需要子类实现）
        
        Args:
            data: WebSocket 响应数据
        
        Returns:
            OpenAI 格式的流式数据块，如果为 None 表示忽略
        """
        raise NotImplementedError("子类必须实现 _parse_response 方法")
    
    async def _is_response_complete(self, data: Dict[str, Any]) -> bool:
        """
        判断响应是否完成（需要子类实现）
        
        Args:
            data: WebSocket 响应数据
        
        Returns:
            True 如果响应完成，False 否则
        """
        raise NotImplementedError("子类必须实现 _is_response_complete 方法")
    
    async def chat(
        self,
        request: OpenAIChatRequest,
        timeout: Optional[int] = None
    ) -> OpenAIChatResponse:
        """
        发送聊天请求（非流式，收集所有响应后返回）
        """
        content = ""
        usage = None
        response_id = f"chatcmpl-{int(time.time())}"
        
        async for chunk in self.chat_stream(request, timeout):
            if chunk.choices:
                delta_content = chunk.choices[0].get("delta", {}).get("content", "")
                if delta_content:
                    content += delta_content
            
            if chunk.usage:
                usage = chunk.usage
            
            if chunk.choices and chunk.choices[0].get("finish_reason"):
                response_id = chunk.id
                break
        
        return OpenAIChatResponse(
            id=response_id,
            object="chat.completion",
            created=int(time.time()),
            model=request.model,
            choices=[{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            usage=usage
        )
    
    async def chat_stream(
        self,
        request: OpenAIChatRequest,
        timeout: Optional[int] = None
    ) -> AsyncIterator[OpenAIStreamChunk]:
        """
        发送流式聊天请求（WebSocket）
        """
        if not HAS_WEBSOCKETS:
            raise RuntimeError("websockets 库未安装，请运行: pip install websockets")
        
        # 构建认证 URL
        auth_url = await self._build_auth_url()
        
        # 转换请求
        ws_request = await self._convert_request(request)
        
        # 设置超时
        request_timeout = timeout or self.config.get('timeout', 60)
        
        # 连接 WebSocket
        try:
            async with websockets.connect(
                auth_url,
                ping_interval=None,  # 禁用 ping，由服务端控制
                close_timeout=10
            ) as websocket:
                # 发送请求
                await websocket.send(json.dumps(ws_request, ensure_ascii=False))
                
                # 接收响应
                response_id = None
                response_created = int(time.time())
                accumulated_usage = None
                
                async for message in websocket:
                    try:
                        data = json.loads(message)
                    except json.JSONDecodeError:
                        continue
                    
                    # 解析响应
                    chunk = await self._parse_response(data)
                    if chunk is None:
                        continue
                    
                    # 保存响应元数据
                    if not response_id:
                        response_id = chunk.id or f"chatcmpl-{int(time.time())}"
                    
                    # 累积 usage
                    if chunk.usage:
                        if accumulated_usage is None:
                            accumulated_usage = {
                                "prompt_tokens": 0,
                                "completion_tokens": 0,
                                "total_tokens": 0
                            }
                        accumulated_usage["prompt_tokens"] += chunk.usage.get("prompt_tokens", 0)
                        accumulated_usage["completion_tokens"] += chunk.usage.get("completion_tokens", 0)
                        accumulated_usage["total_tokens"] += chunk.usage.get("total_tokens", 0)
                    
                    # 更新 chunk 的 id 和 created
                    chunk.id = response_id
                    chunk.created = response_created
                    
                    yield chunk
                    
                    # 检查是否完成
                    if await self._is_response_complete(data):
                        # 发送最后一个包含 usage 的 chunk
                        if accumulated_usage:
                            yield OpenAIStreamChunk(
                                id=response_id,
                                created=response_created,
                                model=request.model,
                                choices=[{
                                    "index": 0,
                                    "delta": {},
                                    "finish_reason": "stop"
                                }],
                                usage=accumulated_usage
                            )
                        break
                
                # 确保发送完成标记
                yield OpenAIStreamChunk(
                    id=response_id or f"chatcmpl-{int(time.time())}",
                    created=response_created,
                    model=request.model,
                    choices=[{
                        "index": 0,
                        "delta": {},
                        "finish_reason": "stop"
                    }],
                    usage=accumulated_usage
                )
        
        except websockets.exceptions.ConnectionClosed:
            raise Exception("WebSocket 连接已关闭")
        except asyncio.TimeoutError:
            raise Exception(f"WebSocket 请求超时（{request_timeout}秒）")
        except Exception as e:
            raise Exception(f"WebSocket 请求失败: {str(e)}")

