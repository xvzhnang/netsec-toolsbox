# Process 适配器使用指南

## 概述

Process 适配器用于运行本地命令行工具（CLI），通过 stdin/stdout 与进程通信。适用于：
- llama.cpp
- Ollama CLI
- 自定义 Python/Node.js 脚本
- 其他支持 stdin/stdout 的本地工具

## 配置格式

```json
{
  "id": "local-llama-cpp",
  "adapter": "process",
  "command": "llama-cli",
  "args": [
    "-m",
    "/path/to/model.gguf",
    "--ctx-size",
    "4096"
  ],
  "enabled": true,
  "input_format": "prompt",
  "output_format": "text",
  "timeout": 120,
  "working_dir": null,
  "env": null
}
```

## 配置字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | ✅ | 模型唯一标识 |
| `adapter` | string | ✅ | 必须为 `"process"` |
| `command` | string | ✅ | 可执行文件路径或命令名（支持 `ENV:VAR_NAME`） |
| `args` | array | ✅ | 命令行参数列表（支持环境变量） |
| `enabled` | boolean | ✅ | 是否启用 |
| `input_format` | string | ❌ | 输入格式：`json`（默认）、`prompt`、`openai`、`text` |
| `output_format` | string | ❌ | 输出格式：`json`（默认）、`text` |
| `timeout` | number | ❌ | 超时时间（秒，默认 120） |
| `working_dir` | string | ❌ | 工作目录（null 表示当前目录） |
| `env` | object | ❌ | 环境变量（null 表示继承当前环境） |

## 输入格式

### 1. `json`（默认）

发送完整的 OpenAI 请求 JSON：

```json
{
  "model": "local-llama-cpp",
  "messages": [
    {"role": "user", "content": "Hello"}
  ],
  "temperature": 0.7,
  "max_tokens": 2000
}
```

### 2. `prompt`

提取所有消息并拼接成 prompt：

```
System: You are a helpful assistant.
User: Hello
Assistant: Hi there!
User: How are you?
```

### 3. `openai`

只发送 messages 数组：

```json
{
  "messages": [
    {"role": "user", "content": "Hello"}
  ]
}
```

### 4. `text`

只发送最后一个 user 消息的文本内容：

```
Hello
```

## 输出格式

### 1. `json`（默认）

期望 JSON 格式输出：

```json
{
  "content": "Hello! How can I help you?"
}
```

或者完整的 OpenAI 格式：

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "local-llama-cpp",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Hello! How can I help you?"
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 8,
    "total_tokens": 18
  }
}
```

### 2. `text`

纯文本输出，直接作为 assistant 的回复：

```
Hello! How can I help you?
```

## 使用示例

### 示例 1: llama.cpp

```json
{
  "id": "local-llama-cpp",
  "adapter": "process",
  "command": "/path/to/llama-cli",
  "args": [
    "-m",
    "/path/to/llama-7b.gguf",
    "--ctx-size",
    "4096",
    "--temp",
    "0.7",
    "--prompt-cache",
    "/tmp/llama-cache"
  ],
  "enabled": true,
  "input_format": "prompt",
  "output_format": "text",
  "timeout": 120
}
```

### 示例 2: Ollama CLI

```json
{
  "id": "local-ollama-cli",
  "adapter": "process",
  "command": "ollama",
  "args": [
    "run",
    "llama3"
  ],
  "enabled": true,
  "input_format": "prompt",
  "output_format": "text",
  "timeout": 120
}
```

### 示例 3: 自定义 Python 脚本

```json
{
  "id": "local-python-script",
  "adapter": "process",
  "command": "python",
  "args": [
    "/path/to/custom_ai.py"
  ],
  "enabled": true,
  "input_format": "json",
  "output_format": "json",
  "timeout": 60
}
```

Python 脚本示例 (`custom_ai.py`):

```python
#!/usr/bin/env python3
import sys
import json

# 读取输入
input_data = json.load(sys.stdin)

# 提取 prompt
messages = input_data.get("messages", [])
prompt = messages[-1].get("content", "") if messages else ""

# 生成响应（这里只是示例）
response = {
    "content": f"Echo: {prompt}"
}

# 输出 JSON
print(json.dumps(response))
```

### 示例 4: 使用环境变量

```json
{
  "id": "local-llama-env",
  "adapter": "process",
  "command": "ENV:LLAMA_CLI_PATH",
  "args": [
    "-m",
    "ENV:LLAMA_MODEL_PATH",
    "--ctx-size",
    "4096"
  ],
  "enabled": true,
  "input_format": "prompt",
  "output_format": "text",
  "timeout": 120
}
```

## 超时处理

Process 适配器使用 `ThreadPoolExecutor` 实现跨平台超时：

- 进程执行超时后会被自动终止
- 不会使用 `signal.SIGALRM`（Windows 不支持）
- 默认超时 120 秒，可通过 `timeout` 配置

## 错误处理

- 如果命令不存在或不可执行，`is_available()` 返回 `False`
- 如果进程执行失败（非零返回码），会抛出异常
- 如果超时，会抛出超时异常
- stderr 会包含在错误信息中

## 限制

1. **流式响应**：目前不支持流式输出（SSE），进程必须返回完整响应
2. **二进制数据**：只支持文本输入/输出（UTF-8 编码）
3. **交互式工具**：不支持需要交互式输入的工具
4. **多进程**：每次请求都会启动新进程（不考虑进程池）

## 最佳实践

1. **路径管理**：使用环境变量管理可执行文件和模型路径
2. **超时设置**：根据模型大小和硬件性能设置合适的超时时间
3. **输入格式**：优先使用 `prompt` 格式（更简单）或 `json` 格式（更灵活）
4. **输出格式**：如果工具输出 JSON，使用 `json` 格式；如果是纯文本，使用 `text` 格式
5. **错误调试**：查看 stderr 输出以诊断问题

## 安全注意事项

1. **命令注入**：不要直接使用用户输入构建命令
2. **路径验证**：验证可执行文件路径，避免执行恶意程序
3. **权限控制**：确保进程在受限环境中运行
4. **资源限制**：考虑设置资源限制（CPU、内存）

