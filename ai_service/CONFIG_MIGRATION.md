# 配置迁移指南

从 One API 的 Channel 配置迁移到 AI Gateway 的 models.json 配置。

## One API Channel 配置 vs AI Gateway Model 配置

### One API Channel 字段映射

| One API Channel 字段 | AI Gateway Model 字段 | 说明 |
|---------------------|---------------------|------|
| `Channel.Id` | `id` | 模型唯一标识 |
| `Channel.Type` | `adapter` | 适配器类型（需要转换） |
| `Channel.Key` | `api_key` | API Key |
| `Channel.BaseURL` | `base_url` | API 基础 URL |
| `Channel.Models` | `model` | 实际使用的模型名称（取第一个） |
| `Channel.Status` | `enabled` | 是否启用（1 = true, 其他 = false） |
| `Channel.Config` | `config` | 额外配置（JSON 字符串 → 对象） |

### Channel Type → Adapter 映射

| One API Channel Type | AI Gateway Adapter | 说明 |
|---------------------|-------------------|------|
| `OpenAI` | `openai_compat` | OpenAI 兼容 |
| `Anthropic` | `custom_http` | 需要协议转换 |
| `Gemini` | `custom_http` | 需要协议转换 |
| `Zhipu` | `custom_http` | 需要协议转换 |
| `Baidu` | `custom_http` | 需要协议转换 |
| `Ali` | `custom_http` | 需要协议转换 |
| `Xunfei` | `custom_http` | 需要协议转换（WebSocket） |
| `Tencent` | `custom_http` | 需要协议转换 |
| `Moonshot` | `custom_http` | 需要协议转换 |
| `Baichuan` | `custom_http` | 需要协议转换 |
| `Minimax` | `custom_http` | 需要协议转换 |
| `Doubao` | `custom_http` | 需要协议转换 |
| `LingYiWanWu` | `custom_http` | 需要协议转换 |
| `StepFun` | `custom_http` | 需要协议转换 |
| `Cohere` | `custom_http` | 需要协议转换 |
| `Coze` | `custom_http` | 需要协议转换 |
| `SiliconFlow` | `custom_http` | 需要协议转换 |
| `XAI` | `custom_http` | 需要协议转换 |
| `Replicate` | `custom_http` | 需要协议转换 |
| `DeepL` | `custom_http` | 需要协议转换 |
| `Novita` | `custom_http` | 需要协议转换 |
| `AI360` | `custom_http` | 需要协议转换 |
| `VertexAI` | `custom_http` | 需要协议转换 |
| `AwsClaude` | `custom_http` | 需要协议转换 |
| `AliBailian` | `custom_http` | 需要协议转换 |
| `Ollama` | `openai_compat` | OpenAI 兼容 |
| `Groq` | `openai_compat` | OpenAI 兼容 |
| `TogetherAI` | `openai_compat` | OpenAI 兼容 |
| `Mistral` | `openai_compat` | OpenAI 兼容 |
| `DeepSeek` | `openai_compat` | OpenAI 兼容 |
| `Cloudflare` | `openai_compat` | OpenAI 兼容 |

### Channel Config → Model Config 映射

One API 的 `Channel.Config` 是 JSON 字符串，包含以下字段：

```go
type ChannelConfig struct {
    Region            string `json:"region,omitempty"`
    SK                string `json:"sk,omitempty"`
    AK                string `json:"ak,omitempty"`
    UserID            string `json:"user_id,omitempty"`
    APIVersion        string `json:"api_version,omitempty"`
    LibraryID         string `json:"library_id,omitempty"`
    Plugin            string `json:"plugin,omitempty"`
    VertexAIProjectID string `json:"vertex_ai_project_id,omitempty"`
    VertexAIADC       string `json:"vertex_ai_adc,omitempty"`
}
```

这些字段需要映射到 AI Gateway 的 `config` 对象中：

| One API Config 字段 | AI Gateway Config 字段 | 说明 |
|-------------------|----------------------|------|
| `region` | `region` | 区域（AWS、腾讯等） |
| `sk` | `secret_key` | 密钥（腾讯等） |
| `ak` | `access_key` | 访问密钥 |
| `user_id` | `user_id` | 用户 ID |
| `api_version` | `api_version` | API 版本（Azure 等） |
| `library_id` | `library_id` | 库 ID（AIProxyLibrary） |
| `plugin` | `plugin` | 插件（阿里等） |
| `vertex_ai_project_id` | `project_id` | 项目 ID（Vertex AI） |
| `vertex_ai_adc` | `credentials` | 凭证（Vertex AI） |

## 迁移步骤

### 1. 导出 One API Channel 配置

从 One API 数据库导出 Channel 数据：

```sql
SELECT id, type, name, key, base_url, models, status, config 
FROM channels;
```

### 2. 转换为 models.json 格式

使用以下 Python 脚本转换：

```python
import json

# One API Channel 数据
channels = [
    {
        "id": 1,
        "type": 0,  # OpenAI
        "name": "OpenAI GPT-3.5",
        "key": "sk-...",
        "base_url": "",
        "models": "gpt-3.5-turbo",
        "status": 1,
        "config": "{}"
    },
    # ... 更多 Channel
]

# Channel Type 映射
TYPE_TO_ADAPTER = {
    0: "openai_compat",  # OpenAI
    18: "custom_http",   # Anthropic
    28: "custom_http",   # Gemini
    20: "custom_http",   # Zhipu
    19: "custom_http",   # Baidu
    21: "custom_http",   # Ali
    22: "custom_http",   # Xunfei
    27: "custom_http",   # Tencent
    29: "custom_http",   # Moonshot
    30: "custom_http",   # Baichuan
    31: "custom_http",   # Minimax
    44: "custom_http",   # Doubao
    35: "custom_http",   # LingYiWanWu
    36: "custom_http",   # StepFun
    17: "custom_http",   # Cohere
    38: "custom_http",   # Coze
    48: "custom_http",   # SiliconFlow
    49: "custom_http",   # XAI
    50: "custom_http",   # Replicate
    42: "custom_http",   # DeepL
    45: "custom_http",   # Novita
    23: "custom_http",   # AI360
    46: "custom_http",   # VertexAI
    37: "custom_http",   # AwsClaude
    53: "custom_http",   # AliBailian
    34: "openai_compat", # Ollama
    33: "openai_compat", # Groq
    43: "openai_compat", # TogetherAI
    32: "openai_compat", # Mistral
    40: "openai_compat", # DeepSeek
    41: "openai_compat", # Cloudflare
}

models = []

for channel in channels:
    # 获取模型名称（取第一个）
    model_name = channel["models"].split(",")[0].strip() if channel["models"] else "unknown"
    
    # 构建模型配置
    model_config = {
        "id": f"{model_name}-{channel['id']}",  # 使用模型名 + Channel ID 作为唯一标识
        "adapter": TYPE_TO_ADAPTER.get(channel["type"], "openai_compat"),
        "base_url": channel["base_url"] or get_default_base_url(channel["type"]),
        "api_key": channel["key"],
        "enabled": channel["status"] == 1,
        "model": model_name,
    }
    
    # 解析 Config
    if channel["config"]:
        try:
            config_obj = json.loads(channel["config"])
            if config_obj:
                model_config["config"] = config_obj
        except:
            pass
    
    models.append(model_config)

# 输出 models.json
output = {
    "models": models
}

with open("models.json", "w", encoding="utf-8") as f:
    json.dump(output, f, indent=2, ensure_ascii=False)
```

### 3. 手动调整配置

转换后需要手动调整：

1. **检查 `base_url`**：确保 URL 正确
2. **设置 `endpoint`**：对于 `custom_http` 适配器，需要设置正确的端点
3. **设置 `request_format` 和 `response_format`**：对于 `custom_http` 适配器
4. **环境变量**：将 API Key 改为 `ENV:VAR_NAME` 格式
5. **测试**：启用一个模型并测试

## 注意事项

1. **模型 ID 唯一性**：确保每个模型的 `id` 字段唯一
2. **环境变量**：建议使用环境变量存储 API Key，而不是直接写在配置文件中
3. **默认禁用**：迁移后建议先禁用所有模型，逐个启用并测试
4. **custom_http 适配器**：目前尚未实现，需要协议转换的模型暂时无法使用

## 参考

- [One API 项目](https://github.com/songquanpeng/one-api)
- [模型配置指南](./MODELS_CONFIG_GUIDE.md)
- [架构分析文档](../ONE_API_ARCHITECTURE_ANALYSIS.md)

