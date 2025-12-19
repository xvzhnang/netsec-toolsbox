# Markdown-it 本地文件说明

## 文件位置

请将打包后的 markdown-it 文件放置在此目录下：

### 必需文件

- `markdown-it.bundle.min.js` - 打包后的完整 bundle（包含所有插件）

## 构建方式

### 方式 1：使用 npm 脚本（推荐）

```bash
# 安装依赖
npm install

# 构建 markdown-it bundle
npm run build:markdown-it
```

### 方式 2：手动使用 esbuild

```bash
# 安装 esbuild（如果未安装）
npm install -D esbuild

# 构建
npx esbuild markdown-it.bundle.js --bundle --minify --format=iife --global-name=MarkdownItBundle --outfile=public/markdown-it/markdown-it.bundle.min.js
```

## 包含的插件

打包后的 bundle 包含以下插件：

1. **markdown-it** - 核心解析器
2. **markdown-it-anchor** - 锚点支持
3. **markdown-it-toc-done-right** - 目录生成
4. **markdown-it-task-lists** - 任务列表
5. **markdown-it-attrs** - 属性扩展
6. **markdown-it-footnote** - 脚注
7. **markdown-it-emoji** - Emoji
8. **markdown-it-katex** - 数学公式
9. **markdown-it-container** - 自定义容器（tip, info, warning, danger, note）
10. **markdown-it-mermaid** - Mermaid 图表
11. **highlight.js** - 代码高亮

## 使用方式

构建完成后，代码会自动从 `/markdown-it/markdown-it.bundle.min.js` 加载。

全局对象：
- `window.markdownit` - markdown-it 实例（已配置所有插件）
- `window.MarkdownIt` - MarkdownIt 构造函数

## 注意事项

- 确保在运行应用前已执行 `npm run build:markdown-it`
- 如果修改了 `markdown-it.bundle.js`，需要重新构建
- bundle 文件较大（包含所有插件），但可以离线使用

