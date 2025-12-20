# 代码结构说明

## 目录结构

```
ai_service/
├── main_gateway.py          # 主服务入口
├── requirements.txt          # Python 依赖
├── test_gateway.py          # 测试脚本
├── README.md                # 使用文档
├── .gitignore               # Git 忽略文件
│
├── api/                     # API 层
│   ├── __init__.py
│   └── openai_handler.py    # OpenAI-compatible API Handler
│
├── core/                    # 核心模块
│   ├── __init__.py
│   ├── registry.py          # 模型注册表
│   ├── router.py            # 请求路由器
│   └── adapter/             # 适配器模块
│       ├── __init__.py
│       ├── base_adapter.py  # 适配器基类
│       ├── openai_compat_adapter.py    # OpenAI 兼容适配器
│       ├── custom_http_adapter.py      # Custom HTTP 适配器
│       └── converters/      # 协议转换器
│           ├── __init__.py
│           ├── base_converter.py       # 转换器基类
│           ├── registry.py             # 转换器注册表
│           ├── anthropic_converter.py  # Anthropic (Claude) 转换器
│           ├── gemini_converter.py     # Google Gemini 转换器
│           ├── zhipu_converter.py      # 智谱 AI 转换器
│           ├── baidu_converter.py      # 百度文心一言转换器
│           └── ali_converter.py        # 阿里通义千问转换器
│
├── config/                  # 配置目录
│   ├── __init__.py
│   ├── models.json          # 模型配置文件
│   └── models.json.example  # 配置示例文件
│
└── docs/                    # 文档目录（可选）
    ├── MODELS_CONFIG_GUIDE.md
    ├── CONFIG_MIGRATION.md
    └── CUSTOM_HTTP_ADAPTER_IMPLEMENTATION.md
```

## 代码层次

### 1. 入口层 (`main_gateway.py`)
- 启动 HTTP 服务器
- 初始化 Registry 和 Router
- 处理命令行参数

### 2. API 层 (`api/`)
- `openai_handler.py`: 处理 HTTP 请求，提供 OpenAI-compatible API
- 支持 `/v1/chat/completions`, `/v1/models`, `/health` 端点

### 3. 核心层 (`core/`)
- `registry.py`: 模型注册表，从配置文件加载模型，管理 Adapter 实例
- `router.py`: 请求路由器，根据 model_id 路由请求到对应的 Adapter

### 4. 适配器层 (`core/adapter/`)
- `base_adapter.py`: 定义 `ChatAdapter` 接口和数据结构
- `openai_compat_adapter.py`: OpenAI 兼容适配器（直接转发）
- `custom_http_adapter.py`: Custom HTTP 适配器（协议转换）
- `converters/`: 各种协议转换器实现

## 代码规范

### 编码声明
所有 Python 文件开头应包含：
```python
# -*- coding: utf-8 -*-
```

### 导入顺序
1. 标准库导入
2. 第三方库导入
3. 本地模块导入
4. 路径设置（如需要）

### 模块文档字符串
每个模块应包含文档字符串，说明其职责和对应 One API 的模块。

### 类型注解
使用类型注解提高代码可读性：
```python
def function(param: str) -> Dict[str, Any]:
    ...
```

## 依赖管理

- `requirements.txt`: 列出所有 Python 依赖
- 使用 `>=` 指定最低版本
- 主要依赖：
  - `aiohttp`: 异步 HTTP 客户端
  - `python-dotenv`: 环境变量支持
  - `PyJWT`: JWT Token 生成（Zhipu）

## 配置管理

- 配置文件：`config/models.json`
- 配置示例：`config/models.json.example`
- 环境变量支持：`ENV:VAR_NAME` 格式

## 测试

- `test_gateway.py`: 基础功能测试脚本
- 测试端点：健康检查、模型列表、聊天接口

## 文档

- `README.md`: 主要使用文档
- `MODELS_CONFIG_GUIDE.md`: 模型配置指南
- `CONFIG_MIGRATION.md`: 配置迁移指南
- `CUSTOM_HTTP_ADAPTER_IMPLEMENTATION.md`: Custom HTTP 适配器实现说明

