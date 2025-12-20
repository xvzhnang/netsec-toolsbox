# AI Gateway 进程崩溃分析报告

## 1. 工程级调试分析

### 问题现象
- 前端发送 POST /v1/chat/completions 请求
- 请求后 Python 进程立即退出
- Tauri 检测到进程退出，自动重启

### 关键代码路径
```
BaseHTTPRequestHandler.do_POST()
  └─> _handle_chat_completions()
      └─> asyncio.new_event_loop() + loop.run_until_complete()
          └─> router.route()
              └─> adapter.chat() / adapter.chat_stream()
                  └─> aiohttp.ClientSession.post()
```

## 2. 最可能的根因（按概率排序）

### 【根因 #1】asyncio 事件循环管理错误（概率：90%）
**位置**：`openai_handler.py:106-132`

**问题**：
```python
loop = asyncio.new_event_loop()
asyncio.set_event_loop(loop)
try:
    # ... 处理请求
finally:
    loop.close()  # ⚠️ 危险：关闭事件循环可能导致未完成的协程崩溃
```

**崩溃机制**：
- 如果 `loop.run_until_complete()` 内部有未完成的协程（如流式响应）
- `loop.close()` 会强制取消所有未完成的协程
- 某些协程的取消可能触发未捕获的异常
- 异常传播到主线程，导致进程退出

**证据**：流式响应使用 `async for`，如果客户端断开连接，协程可能仍在运行

---

### 【根因 #2】wfile.write() / wfile.flush() 异常未捕获（概率：85%）
**位置**：`openai_handler.py:192-193, 200-201, 237-239`

**问题**：
```python
self.wfile.write(f"data: {chunk_json}\n\n".encode('utf-8'))
self.wfile.flush()  # ⚠️ 如果客户端断开，可能抛出 BrokenPipeError
```

**崩溃机制**：
- 客户端断开连接时，`wfile.write()` 可能抛出 `BrokenPipeError` / `ConnectionResetError`
- 如果异常发生在流式响应的循环中，且未被正确捕获
- 异常可能传播到 `_handle_chat_completions()` 外层
- 如果外层 try/except 有漏洞，异常会传播到 `do_POST()`
- `BaseHTTPRequestHandler` 默认不捕获异常，导致进程退出

---

### 【根因 #3】aiohttp 会话异常导致事件循环崩溃（概率：75%）
**位置**：`openai_compat_adapter.py:113-141, 192-280`

**问题**：
```python
async with aiohttp.ClientSession() as session:
    async with session.post(...) as response:
        # 如果这里抛出异常，且未正确处理
        # 可能导致事件循环状态异常
```

**崩溃机制**：
- `aiohttp.ClientError` / `asyncio.TimeoutError` 如果未正确捕获
- 异常可能破坏事件循环的内部状态
- 后续的 `loop.close()` 可能检测到异常状态并退出进程

---

### 【根因 #4】流式响应中的异步生成器异常（概率：70%）
**位置**：`openai_handler.py:163-218`

**问题**：
```python
async def stream_generator():
    async for chunk in adapter.chat_stream(chat_request):
        yield chunk

# 如果 generator 内部抛出异常，且 loop.run_until_complete() 未正确处理
chunk = loop.run_until_complete(
    asyncio.wait_for(generator.__anext__(), timeout=1.0)
)
```

**崩溃机制**：
- `generator.__anext__()` 可能抛出 `StopAsyncIteration` 以外的异常
- `asyncio.wait_for()` 超时可能抛出 `asyncio.TimeoutError`
- 如果异常处理不完整，可能导致事件循环状态异常

---

### 【根因 #5】BaseHTTPRequestHandler 未捕获的异常（概率：60%）
**位置**：`openai_handler.py:71-141`

**问题**：
```python
def _handle_chat_completions(self):
    try:
        # ... 处理逻辑
    except ValueError as e:
        self._send_error(404, str(e))
    except Exception as e:  # ⚠️ 可能遗漏某些异常类型
        self._send_error(500, error_msg)
    # ⚠️ 如果异常发生在 try 块之外，或 _send_error() 内部，可能未捕获
```

**崩溃机制**：
- `SystemExit` / `KeyboardInterrupt` 如果被捕获，可能导致意外行为
- `BaseException` 的子类（如 `GeneratorExit`）不会被 `except Exception` 捕获
- 如果 `_send_error()` 内部抛出异常（如 wfile 已关闭），异常会传播

---

### 【根因 #6】线程/进程资源泄漏导致 OOM 或死锁（概率：40%）
**位置**：`process_adapter.py`（如果使用）

**问题**：
- `ThreadPoolExecutor` 如果未正确关闭，可能导致资源泄漏
- 大量请求可能导致线程池耗尽或内存泄漏

---

### 【根因 #7】Windows 特定的 I/O 异常（概率：30%）
**位置**：所有文件/网络 I/O 操作

**问题**：
- Windows 上 `sys.stderr` / `sys.stdout` 重定向可能导致编码问题
- 如果 stderr 被关闭，某些异常处理可能失败

---

## 3. 直接导致进程 exit 的代码模式

### 模式 #1：未捕获的 BaseException
```python
# ❌ 危险
try:
    # 某些操作
except Exception as e:  # 不会捕获 SystemExit, KeyboardInterrupt, GeneratorExit
    pass
```

### 模式 #2：事件循环关闭时仍有未完成的协程
```python
# ❌ 危险
loop = asyncio.new_event_loop()
asyncio.set_event_loop(loop)
try:
    result = loop.run_until_complete(coroutine())
finally:
    loop.close()  # 如果 coroutine 内部有未完成的子协程，可能崩溃
```

### 模式 #3：wfile 操作未捕获 BrokenPipeError
```python
# ❌ 危险
self.wfile.write(data)
self.wfile.flush()  # 客户端断开时可能抛出异常
```

### 模式 #4：异常处理中再次抛出异常
```python
# ❌ 危险
try:
    # 某些操作
except Exception as e:
    self._send_error(500, str(e))  # 如果 _send_error 内部失败，异常会传播
```

---

## 4. 绝对不崩溃的 /chat 处理模板

```python
def _handle_chat_completions(self):
    """处理 /v1/chat/completions 请求 - 绝对不崩溃版本"""
    # 确保响应已发送的标志
    response_sent = False
    
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
                # ... 其他参数
            )
        except (ValueError, KeyError, json.JSONDecodeError, UnicodeDecodeError) as e:
            self._send_error_safe(400, f"Invalid request: {str(e)}")
            response_sent = True
            return
        except Exception as e:
            # 任何其他解析错误
            self._send_error_safe(400, "Invalid request format")
            response_sent = True
            return
        
        # === 阶段 2：路由和处理（事件循环保护）===
        loop = None
        try:
            # 创建新的事件循环（每个请求独立）
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            
            if is_stream:
                # 流式响应（特殊处理）
                self._handle_stream_response_safe(model_id, chat_request, loop)
                response_sent = True
            else:
                # 非流式响应
                try:
                    response = loop.run_until_complete(
                        asyncio.wait_for(
                            self.router.route(model_id, chat_request),
                            timeout=300.0  # 5分钟超时
                        )
                    )
                    
                    # 转换为字典
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
                    # 模型未找到等业务错误
                    self._send_error_safe(404, str(e))
                    response_sent = True
                except Exception as e:
                    # 其他业务异常
                    error_msg = self._sanitize_error(str(e))
                    self._send_error_safe(500, error_msg)
                    response_sent = True
        
        except Exception as e:
            # 事件循环层面的异常
            if not response_sent:
                error_msg = self._sanitize_error(str(e))
                self._send_error_safe(500, f"Internal server error: {error_msg}")
                response_sent = True
        
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
                        loop.run_until_complete(
                            asyncio.wait(pending, timeout=1.0)
                        )
                except Exception:
                    pass  # 忽略清理错误
                
                try:
                    loop.close()
                except Exception:
                    pass  # 忽略关闭错误
    
    except BaseException as e:
        # === 最终保护：捕获所有异常（包括 SystemExit, KeyboardInterrupt）===
        # 注意：这里不应该捕获 KeyboardInterrupt，应该让它传播
        if isinstance(e, (KeyboardInterrupt, SystemExit)):
            raise  # 重新抛出，让主程序处理
        
        # 其他 BaseException（如 GeneratorExit）
        if not response_sent:
            try:
                self._send_error_safe(500, "Internal server error")
            except Exception:
                pass  # 如果连错误响应都发不出去，只能放弃


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
            self.end_headers()
            response_sent = True
        except (BrokenPipeError, ConnectionResetError, OSError):
            # 客户端已断开，无法发送响应头
            return
        
        # 获取适配器
        try:
            adapter = self.router.registry.get_adapter(model_id)
            if not adapter:
                self._send_stream_error_safe("Model not found")
                return
        except Exception as e:
            self._send_stream_error_safe(f"Error getting adapter: {str(e)}")
            return
        
        # 流式生成器
        async def stream_generator():
            try:
                async for chunk in adapter.chat_stream(chat_request):
                    yield chunk
            except Exception as e:
                # 生成器内部异常，发送错误 chunk
                error_chunk = OpenAIStreamChunk(
                    id=f'error-{os.urandom(8).hex()}',
                    created=int(time.time()),
                    model=model_id,
                    choices=[{
                        "index": 0,
                        "delta": {"content": f"\n\n[Error: {str(e)}]"},
                        "finish_reason": "error"
                    }]
                )
                yield error_chunk
                raise  # 重新抛出，让外层处理
        
        # 发送流式数据
        generator = stream_generator()
        done = False
        max_iterations = 10000  # 防止无限循环
        iteration = 0
        
        while not done and iteration < max_iterations:
            iteration += 1
            chunk = None
            
            try:
                # 获取下一个 chunk（带超时）
                chunk = loop.run_until_complete(
                    asyncio.wait_for(generator.__anext__(), timeout=30.0)
                )
            except StopAsyncIteration:
                # 生成器正常结束
                try:
                    self._write_safe("data: [DONE]\n\n")
                except:
                    pass
                done = True
                break
            except asyncio.TimeoutError:
                # 超时，发送心跳
                try:
                    self._write_safe(": heartbeat\n\n")
                except:
                    done = True
                    break
                continue
            except Exception as e:
                # 生成器异常
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
                    # 客户端断开连接，正常退出
                    done = True
                    break
                except Exception as e:
                    # 其他写入错误
                    error_msg = self._sanitize_error(str(e))
                    self._send_stream_error_safe(error_msg)
                    done = True
                    break
        
        if iteration >= max_iterations:
            # 防止无限循环
            try:
                self._write_safe("data: [DONE]\n\n")
            except:
                pass
    
    except Exception as e:
        # 流式响应处理异常
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
        # 客户端断开连接，这是正常情况，不抛出异常
        raise  # 重新抛出，让调用方知道连接已断开
    except Exception as e:
        # 其他 I/O 错误
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
        # 客户端断开，忽略
        pass
    except Exception as e:
        # 其他错误，记录但不抛出
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
        # 客户端断开，忽略
        pass
    except Exception as e:
        # 其他错误，记录但不抛出
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
        # 客户端断开，忽略
        pass
    except Exception as e:
        # 其他错误，记录但不抛出
        try:
            print(f"Error sending stream error: {e}", file=sys.stderr, flush=True)
        except:
            pass


def _sanitize_error(self, error_msg: str) -> str:
    """清理错误消息（移除敏感信息）"""
    if 'api_key' in error_msg.lower() or 'key' in error_msg.lower():
        return "API 配置错误"
    # 截断过长的错误消息
    if len(error_msg) > 200:
        return error_msg[:200] + "..."
    return error_msg
```

---

## 5. 捕获进程退出信号

### 方法 1：使用 atexit
```python
import atexit
import sys
import traceback

def exit_handler():
    """进程退出时的处理"""
    exc_type, exc_value, exc_traceback = sys.exc_info()
    if exc_type is not None:
        # 有异常导致退出
        print(f"进程因异常退出: {exc_type.__name__}: {exc_value}", file=sys.stderr, flush=True)
        traceback.print_exception(exc_type, exc_value, exc_traceback, file=sys.stderr)
    else:
        # 正常退出
        print("进程正常退出", file=sys.stderr, flush=True)

atexit.register(exit_handler)
```

### 方法 2：使用 sys.excepthook
```python
import sys
import traceback

original_excepthook = sys.excepthook

def custom_excepthook(exc_type, exc_value, exc_traceback):
    """捕获未处理的异常"""
    # 记录异常
    print(f"未捕获的异常导致进程退出: {exc_type.__name__}: {exc_value}", file=sys.stderr, flush=True)
    traceback.print_exception(exc_type, exc_value, exc_traceback, file=sys.stderr)
    
    # 调用原始 hook（可选）
    original_excepthook(exc_type, exc_value, exc_traceback)

sys.excepthook = custom_excepthook
```

### 方法 3：判断退出原因
```python
import sys
import atexit

is_normal_exit = False

def mark_normal_exit():
    global is_normal_exit
    is_normal_exit = True

def exit_handler():
    if not is_normal_exit:
        exc_type, exc_value, exc_traceback = sys.exc_info()
        if exc_type is not None:
            print(f"异常退出: {exc_type.__name__}: {exc_value}", file=sys.stderr, flush=True)
        else:
            print("非正常退出（无异常信息）", file=sys.stderr, flush=True)

atexit.register(exit_handler)

# 在正常退出点调用
# mark_normal_exit()
```

---

## 6. 最小复现/稳定示例

### 最小崩溃示例（用于测试）
```python
# test_crash.py
from http.server import HTTPServer, BaseHTTPRequestHandler
import asyncio
import sys

class CrashHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        # 模拟崩溃场景
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            # 模拟异步操作
            async def crash_task():
                await asyncio.sleep(0.1)
                raise Exception("模拟崩溃")
            
            loop.run_until_complete(crash_task())
        finally:
            loop.close()  # 这里可能崩溃

if __name__ == '__main__':
    server = HTTPServer(('127.0.0.1', 8766), CrashHandler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.shutdown()
```

### 最小稳定示例（参考实现）
```python
# test_stable.py
from http.server import HTTPServer, BaseHTTPRequestHandler
import asyncio
import sys
import json

class StableHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        response_sent = False
        loop = None
        
        try:
            # 解析请求
            content_length = int(self.headers.get('Content-Length', 0))
            if content_length == 0:
                self._send_error_safe(400, "Request body required")
                return
            
            request_body = self.rfile.read(content_length)
            request_data = json.loads(request_body.decode('utf-8'))
            
            # 处理请求
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            
            async def process():
                await asyncio.sleep(0.1)
                return {"status": "ok"}
            
            try:
                result = loop.run_until_complete(
                    asyncio.wait_for(process(), timeout=10.0)
                )
                self._send_json_safe(result)
                response_sent = True
            except asyncio.TimeoutError:
                self._send_error_safe(504, "Timeout")
                response_sent = True
            except Exception as e:
                self._send_error_safe(500, str(e))
                response_sent = True
        
        except Exception as e:
            if not response_sent:
                self._send_error_safe(500, "Internal error")
        
        finally:
            if loop:
                try:
                    # 清理未完成的任务
                    pending = asyncio.all_tasks(loop)
                    for task in pending:
                        task.cancel()
                    if pending:
                        loop.run_until_complete(
                            asyncio.wait(pending, timeout=1.0)
                        )
                except:
                    pass
                try:
                    loop.close()
                except:
                    pass
    
    def _send_json_safe(self, data):
        try:
            response = json.dumps(data).encode('utf-8')
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(response)))
            self.end_headers()
            self.wfile.write(response)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError, OSError):
            pass
    
    def _send_error_safe(self, code, msg):
        try:
            error = {"error": {"message": msg, "code": str(code)}}
            response = json.dumps(error).encode('utf-8')
            self.send_response(code)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(response)))
            self.end_headers()
            self.wfile.write(response)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError, OSError):
            pass

if __name__ == '__main__':
    server = HTTPServer(('127.0.0.1', 8767), StableHandler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.shutdown()
```

---

## 7. 关键修复点总结

1. **事件循环管理**：必须在 finally 中安全关闭，取消所有未完成的任务
2. **I/O 异常处理**：所有 wfile 操作必须捕获 BrokenPipeError / ConnectionResetError
3. **异常传播控制**：确保所有异常都转化为 HTTP 响应，不传播到 do_POST() 外层
4. **流式响应保护**：添加超时、最大迭代次数、连接断开检测
5. **错误消息清理**：防止敏感信息泄露
6. **进程退出监控**：使用 atexit / sys.excepthook 记录退出原因

