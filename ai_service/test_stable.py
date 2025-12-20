# -*- coding: utf-8 -*-
"""
最小稳定示例 - 参考实现
演示绝对不崩溃的代码模式
"""
from http.server import HTTPServer, BaseHTTPRequestHandler
import asyncio
import sys
import json
import traceback
import atexit

# 进程退出监控
_is_normal_exit = False

def _exit_handler():
    global _is_normal_exit
    if not _is_normal_exit:
        exc_type, exc_value, exc_traceback = sys.exc_info()
        if exc_type is not None:
            print(f"[EXIT] 进程因异常退出: {exc_type.__name__}: {exc_value}", file=sys.stderr, flush=True)
            traceback.print_exception(exc_type, exc_value, exc_traceback, file=sys.stderr)

atexit.register(_exit_handler)

_original_excepthook = sys.excepthook

def _custom_excepthook(exc_type, exc_value, exc_traceback):
    print(f"[UNHANDLED] 未捕获的异常: {exc_type.__name__}: {exc_value}", file=sys.stderr, flush=True)
    traceback.print_exception(exc_type, exc_value, exc_traceback, file=sys.stderr)
    _original_excepthook(exc_type, exc_value, exc_traceback)

sys.excepthook = _custom_excepthook


class StableHandler(BaseHTTPRequestHandler):
    """稳定的 Handler - 参考实现"""
    
    def do_POST(self):
        """处理 POST 请求 - 绝对不崩溃"""
        response_sent = False
        loop = None
        
        try:
            # 解析请求
            try:
                content_length = int(self.headers.get('Content-Length', 0))
                if content_length == 0:
                    self._send_error_safe(400, "Request body required")
                    return
                
                request_body = self.rfile.read(content_length)
                request_data = json.loads(request_body.decode('utf-8'))
            except Exception as e:
                self._send_error_safe(400, f"Invalid request: {str(e)}")
                return
            
            # 处理请求
            try:
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                
                async def process():
                    await asyncio.sleep(0.1)
                    return {"status": "ok", "message": "Request processed"}
                
                try:
                    result = loop.run_until_complete(
                        asyncio.wait_for(process(), timeout=10.0)
                    )
                    self._send_json_safe(result)
                    response_sent = True
                except asyncio.TimeoutError:
                    self._send_error_safe(504, "Request timeout")
                    response_sent = True
                except Exception as e:
                    self._send_error_safe(500, str(e))
                    response_sent = True
            
            except Exception as e:
                if not response_sent:
                    self._send_error_safe(500, "Internal error")
        
        except BaseException as e:
            if isinstance(e, (KeyboardInterrupt, SystemExit)):
                raise
            
            if not response_sent:
                try:
                    self._send_error_safe(500, "Internal error")
                except:
                    pass
        
        finally:
            # 安全清理事件循环
            if loop:
                try:
                    pending = asyncio.all_tasks(loop)
                    for task in pending:
                        task.cancel()
                    if pending:
                        try:
                            loop.run_until_complete(
                                asyncio.wait(pending, timeout=1.0)
                            )
                        except:
                            pass
                except:
                    pass
                try:
                    loop.close()
                except:
                    pass
    
    def _send_json_safe(self, data):
        """安全发送 JSON 响应"""
        try:
            response = json.dumps(data).encode('utf-8')
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(response)))
            self.end_headers()
            self.wfile.write(response)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError, OSError, IOError):
            pass  # 客户端断开，忽略
    
    def _send_error_safe(self, code, msg):
        """安全发送错误响应"""
        try:
            error = {"error": {"message": msg, "code": str(code)}}
            response = json.dumps(error).encode('utf-8')
            self.send_response(code)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(response)))
            self.end_headers()
            self.wfile.write(response)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError, OSError, IOError):
            pass  # 客户端断开，忽略
    
    def log_message(self, format, *args):
        pass

if __name__ == '__main__':
    server = HTTPServer(('127.0.0.1', 8767), StableHandler)
    try:
        print("稳定服务器启动在 http://127.0.0.1:8767", file=sys.stderr, flush=True)
        server.serve_forever()
    except KeyboardInterrupt:
        _is_normal_exit = True
        server.shutdown()

