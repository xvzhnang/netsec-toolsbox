# Wiki Python 服务器设置说明

## 环境要求

- **Python 3.7+**（任何 Python 环境，不限于 conda）
- 必需的 Python 包（见 `requirements.txt`）

## 安装步骤

### 方式 1：使用系统 Python（推荐）

```bash
# 检查 Python 版本
python --version
# 或
python3 --version

# 安装依赖
pip install -r requirements.txt
# 或
pip3 install -r requirements.txt
```

### 方式 2：使用 conda 环境（可选）

```bash
# 激活 conda 环境（如果使用 conda）
conda activate python312

# 安装依赖
pip install -r requirements.txt
```

## 自动启动逻辑

Wiki 服务器现在支持**智能启动**：

1. **优先使用 Python 服务器**（如果 Python 可用且依赖完整）
2. **自动回退到 Rust 服务器**（如果 Python 不可用或缺少依赖）

**无需手动配置**，系统会自动选择最合适的启动方式。

或者手动安装：

```bash
pip install Flask>=3.0.0 Flask-CORS>=4.0.0 Markdown>=3.5.0 Pygments>=2.16.0
```

## 文件结构

```
netsec-toolbox/
├── wiki_server.py          # Python Wiki 服务器脚本
├── requirements.txt         # Python 依赖列表
├── wiki/                   # Wiki 内容目录
│   ├── index.md           # Wiki 首页
│   ├── theme/             # 主题目录
│   │   ├── default.css
│   │   └── dark.css
│   └── ...
└── src-tauri/
    └── static/
        ├── wiki_index.html # Wiki 首页模板
        └── wiki_styles.css # Wiki 样式文件
```

## 工作原理

1. **Rust 后端启动 Python 服务器**：
   - 当调用 `start_wiki_server` 命令时，Rust 代码会启动 Python 进程
   - 使用 `conda run -n python312 python wiki_server.py` 命令
   - Python 服务器监听 `127.0.0.1:8777` 端口

2. **Python 服务器功能**：
   - 提供 HTTP API 端点（与 Rust 版本兼容）
   - 使用 Flask 框架
   - 使用 `markdown` 库解析 Markdown
   - 使用 `Pygments` 进行代码高亮
   - 支持主题系统

3. **API 端点**：
   - `GET /` - Wiki 首页
   - `GET /api/files` - 文件列表
   - `GET /api/render?path=xxx` - 渲染 Markdown
   - `GET /api/tree` - 目录树
   - `GET /api/search?q=xxx` - 搜索
   - `GET /api/themes` - 主题列表
   - `GET /file/<path>` - 访问文件

## 手动测试

如果想手动测试 Python 服务器：

```bash
# 激活 conda 环境
conda activate python312

# 运行服务器
python wiki_server.py
```

服务器将在 `http://127.0.0.1:8777` 启动。

## 故障排除

### 问题1: conda 命令未找到

确保 conda 已正确安装并添加到 PATH。

### 问题2: Python 包导入错误

确保已安装所有依赖：
```bash
conda activate python312
pip install -r requirements.txt
```

### 问题3: 端口被占用

如果端口 8777 已被占用，可以修改 `wiki_server.py` 中的 `PORT` 变量。

### 问题4: Wiki 目录未找到

确保项目根目录下有 `wiki/` 文件夹，或者通过环境变量 `WIKI_DIR` 指定。

## 注意事项

- Python 服务器进程由 Rust 后端管理
- 服务器在后台运行，不会阻塞主程序
- 如果 Python 服务器崩溃，Rust 后端会检测到并更新状态
- 主题文件应放在 `wiki/theme/` 目录下

