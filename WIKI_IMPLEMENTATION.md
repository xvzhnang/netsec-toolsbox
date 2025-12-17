# Wiki 功能实现方案

## 1. 架构设计

### 1.1 文件结构
```
netsec-toolbox/
├── wiki/                    # Wiki 根目录（同级目录，不在 .config 下）
│   ├── index.md            # Wiki 首页
│   ├── theme/              # 主题目录
│   │   ├── default.css     # 默认主题
│   │   ├── dark.css        # 深色主题
│   │   └── ...             # 用户自定义主题
│   ├── tools/              # 工具相关文档
│   │   ├── tool1.md
│   │   └── tool2.md
│   ├── guides/             # 使用指南
│   │   └── getting-started.md
│   └── ...
└── src-tauri/
    └── src/
        └── wiki/           # Wiki 后端模块
            ├── server.rs   # Wiki 服务器管理
            ├── parser.rs   # Markdown 解析和代码高亮
            ├── http_server.rs # HTTP 服务器（axum）
            ├── commands.rs # Tauri 命令
            └── types.rs    # 数据类型定义
```

### 1.2 技术栈

#### 后端（Rust）
- **Markdown 解析**: `pulldown-cmark` - 支持 GitHub 风格 Markdown
- **代码高亮**: `syntect` - 支持多种编程语言语法高亮
- **HTTP 服务器**: `axum` - 轻量级异步 HTTP 服务器
- **文件系统**: 自动扫描 `wiki/` 目录，支持嵌套文件夹

#### 前端（Vue 3）
- **WebView 组件**: `WikiViewer.vue` - 支持内嵌和独立窗口两种模式
- **工具集成**: 每个工具可以绑定 Wiki URL，点击帮助按钮显示 Wiki

## 2. 核心功能实现

### 2.1 Markdown 文件管理

#### 文件解析逻辑
- **自动扫描**: 启动时自动扫描 `wiki/` 目录下的所有 `.md` 文件
- **目录结构**: 递归扫描，支持任意深度的文件夹嵌套
- **文件信息提取**: 
  - 从文件第一行提取标题（`# 标题`）
  - 生成相对路径用于 URL 映射
  - 构建树形结构用于导航

**实现位置**: `src-tauri/src/wiki/server.rs::list_wiki_files()`

### 2.2 Markdown 转 HTML 渲染

#### 渲染流程
1. **读取 Markdown 文件**
2. **提取目录结构（TOC）**: 从标题（`#` 到 `######`）生成目录树
3. **代码高亮处理**: 
   - 识别代码块语言
   - 使用 `syntect` 进行语法高亮
   - 转换为带样式的 HTML
4. **Markdown 解析**: 使用 `pulldown-cmark` 解析为 HTML
5. **HTML 包装**: 添加 GitHub 风格样式、侧边栏导航、主题样式

**实现位置**: `src-tauri/src/wiki/parser.rs::render()`

#### 支持的 Markdown 特性
- ✅ 标题（H1-H6）
- ✅ 列表（有序、无序）
- ✅ 表格
- ✅ 代码块（带语法高亮）
- ✅ 行内代码
- ✅ 链接和图片
- ✅ 引用块
- ✅ 任务列表
- ✅ 删除线
- ✅ 脚注

### 2.3 代码高亮

#### 实现方式
- **语法识别**: 根据代码块语言标识符（如 ````python`）识别语言
- **高亮引擎**: 使用 `syntect` 库，支持 100+ 种编程语言
- **主题**: 默认使用 Monokai 主题，可扩展
- **输出格式**: 转换为带 `<span>` 标签的 HTML，每个 token 都有颜色样式

**实现位置**: `src-tauri/src/wiki/parser.rs::highlight_code()`

### 2.4 目录导航和搜索

#### 目录导航
- **侧边栏文件树**: 显示所有 Wiki 文件，支持文件夹展开/折叠
- **页面目录（TOC）**: 显示当前页面的标题结构，支持快速跳转
- **锚点链接**: 所有标题自动生成锚点 ID，支持直接跳转

#### 搜索功能
- **全文搜索**: 在所有 Markdown 文件中搜索关键词
- **搜索结果**: 显示匹配的文件标题和匹配行内容
- **实时搜索**: 输入关键词后按 Enter 执行搜索

**实现位置**: 
- 文件树: `src-tauri/src/wiki/http_server.rs::get_file_tree_html()`
- 搜索: `src-tauri/src/wiki/server.rs::search_wiki_files()`

### 2.5 主题系统

#### 主题加载机制
1. **主题目录**: `wiki/theme/` 目录存放所有主题文件
2. **主题文件**: 每个主题是一个 `.css` 文件（如 `default.css`、`dark.css`）
3. **主题选择**: 
   - 通过 URL 查询参数 `?theme=dark` 切换主题
   - 通过侧边栏下拉菜单选择主题
   - 主题选择保存到 `localStorage`，下次自动应用
4. **主题应用**: 主题 CSS 文件内容直接注入到 HTML `<head>` 中

#### 自定义主题
- **创建主题**: 在 `wiki/theme/` 目录下创建新的 `.css` 文件
- **主题变量**: 支持 CSS 变量（`--wiki-primary-color` 等）方便自定义
- **样式覆盖**: 主题 CSS 会覆盖默认样式

**实现位置**: 
- 主题加载: `src-tauri/src/wiki/http_server.rs::load_custom_theme()`
- 主题列表: `src-tauri/src/wiki/http_server.rs::get_available_themes()`

### 2.6 工具集成

#### Wiki URL 绑定
- **工具配置**: 每个工具可以设置 `wikiUrl` 字段
- **URL 格式**:
  - 相对路径: `tools/tool-name.md`
  - 绝对路径: `/tools/tool-name.md`
  - 完整 URL: `http://127.0.0.1:8777/file/tools/tool-name.md`

#### 自动查找
- **智能匹配**: 如果工具未设置 `wikiUrl`，系统会尝试自动查找：
  1. 精确匹配工具 ID: `tools/{tool-id}.md`
  2. 匹配工具名称: `tools/{tool-name}.md`
  3. 文件名包含工具 ID 或名称
  4. 路径包含工具 ID 或名称

**实现位置**: `src-tauri/src/wiki/commands.rs::find_wiki_for_tool()`

#### 前端集成
- **帮助按钮**: 工具卡片上的 "📚 Wiki" 按钮
- **内嵌显示**: 点击后在工具箱内使用 WebView 显示 Wiki
- **浏览器打开**: 也可以选择在浏览器中打开完整 Wiki

**实现位置**: `src/views/CategoryView.vue::openWiki()`

### 2.7 内嵌 WebView 和浏览器两种展示方式

#### 内嵌模式（WebView）
- **组件**: `WikiViewer.vue` 组件
- **显示方式**: 使用 `<iframe>` 嵌入 Wiki 页面
- **URL**: `http://127.0.0.1:8777/file/{path}?theme={theme}`
- **优势**: 无需离开工具箱，快速查看工具文档

#### 浏览器模式
- **完整功能**: 在浏览器中打开，支持完整的 Wiki 浏览体验
- **URL**: `http://127.0.0.1:8777`（首页）或 `http://127.0.0.1:8777/file/{path}`
- **优势**: 更大的显示区域，更好的阅读体验

**实现位置**: `src/components/WikiViewer.vue`

## 3. HTTP API 端点

### 3.1 路由列表
- `GET /` - Wiki 首页（显示文件列表）
- `GET /api/files` - 获取文件列表（JSON）
- `GET /api/render?path=xxx` - 渲染 Markdown 文件（JSON）
- `GET /api/tree` - 获取目录树结构（JSON）
- `GET /api/search?q=xxx` - 搜索 Wiki（JSON）
- `GET /api/themes` - 获取可用主题列表（JSON）
- `GET /file/*path` - 直接访问文件（HTML 页面）
- `GET /file/*path?theme=xxx` - 访问文件并指定主题

### 3.2 响应格式

#### 文件列表
```json
[
  {
    "path": "tools/tool1.md",
    "name": "tool1.md",
    "title": "工具1使用说明",
    "is_dir": false,
    "children": null
  },
  {
    "path": "tools",
    "name": "tools",
    "title": "tools",
    "is_dir": true,
    "children": [...]
  }
]
```

#### 渲染结果
```json
{
  "html": "<article>...</article>",
  "title": "工具1使用说明",
  "toc": [
    {
      "id": "工具1使用说明",
      "text": "工具1使用说明",
      "level": 1,
      "children": [...]
    }
  ]
}
```

## 4. 用户交互流程

### 4.1 查看工具 Wiki
1. 用户在工具卡片上点击 "📚 Wiki" 按钮
2. 系统检查工具的 `wikiUrl` 字段
3. 如果没有设置，尝试根据工具 ID/名称自动查找
4. 启动 Wiki 服务器（如果未启动）
5. 在内嵌 WebView 中显示 Wiki 页面

### 4.2 编辑 Wiki
1. 用户在 Typora 等编辑器中打开 `wiki/` 目录下的 Markdown 文件
2. 编辑并保存文件
3. Wiki 服务器检测到文件变化（通过重新读取）
4. 刷新页面即可看到更新

### 4.3 切换主题
1. 在 Wiki 页面侧边栏找到主题选择器
2. 选择主题（如 `dark`）
3. 页面自动刷新并应用新主题
4. 主题选择保存到 `localStorage`

## 5. 样式设计

### 5.1 GitHub 风格样式
- **字体**: 系统默认字体栈（-apple-system, BlinkMacSystemFont, 'Segoe UI' 等）
- **配色**: 
  - 浅色主题: 白色背景 `#ffffff`，深色文字 `#24292e`
  - 深色主题: 深色背景 `#0d1117`，浅色文字 `#c9d1d9`
- **布局**: 侧边栏 + 主内容区，响应式设计
- **代码块**: 深色背景，语法高亮

**实现位置**: `src-tauri/static/wiki_styles.css`

### 5.2 主题自定义
用户可以在 `wiki/theme/` 目录下创建自定义主题文件，例如：

```css
/* wiki/theme/my-theme.css */
:root {
  --wiki-primary-color: #ff6b6b;
  --wiki-bg-color: #f8f9fa;
  --wiki-text-color: #212529;
}

body {
  font-family: 'Microsoft YaHei', sans-serif;
}

.markdown-body {
  max-width: 900px;
}
```

## 6. 性能优化

### 6.1 文件缓存
- Wiki 文件列表在服务器启动时缓存
- 文件内容按需读取，不预加载

### 6.2 代码高亮
- 使用 `syntect` 进行高效语法高亮
- 高亮结果可以缓存（未来优化）

### 6.3 HTTP 服务器
- 使用 `axum` 异步框架，支持高并发
- 静态文件服务使用 `tower-http`

## 7. 错误处理

### 7.1 文件不存在
- 返回 404 错误页面
- 显示友好的错误提示

### 7.2 主题加载失败
- 降级到默认主题
- 不影响 Wiki 内容显示

### 7.3 服务器启动失败
- 检查端口是否被占用
- 显示错误信息给用户

## 8. 未来扩展

### 8.1 功能扩展
- [ ] 支持数学公式（LaTeX）
- [ ] 支持 Mermaid 图表
- [ ] 支持 PlantUML 图表
- [ ] 支持文件附件上传
- [ ] 支持 Wiki 页面之间的链接自动解析

### 8.2 性能优化
- [ ] 文件变化监听（使用 `notify` 库）
- [ ] 代码高亮结果缓存
- [ ] 静态资源 CDN 加速

### 8.3 用户体验
- [ ] 支持 Wiki 页面编辑（在线编辑器）
- [ ] 支持版本历史
- [ ] 支持全文搜索高亮
- [ ] 支持打印样式优化
