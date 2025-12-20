# AI Gateway 文档索引

本文档目录包含了 AI Gateway 服务的所有详细文档。

## 📚 文档分类

### 🚀 快速开始
- [模型配置指南](./MODELS_CONFIG_GUIDE.md) - 所有支持的模型及其配置方法
- [配置迁移指南](./CONFIG_MIGRATION.md) - 从 One API 迁移配置

### 🔧 适配器实现
- [Custom HTTP 适配器实现](./CUSTOM_HTTP_ADAPTER_IMPLEMENTATION.md) - Custom HTTP 适配器详细说明
- [Process 适配器指南](./PROCESS_ADAPTER_GUIDE.md) - 本地 CLI 工具支持

### ⚡ 高级功能
- [流式响应指南](./STREAMING_GUIDE.md) - SSE 流式响应实现
- [WebSocket 支持指南](./WEBSOCKET_GUIDE.md) - WebSocket 协议支持
- [错误重试机制](./RETRY_GUIDE.md) - 重试策略与配置

### 📊 状态与架构
- [实现状态对比](./IMPLEMENTATION_STATUS.md) - 与 One API 的对比
- [代码结构说明](./CODE_STRUCTURE.md) - 代码组织架构
- [代码审计报告](./CODE_AUDIT_REPORT.md) - 代码质量审计
- [清理总结](./CLEANUP_SUMMARY.md) - 代码清理记录
- [转换器实现总结](./CONVERTERS_IMPLEMENTATION_SUMMARY.md) - 协议转换器实现详情

## 📖 文档说明

### 配置相关
这些文档帮助你配置和使用 AI Gateway：

- **MODELS_CONFIG_GUIDE.md**: 详细的模型配置说明，包括所有支持的模型类型和配置示例
- **CONFIG_MIGRATION.md**: 如果你之前使用 One API，这个文档帮助你迁移配置

### 实现相关
这些文档深入解释实现细节：

- **CUSTOM_HTTP_ADAPTER_IMPLEMENTATION.md**: Custom HTTP 适配器的实现原理和扩展方法
- **PROCESS_ADAPTER_GUIDE.md**: 如何使用 Process 适配器运行本地 CLI 工具

### 功能相关
这些文档说明高级功能的使用：

- **STREAMING_GUIDE.md**: 如何启用和使用流式响应（SSE）
- **WEBSOCKET_GUIDE.md**: WebSocket 支持的配置和使用
- **RETRY_GUIDE.md**: 错误重试机制的配置和原理

### 架构相关
这些文档描述系统的架构和状态：

- **IMPLEMENTATION_STATUS.md**: 与 One API 的功能对比，了解已实现的功能
- **CODE_STRUCTURE.md**: 代码的组织结构和模块职责
- **CODE_AUDIT_REPORT.md**: 代码质量审计报告

## 🔍 快速查找

### 我需要...

- **配置一个新模型** → [MODELS_CONFIG_GUIDE.md](./MODELS_CONFIG_GUIDE.md)
- **从 One API 迁移** → [CONFIG_MIGRATION.md](./CONFIG_MIGRATION.md)
- **实现一个自定义转换器** → [CUSTOM_HTTP_ADAPTER_IMPLEMENTATION.md](./CUSTOM_HTTP_ADAPTER_IMPLEMENTATION.md)
- **使用本地模型** → [PROCESS_ADAPTER_GUIDE.md](./PROCESS_ADAPTER_GUIDE.md)
- **启用流式响应** → [STREAMING_GUIDE.md](./STREAMING_GUIDE.md)
- **配置 WebSocket 模型** → [WEBSOCKET_GUIDE.md](./WEBSOCKET_GUIDE.md)
- **了解重试机制** → [RETRY_GUIDE.md](./RETRY_GUIDE.md)
- **查看实现状态** → [IMPLEMENTATION_STATUS.md](./IMPLEMENTATION_STATUS.md)
- **理解代码结构** → [CODE_STRUCTURE.md](./CODE_STRUCTURE.md)

## 📝 文档维护

所有文档都应与代码保持同步。如果发现文档过时或有误，请及时更新。

## 🔗 外部链接

- 主 README: [../README.md](../README.md)
- One API 项目: [https://github.com/songquanpeng/one-api](https://github.com/songquanpeng/one-api)

