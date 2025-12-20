# -*- coding: utf-8 -*-
"""
OpenAI 兼容适配器
适用于 OpenAI, DeepSeek, Ollama, vLLM, LocalAI, LM Studio, Groq, Together, Mistral 等
对应 One API 的 relay/adaptor/openai/
"""
import os
import json
from typing import Dict, Any, Optional, AsyncIterator

try:
    import aiohttp
    HAS_AIOHTTP = True
except ImportError:
    HAS_AIOHTTP = False

import sys

# 添加 ai_service 目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from core.adapter.base_adapter import (
    ChatAdapter, 
    OpenAIChatRequest, 
    OpenAIChatResponse,
    OpenAIStreamChunk
)


class OpenAICompatAdapter(ChatAdapter):
    """
    OpenAI 兼容适配器
    直接转发请求，不做协议转换
    """
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        # 直接使用配置文件中的 api_key，不再支持环境变量
        self.api_key = config.get('api_key')
        # 如果 api_key 是 "not-needed"、空字符串或 "ENV:" 开头的环境变量占位符，则设为 None
        if self.api_key == 'not-needed' or self.api_key == '' or (isinstance(self.api_key, str) and self.api_key.startswith('ENV:')):
            self.api_key = None
        
        self.base_url = config.get('base_url')
        self.model = config.get('model', self.model_id)  # 默认使用 model_id 作为模型名称
    
    @property
    def adapter_type(self) -> str:
        return "openai_compat"
    
    def is_available(self) -> bool:
        """检查适配器是否可用"""
        if not HAS_AIOHTTP:
            return False
        
        # 对于某些本地服务，可能不需要 API Key
        if self.base_url and ('ollama' in self.base_url.lower() or 'lmstudio' in self.base_url.lower()):
            return self.base_url is not None
        
        return self.api_key is not None and self.base_url is not None
    
    async def chat(
        self,
        request: OpenAIChatRequest,
        timeout: Optional[int] = None
    ) -> OpenAIChatResponse:
        """
        发送聊天请求（非流式）
        合并了 One API 的 GetRequestURL + SetupRequestHeader + DoRequest + DoResponse
        """
        if not self.is_available():
            raise ValueError(f"模型 {self.model_id} 未配置或不可用")
        
        # 构建 URL（对应 One API 的 GetRequestURL）
        url = f"{self.base_url.rstrip('/')}/chat/completions"
        
        # 构建请求体（对应 One API 的 ConvertRequest，这里是直通）
        request_body = {
            "model": self.model,  # 使用配置中的实际模型名
            "messages": request.messages,
            "stream": False,  # 非流式请求
        }
        
        # 添加可选参数
        if request.temperature is not None:
            request_body["temperature"] = request.temperature
        if request.max_tokens is not None:
            request_body["max_tokens"] = request.max_tokens
        if request.top_p is not None:
            request_body["top_p"] = request.top_p
        if request.frequency_penalty is not None:
            request_body["frequency_penalty"] = request.frequency_penalty
        if request.presence_penalty is not None:
            request_body["presence_penalty"] = request.presence_penalty
        if request.stop is not None:
            request_body["stop"] = request.stop
        if request.user is not None:
            request_body["user"] = request.user
        
        # 设置请求头（对应 One API 的 SetupRequestHeader）
        headers = {
            'Content-Type': 'application/json',
        }
        
        # 某些本地服务可能不需要 API Key
        if self.api_key and self.api_key != 'not-needed':
            headers['Authorization'] = f'Bearer {self.api_key}'
        
        # 发送请求（对应 One API 的 DoRequest）
        request_timeout = timeout or self.config.get('timeout', 60)
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    url,
                    headers=headers,
                    json=request_body,
                    timeout=aiohttp.ClientTimeout(total=request_timeout)
                ) as response:
                    response_data = await response.json()
                    
                    # 处理错误响应
                    if response.status != 200:
                        error_info = response_data.get('error', {})
                        error_msg = error_info.get('message', f'HTTP {response.status}')
                        raise Exception(f"API 错误: {error_msg}")
                    
                    # 解析响应（对应 One API 的 DoResponse，这里是直通）
                    return OpenAIChatResponse(
                        id=response_data.get('id', f'chatcmpl-{os.urandom(12).hex()}'),
                        object=response_data.get('object', 'chat.completion'),
                        created=response_data.get('created', 0),
                        model=response_data.get('model', self.model),
                        choices=response_data.get('choices', []),
                        usage=response_data.get('usage')
                    )
        
        except aiohttp.ClientError as e:
            raise Exception(f"网络请求失败: {str(e)}")
        except Exception as e:
            raise Exception(f"请求处理失败: {str(e)}")
    
    async def chat_stream(
        self,
        request: OpenAIChatRequest,
        timeout: Optional[int] = None
    ) -> AsyncIterator[OpenAIStreamChunk]:
        """
        发送流式聊天请求（SSE）
        对应 One API 的 StreamHandler
        """
        if not self.is_available():
            raise ValueError(f"模型 {self.model_id} 未配置或不可用")
        
        # 构建 URL
        url = f"{self.base_url.rstrip('/')}/chat/completions"
        
        # 构建请求体（流式请求）
        request_body = {
            "model": self.model,
            "messages": request.messages,
            "stream": True,  # 启用流式
        }
        
        # 添加可选参数
        if request.temperature is not None:
            request_body["temperature"] = request.temperature
        if request.max_tokens is not None:
            request_body["max_tokens"] = request.max_tokens
        if request.top_p is not None:
            request_body["top_p"] = request.top_p
        if request.frequency_penalty is not None:
            request_body["frequency_penalty"] = request.frequency_penalty
        if request.presence_penalty is not None:
            request_body["presence_penalty"] = request.presence_penalty
        if request.stop is not None:
            request_body["stop"] = request.stop
        if request.user is not None:
            request_body["user"] = request.user
        
        # 设置请求头
        headers = {
            'Content-Type': 'application/json',
        }
        
        if self.api_key and self.api_key != 'not-needed':
            headers['Authorization'] = f'Bearer {self.api_key}'
        
        request_timeout = timeout or self.config.get('timeout', 60)
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    url,
                    headers=headers,
                    json=request_body,
                    timeout=aiohttp.ClientTimeout(total=request_timeout)
                ) as response:
                    # 处理错误响应
                    if response.status != 200:
                        try:
                            error_data = await response.json()
                            error_info = error_data.get('error', {})
                            error_msg = error_info.get('message', f'HTTP {response.status}')
                        except:
                            error_msg = f'HTTP {response.status}'
                        raise Exception(f"API 错误: {error_msg}")
                    
                    # 读取流式响应（SSE 格式）
                    response_id = None
                    response_created = 0
                    response_model = self.model
                    usage = None
                    buffer = ''
                    
                    # 逐块读取响应内容
                    async for chunk_bytes in response.content.iter_chunked(8192):
                        buffer += chunk_bytes.decode('utf-8', errors='replace')
                        
                        # 按行处理（SSE 格式每行以 \n 结尾）
                        while '\n' in buffer:
                            line, buffer = buffer.split('\n', 1)
                            line = line.strip()
                            
                            # 跳过空行
                            if not line:
                                continue
                            
                            # 处理 SSE 格式：data: {json}
                            if line.startswith('data: '):
                                data_str = line[6:]  # 移除 'data: ' 前缀
                                
                                # 检查是否完成
                                if data_str.strip() == '[DONE]':
                                    # 发送最后一个包含 usage 的 chunk（如果有）
                                    if usage:
                                        yield OpenAIStreamChunk(
                                            id=response_id or f'chatcmpl-{os.urandom(12).hex()}',
                                            created=response_created,
                                            model=response_model,
                                            choices=[{
                                                "index": 0,
                                                "delta": {},
                                                "finish_reason": "stop"
                                            }],
                                            usage=usage
                                        )
                                    return
                                
                                try:
                                    chunk_data = json.loads(data_str)
                                    
                                    # 提取响应元数据（通常在第一个 chunk）
                                    if not response_id:
                                        response_id = chunk_data.get('id', f'chatcmpl-{os.urandom(12).hex()}')
                                        response_created = chunk_data.get('created', 0)
                                        response_model = chunk_data.get('model', self.model)
                                    
                                    # 提取 usage（通常在最后一个 chunk）
                                    if 'usage' in chunk_data:
                                        usage = chunk_data['usage']
                                    
                                    # 跳过空 choices（Azure 等可能发送）
                                    choices = chunk_data.get('choices', [])
                                    if not choices and not usage:
                                        continue
                                    
                                    # 生成流式 chunk
                                    yield OpenAIStreamChunk(
                                        id=response_id,
                                        created=response_created,
                                        model=response_model,
                                        choices=choices,
                                        usage=usage if 'usage' in chunk_data else None
                                    )
                                
                                except json.JSONDecodeError:
                                    # 忽略 JSON 解析错误，继续处理下一行
                                    continue
                    
                    # 处理剩余的 buffer
                    if buffer.strip():
                        if buffer.strip().startswith('data: '):
                            data_str = buffer.strip()[6:]
                            if data_str != '[DONE]':
                                try:
                                    chunk_data = json.loads(data_str)
                                    choices = chunk_data.get('choices', [])
                                    if choices:
                                        if not response_id:
                                            response_id = chunk_data.get('id', f'chatcmpl-{os.urandom(12).hex()}')
                                            response_created = chunk_data.get('created', 0)
                                            response_model = chunk_data.get('model', self.model)
                                        yield OpenAIStreamChunk(
                                            id=response_id,
                                            created=response_created,
                                            model=response_model,
                                            choices=choices,
                                            usage=chunk_data.get('usage')
                                        )
                                except json.JSONDecodeError:
                                    pass
                    
                    # 确保发送完成标记（如果没有 usage）
                    if usage is None:
                        yield OpenAIStreamChunk(
                            id=response_id or f'chatcmpl-{os.urandom(12).hex()}',
                            created=response_created,
                            model=response_model,
                            choices=[{
                                "index": 0,
                                "delta": {},
                                "finish_reason": "stop"
                            }]
                        )
        
        except aiohttp.ClientError as e:
            raise Exception(f"网络请求失败: {str(e)}")
        except Exception as e:
            raise Exception(f"流式请求处理失败: {str(e)}")
