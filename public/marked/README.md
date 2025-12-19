# Marked.js 本地文件说明

## 文件位置

请将 Marked.js 17.0.1 的构建文件放置在此目录下：

### 必需文件

1. **UMD 版本（推荐用于浏览器）**：
   - `marked.umd.js` 或 `marked.min.js`

2. **ES 模块版本（备选）**：
   - `marked.esm.js`

## 下载方式

### 方式 1：从 npm 包提取（推荐）

```bash
# 安装 Marked.js
npm install marked@17.0.1

# 复制构建文件到 public/marked/
# Windows PowerShell:
Copy-Item node_modules/marked/lib/marked.umd.js public/marked/
# 或者使用压缩版本（如果有）
Copy-Item node_modules/marked/lib/marked.umd.min.js public/marked/marked.umd.js
```

### 方式 2：从 CDN 下载

1. 访问：https://cdn.jsdelivr.net/npm/marked@17.0.1/lib/marked.umd.js
2. 保存为 `public/marked/marked.umd.js`

### 方式 3：从 GitHub Releases 下载

1. 访问：https://github.com/markedjs/marked/releases/tag/v17.0.1
2. 下载源代码包并构建，或从 npm 包中提取

## 目录结构

推荐结构：

```
public/marked/
├── marked.umd.js      # UMD 版本（浏览器用）
└── README.md          # 本说明文件
```

## 注意事项

- UMD 版本会在全局创建 `marked` 对象
- ES 模块版本需要支持 ES modules 的浏览器
- 确保文件路径正确，应用会从 `/marked/marked.umd.js` 加载
- 如果文件不存在，应用会尝试多个路径，但最终会失败并记录错误

