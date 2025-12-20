# -*- coding: utf-8 -*-
"""
OpenAI-compatible API Handler
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
from typing import Optional
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
    _original_excepthook(exc_type, exc_value, exc_traceback)

sys.excepthook = _custom_excepthook


class AIRequestHandler(BaseHTTPRequestHandler):
    """OpenAI-compatible API Handler"""
    
    def __init__(self, *args, router: Router, **kwargs):
        """
        初始化 Handler
        
        Args:
            router: 路由器实例
        """
        try:
            print(f"[HANDLER-INIT] 初始化 AIRequestHandler", file=sys.stderr, flush=True)
            self.router = router
            super().__init__(*args, **kwargs)
            print(f"[HANDLER-INIT] AIRequestHandler 初始化完成", file=sys.stderr, flush=True)
        except Exception as e:
            print(f"[HANDLER-INIT] 初始化失败: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            raise
    
    def handle(self):
        """重写 handle 方法，添加调试信息"""
        try:
            print(f"[HANDLER] ===== 收到新请求 =====", file=sys.stderr, flush=True)
            print(f"[HANDLER] 客户端地址: {self.client_address}", file=sys.stderr, flush=True)
            print(f"[HANDLER] 请求方法: {self.command}", file=sys.stderr, flush=True)
            print(f"[HANDLER] 请求路径: {self.path}", file=sys.stderr, flush=True)
            super().handle()
            print(f"[HANDLER] ===== 请求处理完成 =====", file=sys.stderr, flush=True)
        except BaseException as e:
            print(f"[HANDLER] [FATAL] BaseException 在 handle 中: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            # 不重新抛出，避免进程退出
            try:
                self.send_error(500, "Internal server error")
            except:
                pass
        except Exception as e:
            print(f"[HANDLER] [ERROR] Exception 在 handle 中: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            try:
                self.send_error(500, "Internal server error")
            except:
                pass
    
    def do_OPTIONS(self):
        """处理 CORS 预检请求"""
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
        self.end_headers()
    
    def do_GET(self):
        """处理 GET 请求"""
        parsed_path = urlparse(self.path)
        path = parsed_path.path
        
        if path == '/v1/models':
            self._handle_list_models()
        elif path == '/health':
            self._send_json_response({"status": "ok"})
        elif path == '/reload':
            self._handle_reload_config()
        else:
            # 调试：输出路径信息
            try:
                if sys.stdout and not sys.stdout.closed:
                    print(f"DEBUG: 未匹配的路径: {path}, 完整路径: {self.path}", flush=True)
            except:
                pass
            self._send_error(404, "Not Found")
    
    def do_POST(self):
        """处理 POST 请求 - 绝对不退出进程版本"""
        # 生成请求 ID 用于跟踪
        request_id = os.urandom(4).hex()
        
        # 立即输出日志，确保即使后续崩溃也能看到
        print(f"[REQUEST-{request_id}] ===== do_POST 被调用 =====", file=sys.stderr, flush=True)
        print(f"[REQUEST-{request_id}] 时间戳: {time.time()}", file=sys.stderr, flush=True)
        print(f"[REQUEST-{request_id}] 路径: {self.path}", file=sys.stderr, flush=True)
        
        try:
            print(f"[REQUEST-{request_id}] ===== 开始处理 POST 请求 =====", file=sys.stderr, flush=True)
            print(f"[REQUEST-{request_id}] 路径: {self.path}", file=sys.stderr, flush=True)
            
            parsed_path = urlparse(self.path)
            print(f"[REQUEST-{request_id}] 解析后的路径: {parsed_path.path}", file=sys.stderr, flush=True)
            
            if parsed_path.path == '/v1/chat/completions':
                print(f"[REQUEST-{request_id}] 调用 _handle_chat_completions", file=sys.stderr, flush=True)
                self._handle_chat_completions(request_id)
                print(f"[REQUEST-{request_id}] _handle_chat_completions 返回", file=sys.stderr, flush=True)
            else:
                print(f"[REQUEST-{request_id}] 路径不匹配，返回 404", file=sys.stderr, flush=True)
                self._send_error(404, "Not Found")
            
            print(f"[REQUEST-{request_id}] ===== POST 请求处理完成 =====", file=sys.stderr, flush=True)
        
        except (KeyboardInterrupt, SystemExit) as e:
            # 系统级异常：不应该在请求处理中发生
            # 如果发生，记录日志但不退出进程（不重新抛出）
            try:
                print(f"[REQUEST-{request_id}] [FATAL] 系统级异常: {type(e).__name__}: {e}", 
                      file=sys.stderr, flush=True)
                print(f"[REQUEST-{request_id}] [FATAL] 堆栈跟踪:", file=sys.stderr, flush=True)
                traceback.print_exc(file=sys.stderr)
                print(f"[REQUEST-{request_id}] [FATAL] 尝试发送错误响应...", file=sys.stderr, flush=True)
                self._send_error(500, "Internal server error")
                print(f"[REQUEST-{request_id}] [FATAL] 错误响应已发送", file=sys.stderr, flush=True)
            except Exception as send_error:
                print(f"[REQUEST-{request_id}] [FATAL] 发送错误响应失败: {send_error}", 
                      file=sys.stderr, flush=True)
                traceback.print_exc(file=sys.stderr)
            # 关键：不重新抛出，不退出进程
            print(f"[REQUEST-{request_id}] [FATAL] 不退出进程，继续运行", file=sys.stderr, flush=True)
        
        except BaseException as e:
            # 捕获所有其他 BaseException（如 GeneratorExit）
            try:
                print(f"[REQUEST-{request_id}] [FATAL] BaseException: {type(e).__name__}: {e}", 
                      file=sys.stderr, flush=True)
                print(f"[REQUEST-{request_id}] [FATAL] 堆栈跟踪:", file=sys.stderr, flush=True)
                traceback.print_exc(file=sys.stderr)
                print(f"[REQUEST-{request_id}] [FATAL] 尝试发送错误响应...", file=sys.stderr, flush=True)
                self._send_error(500, "Internal server error")
                print(f"[REQUEST-{request_id}] [FATAL] 错误响应已发送", file=sys.stderr, flush=True)
            except Exception as send_error:
                print(f"[REQUEST-{request_id}] [FATAL] 发送错误响应失败: {send_error}", 
                      file=sys.stderr, flush=True)
                traceback.print_exc(file=sys.stderr)
            # 关键：不重新抛出，不退出进程
            print(f"[REQUEST-{request_id}] [FATAL] 不退出进程，继续运行", file=sys.stderr, flush=True)
        
        except Exception as e:
            # 捕获所有普通异常
            try:
                print(f"[REQUEST-{request_id}] [ERROR] 未预期的异常: {type(e).__name__}: {e}", 
                      file=sys.stderr, flush=True)
                print(f"[REQUEST-{request_id}] [ERROR] 堆栈跟踪:", file=sys.stderr, flush=True)
                traceback.print_exc(file=sys.stderr)
                print(f"[REQUEST-{request_id}] [ERROR] 尝试发送错误响应...", file=sys.stderr, flush=True)
                self._send_error(500, "Internal server error")
                print(f"[REQUEST-{request_id}] [ERROR] 错误响应已发送", file=sys.stderr, flush=True)
            except Exception as send_error:
                print(f"[REQUEST-{request_id}] [ERROR] 发送错误响应失败: {send_error}", 
                      file=sys.stderr, flush=True)
                traceback.print_exc(file=sys.stderr)
        
        finally:
            print(f"[REQUEST-{request_id}] ===== POST 请求处理结束（finally）=====", file=sys.stderr, flush=True)
        
        # === 关键：无论发生什么，都不退出进程 ===
        # 不调用 sys.exit()
        # 不调用 exit()
        # 不调用 os._exit()
        # 不重新抛出 SystemExit 或 KeyboardInterrupt
    
    def _handle_chat_completions(self, request_id: str = "unknown"):
        """处理 /v1/chat/completions 请求 - 绝对不崩溃版本"""
        response_sent = False
        loop = None
        
        try:
            print(f"[REQUEST-{request_id}] [STEP-1] 开始处理聊天请求", file=sys.stderr, flush=True)
            # === 阶段 1：解析请求（最外层保护）===
            try:
                print(f"[REQUEST-{request_id}] [STEP-1.1] 读取请求体", file=sys.stderr, flush=True)
                content_length = int(self.headers.get('Content-Length', 0))
                print(f"[REQUEST-{request_id}] [STEP-1.1] Content-Length: {content_length}", file=sys.stderr, flush=True)
                
                if content_length == 0:
                    print(f"[REQUEST-{request_id}] [STEP-1.1] 请求体为空，返回 400", file=sys.stderr, flush=True)
                    self._send_error(400, "Request body is required")
                    response_sent = True
                    return
                
                print(f"[REQUEST-{request_id}] [STEP-1.2] 读取 {content_length} 字节", file=sys.stderr, flush=True)
                request_body = self.rfile.read(content_length)
                print(f"[REQUEST-{request_id}] [STEP-1.2] 已读取 {len(request_body)} 字节", file=sys.stderr, flush=True)
                
                print(f"[REQUEST-{request_id}] [STEP-1.3] 解析 JSON", file=sys.stderr, flush=True)
                request_data = json.loads(request_body.decode('utf-8'))
                print(f"[REQUEST-{request_id}] [STEP-1.3] JSON 解析成功", file=sys.stderr, flush=True)
                
                model_id = request_data.get('model')
                print(f"[REQUEST-{request_id}] [STEP-1.4] 模型 ID: {model_id}", file=sys.stderr, flush=True)
                
                if not model_id:
                    print(f"[REQUEST-{request_id}] [STEP-1.4] 模型 ID 为空，返回 400", file=sys.stderr, flush=True)
                    self._send_error(400, "Missing 'model' field")
                    response_sent = True
                    return
                
                is_stream = request_data.get('stream', False)
                print(f"[REQUEST-{request_id}] [STEP-1.5] 流式模式: {is_stream}", file=sys.stderr, flush=True)
                
                print(f"[REQUEST-{request_id}] [STEP-1.6] 构建 OpenAIChatRequest", file=sys.stderr, flush=True)
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
                print(f"[REQUEST-{request_id}] [STEP-1.6] OpenAIChatRequest 构建成功", file=sys.stderr, flush=True)
                
            except (ValueError, KeyError, json.JSONDecodeError, UnicodeDecodeError) as e:
                print(f"[REQUEST-{request_id}] [STEP-1] 请求解析错误: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
                traceback.print_exc(file=sys.stderr)
                self._send_error(400, f"Invalid request: {self._sanitize_error(str(e))}")
                response_sent = True
                return
            except Exception as e:
                print(f"[REQUEST-{request_id}] [STEP-1] 请求解析异常: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
                traceback.print_exc(file=sys.stderr)
                self._send_error(400, "Invalid request format")
                response_sent = True
                return
            
            # === 阶段 2：路由和处理（事件循环保护）===
            try:
                print(f"[REQUEST-{request_id}] [STEP-2] 开始路由和处理", file=sys.stderr, flush=True)
                print(f"[REQUEST-{request_id}] [STEP-2.1] 创建新的事件循环", file=sys.stderr, flush=True)
                loop = asyncio.new_event_loop()
                print(f"[REQUEST-{request_id}] [STEP-2.1] 事件循环已创建", file=sys.stderr, flush=True)
                
                print(f"[REQUEST-{request_id}] [STEP-2.2] 设置事件循环", file=sys.stderr, flush=True)
                asyncio.set_event_loop(loop)
                print(f"[REQUEST-{request_id}] [STEP-2.2] 事件循环已设置", file=sys.stderr, flush=True)
                
                if is_stream:
                    print(f"[REQUEST-{request_id}] [STEP-2.3] 处理流式响应", file=sys.stderr, flush=True)
                    self._handle_stream_response(model_id, chat_request, loop, request_id)
                    print(f"[REQUEST-{request_id}] [STEP-2.3] 流式响应处理完成", file=sys.stderr, flush=True)
                    response_sent = True
                else:
                    print(f"[REQUEST-{request_id}] [STEP-2.3] 处理非流式响应", file=sys.stderr, flush=True)
                    try:
                        print(f"[REQUEST-{request_id}] [STEP-2.3.1] 调用 router.route", file=sys.stderr, flush=True)
                        response = loop.run_until_complete(
                            asyncio.wait_for(
                                self.router.route(model_id, chat_request),
                                timeout=300.0  # 5分钟超时
                            )
                        )
                        print(f"[REQUEST-{request_id}] [STEP-2.3.1] router.route 返回成功", file=sys.stderr, flush=True)
                        
                        print(f"[REQUEST-{request_id}] [STEP-2.3.2] 构建响应字典", file=sys.stderr, flush=True)
                        response_dict = {
                            "id": response.id,
                            "object": response.object,
                            "created": response.created,
                            "model": response.model,
                            "choices": response.choices,
                        }
                        if response.usage:
                            response_dict["usage"] = response.usage
                        print(f"[REQUEST-{request_id}] [STEP-2.3.2] 响应字典构建完成", file=sys.stderr, flush=True)
                        
                        print(f"[REQUEST-{request_id}] [STEP-2.3.3] 发送 JSON 响应", file=sys.stderr, flush=True)
                        self._send_json_response(response_dict)
                        print(f"[REQUEST-{request_id}] [STEP-2.3.3] JSON 响应已发送", file=sys.stderr, flush=True)
                        response_sent = True
                    except asyncio.TimeoutError:
                        print(f"[REQUEST-{request_id}] [STEP-2.3] 请求超时", file=sys.stderr, flush=True)
                        self._send_error(504, "Request timeout")
                        response_sent = True
                    except ValueError as e:
                        print(f"[REQUEST-{request_id}] [STEP-2.3] ValueError: {e}", file=sys.stderr, flush=True)
                        self._send_error(404, str(e))
                        response_sent = True
                    except Exception as e:
                        print(f"[REQUEST-{request_id}] [STEP-2.3] 异常: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
                        traceback.print_exc(file=sys.stderr)
                        error_msg = self._sanitize_error(str(e))
                        self._send_error(500, error_msg)
                        response_sent = True
                
                print(f"[REQUEST-{request_id}] [STEP-2] 路由和处理完成", file=sys.stderr, flush=True)
            
            except Exception as e:
                print(f"[REQUEST-{request_id}] [STEP-2] 路由和处理异常: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
                traceback.print_exc(file=sys.stderr)
                if not response_sent:
                    error_msg = self._sanitize_error(str(e))
                    self._send_error(500, f"Internal server error: {error_msg}")
                    response_sent = True
        
        except BaseException as e:
            # 捕获所有 BaseException（包括 SystemExit, GeneratorExit 等）
            # 关键修复：不重新抛出 SystemExit 或 KeyboardInterrupt，而是转换为 HTTP 错误响应
            print(f"[REQUEST-{request_id}] [FATAL] BaseException 捕获: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            
            if isinstance(e, (KeyboardInterrupt, SystemExit)):
                # 系统级异常：记录但不退出进程（不重新抛出）
                print(f"[REQUEST-{request_id}] [FATAL] 系统级异常，不退出进程", file=sys.stderr, flush=True)
                try:
                    if not response_sent:
                        print(f"[REQUEST-{request_id}] [FATAL] 尝试发送错误响应", file=sys.stderr, flush=True)
                        self._send_error(500, "Internal server error")
                        print(f"[REQUEST-{request_id}] [FATAL] 错误响应已发送", file=sys.stderr, flush=True)
                except Exception as send_error:
                    print(f"[REQUEST-{request_id}] [FATAL] 发送错误响应失败: {send_error}", file=sys.stderr, flush=True)
                return  # 不重新抛出，不退出进程
            
            # 其他 BaseException（如 GeneratorExit）
            print(f"[REQUEST-{request_id}] [FATAL] 其他 BaseException，不退出进程", file=sys.stderr, flush=True)
            try:
                if not response_sent:
                    print(f"[REQUEST-{request_id}] [FATAL] 尝试发送错误响应", file=sys.stderr, flush=True)
                    self._send_error(500, "Internal server error")
                    print(f"[REQUEST-{request_id}] [FATAL] 错误响应已发送", file=sys.stderr, flush=True)
            except Exception as send_error:
                print(f"[REQUEST-{request_id}] [FATAL] 发送错误响应失败: {send_error}", file=sys.stderr, flush=True)
            return  # 不重新抛出，不退出进程
        
        finally:
            # === 阶段 3：清理事件循环（绝对安全）===
            print(f"[REQUEST-{request_id}] [STEP-3] 开始清理事件循环", file=sys.stderr, flush=True)
            if loop:
                try:
                    print(f"[REQUEST-{request_id}] [STEP-3.1] 获取未完成的任务", file=sys.stderr, flush=True)
                    # 取消所有未完成的任务
                    pending = asyncio.all_tasks(loop)
                    print(f"[REQUEST-{request_id}] [STEP-3.1] 未完成的任务数: {len(pending)}", file=sys.stderr, flush=True)
                    
                    for task in pending:
                        try:
                            print(f"[REQUEST-{request_id}] [STEP-3.2] 取消任务: {task}", file=sys.stderr, flush=True)
                            task.cancel()
                        except Exception as cancel_error:
                            print(f"[REQUEST-{request_id}] [STEP-3.2] 取消任务失败: {cancel_error}", file=sys.stderr, flush=True)
                    
                    # 等待任务取消完成（最多等待2秒）
                    if pending:
                        try:
                            print(f"[REQUEST-{request_id}] [STEP-3.3] 等待任务取消完成（{len(pending)} 个任务）", file=sys.stderr, flush=True)
                            # 修复：正确使用 asyncio.wait_for 和 asyncio.gather
                            loop.run_until_complete(
                                asyncio.wait_for(
                                    asyncio.gather(*pending, return_exceptions=True),
                                    timeout=2.0
                                )
                            )
                            print(f"[REQUEST-{request_id}] [STEP-3.3] 任务取消完成", file=sys.stderr, flush=True)
                        except asyncio.TimeoutError:
                            print(f"[REQUEST-{request_id}] [STEP-3.3] 等待任务取消超时，强制关闭", file=sys.stderr, flush=True)
                        except Exception as wait_error:
                            print(f"[REQUEST-{request_id}] [STEP-3.3] 等待任务取消失败: {wait_error}", file=sys.stderr, flush=True)
                            traceback.print_exc(file=sys.stderr)
                except Exception as cleanup_error:
                    print(f"[REQUEST-{request_id}] [STEP-3] 清理任务失败: {cleanup_error}", file=sys.stderr, flush=True)
                    traceback.print_exc(file=sys.stderr)
                
                try:
                    print(f"[REQUEST-{request_id}] [STEP-3.4] 关闭事件循环", file=sys.stderr, flush=True)
                    loop.close()
                    print(f"[REQUEST-{request_id}] [STEP-3.4] 事件循环已关闭", file=sys.stderr, flush=True)
                except Exception as close_error:
                    print(f"[REQUEST-{request_id}] [STEP-3.4] 关闭事件循环失败: {close_error}", file=sys.stderr, flush=True)
                    traceback.print_exc(file=sys.stderr)
            else:
                print(f"[REQUEST-{request_id}] [STEP-3] 事件循环为 None，跳过清理", file=sys.stderr, flush=True)
            
            print(f"[REQUEST-{request_id}] [STEP-3] 事件循环清理完成", file=sys.stderr, flush=True)
            print(f"[REQUEST-{request_id}] ===== _handle_chat_completions 结束 =====", file=sys.stderr, flush=True)
    
    def _handle_stream_response(self, model_id: str, chat_request: OpenAIChatRequest, loop: asyncio.AbstractEventLoop, request_id: str = "unknown"):
        """处理流式响应（SSE）- 绝对不崩溃版本"""
        response_sent = False
        
        try:
            print(f"[REQUEST-{request_id}] [STREAM] 开始处理流式响应", file=sys.stderr, flush=True)
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
            except (BrokenPipeError, ConnectionResetError, OSError, IOError):
                return  # 客户端已断开
            
            # 获取适配器
            try:
                adapter = self.router.registry.get_adapter(model_id)
                if not adapter:
                    self._send_stream_error("Model not found")
                    return
            except Exception as e:
                self._send_stream_error(f"Error getting adapter: {self._sanitize_error(str(e))}")
                return
            
            # 流式生成器（带异常处理）
            async def stream_generator():
                try:
                    async for chunk in adapter.chat_stream(chat_request):
                        yield chunk
                except Exception as e:
                    # 生成器内部异常，发送错误 chunk
                    from core.adapter.base_adapter import OpenAIStreamChunk
                    import os
                    import time
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
            
            try:
                while not done and iteration < max_iterations:
                    iteration += 1
                    chunk = None
                    
                    try:
                        print(f"[REQUEST-{request_id}] [STREAM] 等待下一个 chunk (迭代 {iteration})", file=sys.stderr, flush=True)
                        chunk = loop.run_until_complete(
                            asyncio.wait_for(generator.__anext__(), timeout=30.0)
                        )
                        print(f"[REQUEST-{request_id}] [STREAM] 收到 chunk", file=sys.stderr, flush=True)
                    except StopAsyncIteration:
                        print(f"[REQUEST-{request_id}] [STREAM] 生成器结束", file=sys.stderr, flush=True)
                        try:
                            self._write_safe("data: [DONE]\n\n")
                        except:
                            pass
                        done = True
                        break
                    except asyncio.TimeoutError:
                        print(f"[REQUEST-{request_id}] [STREAM] 超时，发送心跳", file=sys.stderr, flush=True)
                        try:
                            self._write_safe(": heartbeat\n\n")
                        except (BrokenPipeError, ConnectionResetError, OSError, IOError):
                            print(f"[REQUEST-{request_id}] [STREAM] 发送心跳时客户端断开", file=sys.stderr, flush=True)
                            done = True
                            break
                        continue
                    except (BrokenPipeError, ConnectionResetError, OSError, IOError) as e:
                        # 客户端断开连接，正常退出
                        print(f"[REQUEST-{request_id}] [STREAM] 客户端断开连接: {type(e).__name__}", file=sys.stderr, flush=True)
                        done = True
                        break
                    except Exception as e:
                        print(f"[REQUEST-{request_id}] [STREAM] 获取 chunk 异常: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
                        traceback.print_exc(file=sys.stderr)
                        error_msg = self._sanitize_error(str(e))
                        try:
                            self._send_stream_error(error_msg)
                        except:
                            pass
                        done = True
                        break
                    
                    # 发送 chunk
                    if chunk:
                        try:
                            print(f"[REQUEST-{request_id}] [STREAM] 发送 chunk", file=sys.stderr, flush=True)
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
                                    print(f"[REQUEST-{request_id}] [STREAM] 收到完成标记: {finish_reason}", file=sys.stderr, flush=True)
                                    self._write_safe("data: [DONE]\n\n")
                                    done = True
                        except (BrokenPipeError, ConnectionResetError, OSError, IOError) as e:
                            print(f"[REQUEST-{request_id}] [STREAM] 发送 chunk 时客户端断开: {type(e).__name__}", file=sys.stderr, flush=True)
                            done = True
                            break
                        except Exception as e:
                            print(f"[REQUEST-{request_id}] [STREAM] 发送 chunk 异常: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
                            traceback.print_exc(file=sys.stderr)
                            error_msg = self._sanitize_error(str(e))
                            try:
                                self._send_stream_error(error_msg)
                            except:
                                pass
                            done = True
                            break
                
                if iteration >= max_iterations:
                    print(f"[REQUEST-{request_id}] [STREAM] 达到最大迭代次数，强制结束", file=sys.stderr, flush=True)
                    try:
                        self._write_safe("data: [DONE]\n\n")
                    except:
                        pass
            finally:
                # 确保生成器被正确关闭
                print(f"[REQUEST-{request_id}] [STREAM] 清理生成器", file=sys.stderr, flush=True)
                try:
                    # 尝试关闭生成器
                    if hasattr(generator, 'aclose'):
                        loop.run_until_complete(
                            asyncio.wait_for(generator.aclose(), timeout=1.0)
                        )
                except Exception as close_error:
                    print(f"[REQUEST-{request_id}] [STREAM] 关闭生成器失败: {close_error}", file=sys.stderr, flush=True)
                
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
                    except (BrokenPipeError, ConnectionResetError, OSError, IOError):
                        done = True
                        break
                    except Exception as e:
                        error_msg = self._sanitize_error(str(e))
                        self._send_stream_error(error_msg)
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
                    self._send_stream_error(self._sanitize_error(str(e)))
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
    
    def _send_stream_error(self, message: str):
        """发送流式错误响应 - 绝对安全版本，不抛出任何异常"""
        try:
            error_chunk = {
                "error": {
                    "message": message,
                    "type": "server_error",
                    "code": "500"
                }
            }
            error_json = json.dumps(error_chunk, ensure_ascii=False)
            try:
                self._write_safe(f"data: {error_json}\n\n")
            except:
                pass  # 如果写入失败，继续尝试发送 [DONE]
            try:
                self._write_safe("data: [DONE]\n\n")
            except:
                pass  # 如果发送 [DONE] 也失败，忽略
        except (BrokenPipeError, ConnectionResetError, OSError, IOError):
            pass  # 客户端断开，忽略
        except BaseException as e:
            # 即使 BaseException 也要捕获，不重新抛出
            try:
                print(f"[FATAL] BaseException 在发送流式错误响应时发生: {type(e).__name__}: {e}", 
                      file=sys.stderr, flush=True)
            except:
                pass
        except Exception as e:
            try:
                print(f"[ERROR] 发送流式错误响应失败: {type(e).__name__}: {e}", 
                      file=sys.stderr, flush=True)
            except:
                pass
    
    def _handle_list_models(self):
        """处理 /v1/models 请求"""
        models = self.router.registry.list_models()
        self._send_json_response(models)
    
    def _handle_reload_config(self):
        """处理 /reload 请求 - 重新加载配置文件"""
        try:
            self.router.registry.reload()
            self._send_json_response({"status": "ok", "message": "配置已重新加载"})
        except Exception as e:
            self._send_error(500, f"重新加载配置失败: {str(e)}")
    
    def _send_json_response(self, data: dict):
        """发送 JSON 响应 - 绝对安全版本，不抛出任何异常"""
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
        except BaseException as e:
            # 即使 BaseException 也要捕获，不重新抛出
            try:
                print(f"[FATAL] BaseException 在发送 JSON 响应时发生: {type(e).__name__}: {e}", 
                      file=sys.stderr, flush=True)
            except:
                pass
        except Exception as e:
            try:
                print(f"[ERROR] 发送 JSON 响应失败: {type(e).__name__}: {e}", 
                      file=sys.stderr, flush=True)
            except:
                pass
    
    def _send_error(self, status_code: int, message: str):
        """发送错误响应（OpenAI 格式）- 安全版本"""
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
    
    def _sanitize_error(self, error_msg: str) -> str:
        """清理错误消息（移除敏感信息）"""
        if 'api_key' in error_msg.lower() or 'key' in error_msg.lower():
            return "API 配置错误"
        if len(error_msg) > 200:
            return error_msg[:200] + "..."
        return error_msg
    
    def log_message(self, format, *args):
        """重写日志方法，避免输出到 stderr"""
        # 可以在这里添加自定义日志逻辑
        pass

