# KaTeX 本地文件说明

## 文件位置

请将 KaTeX 0.16.9 的文件放置在此目录下：

### 必需文件

1. **JavaScript 文件**：
   - `katex.min.js`（根目录）

2. **CSS 文件**：
   - `katex.min.css`（根目录）

3. **字体文件**：
   - `fonts/` 目录下的所有字体文件

## 下载方式

### 方式 1：从 GitHub Releases 下载（推荐）

1. 访问：https://github.com/KaTeX/KaTeX/releases/tag/v0.16.9
2. 下载 `katex-v0.16.9.zip` 或 `katex-v0.16.9.tar.gz`
3. 解压后将以下内容复制到 `public/katex/`：
   - `katex.min.js`
   - `katex.min.css`
   - `fonts/` 目录（包含所有字体文件）

**注意**：GitHub Releases 版本的文件直接在根目录，**没有 `dist` 目录**。

### 方式 2：从 npm 包提取

```bash
# 安装 KaTeX
npm install katex@0.16.9

# 复制文件到 public/katex/
# Windows PowerShell:
Copy-Item node_modules/katex/dist/* public/katex/dist/ -Recurse
Copy-Item node_modules/katex/katex.min.js public/katex/
Copy-Item node_modules/katex/katex.min.css public/katex/
```

**注意**：npm 包的文件在 `dist/` 目录下，需要创建 `dist` 目录。

## 目录结构

### GitHub Releases 版本（推荐）：

```
public/katex/
├── katex.min.js
├── katex.min.css
└── fonts/
    ├── KaTeX_AMS-Regular.woff2
    ├── KaTeX_AMS-Regular.woff
    ├── KaTeX_AMS-Regular.ttf
    └── ... (其他字体文件)
```

### npm 包版本：

```
public/katex/
├── katex.min.js
├── katex.min.css
└── dist/
    ├── katex.min.js
    ├── katex.min.css
    └── fonts/
        ├── KaTeX_AMS-Regular.woff2
        └── ... (其他字体文件)
```

## 字体路径修复

如果从 GitHub Releases 下载，CSS 文件中的字体路径可能指向 `fonts/` 而不是 `dist/fonts/`，这通常是正确的。

如果字体无法加载，可能需要检查 `katex.min.css` 中的字体路径，确保指向 `fonts/` 目录。

## 注意事项

- GitHub Releases 版本的文件直接在根目录，**没有 `dist` 目录**
- npm 包版本的文件在 `dist/` 目录下
- 应用会优先尝试根目录的文件，然后尝试 `dist/` 目录
- 确保所有字体文件都在 `fonts/` 或 `dist/fonts/` 目录下
- 如果文件不存在，应用会尝试多个路径，但最终会失败并记录错误

