# 构建说明

## 构建为 EXE 程序

### 前置要求

1. **Node.js 18+** 和 **npm**
2. **Rust 1.77.2+**
3. **Windows 10+**（当前主要支持 Windows）

### 构建步骤

```bash
# 1. 安装前端依赖
npm install

# 2. 构建前端（Vue 应用）
npm run build

# 3. 构建 Tauri 应用（生成 EXE）
npm run tauri build
```

### 构建产物位置

构建完成后，EXE 文件位于：
```
src-tauri/target/release/netsec-toolbox.exe
```

完整的发布包位于：
```
src-tauri/target/release/bundle/
```

## 配置文件位置

### 开发环境

配置文件位于项目根目录下的 `.config` 文件夹：
```
项目根目录/
├── .config/
│   ├── categories.json      # 分类和工具配置
│   ├── icons/               # 图标缓存
│   └── uploads/             # 上传文件
├── wiki/                    # Wiki 文档
└── ai_service/              # AI 服务配置
```

### 发布环境（EXE）

**配置文件应放在 EXE 文件所在目录的 `.config` 文件夹中：**

```
应用程序目录/
├── netsec-toolbox.exe       # 主程序
├── .config/                 # 配置文件目录（需要手动创建）
│   ├── categories.json      # 分类和工具配置
│   ├── icons/               # 图标缓存
│   └── uploads/             # 上传文件
├── wiki/                    # Wiki 文档目录（可选）
└── ai_service/              # AI 服务目录（可选）
    ├── main_gateway.py
    └── config/
        └── models.json
```

### 配置文件说明

#### 1. `.config/categories.json`
- **作用**：存储所有分类、子分类和工具的配置
- **格式**：JSON 格式
- **位置**：EXE 所在目录的 `.config` 文件夹

#### 2. `.config/icons/`
- **作用**：缓存提取的图标文件
- **自动创建**：应用首次提取图标时自动创建

#### 3. `.config/uploads/`
- **作用**：存储上传的文件
- **自动创建**：应用首次上传文件时自动创建

#### 4. `wiki/`
- **作用**：Wiki 文档目录
- **结构**：
  ```
  wiki/
  ├── tools/          # 工具文档
  ├── notes/          # 笔记
  ├── labs/           # 实验记录
  └── themes/         # 主题文件
  ```

#### 5. `ai_service/`
- **作用**：AI Gateway 服务相关文件
- **必需文件**：
  - `main_gateway.py`：AI Gateway 主程序
  - `config/models.json`：AI 模型配置

### 首次运行配置

1. **创建配置目录**：
   - 在 EXE 文件所在目录创建 `.config` 文件夹
   - 应用首次运行时会自动创建，但建议提前创建

2. **准备配置文件**：
   - 从开发环境复制 `categories.json` 到 `.config/` 目录
   - 或让应用首次运行时自动创建默认配置

3. **准备 AI 服务**（如果使用 AI 功能）：
   - 复制 `ai_service/` 目录到 EXE 所在目录
   - 确保 `python313/python.exe` 存在（或修改代码中的 Python 路径）

4. **准备 Wiki 文档**（可选）：
   - 复制 `wiki/` 目录到 EXE 所在目录

### 配置文件路径逻辑

应用使用以下逻辑确定配置文件位置：

1. **开发环境**：
   - 查找 `src-tauri` 目录
   - 使用项目根目录作为基础目录
   - 配置文件在：`项目根目录/.config/`

2. **发布环境**：
   - 找不到 `src-tauri` 目录时
   - 使用 EXE 文件所在目录作为基础目录
   - 配置文件在：`EXE所在目录/.config/`

### 注意事项

1. **相对路径**：所有配置文件使用相对路径，确保整个应用目录可以移动
2. **自动创建**：应用会自动创建必要的目录结构
3. **权限问题**：确保应用对配置目录有读写权限
4. **中文路径**：支持中文路径，但建议使用英文路径避免潜在问题

## 打包发布

### 方式 1：使用 Tauri Bundle（推荐）

Tauri 会自动创建安装包：
```
src-tauri/target/release/bundle/
├── msi/              # Windows 安装包
└── nsis/             # NSIS 安装包
```

### 方式 2：手动打包

1. 复制以下文件到目标目录：
   - `netsec-toolbox.exe`
   - `.config/` 目录（包含默认配置）
   - `wiki/` 目录（可选）
   - `ai_service/` 目录（如果使用 AI 功能）
   - `python313/` 目录（如果使用 AI 功能）

2. 创建启动脚本（可选）：
   ```batch
   @echo off
   cd /d %~dp0
   start netsec-toolbox.exe
   ```

## 构建优化

### 减小体积

1. **使用 release 模式**：
   ```bash
   npm run tauri build -- --release
   ```

2. **启用压缩**：
   - 前端代码已自动压缩
   - Rust 代码使用 release 模式优化

### 调试版本

如果需要调试信息：
```bash
npm run tauri build -- --debug
```

调试版本位于：`src-tauri/target/debug/`

## 常见问题

### Q: 构建失败，提示找不到依赖？
A: 确保已安装所有依赖：
```bash
npm install
cd src-tauri
cargo build
```

### Q: EXE 运行时找不到配置文件？
A: 确保 `.config` 目录在 EXE 文件所在目录，且应用有读写权限。

### Q: AI 功能无法启动？
A: 检查：
1. `ai_service/` 目录是否存在
2. `python313/python.exe` 是否存在
3. `ai_service/config/models.json` 配置是否正确

### Q: 如何迁移配置？
A: 复制整个 `.config` 目录到新位置即可。

