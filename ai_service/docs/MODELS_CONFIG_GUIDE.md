# AI 模型配置指南

本文档基于 One API 的实现，列出了所有支持的 AI 模型及其配置方法。

## 目录

- [OpenAI 兼容模型](#openai-兼容模型)
- [需要协议转换的模型](#需要协议转换的模型)
- [配置说明](#配置说明)

---

## OpenAI 兼容模型

这些模型使用 OpenAI-compatible API，可以直接使用 `openai_compat` 适配器。

### 1. OpenAI 系列

```json
{
  "id": "gpt-3.5-turbo",
  "adapter": "openai_compat",
  "base_url": "https://api.openai.com/v1",
  "api_key": "ENV:OPENAI_API_KEY",
  "enabled": true,
  "model": "gpt-3.5-turbo"
}
```

**支持的模型**：
- `gpt-3.5-turbo`
- `gpt-4`
- `gpt-4-turbo-preview`
- `gpt-4o`
- `gpt-4o-mini`

### 2. DeepSeek

```json
{
  "id": "deepseek-chat",
  "adapter": "openai_compat",
  "base_url": "https://api.deepseek.com/v1",
  "api_key": "ENV:DEEPSEEK_API_KEY",
  "enabled": true,
  "model": "deepseek-chat"
}
```

**支持的模型**：
- `deepseek-chat`
- `deepseek-coder`

### 3. Groq

```json
{
  "id": "groq-llama3",
  "adapter": "openai_compat",
  "base_url": "https://api.groq.com/openai/v1",
  "api_key": "ENV:GROQ_API_KEY",
  "enabled": true,
  "model": "llama-3.1-70b-versatile"
}
```

**支持的模型**：
- `llama-3.1-70b-versatile`
- `llama-3.1-8b-instant`
- `mixtral-8x7b-32768`
- `gemma-7b-it`

### 4. Together AI

```json
{
  "id": "together-llama",
  "adapter": "openai_compat",
  "base_url": "https://api.together.xyz/v1",
  "api_key": "ENV:TOGETHER_API_KEY",
  "enabled": true,
  "model": "meta-llama/Llama-3-70b-chat-hf"
}
```

### 5. Mistral AI

```json
{
  "id": "mistral-large",
  "adapter": "openai_compat",
  "base_url": "https://api.mistral.ai/v1",
  "api_key": "ENV:MISTRAL_API_KEY",
  "enabled": true,
  "model": "mistral-large-latest"
}
```

**支持的模型**：
- `mistral-large-latest`
- `mistral-medium-latest`
- `mistral-small-latest`
- `pixtral-large-latest`

### 6. Ollama（本地）

```json
{
  "id": "ollama-llama3",
  "adapter": "openai_compat",
  "base_url": "http://localhost:11434/v1",
  "api_key": "not-needed",
  "enabled": true,
  "model": "llama3"
}
```

**注意**：Ollama 不需要 API Key，`api_key` 设置为 `"not-needed"`。

### 7. LM Studio（本地）

```json
{
  "id": "lmstudio-local",
  "adapter": "openai_compat",
  "base_url": "http://localhost:1234/v1",
  "api_key": "not-needed",
  "enabled": true,
  "model": "local-model"
}
```

### 8. vLLM（本地）

```json
{
  "id": "vllm-local",
  "adapter": "openai_compat",
  "base_url": "http://localhost:8000/v1",
  "api_key": "not-needed",
  "enabled": true,
  "model": "local-model"
}
```

### 9. LocalAI

```json
{
  "id": "localai",
  "adapter": "openai_compat",
  "base_url": "http://localhost:8080/v1",
  "api_key": "not-needed",
  "enabled": true,
  "model": "local-model"
}
```

### 10. Cloudflare Workers AI

```json
{
  "id": "cloudflare-workers-ai",
  "adapter": "openai_compat",
  "base_url": "https://api.cloudflare.com/client/v4/accounts/{ACCOUNT_ID}/ai/run",
  "api_key": "ENV:CLOUDFLARE_API_KEY",
  "enabled": true,
  "model": "@cf/meta/llama-3-8b-instruct"
}
```

---

## 需要协议转换的模型

这些模型使用非 OpenAI 兼容的 API，需要使用 `custom_http` 适配器（待实现）。

### 1. Anthropic Claude

```json
{
  "id": "claude-3-5-sonnet",
  "adapter": "custom_http",
  "base_url": "https://api.anthropic.com",
  "api_key": "ENV:ANTHROPIC_API_KEY",
  "enabled": true,
  "model": "claude-3-5-sonnet-20241022",
  "endpoint": "/v1/messages",
  "request_format": "anthropic",
  "response_format": "openai"
}
```

**支持的模型**（来自 One API）：
- `claude-instant-1.2`
- `claude-2.0`, `claude-2.1`
- `claude-3-haiku-20240307`
- `claude-3-5-haiku-20241022`
- `claude-3-5-haiku-latest`
- `claude-3-sonnet-20240229`
- `claude-3-opus-20240229`
- `claude-3-5-sonnet-20240620`
- `claude-3-5-sonnet-20241022`
- `claude-3-5-sonnet-latest`

### 2. Google Gemini

```json
{
  "id": "gemini-pro",
  "adapter": "custom_http",
  "base_url": "https://generativelanguage.googleapis.com",
  "api_key": "ENV:GEMINI_API_KEY",
  "enabled": true,
  "model": "gemini-pro",
  "endpoint": "/v1beta/models/{model}:generateContent",
  "request_format": "gemini",
  "response_format": "openai"
}
```

**支持的模型**（来自 One API）：
- `gemini-2.0-flash`
- `gemini-2.0-flash-exp`
- `gemini-2.0-flash-thinking-exp-01-21`
- `gemini-1.5-pro`
- `gemini-1.5-flash`
- `gemini-1.0-pro`

### 3. 智谱 AI (Zhipu)

```json
{
  "id": "zhipu-glm-4",
  "adapter": "custom_http",
  "base_url": "https://open.bigmodel.cn/api/paas/v4",
  "api_key": "ENV:ZHIPU_API_KEY",
  "enabled": true,
  "model": "glm-4",
  "endpoint": "/chat/completions",
  "request_format": "zhipu",
  "response_format": "openai"
}
```

**支持的模型**（来自 One API）：
- `glm-4-plus`
- `glm-4-0520`
- `glm-4-airx`
- `glm-4-air`
- `glm-4-long`
- `glm-4-flashx`
- `glm-4-flash`
- `glm-4`
- `glm-3-turbo`
- `glm-4v-plus`
- `glm-4v`
- `glm-4v-flash`
- `cogview-3-plus`
- `cogview-3`
- `cogview-3-flash`
- `cogviewx`
- `cogviewx-flash`
- `charglm-4`
- `emohaa`
- `codegeex-4`
- `embedding-2`
- `embedding-3`

### 4. 百度文心一言

```json
{
  "id": "baidu-ernie-4.0",
  "adapter": "custom_http",
  "base_url": "https://aip.baidubce.com",
  "api_key": "ENV:BAIDU_API_KEY",
  "enabled": true,
  "model": "ERNIE-4.0-8K",
  "endpoint": "/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/completions",
  "request_format": "baidu",
  "response_format": "openai",
  "config": {
    "access_token": "ENV:BAIDU_ACCESS_TOKEN"
  }
}
```

**支持的模型**（来自 One API）：
- `ERNIE-4.0-8K`
- `ERNIE-3.5-8K`
- `ERNIE-3.5-8K-0205`
- `ERNIE-3.5-8K-1222`
- `ERNIE-Bot-8K`
- `ERNIE-3.5-4K-0205`
- `ERNIE-Speed-8K`
- `ERNIE-Speed-128K`
- `ERNIE-Lite-8K-0922`
- `ERNIE-Lite-8K-0308`
- `ERNIE-Tiny-8K`
- `BLOOMZ-7B`
- `Embedding-V1`
- `bge-large-zh`
- `bge-large-en`
- `tao-8k`

### 5. 阿里通义千问

```json
{
  "id": "ali-qwen-max",
  "adapter": "custom_http",
  "base_url": "https://dashscope.aliyuncs.com",
  "api_key": "ENV:ALI_API_KEY",
  "enabled": true,
  "model": "qwen-max",
  "endpoint": "/api/v1/services/aigc/text-generation/generation",
  "request_format": "ali",
  "response_format": "openai"
}
```

**支持的模型**（来自 One API，部分）：
- `qwen-turbo`
- `qwen-plus`
- `qwen-max`
- `qwen-max-longcontext`
- `qwen-vl-max`
- `qwen-vl-plus`
- `qwen-vl-ocr`
- `qwen-audio-turbo`
- `qwen-math-plus`
- `qwen-math-turbo`
- `qwen-coder-plus`
- `qwen-coder-turbo`
- `qwen2.5-72b-instruct`
- `qwen2.5-32b-instruct`
- `qwen2.5-14b-instruct`
- `qwen2.5-7b-instruct`
- `qwen2.5-3b-instruct`
- `qwen2.5-1.5b-instruct`
- `qwen2.5-0.5b-instruct`
- `qwen2-72b-instruct`
- `qwen2-7b-instruct`
- `qwen1.5-110b-chat`
- `qwen1.5-72b-chat`
- `qwen1.5-32b-chat`
- `qwen1.5-14b-chat`
- `qwen1.5-7b-chat`
- `qwen1.5-1.8b-chat`
- `qwen1.5-0.5b-chat`
- `qwen-72b-chat`
- `qwen-14b-chat`
- `qwen-7b-chat`
- `qwen-1.8b-chat`
- `qwen-1.8b-longcontext-chat`
- `deepseek-r1`
- `deepseek-v3`
- `text-embedding-v1`
- `text-embedding-v2`
- `text-embedding-v3`
- `ali-stable-diffusion-xl`
- `ali-stable-diffusion-v1.5`
- `wanx-v1`
- `qwen-mt-plus`
- `qwen-mt-turbo`

### 6. 讯飞星火

```json
{
  "id": "xunfei-spark-max",
  "adapter": "custom_http",
  "base_url": "wss://spark-api.xf-yun.com",
  "api_key": "ENV:XUNFEI_API_KEY",
  "enabled": true,
  "model": "Spark-Max",
  "endpoint": "/v3.5/chat",
  "request_format": "xunfei",
  "response_format": "openai",
  "config": {
    "app_id": "ENV:XUNFEI_APP_ID",
    "api_secret": "ENV:XUNFEI_API_SECRET"
  }
}
```

**支持的模型**（来自 One API）：
- `Spark-Lite`
- `Spark-Pro`
- `Spark-Pro-128K`
- `Spark-Max`
- `Spark-Max-32K`
- `Spark-4.0-Ultra`

**注意**：讯飞星火使用 WebSocket，需要特殊处理。

### 7. 腾讯混元

```json
{
  "id": "tencent-hunyuan-pro",
  "adapter": "custom_http",
  "base_url": "https://hunyuan.tencentcloudapi.com",
  "api_key": "ENV:TENCENT_SECRET_ID",
  "enabled": true,
  "model": "hunyuan-pro",
  "endpoint": "/",
  "request_format": "tencent",
  "response_format": "openai",
  "config": {
    "secret_key": "ENV:TENCENT_SECRET_KEY",
    "region": "ap-beijing"
  }
}
```

**支持的模型**（来自 One API）：
- `hunyuan-lite`
- `hunyuan-standard`
- `hunyuan-standard-256K`
- `hunyuan-pro`
- `hunyuan-vision`
- `hunyuan-embedding`

### 8. Moonshot (Kimi)

```json
{
  "id": "moonshot-kimi",
  "adapter": "custom_http",
  "base_url": "https://api.moonshot.cn/v1",
  "api_key": "ENV:MOONSHOT_API_KEY",
  "enabled": true,
  "model": "moonshot-v1-8k",
  "endpoint": "/chat/completions",
  "request_format": "moonshot",
  "response_format": "openai"
}
```

### 9. 百川智能

```json
{
  "id": "baichuan-turbo",
  "adapter": "custom_http",
  "base_url": "https://api.baichuan-ai.com/v1",
  "api_key": "ENV:BAICHUAN_API_KEY",
  "enabled": true,
  "model": "Baichuan2-Turbo",
  "endpoint": "/chat/completions",
  "request_format": "baichuan",
  "response_format": "openai"
}
```

### 10. MINIMAX

```json
{
  "id": "minimax-abab5",
  "adapter": "custom_http",
  "base_url": "https://api.minimax.chat/v1",
  "api_key": "ENV:MINIMAX_API_KEY",
  "enabled": true,
  "model": "abab5.5-chat",
  "endpoint": "/text/chatcompletion_v2",
  "request_format": "minimax",
  "response_format": "openai",
  "config": {
    "group_id": "ENV:MINIMAX_GROUP_ID"
  }
}
```

### 11. 字节跳动豆包

```json
{
  "id": "doubao-pro",
  "adapter": "custom_http",
  "base_url": "https://ark.cn-beijing.volces.com/api/v3",
  "api_key": "ENV:DOUBAO_API_KEY",
  "enabled": true,
  "model": "doubao-pro-32k",
  "endpoint": "/chat/completions",
  "request_format": "doubao",
  "response_format": "openai"
}
```

### 12. 零一万物

```json
{
  "id": "lingyiwanwu-yi-34b",
  "adapter": "custom_http",
  "base_url": "https://api.lingyiwanwu.com/v1",
  "api_key": "ENV:LINGYIWANWU_API_KEY",
  "enabled": true,
  "model": "yi-34b-chat-0205",
  "endpoint": "/chat/completions",
  "request_format": "lingyiwanwu",
  "response_format": "openai"
}
```

### 13. 阶跃星辰

```json
{
  "id": "stepfun-step-1",
  "adapter": "custom_http",
  "base_url": "https://api.stepfun.com/v1",
  "api_key": "ENV:STEPFUN_API_KEY",
  "enabled": true,
  "model": "step-1-32k",
  "endpoint": "/chat/completions",
  "request_format": "stepfun",
  "response_format": "openai"
}
```

### 14. Cohere

```json
{
  "id": "cohere-command",
  "adapter": "custom_http",
  "base_url": "https://api.cohere.ai/v1",
  "api_key": "ENV:COHERE_API_KEY",
  "enabled": true,
  "model": "command",
  "endpoint": "/chat",
  "request_format": "cohere",
  "response_format": "openai"
}
```

### 15. Coze

```json
{
  "id": "coze-chat",
  "adapter": "custom_http",
  "base_url": "https://api.coze.cn/open_api/v2",
  "api_key": "ENV:COZE_API_KEY",
  "enabled": true,
  "model": "coze-chat",
  "endpoint": "/chat",
  "request_format": "coze",
  "response_format": "openai"
}
```

### 16. SiliconFlow

```json
{
  "id": "siliconflow-llama",
  "adapter": "custom_http",
  "base_url": "https://api.siliconflow.cn/v1",
  "api_key": "ENV:SILICONFLOW_API_KEY",
  "enabled": true,
  "model": "meta-llama/Llama-3-70B-Instruct",
  "endpoint": "/chat/completions",
  "request_format": "siliconflow",
  "response_format": "openai"
}
```

### 17. xAI (Grok)

```json
{
  "id": "xai-grok",
  "adapter": "custom_http",
  "base_url": "https://api.x.ai/v1",
  "api_key": "ENV:XAI_API_KEY",
  "enabled": true,
  "model": "grok-beta",
  "endpoint": "/chat/completions",
  "request_format": "xai",
  "response_format": "openai"
}
```

### 18. Replicate

```json
{
  "id": "replicate-llama",
  "adapter": "custom_http",
  "base_url": "https://api.replicate.com/v1",
  "api_key": "ENV:REPLICATE_API_KEY",
  "enabled": true,
  "model": "meta/llama-2-70b-chat",
  "endpoint": "/predictions",
  "request_format": "replicate",
  "response_format": "openai"
}
```

### 19. DeepL（翻译）

```json
{
  "id": "deepl-translate",
  "adapter": "custom_http",
  "base_url": "https://api-free.deepl.com",
  "api_key": "ENV:DEEPL_API_KEY",
  "enabled": true,
  "model": "deepl-translate",
  "endpoint": "/v2/translate",
  "request_format": "deepl",
  "response_format": "openai"
}
```

### 20. Novita（图像生成）

```json
{
  "id": "novita-image",
  "adapter": "custom_http",
  "base_url": "https://api.novita.ai/v3",
  "api_key": "ENV:NOVITA_API_KEY",
  "enabled": true,
  "model": "novita-image",
  "endpoint": "/image/generation",
  "request_format": "novita",
  "response_format": "openai"
}
```

### 21. 360 智脑

```json
{
  "id": "ai360-360gpt",
  "adapter": "custom_http",
  "base_url": "https://api.360.cn/v1",
  "api_key": "ENV:AI360_API_KEY",
  "enabled": true,
  "model": "360gpt-pro",
  "endpoint": "/chat/completions",
  "request_format": "ai360",
  "response_format": "openai"
}
```

### 22. Google Vertex AI

```json
{
  "id": "vertexai-gemini",
  "adapter": "custom_http",
  "base_url": "https://{REGION}-aiplatform.googleapis.com",
  "api_key": "ENV:VERTEXAI_CREDENTIALS",
  "enabled": true,
  "model": "gemini-pro",
  "endpoint": "/v1/projects/{PROJECT_ID}/locations/{REGION}/publishers/google/models/{model}:predict",
  "request_format": "vertexai",
  "response_format": "openai",
  "config": {
    "project_id": "ENV:VERTEXAI_PROJECT_ID",
    "region": "us-central1"
  }
}
```

### 23. AWS Bedrock

```json
{
  "id": "aws-claude",
  "adapter": "custom_http",
  "base_url": "https://bedrock-runtime.{REGION}.amazonaws.com",
  "api_key": "ENV:AWS_ACCESS_KEY_ID",
  "enabled": true,
  "model": "anthropic.claude-3-5-sonnet-20241022-v2:0",
  "endpoint": "/model/{model}/invoke",
  "request_format": "aws",
  "response_format": "openai",
  "config": {
    "secret_access_key": "ENV:AWS_SECRET_ACCESS_KEY",
    "region": "us-east-1"
  }
}
```

### 24. 阿里百炼

```json
{
  "id": "alibailian-qwen",
  "adapter": "custom_http",
  "base_url": "https://dashscope.aliyuncs.com",
  "api_key": "ENV:ALI_API_KEY",
  "enabled": true,
  "model": "qwen-max",
  "endpoint": "/api/v1/services/aigc/text-generation/generation",
  "request_format": "alibailian",
  "response_format": "openai"
}
```

---

## 配置说明

### 通用字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | ✅ | 模型唯一标识（用于路由） |
| `adapter` | string | ✅ | 适配器类型：`openai_compat` 或 `custom_http` |
| `base_url` | string | ✅ | API 基础 URL |
| `api_key` | string | ✅ | API Key（支持 `ENV:VAR_NAME` 环境变量） |
| `enabled` | boolean | ✅ | 是否启用 |
| `model` | string | ✅ | 实际使用的模型名称 |
| `temperature` | number | ❌ | 默认温度参数 |
| `max_tokens` | number | ❌ | 默认最大 token 数 |
| `timeout` | number | ❌ | 请求超时时间（秒） |

### custom_http 适配器专用字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `endpoint` | string | ✅ | API 端点路径 |
| `request_format` | string | ✅ | 请求格式：`anthropic`, `gemini`, `zhipu`, `baidu`, `ali`, `xunfei`, `tencent`, 等 |
| `response_format` | string | ✅ | 响应格式（通常为 `openai`） |
| `config` | object | ❌ | 额外配置（如 `app_id`, `secret_key`, `region` 等） |

### 环境变量支持

所有 `api_key` 和 `config` 中的值都支持环境变量引用：
- `"ENV:OPENAI_API_KEY"` - 从环境变量 `OPENAI_API_KEY` 读取
- `"ENV:BAIDU_ACCESS_TOKEN"` - 从环境变量 `BAIDU_ACCESS_TOKEN` 读取

### 本地服务配置

对于本地服务（如 Ollama、LM Studio），`api_key` 可以设置为 `"not-needed"`。

---

## 使用示例

### 1. 启用 OpenAI 模型

编辑 `config/models.json`，将 `gpt-3.5-turbo` 的 `enabled` 设置为 `true`，并设置环境变量：

```bash
export OPENAI_API_KEY="sk-..."
```

### 2. 启用 DeepSeek 模型

编辑 `config/models.json`，将 `deepseek-chat` 的 `enabled` 设置为 `true`，并设置环境变量：

```bash
export DEEPSEEK_API_KEY="sk-..."
```

### 3. 启用本地 Ollama

1. 确保 Ollama 正在运行：`ollama serve`
2. 编辑 `config/models.json`，将 `ollama-llama3` 的 `enabled` 设置为 `true`
3. 确保 `base_url` 指向正确的 Ollama 地址

---

## 注意事项

1. **custom_http 适配器**：目前 `custom_http` 适配器尚未实现，需要协议转换的模型暂时无法使用。这些模型的配置已准备好，等待适配器实现。

2. **WebSocket 支持**：讯飞星火使用 WebSocket，需要特殊处理。

3. **认证方式**：不同模型的认证方式可能不同：
   - OpenAI 兼容：`Authorization: Bearer {api_key}`
   - Anthropic：`x-api-key: {api_key}`
   - 百度：需要 `access_token`
   - 腾讯：需要 `SecretId` 和 `SecretKey`

4. **模型名称**：某些模型的 `id` 和 `model` 可能不同，`id` 用于路由，`model` 是实际发送给 API 的模型名称。

---

## 参考

- [One API 项目](https://github.com/songquanpeng/one-api)
- [One API 架构分析](./ONE_API_ARCHITECTURE_ANALYSIS.md)
- [AI Gateway 实现指南](./AI_GATEWAY_IMPLEMENTATION_GUIDE.md)

