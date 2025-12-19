# Markdown 功能实现检查清单

## ⚠️ 重要说明

**当前项目使用的是 `marked.js`，而不是 `markdown-it`。**

你列出的插件都是 `markdown-it` 的插件，不能直接用于 `marked.js`。

## 功能实现对比

| 功能 | markdown-it 插件 | 当前实现（marked.js） | 状态 |
|------|-----------------|---------------------|------|
| **基础渲染** | markdown-it 原生 | marked.js 原生 | ✅ 已实现 |
| **锚点与目录** | markdown-it-anchor<br>markdown-it-toc-done-right | 自定义 renderer + extractTOC | ✅ 已实现 |
| **删除线** | markdown-it 原生（GFM） | marked.js `gfm: true` | ✅ 已实现 |
| **任务列表** | markdown-it-task-lists | marked.js `gfm: true` + 自定义 renderer | ✅ 已实现 |
| **表格** | markdown-it-multimd-table | marked.js `gfm: true` | ✅ 已实现 |
| **属性扩展** | markdown-it-attrs | ❌ 不支持 | ❌ 未实现 |
| **代码高亮** | highlight.js | highlight.js（本地加载） | ✅ 已实现 |
| **脚注** | markdown-it-footnote | 自定义预处理 | ✅ 已实现 |
| **Emoji** | markdown-it-emoji | 自定义映射 | ✅ 已实现 |
| **数学公式** | markdown-it-katex | 自定义处理 + KaTeX | ✅ 已实现 |
| **HTML 内联** | markdown-it `html: true` | marked.js `sanitize: false` | ✅ 已实现 |
| **可折叠内容** | HTML `<details>` | 自定义处理 | ✅ 已实现 |
| **自定义容器** | markdown-it-container | 自定义处理 `> [!NOTE]` | ✅ 已实现 |
| **Mermaid** | markdown-it-mermaid | 自定义处理 + Mermaid | ✅ 已实现 |

## 结论

**当前实现已经支持你列出的几乎所有功能**（除了 `markdown-it-attrs`）。

## 如果要切换到 markdown-it

如果你需要 `markdown-it-attrs` 或其他 markdown-it 插件，需要下载以下文件：

### 必需文件（放置到 `public/markdown-it/` 目录）

1. **核心库**
   - `markdown-it.min.js` - 核心解析器

2. **插件（按需下载）**
   - `markdown-it-anchor.min.js` - 锚点支持
   - `markdown-it-toc-done-right.min.js` - 目录生成
   - `markdown-it-task-lists.min.js` - 任务列表
   - `markdown-it-multimd-table.min.js` - 表格
   - `markdown-it-attrs.min.js` - 属性扩展 ⭐（唯一缺失的功能）
   - `markdown-it-footnote.min.js` - 脚注
   - `markdown-it-emoji.min.js` - Emoji
   - `markdown-it-katex.min.js` - 数学公式
   - `markdown-it-container.min.js` - 自定义容器
   - `markdown-it-mermaid.min.js` - Mermaid

### 下载方式

#### 方式 1：从 npm 提取（推荐）

```bash
# 安装所有依赖
npm install markdown-it markdown-it-anchor markdown-it-toc-done-right markdown-it-task-lists markdown-it-multimd-table markdown-it-attrs markdown-it-footnote markdown-it-emoji markdown-it-katex markdown-it-container markdown-it-mermaid

# 创建目录
mkdir -p public/markdown-it

# 复制文件（Windows PowerShell）
Copy-Item node_modules/markdown-it/dist/markdown-it.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-anchor/dist/markdown-it-anchor.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-toc-done-right/dist/markdown-it-toc-done-right.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-task-lists/dist/markdown-it-task-lists.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-multimd-table/dist/markdown-it-multimd-table.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-attrs/dist/markdown-it-attrs.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-footnote/dist/markdown-it-footnote.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-emoji/dist/markdown-it-emoji.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-katex/dist/markdown-it-katex.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-container/dist/markdown-it-container.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-mermaid/dist/markdown-it-mermaid.min.js public/markdown-it/
```

#### 方式 2：从 CDN 下载

访问以下链接并保存到 `public/markdown-it/` 目录：

- https://cdn.jsdelivr.net/npm/markdown-it/dist/markdown-it.min.js
- https://cdn.jsdelivr.net/npm/markdown-it-anchor/dist/markdown-it-anchor.min.js
- https://cdn.jsdelivr.net/npm/markdown-it-toc-done-right/dist/markdown-it-toc-done-right.min.js
- https://cdn.jsdelivr.net/npm/markdown-it-task-lists/dist/markdown-it-task-lists.min.js
- https://cdn.jsdelivr.net/npm/markdown-it-multimd-table/dist/markdown-it-multimd-table.min.js
- https://cdn.jsdelivr.net/npm/markdown-it-attrs/dist/markdown-it-attrs.min.js
- https://cdn.jsdelivr.net/npm/markdown-it-footnote/dist/markdown-it-footnote.min.js
- https://cdn.jsdelivr.net/npm/markdown-it-emoji/dist/markdown-it-emoji.min.js
- https://cdn.jsdelivr.net/npm/markdown-it-katex/dist/markdown-it-katex.min.js
- https://cdn.jsdelivr.net/npm/markdown-it-container/dist/markdown-it-container.min.js
- https://cdn.jsdelivr.net/npm/markdown-it-mermaid/dist/markdown-it-mermaid.min.js

#### 方式 3：从 GitHub Releases 下载

访问各插件的 GitHub 仓库 Releases 页面下载构建文件。

## 建议

**当前实现已经支持你列出的几乎所有功能**（除了 `markdown-it-attrs`）。

如果你只需要 `markdown-it-attrs` 功能，可以考虑：
1. **继续使用 marked.js**，通过 HTML 标签添加属性（如 `<img class="custom" id="my-img" src="...">`）
2. **切换到 markdown-it**（需要重写渲染逻辑，工作量较大）

请告诉我你的选择，我可以帮你实现。

