# Wiki 使用指南

## 目录结构

将您的 Markdown 文件放在 `wiki` 目录下，支持文件夹嵌套：

```
wiki/
├── README.md          # Wiki 首页（可选）
├── tools/             # 工具相关文档
│   ├── tool1.md
│   └── tool2.md
├── guides/            # 使用指南
│   └── getting-started.md
└── theme/             # 主题文件夹
    ├── default.css    # 默认主题
    └── dark.css       # 深色主题（示例）
```

## 工具与 Wiki 关联

### 方法 1：在工具配置中设置 Wiki URL

在工具编辑界面，填写 `Wiki URL` 字段：

- **相对路径**：`tools/tool-name.md` 或 `/tools/tool-name.md`
- **完整 URL**：`http://127.0.0.1:8777/file/tools/tool-name.md`

### 方法 2：自动关联

如果工具未设置 Wiki URL，系统会根据工具 ID 或名称自动查找对应的 Wiki 文件：

- `tools/{tool-id}.md`
- `tools/{tool-name}.md`
- 文件名或路径包含工具 ID/名称的文件

## 自定义主题

### 创建主题

1. 在 `wiki/theme/` 目录下创建 CSS 文件，例如 `my-theme.css`
2. 在 Wiki 页面的主题选择器中选择您的主题

### 主题文件示例

参考 `wiki/theme/default.css`，您可以自定义：

- 颜色方案（主色调、背景色、文字颜色）
- 字体样式
- 布局间距
- 代码高亮样式
- 等等

### 使用主题

在 Wiki 页面右上角的主题选择器中选择您创建的主题。

## Markdown 功能支持

- ✅ GitHub 风格 Markdown
- ✅ 表格
- ✅ 代码块（带语法高亮）
- ✅ 任务列表
- ✅ 脚注
- ✅ 数学公式（如果支持）
- ✅ 目录自动生成

## 访问方式

### 浏览器访问

Wiki 服务器默认运行在 `http://127.0.0.1:8777`

- 首页：`http://127.0.0.1:8777`
- 查看文件：`http://127.0.0.1:8777/file/tools/tool-name.md`

### 内嵌查看

在工具箱中点击工具的 "📚 Wiki" 按钮，Wiki 内容会在工具箱内嵌显示。

## 搜索功能

在 Wiki 页面点击 "🔍 搜索" 按钮，输入关键词即可搜索所有 Markdown 文件内容。

## 目录导航

Wiki 页面左侧显示：

1. **文件导航**：所有 Wiki 文件的树形结构
2. **页面目录**：当前页面的标题目录（TOC）

点击即可快速跳转。

