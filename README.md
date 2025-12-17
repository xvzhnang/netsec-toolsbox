# Ne0XSec's Toolbox

一个基于 Tauri + Vue 3 构建的网络安全工具箱桌面应用，用于管理和快速启动各种安全工具。

## ✨ 功能特性

### 🎯 核心功能

- **分类管理**：支持多级分类和子分类，灵活组织工具
- **工具管理**：支持多种工具类型（GUI、CLI、JAR、Python、HTML、LNK、网页、其他）
- **智能搜索**：模糊搜索功能，支持多关键词匹配
- **图标系统**：支持三种图标来源
  - URL 远程图片（自动抓取 favicon）
  - 本地图片（自动裁剪压缩为 160x160）
  - 应用自身图标（自动从 EXE/LNK 文件提取）
- **视图切换**：支持网格视图和列表视图，用户可自由切换
- **数据持久化**：使用 JSON 配置文件存储，便于备份和迁移
- **自动图标提取**：输入路径后自动提取图标，支持 Windows EXE/LNK 文件
- **Wiki 文档系统**：完整的 Markdown 文档管理，支持 GitHub 风格渲染、代码高亮、主题自定义、搜索和目录导航

### 🛠️ 工具类型支持

#### GUI 工具
- 直接启动图形界面应用程序
- 支持自定义工作目录
- 支持命令行参数

#### CLI 工具
- 在可执行文件所在目录打开终端并执行
- 支持命令行参数
- 自动检测 PATH 环境变量中的可执行文件

#### JAR 工具（Java 应用）
- 支持自定义 Java 路径（可选，默认使用 PATH）
- 支持 JVM 参数配置（如 `-Xmx512m`、`-Dxxx=yyy`）
- 支持程序参数配置
- 在 JAR 文件所在目录打开终端执行

#### Python 脚本
- 在脚本所在目录打开终端执行
- 自动使用 `python` 或 `python3` 命令（Windows 使用 `python`，Unix 使用 `python3`）

#### HTML 工具（本地网页）
- 支持本地 HTML 文件
- 自动提取 HTML 文件中的 favicon
- 在浏览器中打开本地 HTML 文件

#### LNK 工具（Windows 快捷方式）
- 支持 Windows 快捷方式文件
- 自动提取快捷方式的目标图标
- 正确解析快捷方式路径（支持中文路径）

#### 网页工具（在线工具）
- 配置 URL 地址
- 自动抓取网页 favicon
- 点击后在浏览器新标签页打开

### 🎨 界面特性

- **现代化 UI**：深色主题，渐变背景
- **应用图标风格**：工具卡片采用类似应用图标的展示方式
- **响应式布局**：支持窗口大小调整，默认最大化启动
- **视图切换**：支持网格视图和列表视图切换
  - **网格视图**：卡片式布局，显示完整信息
  - **列表视图**：紧凑列表布局，仅显示图标、名称、描述和操作按钮
- **虚拟滚动**：大量工具时自动启用虚拟滚动优化性能（超过 50 个工具）
- **独立滚动区域**：分类、子分类、工具卡片、设置等区域独立滚动
- **图标缓存**：自动缓存提取的图标，提高加载速度

### 🔍 搜索功能

- **全局搜索**：首页和分类页都支持搜索
- **模糊匹配**：支持多关键词搜索（空格分隔）
- **搜索结果覆盖**：半透明遮罩显示搜索结果
- **搜索范围**：支持搜索分类、子分类和工具

### 🖼️ 图标功能

- **自动提取**：输入路径后自动提取图标（EXE/LNK/HTML/URL）
- **智能检测**：自动检测文件类型并选择合适的提取方式
- **路径变更检测**：路径改变时自动清除旧图标并重新提取
- **图标缓存**：提取的图标自动缓存到 `.config/icons/` 目录
- **统一尺寸**：所有图标统一调整为 160x160 像素显示

## 📦 技术栈

- **前端框架**：Vue 3 + TypeScript
- **构建工具**：Vite
- **桌面框架**：Tauri 2.9.5
- **后端语言**：Rust 1.77.2+
- **路由**：Vue Router 4
- **数据存储**：JSON 配置文件
- **图标处理**：PowerShell/.NET（Windows）、image crate（Rust）

## 🚀 快速开始

### 环境要求

- Node.js 18+
- Rust 1.77.2+
- npm 或 yarn
- Windows 10+（当前主要支持 Windows，其他平台部分功能可能受限）

### 安装依赖

```bash
# 安装前端依赖
npm install

# 安装 Rust 依赖（首次运行会自动安装）
cd src-tauri
cargo build
```

### 开发模式

```bash
npm run tauri dev
```

这将启动开发服务器，前端运行在 `http://localhost:5173`，Tauri 应用会自动启动。

### 构建生产版本

```bash
# 构建前端和 Tauri 应用
npm run tauri build
```

构建产物位于 `src-tauri/target/release/` 目录。

## 📁 项目结构

```
netsec-toolbox/
├── src/                    # 前端源代码
│   ├── components/         # Vue 组件
│   │   ├── AiAssistantPanel.vue    # AI 助手面板
│   │   ├── ConfirmDialog.vue       # 确认对话框
│   │   ├── ContextMenu.vue         # 右键菜单
│   │   ├── ErrorBoundary.vue       # 错误边界
│   │   ├── ModalDialog.vue         # 模态对话框
│   │   └── VirtualList.vue         # 虚拟滚动列表
│   ├── stores/             # 状态管理
│   │   └── categories.ts   # 分类和工具数据管理
│   ├── utils/              # 工具函数
│   │   ├── fileDialog.ts   # 文件选择对话框
│   │   ├── fileStorage.ts  # 文件存储（JSON）
│   │   ├── fileUpload.ts   # 文件上传（已废弃）
│   │   ├── imageProcessor.ts  # 图片处理（裁剪压缩、图标提取）
│   │   ├── storage.ts      # 存储工具
│   │   ├── tauri.ts        # Tauri API 工具
│   │   └── tauriDiagnostics.ts  # Tauri 诊断工具
│   ├── views/              # 页面视图
│   │   ├── DashboardView.vue      # 首页
│   │   ├── CategoryView.vue       # 分类页
│   │   ├── CategorySettingsView.vue  # 分类设置页
│   │   └── SettingsView.vue       # 设置页
│   ├── App.vue             # 根组件
│   ├── main.ts             # 入口文件
│   ├── router.ts           # 路由配置
│   └── style.css           # 全局样式
├── src-tauri/              # Tauri 后端
│   ├── src/
│   │   ├── lib.rs          # 主入口（模块声明和命令注册）
│   │   ├── main.rs         # 程序入口
│   │   ├── types.rs        # 数据类型定义
│   │   ├── utils.rs        # 工具函数（路径、哈希等）
│   │   ├── config.rs       # 配置管理（JSON 文件读写）
│   │   ├── launcher.rs    # 工具启动逻辑
│   │   ├── icon_extractor.rs  # 图标提取逻辑
│   │   └── file_ops.rs    # 文件操作（路径解析、文件对话框）
│   ├── Cargo.toml          # Rust 依赖配置
│   ├── tauri.conf.json     # Tauri 配置
│   └── icons/              # 应用图标
└── README.md               # 本文件
```

## 📝 配置说明

### 数据存储

所有配置数据存储在应用配置目录下的 `.config` 文件夹中，分为多个配置文件：

- **分类配置**: `.config/categories.json` - 一级分类配置（名称、图标、颜色等）
- **工具数据**: `.config/tools.json` - 子分类和工具数据
- **图标缓存**: `.config/icons/` - 自动提取的图标缓存（PNG 格式）
- **AI 配置**: `.config/ai.json` - AI 相关配置（未来使用）

配置文件分离的好处：
- 分类配置和工具数据独立管理
- 便于版本控制和备份
- 减少单个文件过大
- 便于未来扩展（如 AI 配置）

### 配置文件格式

```json
{
  "categories": [
    {
      "id": "web",
      "name": "WEB",
      "label": "Web 攻击与防御",
      "description": "Web 相关攻击与防御工具集合。",
      "icon": "globe",
      "color": "#4DA3FF",
      "order": 1,
      "enabled": true
    }
  ]
}
```

```json
{
  "data": [
    {
      "id": "web",
      "name": "WEB",
      "sub_categories": [
        {
          "id": "web-info",
          "name": "信息收集",
          "tools": [
            {
              "id": "host-info",
              "name": "主机信息探测",
              "description": "对域名/IP 进行 whois、地理位置、ASN 等查询。",
              "iconUrl": "data:image/png;base64,...",
              "toolType": "CLI",
              "execPath": "C:\\Tools\\whois.exe",
              "args": ["-d", "example.com"]
            }
          ]
        }
      ]
    }
  ]
}
```

## 🎯 使用指南

### 添加分类

1. 进入设置页面
2. 点击"添加分类"
3. 填写分类信息（名称、描述、图标、颜色等）
4. 保存

### 添加工具

1. 进入分类页面
2. 选择或创建子分类
3. 点击"新增工具"
4. 填写工具信息：
   - **名称**：工具名称
   - **图标**：自动提取或手动设置
   - **工具类型**：选择 GUI、CLI、JAR、Python、HTML、LNK、网页或其他
   - **可执行路径/URL**：根据工具类型填写
   - **参数**：命令行参数（空格分隔）
   - **描述**：工具用途说明
5. 保存

### 视图切换

- 在分类页面，点击工具头部的"网格"或"列表"按钮
- **网格视图**：适合浏览，显示完整信息
- **列表视图**：适合快速查找，紧凑显示

### JAR 工具配置

对于 Java JAR 类型工具，需要配置：

- **JAR 路径**：选择 JAR 文件
- **Java 路径**（可选）：留空则使用系统 PATH 中的 Java
- **JVM 参数**：如 `-Xmx512m -Dfile.encoding=UTF-8`
- **程序参数**：传递给 Java 程序的参数

### 图标设置

#### 自动提取图标（推荐）

- 输入可执行文件路径后，系统会自动提取图标
- 支持 EXE、LNK、HTML 文件
- 支持 URL（自动抓取 favicon）
- 路径改变时自动清除旧图标并重新提取

#### 手动设置图标

- **URL 图标**：输入图片 URL 地址（支持 http/https）
- **本地图片**：选择本地图片文件，自动裁剪为 160x160 并压缩

### 搜索工具

- 在首页或分类页的搜索框输入关键词
- 支持多关键词搜索（空格分隔）
- 搜索结果会覆盖显示在内容区域上方

## 🔧 技术细节

### 后端架构

后端采用模块化设计，分为以下模块：

- **types.rs**：所有数据结构定义
- **utils.rs**：工具函数（路径处理、哈希计算等）
- **config.rs**：配置文件管理
- **launcher.rs**：工具启动逻辑（支持多种工具类型）
- **icon_extractor.rs**：图标提取（Windows EXE/LNK、HTML、URL favicon）
- **file_ops.rs**：文件操作（路径解析、文件对话框）

### 图标提取机制

#### Windows EXE/LNK 图标提取

- 使用 PowerShell/.NET 的 `System.Drawing.Icon::ExtractAssociatedIcon`
- 通过环境变量传递路径，避免编码问题
- 支持中文路径
- 自动处理 Windows 长路径前缀（`\\?\`）

#### HTML 图标提取

- 解析 HTML 文件，查找 `<link rel="icon">` 或 `<link rel="shortcut icon">`
- 支持相对路径和绝对路径
- 自动下载并缓存图标

#### URL Favicon 抓取

- 尝试多个常见 favicon 路径（`/favicon.ico`、`/favicon.png`、`/apple-touch-icon.png`）
- 自动下载并缓存
- 转换为 base64 格式存储

### 路径解析

- 支持绝对路径和相对路径
- 自动在 PATH 环境变量中查找可执行文件
- 处理 Windows 长路径前缀
- 路径规范化（canonicalize）

### 窗口配置

- 默认最大化启动（`maximized: true`）
- 支持窗口大小调整（`resizable: true`）
- 用户可自由调整窗口大小

## 📄 许可证

本项目为私有项目。

## 👤 作者

By 序章

## 🙏 致谢

感谢所有贡献者和使用者的支持！

---

**版本**：0.1.0  
**最后更新**：2025-12-17
