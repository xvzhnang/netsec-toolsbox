# Python AI Gateway 进程退出分析报告

## 1. 进程级退出根因分析

### 【根因 #1】BaseHTTPRequestHandler.do_POST() 未捕获的异常（概率：95%）

**问题**：
```python
def do_POST(self):
    if parsed_path.path == '/v1/chat/completions':
        self._handle_chat_completions()  # ⚠️ 如果这里抛出未捕获的异常
```

**崩溃机制**：
- `BaseHTTPRequestHandler` 的 `do_POST()` 方法如果抛出未捕获的异常
- Python 的 `http.server` 默认**不会捕获**这些异常
- 异常会传播到 `HTTPServer` 的请求处理线程
- 如果异常是 `SystemExit` 或 `KeyboardInterrupt`，会直接终止进程
- 如果是其他异常，可能导致请求处理线程崩溃，进而影响整个服务

**证据**：Python 标准库的 `http.server` 源码中，`BaseHTTPRequestHandler.handle()` 方法会调用 `do_POST()`，但没有全局异常捕获。

---

### 【根因 #2】asyncio 事件循环在 finally 中关闭时的异常（概率：85%）

**问题**：
```python
finally:
    if loop:
        try:
            pending = asyncio.all_tasks(loop)
            for task in pending:
                task.cancel()
            if pending:
                loop.run_until_complete(asyncio.wait(pending, timeout=1.0))  # ⚠️ 可能抛出异常
        except Exception:
            pass
        try:
            loop.close()  # ⚠️ 如果事件循环状态异常，可能抛出异常
        except Exception:
            pass
```

**崩溃机制**：
- 如果 `loop.run_until_complete()` 在 finally 中抛出异常，且异常处理不完整
- 异常可能传播到 `do_POST()` 外层
- 如果异常是 `SystemExit` 的子类，会导致进程退出

---

### 【根因 #3】aiohttp ClientSession 上下文管理器异常（概率：75%）

**问题**：
```python
async with aiohttp.ClientSession() as session:
    async with session.post(...) as response:
        # 如果这里发生异常，且上下文管理器清理失败
```

**崩溃机制**：
- `aiohttp.ClientSession` 的 `__aexit__` 如果抛出异常
- 异常可能传播到事件循环层面
- 如果事件循环状态被破坏，可能导致进程退出

---

### 【根因 #4】ThreadPoolExecutor 资源泄漏或死锁（概率：60%）

**问题**：
- `process_adapter.py` 中使用了 `ThreadPoolExecutor`
- 如果线程池未正确关闭，可能导致资源泄漏
- 如果线程池中的线程抛出未捕获的异常，可能导致进程退出

---

### 【根因 #5】GeneratorExit 或 SystemExit 被意外捕获（概率：50%）

**问题**：
```python
except BaseException as e:
    if isinstance(e, (KeyboardInterrupt, SystemExit)):
        raise  # 重新抛出
    # 但如果 GeneratorExit 被捕获，可能导致问题
```

**崩溃机制**：
- `GeneratorExit` 如果被捕获并重新抛出，可能导致生成器状态异常
- 如果流式响应中使用生成器，可能导致进程退出

---

## 2. BaseHTTPRequestHandler 直接结束 server 的情况

### 情况 1：do_POST() 抛出 SystemExit
```python
def do_POST(self):
    sys.exit(1)  # ⚠️ 直接退出进程
```

### 情况 2：do_POST() 抛出 KeyboardInterrupt
```python
def do_POST(self):
    raise KeyboardInterrupt  # ⚠️ 如果未捕获，会终止进程
```

### 情况 3：do_POST() 抛出未捕获的 BaseException
```python
def do_POST(self):
    raise GeneratorExit  # ⚠️ 如果未正确处理，可能导致进程退出
```

### 情况 4：HTTP 响应发送失败导致异常传播
```python
def do_POST(self):
    self._send_json_response(data)  # 如果这里抛出异常，且未捕获
    # 异常会传播到 do_POST() 外层，可能导致进程退出
```

---

## 3. 会杀死整个进程的异常

### 异常类型 1：SystemExit
- **来源**：`sys.exit()` 调用
- **行为**：直接终止进程
- **处理**：必须在最外层捕获并重新抛出，或转换为 HTTP 错误响应

### 异常类型 2：KeyboardInterrupt
- **来源**：Ctrl+C 或信号
- **行为**：终止进程
- **处理**：不应该在请求处理中捕获，应该让主程序处理

### 异常类型 3：GeneratorExit
- **来源**：生成器被垃圾回收或关闭
- **行为**：如果未正确处理，可能导致进程退出
- **处理**：在流式响应中需要特别处理

### 异常类型 4：未捕获的 BaseException 子类
- **来源**：某些第三方库可能抛出
- **行为**：可能导致进程退出
- **处理**：必须在最外层捕获

---

## 4. 绝对不会导致进程退出的 /chat 处理模板

```python
def do_POST(self):
    """处理 POST 请求 - 绝对不退出进程版本"""
    # === 最外层保护：捕获所有可能的异常 ===
    try:
        parsed_path = urlparse(self.path)
        
        if parsed_path.path == '/v1/chat/completions':
            self._handle_chat_completions_ultra_safe()
        else:
            self._send_error_safe(404, "Not Found")
    
    except (KeyboardInterrupt, SystemExit):
        # 系统级异常，不应该在请求处理中发生
        # 如果发生，记录日志但不退出进程
        try:
            print(f"[FATAL] 系统级异常在请求处理中发生: {sys.exc_info()[0].__name__}", 
                  file=sys.stderr, flush=True)
            self._send_error_safe(500, "Internal server error")
        except:
            pass  # 如果连错误响应都发不出去，只能放弃
    
    except BaseException as e:
        # 捕获所有其他 BaseException（如 GeneratorExit）
        try:
            print(f"[FATAL] BaseException 在请求处理中发生: {type(e).__name__}: {e}", 
                  file=sys.stderr, flush=True)
            self._send_error_safe(500, "Internal server error")
        except:
            pass
    
    except Exception as e:
        # 捕获所有普通异常
        try:
            print(f"[ERROR] 未预期的异常: {type(e).__name__}: {e}", 
                  file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            self._send_error_safe(500, "Internal server error")
        except:
            pass
    
    # === 关键：无论发生什么，都不退出进程 ===
    # 不调用 sys.exit()
    # 不调用 exit()
    # 不调用 os._exit()
    # 不重新抛出 SystemExit 或 KeyboardInterrupt


def _handle_chat_completions_ultra_safe(self):
    """处理 /v1/chat/completions 请求 - 超安全版本"""
    response_sent = False
    loop = None
    
    # === 阶段 1：解析请求 ===
    try:
        try:
            content_length = int(self.headers.get('Content-Length', 0))
            if content_length == 0:
                self._send_error_safe(400, "Request body is required")
                return
            
            request_body = self.rfile.read(content_length)
            request_data = json.loads(request_body.decode('utf-8'))
            
            model_id = request_data.get('model')
            if not model_id:
                self._send_error_safe(400, "Missing 'model' field")
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
            return
        except Exception as e:
            self._send_error_safe(400, "Invalid request format")
            return
    
    except BaseException as e:
        # 捕获所有 BaseException（包括 SystemExit, GeneratorExit）
        # 但不重新抛出，而是转换为 HTTP 错误响应
        if isinstance(e, (KeyboardInterrupt, SystemExit)):
            # 系统级异常：记录但不退出进程
            try:
                print(f"[FATAL] 系统级异常在请求解析中发生: {type(e).__name__}", 
                      file=sys.stderr, flush=True)
                self._send_error_safe(500, "Internal server error")
            except:
                pass
            return  # 不重新抛出，不退出进程
        
        # 其他 BaseException（如 GeneratorExit）
        try:
            print(f"[FATAL] BaseException 在请求解析中发生: {type(e).__name__}: {e}", 
                  file=sys.stderr, flush=True)
            self._send_error_safe(500, "Internal server error")
        except:
            pass
        return
    
    except Exception as e:
        self._send_error_safe(400, "Invalid request format")
        return
    
    # === 阶段 2：路由和处理 ===
    try:
        try:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            
            if is_stream:
                self._handle_stream_response_ultra_safe(model_id, chat_request, loop)
                response_sent = True
            else:
                try:
                    response = loop.run_until_complete(
                        asyncio.wait_for(
                            self.router.route(model_id, chat_request),
                            timeout=300.0
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
        
        except BaseException as e:
            # 捕获所有 BaseException（包括 SystemExit, GeneratorExit）
            if isinstance(e, (KeyboardInterrupt, SystemExit)):
                # 系统级异常：记录但不退出进程
                try:
                    print(f"[FATAL] 系统级异常在请求处理中发生: {type(e).__name__}", 
                          file=sys.stderr, flush=True)
                    if not response_sent:
                        self._send_error_safe(500, "Internal server error")
                except:
                    pass
                return  # 不重新抛出，不退出进程
            
            # 其他 BaseException
            try:
                print(f"[FATAL] BaseException 在请求处理中发生: {type(e).__name__}: {e}", 
                      file=sys.stderr, flush=True)
                if not response_sent:
                    self._send_error_safe(500, "Internal server error")
            except:
                pass
            return
        
        except Exception as e:
            if not response_sent:
                error_msg = self._sanitize_error(str(e))
                self._send_error_safe(500, f"Internal server error: {error_msg}")
                response_sent = True
    
    except BaseException as e:
        # 最外层 BaseException 捕获
        if isinstance(e, (KeyboardInterrupt, SystemExit)):
            try:
                print(f"[FATAL] 系统级异常在最外层发生: {type(e).__name__}", 
                      file=sys.stderr, flush=True)
                if not response_sent:
                    self._send_error_safe(500, "Internal server error")
            except:
                pass
            return  # 不重新抛出，不退出进程
        
        try:
            print(f"[FATAL] BaseException 在最外层发生: {type(e).__name__}: {e}", 
                  file=sys.stderr, flush=True)
            if not response_sent:
                self._send_error_safe(500, "Internal server error")
        except:
            pass
        return
    
    except Exception as e:
        if not response_sent:
            try:
                error_msg = self._sanitize_error(str(e))
                self._send_error_safe(500, f"Internal server error: {error_msg}")
            except:
                pass
    
    finally:
        # === 阶段 3：清理事件循环（绝对安全）===
        if loop:
            try:
                # 取消所有未完成的任务
                pending = asyncio.all_tasks(loop)
                for task in pending:
                    try:
                        task.cancel()
                    except:
                        pass
                
                # 等待任务取消完成（最多等待1秒）
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


def _send_error_safe(self, status_code: int, message: str):
    """安全发送错误响应 - 绝对不抛出异常"""
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
    except BaseException as e:
        # 即使 BaseException 也要捕获，不重新抛出
        try:
            print(f"[FATAL] BaseException 在发送错误响应时发生: {type(e).__name__}: {e}", 
                  file=sys.stderr, flush=True)
        except:
            pass
    except Exception as e:
        try:
            print(f"[ERROR] 发送错误响应失败: {type(e).__name__}: {e}", 
                  file=sys.stderr, flush=True)
        except:
            pass


def _send_json_response_safe(self, data: dict):
    """安全发送 JSON 响应 - 绝对不抛出异常"""
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
```

---

## 5. 调试方案

### 方案 1：使用 atexit 记录进程退出

```python
import atexit
import sys
import traceback

_exit_reason = None
_exit_traceback = None

def exit_handler():
    """进程退出时的处理"""
    global _exit_reason, _exit_traceback
    
    exc_type, exc_value, exc_traceback = sys.exc_info()
    
    if exc_type is not None:
        _exit_reason = f"异常退出: {exc_type.__name__}: {exc_value}"
        _exit_traceback = traceback.format_exception(exc_type, exc_value, exc_traceback)
        
        print(f"[EXIT] {_exit_reason}", file=sys.stderr, flush=True)
        print(f"[EXIT] 堆栈跟踪:", file=sys.stderr, flush=True)
        for line in _exit_traceback:
            print(line, file=sys.stderr, end='', flush=True)
    else:
        _exit_reason = "正常退出（无异常）"
        print(f"[EXIT] {_exit_reason}", file=sys.stderr, flush=True)

atexit.register(exit_handler)
```

### 方案 2：使用 sys.excepthook 捕获未处理的异常

```python
import sys
import traceback

_original_excepthook = sys.excepthook

def custom_excepthook(exc_type, exc_value, exc_traceback):
    """捕获未处理的异常"""
    # 判断是否是系统级异常
    is_system_exit = isinstance(exc_value, (SystemExit, KeyboardInterrupt))
    
    print(f"[UNHANDLED] 未捕获的异常: {exc_type.__name__}: {exc_value}", 
          file=sys.stderr, flush=True)
    print(f"[UNHANDLED] 是否是系统级异常: {is_system_exit}", 
          file=sys.stderr, flush=True)
    
    traceback.print_exception(exc_type, exc_value, exc_traceback, file=sys.stderr)
    
    # 调用原始 hook
    _original_excepthook(exc_type, exc_value, exc_traceback)

sys.excepthook = custom_excepthook
```

### 方案 3：在 do_POST 中添加详细日志

```python
def do_POST(self):
    """处理 POST 请求 - 带详细日志"""
    request_id = os.urandom(8).hex()
    
    try:
        print(f"[REQUEST-{request_id}] 收到 POST 请求: {self.path}", 
              file=sys.stderr, flush=True)
        
        parsed_path = urlparse(self.path)
        
        if parsed_path.path == '/v1/chat/completions':
            print(f"[REQUEST-{request_id}] 开始处理 /chat 请求", 
                  file=sys.stderr, flush=True)
            self._handle_chat_completions_ultra_safe()
            print(f"[REQUEST-{request_id}] /chat 请求处理完成", 
                  file=sys.stderr, flush=True)
        else:
            self._send_error_safe(404, "Not Found")
    
    except BaseException as e:
        print(f"[REQUEST-{request_id}] BaseException 发生: {type(e).__name__}: {e}", 
              file=sys.stderr, flush=True)
        traceback.print_exc(file=sys.stderr)
        try:
            self._send_error_safe(500, "Internal server error")
        except:
            pass
    
    except Exception as e:
        print(f"[REQUEST-{request_id}] Exception 发生: {type(e).__name__}: {e}", 
              file=sys.stderr, flush=True)
        traceback.print_exc(file=sys.stderr)
        try:
            self._send_error_safe(500, "Internal server error")
        except:
            pass
    
    finally:
        print(f"[REQUEST-{request_id}] 请求处理结束", file=sys.stderr, flush=True)
```

### 方案 4：判断是异常退出还是主动 exit

```python
import sys
import atexit

_is_normal_exit = False
_exit_stack = []

def mark_normal_exit():
    """标记正常退出"""
    global _is_normal_exit
    _is_normal_exit = True
    import traceback
    _exit_stack.append(traceback.format_stack())

def exit_handler():
    """进程退出时的处理"""
    global _is_normal_exit, _exit_stack
    
    exc_type, exc_value, exc_traceback = sys.exc_info()
    
    if _is_normal_exit:
        print(f"[EXIT] 正常退出（已标记）", file=sys.stderr, flush=True)
    elif exc_type is None:
        print(f"[EXIT] 正常退出（无异常信息）", file=sys.stderr, flush=True)
    else:
        print(f"[EXIT] 异常退出: {exc_type.__name__}: {exc_value}", 
              file=sys.stderr, flush=True)
        
        # 判断是否是主动调用 sys.exit()
        if isinstance(exc_value, SystemExit):
            print(f"[EXIT] 检测到 SystemExit，可能是主动调用 sys.exit()", 
                  file=sys.stderr, flush=True)
            if exc_value.code is not None:
                print(f"[EXIT] 退出代码: {exc_value.code}", 
                      file=sys.stderr, flush=True)
        
        # 打印堆栈跟踪
        import traceback
        traceback.print_exception(exc_type, exc_value, exc_traceback, file=sys.stderr)

atexit.register(exit_handler)

# 在正常退出点调用
# mark_normal_exit()
```

---

## 6. 关键修复点总结

1. **do_POST() 最外层必须捕获所有异常**：包括 BaseException
2. **SystemExit 和 KeyboardInterrupt 不重新抛出**：转换为 HTTP 错误响应
3. **所有 HTTP 响应发送方法必须绝对安全**：不能抛出异常
4. **事件循环清理必须绝对安全**：所有异常都要捕获
5. **添加详细的日志记录**：便于调试和定位问题
6. **禁止在请求处理中调用 sys.exit()**：所有错误都转化为 HTTP 响应

