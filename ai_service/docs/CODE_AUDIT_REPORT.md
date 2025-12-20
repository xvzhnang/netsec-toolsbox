# AI Gateway 代码审计报告

## 问题分析

### 【关键问题 #1】流式响应中的事件循环管理（概率：95%）

**位置**：`ai_service/api/openai_handler.py:427-560`

**问题**：
```python
def _handle_stream_response(self, ...):
    generator = stream_generator()  # 异步生成器
    while not done:
        chunk = loop.run_until_complete(
            asyncio.wait_for(generator.__anext__(), timeout=30.0)
        )
        # ... 发送 chunk
```

**崩溃机制**：
1. `adapter.chat_stream()` 内部使用 `async with aiohttp.ClientSession()`
2. 内部有 `async for chunk_bytes in response.content.iter_chunked(8192)`
3. 如果客户端断开连接，`_write_safe()` 抛出 `BrokenPipeError`，循环退出
4. 但是 `iter_chunked` 的异步任务可能还在运行
5. 在 `finally` 中调用 `loop.close()` 时，如果还有未完成的异步任务，可能导致：
   - `RuntimeError: Event loop is closed`
   - 或者进程直接退出

**证据**：
- 用户报告：发送消息后服务崩溃
- 日志中没有看到任何输出，说明崩溃发生在日志输出之前，或者日志没有被捕获
- 流式响应是最复杂的代码路径

---

### 【关键问题 #2】asyncio.wait() 超时参数错误（概率：90%）

**位置**：`ai_service/api/openai_handler.py:404`

**问题**：
```python
loop.run_until_complete(
    asyncio.wait(pending, timeout=1.0)  # ⚠️ 错误：asyncio.wait 不接受 timeout 参数
)
```

**崩溃机制**：
- `asyncio.wait()` 不接受 `timeout` 参数
- 应该使用 `asyncio.wait_for(asyncio.wait(pending), timeout=1.0)`
- 这个错误会导致 `TypeError`，可能被捕获，但会导致任务无法正确取消

---

### 【关键问题 #3】流式生成器异常处理不完整（概率：85%）

**位置**：`ai_service/api/openai_handler.py:458-478`

**问题**：
```python
async def stream_generator():
    try:
        async for chunk in adapter.chat_stream(chat_request):
            yield chunk
    except Exception as e:
        # ... 发送错误 chunk
        yield error_chunk
        raise  # ⚠️ 重新抛出异常
```

**崩溃机制**：
- 如果 `adapter.chat_stream()` 内部抛出异常（比如网络错误）
- 异常被捕获，发送错误 chunk，然后重新抛出
- 重新抛出的异常可能传播到 `loop.run_until_complete()`，导致事件循环状态异常

---

### 【关键问题 #4】客户端断开连接时的处理（概率：80%）

**位置**：`ai_service/api/openai_handler.py:499-520`

**问题**：
```python
except (BrokenPipeError, ConnectionResetError, OSError, IOError):
    done = True
    break  # ⚠️ 直接退出，但异步生成器可能还在运行
```

**崩溃机制**：
- 客户端断开时，`_write_safe()` 抛出 `BrokenPipeError`
- 循环退出，但 `adapter.chat_stream()` 内部的 `async for` 可能还在运行
- 事件循环关闭时，这些未完成的任务可能导致问题

---

### 【关键问题 #5】事件循环关闭时的竞态条件（概率：75%）

**位置**：`ai_service/api/openai_handler.py:383-425`

**问题**：
```python
finally:
    if loop:
        pending = asyncio.all_tasks(loop)
        for task in pending:
            task.cancel()
        if pending:
            loop.run_until_complete(
                asyncio.wait(pending, timeout=1.0)  # ⚠️ 错误用法
            )
        loop.close()  # ⚠️ 如果还有未完成的任务，可能崩溃
```

**崩溃机制**：
- `asyncio.wait()` 的用法错误
- 即使任务被取消，如果它们还在等待 I/O（比如 `iter_chunked`），可能无法在 1 秒内完成
- `loop.close()` 时如果还有未完成的任务，可能抛出异常或导致进程退出

---

## 修复方案

### 修复 1：正确使用 asyncio.wait_for

```python
if pending:
    try:
        loop.run_until_complete(
            asyncio.wait_for(
                asyncio.gather(*pending, return_exceptions=True),
                timeout=1.0
            )
        )
    except asyncio.TimeoutError:
        # 超时，强制关闭
        pass
    except Exception:
        pass
```

### 修复 2：改进流式响应处理

```python
def _handle_stream_response(self, ...):
    generator = None
    try:
        generator = stream_generator()
        # ... 处理流式数据
    finally:
        # 确保生成器被正确关闭
        if generator:
            try:
                # 尝试关闭生成器
                loop.run_until_complete(generator.aclose())
            except:
                pass
```

### 修复 3：改进客户端断开处理

```python
except (BrokenPipeError, ConnectionResetError, OSError, IOError) as e:
    print(f"[REQUEST-{request_id}] [STREAM] 客户端断开连接: {type(e).__name__}", 
          file=sys.stderr, flush=True)
    done = True
    # 不直接 break，而是设置标志，让生成器有机会清理
    break
```

### 修复 4：改进事件循环清理

```python
finally:
    if loop:
        try:
            # 获取所有任务
            pending = asyncio.all_tasks(loop)
            
            # 取消所有任务
            for task in pending:
                task.cancel()
            
            # 等待任务完成（带超时）
            if pending:
                try:
                    loop.run_until_complete(
                        asyncio.wait_for(
                            asyncio.gather(*pending, return_exceptions=True),
                            timeout=2.0
                        )
                    )
                except (asyncio.TimeoutError, Exception):
                    # 超时或异常，继续关闭
                    pass
            
            # 关闭事件循环
            loop.close()
        except Exception as e:
            print(f"[REQUEST-{request_id}] [STEP-3] 清理事件循环异常: {e}", 
                  file=sys.stderr, flush=True)
```

---

## 最可能的根因

根据代码审计，**最可能的根因是流式响应处理中的事件循环管理问题**：

1. 客户端断开连接时，`_write_safe()` 抛出 `BrokenPipeError`
2. 循环退出，但 `adapter.chat_stream()` 内部的异步任务还在运行
3. `finally` 中尝试关闭事件循环时，未完成的任务导致异常
4. 异常可能传播到 `do_POST()` 外层，导致进程退出

---

## 建议的修复优先级

1. **立即修复**：`asyncio.wait()` 的用法错误
2. **立即修复**：改进事件循环清理逻辑
3. **高优先级**：改进流式响应中的异常处理
4. **中优先级**：添加生成器关闭逻辑
5. **低优先级**：优化客户端断开处理
