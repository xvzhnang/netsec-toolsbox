# Markdown 功能实现检查清单

## ⚠️ 重要说明

**当前项目使用的是 `marked.js`，而不是 `markdown-it`。**

你列出的插件都是 `markdown-it` 的插件，不能直接用于 `marked.js`。

## 当前实现状态（基于 marked.js）

### ✅ 已实现的功能

1. **基础渲染（marked.js 原生）**
   - ✅ H1–H6 标题（已实现，支持自动锚点 ID）
   - ✅ 普通段落、换行（已实现）
   - ✅ 加粗、斜体（已实现）
   - ✅ 行内代码（已实现）
   - ✅ 无序列表、有序列表、多级嵌套列表（已实现）
   - ✅ 普通引用块（已实现）
   - ✅ 水平分割线（已实现）
   - ✅ 行内链接与图片（已实现）
   - ✅ 带语言标识的代码块（已实现）

2. **锚点与目录**
   - ✅ 所有标题支持自动锚点（已实现，自定义 renderer）
   - ✅ TOC 生成（已实现，`extractTOC` 函数）

3. **删除线与 GFM 风格**
   - ✅ 删除线语法（已实现，marked.js 的 `gfm: true` 支持）

4. **任务列表**
   - ✅ 任务清单（已实现，marked.js 的 `gfm: true` 支持，自定义 renderer）

5. **表格**
   - ✅ Markdown 表格（已实现，marked.js 的 `gfm: true` 支持）
   - ✅ 对齐方式（左对齐、居中、右对齐）- marked.js 原生支持

6. **代码高亮**
   - ✅ highlight.js（已实现，从 public 目录动态加载）
   - ✅ 多语言代码块（bash、python、c、cpp、rust、sql 等）

7. **脚注**
   - ✅ 脚注支持（已实现，自定义预处理）

8. **Emoji**
   - ✅ Emoji 表情（已实现，自定义映射）

9. **数学公式**
   - ✅ 行内 LaTeX（已实现，自定义处理 + KaTeX）
   - ✅ 块级 LaTeX（已实现，自定义处理 + KaTeX）

10. **HTML 内联**
    - ✅ HTML 标签支持（已实现，`sanitize: false`）
    - ✅ `<kbd>`、`<details>`、`<summary>` 等（已实现）

11. **可折叠内容**
    - ✅ `<details>` / `<summary>`（已实现，自定义处理）

12. **自定义容器**
    - ✅ tip / info / warning / danger 容器（已实现，自定义处理 `> [!NOTE]` 格式）

13. **流程图与关系图**
    - ✅ Mermaid 图表（已实现，自定义处理）

14. **复杂嵌套结构**
    - ✅ 列表、表格、引用、代码块的嵌套组合（已实现，marked.js 原生支持）

## ❌ 未实现的功能（markdown-it 插件）

以下功能需要切换到 `markdown-it` 才能使用对应的插件：

1. **markdown-it-attrs** - 为图片或区块添加自定义属性（如 class、id）
   - 当前状态：marked.js 不支持此功能
   - 解决方案：切换到 markdown-it 或使用 HTML 标签

## 🔄 如果要切换到 markdown-it

如果你需要 `markdown-it-attrs` 或其他 markdown-it 插件，需要：

1. **下载 markdown-it 及其插件**（放置到 `public/markdown-it/` 目录）：
   - `markdown-it` (核心库)
   - `markdown-it-anchor` (锚点)
   - `markdown-it-toc-done-right` (目录)
   - `markdown-it-task-lists` (任务列表)
   - `markdown-it-multimd-table` (表格)
   - `markdown-it-attrs` (属性扩展)
   - `markdown-it-footnote` (脚注)
   - `markdown-it-emoji` (Emoji)
   - `markdown-it-katex` (数学公式)
   - `markdown-it-container` (自定义容器)
   - `markdown-it-mermaid` (Mermaid)

2. **修改代码**：将 `src/utils/markdown.ts` 从 marked.js 切换到 markdown-it

## 📦 下载链接（如果要切换到 markdown-it）

### 核心库
- **markdown-it**: https://github.com/markdown-it/markdown-it/releases
- **CDN**: https://cdn.jsdelivr.net/npm/markdown-it/dist/markdown-it.min.js

### 插件
- **markdown-it-anchor**: https://github.com/valeriangalliat/markdown-it-anchor/releases
- **markdown-it-toc-done-right**: https://github.com/nagaozen/markdown-it-toc-done-right/releases
- **markdown-it-task-lists**: https://github.com/revin/markdown-it-task-lists/releases
- **markdown-it-multimd-table**: https://github.com/redbug312/markdown-it-multimd-table/releases
- **markdown-it-attrs**: https://github.com/arve0/markdown-it-attrs/releases
- **markdown-it-footnote**: https://github.com/markdown-it/markdown-it-footnote/releases
- **markdown-it-emoji**: https://github.com/markdown-it/markdown-it-emoji/releases
- **markdown-it-katex**: https://github.com/waylonflinn/markdown-it-katex/releases
- **markdown-it-container**: https://github.com/markdown-it/markdown-it-container/releases
- **markdown-it-mermaid**: https://github.com/tylingsoft/markdown-it-mermaid/releases

### 从 npm 提取（推荐）

```bash
# 安装所有依赖
npm install markdown-it markdown-it-anchor markdown-it-toc-done-right markdown-it-task-lists markdown-it-multimd-table markdown-it-attrs markdown-it-footnote markdown-it-emoji markdown-it-katex markdown-it-container markdown-it-mermaid

# 复制到 public/markdown-it/
# Windows PowerShell:
New-Item -ItemType Directory -Path public/markdown-it -Force
Copy-Item node_modules/markdown-it/dist/markdown-it.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-anchor/dist/markdown-it-anchor.min.js public/markdown-it/
Copy-Item node_modules/markdown-it-toc-done-right/dist/markdown-it-toc-done-right.min.js public/markdown-it/
# ... 其他插件类似
```

## 💡 建议

**当前实现已经支持你列出的几乎所有功能**（除了 `markdown-it-attrs`）。

如果你只需要 `markdown-it-attrs` 功能，可以考虑：
1. 继续使用 marked.js，通过 HTML 标签添加属性
2. 或者切换到 markdown-it（需要重写渲染逻辑）

请告诉我你的选择，我可以帮你实现。

