# Wiki 功能设计方案

## 1. 架构设计

### 1.1 文件结构
```
.config/
├── wiki/                    # Wiki 根目录
│   ├── index.md            # Wiki 首页
│   ├── tools/              # 工具相关文档
│   │   ├── tool1.md
│   │   └── tool2.md
│   ├── guides/             # 使用指南
│   │   └── getting-started.md
│   └── ...
```

### 1.2 技术方案

#### 后端（Rust）
- **Markdown 解析**: 使用 `pulldown-cmark` 库解析 Markdown
- **代码高亮**: 使用 `syntect` 库进行语法高亮
- **HTTP 服务器**: 使用 `axum` 或 `warp` 创建轻量级 HTTP 服务器
- **文件监听**: 使用 `notify` 库监听文件变化，自动刷新

#### 前端（Vue）
- **WebView 组件**: 使用 iframe 或 Tauri WebView 显示 Wiki
- **目录导航**: 侧边栏显示目录树
- **搜索功能**: 客户端搜索 Markdown 内容

## 2. 实现细节

### 2.1 后端模块

#### wiki_server.rs
- 启动 HTTP 服务器（默认端口 8777）
- 提供以下路由：
  - `GET /` - Wiki 首页
  - `GET /api/files` - 获取文件列表（JSON）
  - `GET /api/render?path=xxx` - 渲染 Markdown 文件
  - `GET /api/tree` - 获取目录树结构
  - `GET /api/search?q=xxx` - 搜索功能
  - `GET /file/:path` - 直接访问文件（支持相对路径）

#### wiki_parser.rs
- 解析 Markdown 文件
- 提取目录结构（TOC）
- 处理代码块高亮
- 转换相对路径为绝对路径

### 2.2 前端组件

#### WikiViewer.vue
- 显示 Wiki 内容
- 支持目录导航
- 支持搜索
- 支持内嵌和独立窗口两种模式

### 2.3 样式设计
- GitHub 风格 Markdown 样式
- 响应式布局
- 深色主题适配
- 代码高亮主题（Monokai 或 GitHub Dark）

## 3. 工具集成

### 3.1 Wiki URL 映射
- 工具可以设置 `wikiUrl`，格式：
  - 相对路径：`tools/tool-name.md`
  - 绝对路径：`/tools/tool-name.md`
  - 完整 URL：`http://127.0.0.1:8777/file/tools/tool-name.md`

### 3.2 自动映射
- 如果工具未设置 wikiUrl，尝试根据工具名称自动查找：
  - `tools/{tool-name}.md`
  - `tools/{tool-id}.md`

## 4. 用户交互

### 4.1 编辑流程
1. 用户在 Typora 等编辑器中编辑 Markdown 文件
2. 保存文件
3. Wiki 服务器检测到文件变化，自动刷新
4. 前端自动更新显示

### 4.2 查看方式
1. **内嵌模式**: 在工具箱内使用 WebView 显示
2. **独立窗口**: 在浏览器中打开完整 Wiki

