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
    
    # 类级别的初始化计数器，用于减少日志输出
    _init_count = 0
    _init_lock = None
    
    def __init__(self, *args, router: Router, **kwargs):
        """
        初始化 Handler
        
        Args:
            router: 路由器实例
        """
        try:
            # 只在第一次初始化时打印日志，减少日志噪音
            import threading
            if AIRequestHandler._init_lock is None:
                AIRequestHandler._init_lock = threading.Lock()
            
            with AIRequestHandler._init_lock:
                AIRequestHandler._init_count += 1
                count = AIRequestHandler._init_count
                # 大幅减少日志输出：只在首次和每1000次打印
                should_log = (
                    count == 1 or  # 首次初始化
                    count % 1000 == 0  # 之后每1000次打印一次
                )
            
            if should_log:
                if count == 1:
                    print(f"[HANDLER-INIT] 初始化 AIRequestHandler (首次)", file=sys.stderr, flush=True)
                else:
                    print(f"[HANDLER-INIT] 已初始化 {count} 次", file=sys.stderr, flush=True)
            
            self.router = router
            super().__init__(*args, **kwargs)
        except Exception as e:
            print(f"[HANDLER-INIT] [ERROR] 初始化失败: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            raise
    
    def handle(self):
        """重写 handle 方法，添加异常保护，确保服务不会崩溃"""
        try:
            # 直接调用父类的 handle()，不做任何可能失败的日志记录
            # 日志记录在 do_GET 和 do_POST 中进行
            super().handle()
        except BaseException as e:
            # 捕获所有异常，包括 SystemExit 和 KeyboardInterrupt
            print(f"[HANDLER] [FATAL] BaseException 在 handle 中: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            # 不重新抛出，避免进程退出
            try:
                # 只有在属性已初始化时才能发送错误
                if hasattr(self, 'send_error'):
                    self.send_error(500, "Internal server error")
            except:
                pass
        except Exception as e:
            print(f"[HANDLER] [ERROR] Exception 在 handle 中: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            try:
                if hasattr(self, 'send_error'):
                    self.send_error(500, "Internal server error")
            except:
                pass
    
    def do_OPTIONS(self):
        """处理 CORS 预检请求"""
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization, X-Health-Check-Id, X-Health-Check-Time')
        self.end_headers()
    
    def do_GET(self):
        """处理 GET 请求"""
        try:
            parsed_path = urlparse(self.path)
            path = parsed_path.path
            
            if path == '/v1/models':
                self._handle_list_models()
            elif path == '/health':
                # 健康检查静默处理（减少日志）
                try:
                    self._send_json_response({"status": "ok"})
                except Exception as e:
                    elapsed = (time.time() - request_start_time) * 1000
                    print(f"[HANDLER] [HEALTH] [ERROR] /health 响应发送失败 (耗时: {elapsed:.2f}ms): {type(e).__name__}: {e}", file=sys.stderr, flush=True)
                    traceback.print_exc(file=sys.stderr)
                    try:
                        self._send_error(500, "Internal server error")
                    except:
                        pass
            elif path == '/reload':
                self._handle_reload_config()
            else:
                # 调试：输出路径信息
                print(f"[HANDLER] 未匹配的路径: {path}, 完整路径: {self.path}", file=sys.stderr, flush=True)
                self._send_error(404, "Not Found")
            
            print(f"[HANDLER] ===== GET 请求处理完成 =====", file=sys.stderr, flush=True)
        except Exception as e:
            print(f"[HANDLER] [ERROR] GET 请求处理异常: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            try:
                self._send_error(500, "Internal server error")
            except:
                pass
    
    def do_POST(self):
        """处理 POST 请求 - 绝对不退出进程版本"""
        # 生成请求 ID 用于跟踪
        request_id = os.urandom(4).hex()
        
        # 减少日志输出（只在错误时输出）
        try:
            parsed_path = urlparse(self.path)
            
            if parsed_path.path == '/v1/chat/completions':
                self._handle_chat_completions(request_id)
            else:
                self._send_error(404, "Not Found")
            
            # 静默完成（减少日志）
        
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
            # 静默清理（减少日志）
            pass
        
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
            # === 阶段 1：解析请求（最外层保护）===
            try:
                content_length = int(self.headers.get('Content-Length', 0))
                
                if content_length == 0:
                    self._send_error(400, "Request body is required")
                    response_sent = True
                    return
                
                request_body = self.rfile.read(content_length)
                request_data = json.loads(request_body.decode('utf-8'))
                
                model_id = request_data.get('model')
                
                if not model_id:
                    self._send_error(400, "Missing 'model' field")
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
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                
                if is_stream:
                    self._handle_stream_response(model_id, chat_request, loop, request_id)
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
                        self._send_json_response(response_dict)
                        response_sent = True
                    except asyncio.TimeoutError:
                        # 只记录超时错误
                        print(f"[REQUEST-{request_id}] [ERROR] 请求超时", file=sys.stderr, flush=True)
                        self._send_error(504, "Request timeout")
                        response_sent = True
                    except ValueError as e:
                        # 只记录错误
                        print(f"[REQUEST-{request_id}] [ERROR] ValueError: {e}", file=sys.stderr, flush=True)
                        self._send_error(404, str(e))
                        response_sent = True
                    except Exception as e:
                        # 只记录错误
                        print(f"[REQUEST-{request_id}] [ERROR] 异常: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
                        traceback.print_exc(file=sys.stderr)
                        error_msg = self._sanitize_error(str(e))
                        self._send_error(500, error_msg)
                        response_sent = True
            
            except Exception as e:
                # 只记录错误
                print(f"[REQUEST-{request_id}] [ERROR] 路由和处理异常: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
                traceback.print_exc(file=sys.stderr)
                if not response_sent:
                    error_msg = self._sanitize_error(str(e))
                    self._send_error(500, f"Internal server error: {error_msg}")
                    response_sent = True
        
        except BaseException as e:
            # 捕获所有 BaseException（包括 SystemExit, GeneratorExit 等）
            # 关键修复：不重新抛出 SystemExit 或 KeyboardInterrupt，而是转换为 HTTP 错误响应
            print(f"[REQUEST-{request_id}] [FATAL] BaseException: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            
            if isinstance(e, (KeyboardInterrupt, SystemExit)):
                # 系统级异常：记录但不退出进程（不重新抛出）
                try:
                    if not response_sent:
                        self._send_error(500, "Internal server error")
                except Exception:
                    pass
                return  # 不重新抛出，不退出进程
            
            # 其他 BaseException（如 GeneratorExit）
            try:
                if not response_sent:
                    self._send_error(500, "Internal server error")
                    print(f"[REQUEST-{request_id}] [FATAL] 错误响应已发送", file=sys.stderr, flush=True)
            except Exception as send_error:
                print(f"[REQUEST-{request_id}] [FATAL] 发送错误响应失败: {send_error}", file=sys.stderr, flush=True)
            return  # 不重新抛出，不退出进程
        
        finally:
            # === 阶段 3：清理事件循环（绝对安全）===
            if loop:
                try:
                    # 检查事件循环是否已关闭
                    if loop.is_closed():
                        return
                    
                    # 只取消临时任务，保留后台任务
                    try:
                        all_tasks = asyncio.all_tasks(loop)
                        
                        # 区分临时任务和后台任务
                        # 临时任务通常是当前请求相关的任务
                        # 后台任务可能是长期运行的服务任务
                        temp_tasks = []
                        for task in all_tasks:
                            task_name = task.get_name() if hasattr(task, 'get_name') else str(task)
                            # 如果任务名称包含 "Task-" 且是数字，可能是后台任务，保留
                            # 否则视为临时任务
                            if "Task-" in task_name:
                                # 检查是否是后台任务（通常有特定的命名模式）
                                # 这里我们假设所有任务都是临时任务，除非明确标记为后台任务
                                # 可以通过任务名称或其他方式识别后台任务
                                if hasattr(task, '_is_background') and task._is_background:
                                    continue
                            temp_tasks.append(task)
                        
                        pending = temp_tasks
                    except RuntimeError as e:
                        if "Event loop is closed" in str(e):
                            try:
                                asyncio.set_event_loop(None)
                            except:
                                pass
                            return
                        raise
                    
                    # 只取消临时任务（静默执行）
                    for task in pending:
                        try:
                            task.cancel()
                        except Exception:
                            pass
                    
                    # 等待临时任务取消完成（最多等待1秒，减少等待时间）
                    if pending:
                        try:
                            loop.run_until_complete(
                                asyncio.wait_for(
                                    asyncio.gather(*pending, return_exceptions=True),
                                    timeout=1.0  # 从 2 秒减少到 1 秒
                                )
                            )
                        except (asyncio.TimeoutError, RuntimeError, Exception):
                            # 静默处理超时和异常
                            pass
                except Exception:
                    # 静默处理清理错误
                    pass
                
                try:
                    # 再次检查事件循环是否已关闭
                    if loop.is_closed():
                        try:
                            asyncio.set_event_loop(None)
                        except:
                            pass
                        return
                    
                    # 先清除线程的默认事件循环设置
                    try:
                        current_loop = asyncio.get_event_loop()
                        if current_loop is loop:
                            asyncio.set_event_loop(None)
                    except:
                        pass
                    
                    # 然后关闭事件循环
                    loop.close()
                except (RuntimeError, Exception):
                    # 静默处理关闭错误
                    try:
                        asyncio.set_event_loop(None)
                    except:
                        pass
    
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
                self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization, X-Health-Check-Id, X-Health-Check-Time')
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
                    if generator and hasattr(generator, 'aclose'):
                        try:
                            # 检查事件循环是否已关闭
                            if loop and not loop.is_closed():
                                loop.run_until_complete(
                                    asyncio.wait_for(generator.aclose(), timeout=1.0)
                                )
                                print(f"[REQUEST-{request_id}] [STREAM] 生成器已关闭", file=sys.stderr, flush=True)
                            else:
                                print(f"[REQUEST-{request_id}] [STREAM] 事件循环已关闭，跳过生成器关闭", file=sys.stderr, flush=True)
                        except asyncio.TimeoutError:
                            print(f"[REQUEST-{request_id}] [STREAM] 关闭生成器超时", file=sys.stderr, flush=True)
                        except RuntimeError as e:
                            # 事件循环已关闭或其他运行时错误
                            if "Event loop is closed" in str(e) or "This event loop is already running" in str(e):
                                print(f"[REQUEST-{request_id}] [STREAM] 事件循环状态异常，跳过生成器关闭: {e}", file=sys.stderr, flush=True)
                            else:
                                print(f"[REQUEST-{request_id}] [STREAM] 关闭生成器失败: {e}", file=sys.stderr, flush=True)
                        except Exception as close_error:
                            print(f"[REQUEST-{request_id}] [STREAM] 关闭生成器失败: {close_error}", file=sys.stderr, flush=True)
                except Exception as e:
                    print(f"[REQUEST-{request_id}] [STREAM] 清理生成器异常: {e}", file=sys.stderr, flush=True)
        
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
            try:
                self.wfile.write(response)
                self.wfile.flush()
            except (BrokenPipeError, ConnectionResetError, OSError, IOError) as io_error:
                print(f"[HANDLER] [IO-ERROR] 写入响应时客户端断开: {type(io_error).__name__}", file=sys.stderr, flush=True)
                # 客户端断开，忽略
        except (BrokenPipeError, ConnectionResetError, OSError, IOError):
            print(f"[HANDLER] [IO-ERROR] 发送 JSON 响应时客户端断开", file=sys.stderr, flush=True)
            pass  # 客户端断开，忽略
        except BaseException as e:
            # 即使 BaseException 也要捕获，不重新抛出
            print(f"[HANDLER] [FATAL] 发送 JSON 响应时发生 BaseException: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
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

