"""
OpenAI-Compatible API Handler
处理 /v1/chat/completions 和 /v1/models 请求
"""

import json
import time
from typing import Dict, Any
from http.server import BaseHTTPRequestHandler

from ..core.registry import ModelRegistry
from ..core.adapter.base_adapter import OpenAIChatRequest, OpenAIChatResponse, OpenAIErrorResponse


class OpenAIHandler:
    """OpenAI-Compatible API Handler"""
    
    def __init__(self, registry: ModelRegistry):
        self.registry = registry
    
    def handle_chat_completions(self, handler: BaseHTTPRequestHandler, data: Dict[str, Any]) -> None:
        """处理 /v1/chat/completions 请求"""
        try:
            # 解析请求
            model = data.get('model')
            if not model:
                self._send_error(handler, 'model is required', 400)
                return
            
            # 获取适配器
            adapter = self.registry.get_adapter(model)
            if not adapter:
                self._send_error(handler, f'Model "{model}" not found', 404)
                return
            
            # 构建请求
            request = OpenAIChatRequest(
                model=model,
                messages=data.get('messages', []),
                temperature=data.get('temperature'),
                max_tokens=data.get('max_tokens'),
                stream=data.get('stream', False),
                timeout=data.get('timeout'),
                top_p=data.get('top_p'),
                frequency_penalty=data.get('frequency_penalty'),
                presence_penalty=data.get('presence_penalty'),
                stop=data.get('stop')
            )
            
            # 调用适配器
            if request.stream:
                # 流式响应（简化实现，先完整生成再拆分）
                response = adapter.chat(request)
                self._send_stream_response(handler, response)
            else:
                # 非流式响应
                response = adapter.chat(request)
                self._send_json_response(handler, response.to_dict(), 200)
        
        except Exception as e:
            error_msg = str(e)
            print(f"❌ [OpenAIHandler] 处理 /v1/chat/completions 失败: {error_msg}", flush=True)
            import traceback
            traceback.print_exc()
            self._send_error(handler, error_msg, 500)
    
    def handle_models(self, handler: BaseHTTPRequestHandler) -> None:
        """处理 /v1/models 请求"""
        try:
            models = self.registry.list_models()
            response = {
                "object": "list",
                "data": models
            }
            self._send_json_response(handler, response, 200)
        except Exception as e:
            error_msg = str(e)
            print(f"❌ [OpenAIHandler] 处理 /v1/models 失败: {error_msg}", flush=True)
            self._send_error(handler, error_msg, 500)
    
    def _send_json_response(self, handler: BaseHTTPRequestHandler, data: Dict[str, Any], status_code: int = 200):
        """发送 JSON 响应"""
        try:
            handler.send_response(status_code)
            handler.send_header('Content-Type', 'application/json; charset=utf-8')
            handler.send_header('Access-Control-Allow-Origin', '*')
            handler.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
            handler.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
            handler.end_headers()
            
            response = json.dumps(data, ensure_ascii=False).encode('utf-8')
            handler.wfile.write(response)
            handler.wfile.flush()
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError, OSError):
            # 客户端提前关闭连接，静默处理
            pass
        except Exception as e:
            error_msg = str(e)
            if 'signal' not in error_msg.lower() and 'SIGALRM' not in error_msg:
                print(f"⚠️ 发送响应失败: {error_msg}", flush=True)
    
    def _send_stream_response(self, handler: BaseHTTPRequestHandler, response: OpenAIChatResponse):
        """发送流式响应（SSE 格式）"""
        try:
            handler.send_response(200)
            handler.send_header('Content-Type', 'text/event-stream; charset=utf-8')
            handler.send_header('Cache-Control', 'no-cache')
            handler.send_header('Connection', 'keep-alive')
            handler.send_header('Access-Control-Allow-Origin', '*')
            handler.end_headers()
            
            # 将完整响应拆分为多个 chunk
            content = response.choices[0]['message']['content'] if response.choices else ''
            chunk_size = 10  # 每次发送 10 个字符
            
            for i in range(0, len(content), chunk_size):
                chunk = content[i:i + chunk_size]
                chunk_response = {
                    "id": response.id,
                    "object": "chat.completion.chunk",
                    "created": response.created,
                    "model": response.model,
                    "choices": [{
                        "index": 0,
                        "delta": {"content": chunk},
                        "finish_reason": None
                    }]
                }
                
                chunk_data = f"data: {json.dumps(chunk_response, ensure_ascii=False)}\n\n"
                handler.wfile.write(chunk_data.encode('utf-8'))
                handler.wfile.flush()
            
            # 发送结束标记
            done_response = {
                "id": response.id,
                "object": "chat.completion.chunk",
                "created": response.created,
                "model": response.model,
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": "stop"
                }]
            }
            done_data = f"data: {json.dumps(done_response, ensure_ascii=False)}\n\n"
            handler.wfile.write(done_data.encode('utf-8'))
            handler.wfile.write(b"data: [DONE]\n\n")
            handler.wfile.flush()
        
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError, OSError):
            pass
        except Exception as e:
            print(f"⚠️ 发送流式响应失败: {str(e)}", flush=True)
    
    def _send_error(self, handler: BaseHTTPRequestHandler, message: str, status_code: int = 400):
        """发送错误响应"""
        error_response = OpenAIErrorResponse({
            "message": message,
            "type": "invalid_request_error",
            "code": status_code
        })
        self._send_json_response(handler, error_response.to_dict(), status_code)
