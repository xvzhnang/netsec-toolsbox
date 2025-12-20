# -*- coding: utf-8 -*-
"""
OpenAI-compatible API Handler - 绝对不崩溃版本
对应 One API 的 router/relay.go + controller/relay.go
"""
import json
import asyncio
import sys
import os
import time
import traceback
import atexit
from http.server import BaseHTTPRequestHandler
from urllib.parse import urlparse

# 添加 ai_service 目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from core.router import Router
from core.adapter.base_adapter import OpenAIChatRequest

# === 进程退出监控 ===
_is_normal_exit = False

def _mark_normal_exit():
    global _is_normal_exit
    _is_normal_exit = True

def _exit_handler():
    """进程退出时的处理"""
    global _is_normal_exit
    if not _is_normal_exit:
        exc_type, exc_value, exc_traceback = sys.exc_info()
        if exc_type is not None:
            try:
                print(f"[EXIT] 进程因异常退出: {exc_type.__name__}: {exc_value}", file=sys.stderr, flush=True)
                traceback.print_exception(exc_type, exc_value, exc_traceback, file=sys.stderr)
            except:
                pass

atexit.register(_exit_handler)

# 设置自定义异常处理
_original_excepthook = sys.excepthook

def _custom_excepthook(exc_type, exc_value, exc_traceback):
    """捕获未处理的异常"""
    try:
        print(f"[UNHANDLED] 未捕获的异常: {exc_type.__name__}: {exc_value}", file=sys.stderr, flush=True)
        traceback.print_exception(exc_type, exc_value, exc_traceback, file=sys.stderr)
    except:
        pass
    # 调用原始 hook
    _original_excepthook(exc_type, exc_value, exc_traceback)

sys.excepthook = _custom_excepthook


class AIRequestHandler(BaseHTTPRequestHandler):
    """OpenAI-compatible API Handler - 绝对不崩溃版本"""
    
    def __init__(self, *args, router: Router, **kwargs):
        """
        初始化 Handler
        
        Args:
            router: 路由器实例
        """
        self.router = router
        super().__init__(*args, **kwargs)
    
    def do_OPTIONS(self):
        """处理 CORS 预检请求"""
        try:
            self.send_response(200)
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
            self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
            self.end_headers()
        except Exception:
            pass  # 忽略 OPTIONS 请求的错误
    
    def do_GET(self):
        """处理 GET 请求"""
        try:
            parsed_path = urlparse(self.path)
            path = parsed_path.path
            
            if path == '/v1/models':
                self._handle_list_models_safe()
            elif path == '/health':
                self._send_json_response_safe({"status": "ok"})
            elif path == '/reload':
                self._handle_reload_config_safe()
            else:
                self._send_error_safe(404, "Not Found")
        except Exception as e:
            # 即使 GET 请求也要保证不崩溃
            try:
                self._send_error_safe(500, "Internal server error")
            except:
                pass
    
    def do_POST(self):
        """处理 POST 请求 - 绝对不崩溃"""
        try:
            parsed_path = urlparse(self.path)
            
            if parsed_path.path == '/v1/chat/completions':
                self._handle_chat_completions_safe()
            else:
                self._send_error_safe(404, "Not Found")
        except BaseException as e:
            # 捕获所有异常（包括 SystemExit, KeyboardInterrupt 以外的）
            if isinstance(e, (KeyboardInterrupt, SystemExit)):
                raise  # 重新抛出，让主程序处理
            
            # 其他异常：尝试发送错误响应
            try:
                self._send_error_safe(500, "Internal server error")
            except:
                pass  # 如果连错误响应都发不出去，只能放弃
    
    def _handle_chat_completions_safe(self):
        """处理 /v1/chat/completions 请求 - 绝对不崩溃版本"""
        response_sent = False
        loop = None
        
        try:
            # === 阶段 1：解析请求（最外层保护）===
            try:
                content_length = int(self.headers.get('Content-Length', 0))
                if content_length == 0:
                    self._send_error_safe(400, "Request body is required")
                    response_sent = True
                    return
                
                request_body = self.rfile.read(content_length)
                request_data = json.loads(request_body.decode('utf-8'))
                
                model_id = request_data.get('model')
                if not model_id:
                    self._send_error_safe(400, "Missing 'model' field")
                    response_sent = True
                    return
                
                is_stream = request_data.get('stream', False)
                chat_request = OpenAIChatRequest(
                    model=model_id,
                    messages=request_data.get('messages', []),
                    temperature=request_data.get('temperature'),
                    max_tokens=request_data.get('max_tokens'),
                    stream=is_stream,
                    top_p=request_data.get('top_p'),
                    frequency_penalty=request_data.get('frequency_penalty'),
                    presence_penalty=request_data.get('presence_penalty'),
                    stop=request_data.get('stop'),
                    user=request_data.get('user')
                )
            except (ValueError, KeyError, json.JSONDecodeError, UnicodeDecodeError) as e:
                self._send_error_safe(400, f"Invalid request: {self._sanitize_error(str(e))}")
                response_sent = True
                return
            except Exception as e:
                self._send_error_safe(400, "Invalid request format")
                response_sent = True
                return
            
            # === 阶段 2：路由和处理（事件循环保护）===
            try:
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                
                if is_stream:
                    self._handle_stream_response_safe(model_id, chat_request, loop)
                    response_sent = True
                else:
                    try:
                        response = loop.run_until_complete(
                            asyncio.wait_for(
                                self.router.route(model_id, chat_request),
                                timeout=300.0  # 5分钟超时
                            )
                        )
                        
                        response_dict = {
                            "id": response.id,
                            "object": response.object,
                            "created": response.created,
                            "model": response.model,
                            "choices": response.choices,
                        }
                        if response.usage:
                            response_dict["usage"] = response.usage
                        
                        self._send_json_response_safe(response_dict)
                        response_sent = True
                    except asyncio.TimeoutError:
                        self._send_error_safe(504, "Request timeout")
                        response_sent = True
                    except ValueError as e:
                        self._send_error_safe(404, str(e))
                        response_sent = True
                    except Exception as e:
                        error_msg = self._sanitize_error(str(e))
                        self._send_error_safe(500, error_msg)
                        response_sent = True
            
            except Exception as e:
                if not response_sent:
                    error_msg = self._sanitize_error(str(e))
                    self._send_error_safe(500, f"Internal server error: {error_msg}")
                    response_sent = True
        
        except BaseException as e:
            if isinstance(e, (KeyboardInterrupt, SystemExit)):
                raise
            
            if not response_sent:
                try:
                    self._send_error_safe(500, "Internal server error")
                except:
                    pass
        
        finally:
            # === 阶段 3：清理事件循环（绝对安全）===
            if loop:
                try:
                    # 取消所有未完成的任务
                    pending = asyncio.all_tasks(loop)
                    for task in pending:
                        task.cancel()
                    
                    # 等待任务取消完成（最多等待1秒）
                    if pending:
                        try:
                            loop.run_until_complete(
                                asyncio.wait(pending, timeout=1.0)
                            )
                        except:
                            pass
                except Exception:
                    pass
                
                try:
                    loop.close()
                except Exception:
                    pass
    
    def _handle_stream_response_safe(self, model_id: str, chat_request: OpenAIChatRequest, loop: asyncio.AbstractEventLoop):
        """处理流式响应 - 绝对不崩溃版本"""
        response_sent = False
        
        try:
            # 设置 SSE 响应头
            try:
                self.send_response(200)
                self.send_header('Content-Type', 'text/event-stream; charset=utf-8')
                self.send_header('Cache-Control', 'no-cache')
                self.send_header('Connection', 'keep-alive')
                self.send_header('Access-Control-Allow-Origin', '*')
                self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
                self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
                self.end_headers()
                response_sent = True
            except (BrokenPipeError, ConnectionResetError, OSError):
                return  # 客户端已断开
        
            # 获取适配器
            try:
                adapter = self.router.registry.get_adapter(model_id)
                if not adapter:
                    self._send_stream_error_safe("Model not found")
                    return
            except Exception as e:
                self._send_stream_error_safe(f"Error getting adapter: {self._sanitize_error(str(e))}")
                return
            
            # 流式生成器（带异常处理）
            async def stream_generator():
                try:
                    async for chunk in adapter.chat_stream(chat_request):
                        yield chunk
                except Exception as e:
                    # 生成器内部异常，发送错误 chunk
                    from core.adapter.base_adapter import OpenAIStreamChunk
                    error_chunk = OpenAIStreamChunk(
                        id=f'error-{os.urandom(8).hex()}',
                        created=int(time.time()),
                        model=model_id,
                        choices=[{
                            "index": 0,
                            "delta": {"content": f"\n\n[Error: {self._sanitize_error(str(e))}]"},
                            "finish_reason": "error"
                        }]
                    )
                    yield error_chunk
                    raise
            
            # 发送流式数据
            generator = stream_generator()
            done = False
            max_iterations = 10000  # 防止无限循环
            iteration = 0
            
            while not done and iteration < max_iterations:
                iteration += 1
                chunk = None
                
                try:
                    chunk = loop.run_until_complete(
                        asyncio.wait_for(generator.__anext__(), timeout=30.0)
                    )
                except StopAsyncIteration:
                    try:
                        self._write_safe("data: [DONE]\n\n")
                    except:
                        pass
                    done = True
                    break
                except asyncio.TimeoutError:
                    try:
                        self._write_safe(": heartbeat\n\n")
                    except:
                        done = True
                        break
                    continue
                except (BrokenPipeError, ConnectionResetError, OSError):
                    # 客户端断开连接，正常退出
                    done = True
                    break
                except Exception as e:
                    error_msg = self._sanitize_error(str(e))
                    self._send_stream_error_safe(error_msg)
                    done = True
                    break
                
                # 发送 chunk
                if chunk:
                    try:
                        chunk_dict = {
                            "id": chunk.id,
                            "object": chunk.object,
                            "created": chunk.created,
                            "model": chunk.model,
                            "choices": chunk.choices,
                        }
                        if chunk.usage:
                            chunk_dict["usage"] = chunk.usage
                        
                        chunk_json = json.dumps(chunk_dict, ensure_ascii=False)
                        self._write_safe(f"data: {chunk_json}\n\n")
                        
                        # 检查是否完成
                        if chunk.choices:
                            finish_reason = chunk.choices[0].get("finish_reason")
                            if finish_reason:
                                self._write_safe("data: [DONE]\n\n")
                                done = True
                    except (BrokenPipeError, ConnectionResetError, OSError):
                        done = True
                        break
                    except Exception as e:
                        error_msg = self._sanitize_error(str(e))
                        self._send_stream_error_safe(error_msg)
                        done = True
                        break
            
            if iteration >= max_iterations:
                try:
                    self._write_safe("data: [DONE]\n\n")
                except:
                    pass
        
        except Exception as e:
            if not response_sent:
                try:
                    self._send_stream_error_safe(self._sanitize_error(str(e)))
                except:
                    pass
    
    def _write_safe(self, data: str):
        """安全写入（捕获所有 I/O 异常）"""
        try:
            if isinstance(data, str):
                data = data.encode('utf-8')
            self.wfile.write(data)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError, OSError, IOError):
            # 客户端断开连接，重新抛出让调用方知道
            raise
        except Exception as e:
            # 其他 I/O 错误，也重新抛出
            raise
    
    def _send_json_response_safe(self, data: dict):
        """安全发送 JSON 响应"""
        try:
            response = json.dumps(data, ensure_ascii=False).encode('utf-8')
            self.send_response(200)
            self.send_header('Content-Type', 'application/json; charset=utf-8')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Content-Length', str(len(response)))
            self.end_headers()
            self.wfile.write(response)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError, OSError, IOError):
            pass  # 客户端断开，忽略
        except Exception as e:
            try:
                print(f"Error sending JSON response: {e}", file=sys.stderr, flush=True)
            except:
                pass
    
    def _send_error_safe(self, status_code: int, message: str):
        """安全发送错误响应"""
        try:
            error_response = {
                "error": {
                    "message": message,
                    "type": "invalid_request_error" if status_code < 500 else "server_error",
                    "code": str(status_code)
                }
            }
            response = json.dumps(error_response, ensure_ascii=False).encode('utf-8')
            self.send_response(status_code)
            self.send_header('Content-Type', 'application/json; charset=utf-8')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Content-Length', str(len(response)))
            self.end_headers()
            self.wfile.write(response)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError, OSError, IOError):
            pass  # 客户端断开，忽略
        except Exception as e:
            try:
                print(f"Error sending error response: {e}", file=sys.stderr, flush=True)
            except:
                pass
    
    def _send_stream_error_safe(self, message: str):
        """安全发送流式错误响应"""
        try:
            error_chunk = {
                "error": {
                    "message": message,
                    "type": "server_error",
                    "code": "500"
                }
            }
            error_json = json.dumps(error_chunk, ensure_ascii=False)
            self._write_safe(f"data: {error_json}\n\n")
            self._write_safe("data: [DONE]\n\n")
        except (BrokenPipeError, ConnectionResetError, OSError, IOError):
            pass  # 客户端断开，忽略
        except Exception as e:
            try:
                print(f"Error sending stream error: {e}", file=sys.stderr, flush=True)
            except:
                pass
    
    def _handle_list_models_safe(self):
        """安全处理 /v1/models 请求"""
        try:
            models = self.router.registry.list_models()
            self._send_json_response_safe(models)
        except Exception as e:
            self._send_error_safe(500, "Failed to list models")
    
    def _handle_reload_config_safe(self):
        """安全处理 /reload 请求"""
        try:
            self.router.registry.reload()
            self._send_json_response_safe({"status": "ok", "message": "配置已重新加载"})
        except Exception as e:
            self._send_error_safe(500, f"重新加载配置失败: {self._sanitize_error(str(e))}")
    
    def _sanitize_error(self, error_msg: str) -> str:
        """清理错误消息（移除敏感信息）"""
        if 'api_key' in error_msg.lower() or 'key' in error_msg.lower():
            return "API 配置错误"
        if len(error_msg) > 200:
            return error_msg[:200] + "..."
        return error_msg
    
    def log_message(self, format, *args):
        """重写日志方法，避免输出到 stderr"""
        pass

