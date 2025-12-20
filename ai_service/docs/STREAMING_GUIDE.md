# 流式响应（SSE）使用指南

## 概述

AI Gateway 支持 Server-Sent Events (SSE) 流式响应，允许实时接收 AI 模型的回复，提供更好的用户体验。

## 请求格式

在 `/v1/chat/completions` 请求中，设置 `stream: true`：

```json
{
  "model": "gpt-3.5-turbo",
  "messages": [
    {"role": "user", "content": "Hello"}
  ],
  "stream": true
}
```

## 响应格式

响应使用 SSE (Server-Sent Events) 格式，每个数据块格式：

```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-3.5-turbo","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-3.5-turbo","choices":[{"index":0,"delta":{"content":" there"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-3.5-turbo","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":8,"total_tokens":18}}

data: [DONE]
```

## 响应头

流式响应会设置以下响应头：

```
Content-Type: text/event-stream; charset=utf-8
Cache-Control: no-cache
Connection: keep-alive
Access-Control-Allow-Origin: *
```

## 数据块结构

每个数据块（除了 `[DONE]`）是 JSON 格式，包含：

- `id`: 响应 ID
- `object`: 固定为 `"chat.completion.chunk"`
- `created`: 时间戳
- `model`: 模型名称
- `choices`: 选择数组
  - `index`: 选择索引
  - `delta`: 增量内容
    - `role`: 角色（通常只在第一个 chunk 中出现）
    - `content`: 文本内容增量
  - `finish_reason`: 完成原因（`null` 表示未完成，`"stop"` 表示正常结束）
- `usage`: Token 使用量（通常在最后一个 chunk 中）

## 前端示例

### JavaScript/TypeScript

```typescript
async function streamChat(model: string, messages: any[]) {
  const response = await fetch('http://localhost:8765/v1/chat/completions', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      model: model,
      messages: messages,
      stream: true
    })
  });

  const reader = response.body?.getReader();
  const decoder = new TextDecoder();

  if (!reader) {
    throw new Error('No response body');
  }

  let buffer = '';
  
  while (true) {
    const { done, value } = await reader.read();
    
    if (done) {
      break;
    }
    
    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop() || '';  // 保留最后一个不完整的行
    
    for (const line of lines) {
      if (line.startsWith('data: ')) {
        const data = line.slice(6);
        
        if (data === '[DONE]') {
          console.log('Stream completed');
          return;
        }
        
        try {
          const chunk = JSON.parse(data);
          
          // 提取内容增量
          if (chunk.choices && chunk.choices[0]?.delta?.content) {
            const content = chunk.choices[0].delta.content;
            console.log('Received:', content);
            // 更新 UI
          }
          
          // 检查是否完成
          if (chunk.choices && chunk.choices[0]?.finish_reason) {
            console.log('Finished:', chunk.choices[0].finish_reason);
            
            // 提取 usage
            if (chunk.usage) {
              console.log('Usage:', chunk.usage);
            }
          }
        } catch (e) {
          console.error('Parse error:', e);
        }
      }
    }
  }
}
```

### 使用 EventSource（注意：POST 请求需要使用 fetch）

EventSource 只支持 GET 请求，所以需要使用 `fetch` API。

## 适配器支持

### OpenAI Compat Adapter ✅

完全支持流式响应，直接从后端 API 流式转发。

### Custom HTTP Adapter ⚠️

当前实现：**降级到非流式请求**

- 如果后端支持流式，可以实现 `chat_stream` 方法
- 如果不支持，使用默认的分块返回（完整响应后按字符分块）

### Process Adapter ⚠️

当前实现：**降级到非流式请求**

- CLI 工具通常不支持流式输出
- 如果工具支持流式输出，可以实现 `chat_stream` 方法

## 实现自定义流式支持

如果适配器需要支持流式响应，需要实现 `chat_stream` 方法：

```python
async def chat_stream(
    self,
    request: OpenAIChatRequest,
    timeout: Optional[int] = None
) -> AsyncIterator[OpenAIStreamChunk]:
    """
    发送流式聊天请求
    
    Yields:
        OpenAIStreamChunk: 流式响应数据块
    """
    # 实现流式逻辑
    async for chunk in your_stream_source:
        yield OpenAIStreamChunk(
            id=chunk.id,
            created=chunk.created,
            model=chunk.model,
            choices=chunk.choices,
            usage=chunk.usage if is_last else None
        )
```

## 错误处理

流式响应中的错误会以错误数据块的形式发送：

```
data: {"error":{"message":"Model not found","type":"server_error","code":"404"}}

data: [DONE]
```

前端应该检查每个数据块是否包含 `error` 字段。

## 最佳实践

1. **超时处理**：设置合理的超时时间，避免长时间等待
2. **错误处理**：始终检查错误数据块
3. **内容累积**：在客户端累积增量内容以显示完整响应
4. **取消请求**：提供取消机制（AbortController）
5. **重连机制**：网络中断时考虑重连

## 限制

1. **兼容性**：某些适配器可能不支持流式（会降级到非流式）
2. **性能**：流式响应会增加服务器负载
3. **网络**：需要保持长连接，可能受到代理/防火墙限制

