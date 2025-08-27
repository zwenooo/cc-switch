# Claude Code 供应商切换器

[![Version](https://img.shields.io/badge/version-3.0.0-blue.svg)](https://github.com/jasonyoung/cc-switch/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/jasonyoung/cc-switch/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202.0-orange.svg)](https://tauri.app/)

一个用于管理和切换 Claude Code 不同供应商配置的桌面应用。

> **v3.0.0 重大更新**：从 Electron 完全迁移到 Tauri 2.0，应用体积减少 85%（从 ~80MB 降至 ~12MB），启动速度提升 10 倍！

## 功能特性

- **极速启动** - 基于 Tauri 2.0，原生性能，秒开应用
- 一键切换不同供应商
- 智谱 GLM、Qwen coder、DeepSeek v3.1、packycode 等预设供应商只需要填写 key 即可一键配置
- 支持添加自定义供应商
- 简洁美观的图形界面
- 信息存储在本地 ~/.cc-switch/config.json，无隐私风险
- 超小体积 - 仅 ~5MB 安装包

## 界面预览

### 主界面

![主界面](screenshots/main.png)

### 添加供应商

![添加供应商](screenshots/add.png)

## 下载安装

### 系统要求

- **Windows**: Windows 10 及以上
- **macOS**: macOS 10.15 (Catalina) 及以上
- **Linux**: Ubuntu 20.04+ / Debian 11+ / Fedora 34+ 等主流发行版

### Windows 用户

从 [Releases](../../releases) 页面下载最新版本的 `CC-Switch_3.0.0_x64.msi` 或 `.exe` 安装包。

### macOS 用户

从 [Releases](../../releases) 页面下载最新版本的 `CC-Switch_3.0.0_x64.dmg` (Intel) 或 `CC-Switch_3.0.0_aarch64.dmg` (Apple Silicon)。

### Linux 用户

从 [Releases](../../releases) 页面下载最新版本的 `.AppImage` 或 `.deb` 包。

## 使用说明

1. 点击"添加供应商"添加你的 API 配置
2. 选择要使用的供应商，点击单选按钮切换
3. 配置会自动保存到 Claude Code 的配置文件中
4. 重启或者新打开终端以生效

## 开发

### 环境要求

- Node.js 18+
- pnpm 8+
- Rust 1.75+
- Tauri CLI 2.0+

### 开发命令

```bash
# 安装依赖
pnpm install

# 开发模式（热重载）
pnpm dev

# 类型检查
pnpm typecheck

# 代码格式化
pnpm format

# 检查代码格式
pnpm format:check

# 构建应用
pnpm build

# 构建调试版本
pnpm tauri build --debug
```

### Rust 后端开发

```bash
cd src-tauri

# 格式化 Rust 代码
cargo fmt

# 运行 clippy 检查
cargo clippy

# 运行测试
cargo test
```

## 技术栈

- **[Tauri 2.0](https://tauri.app/)** - 跨平台桌面应用框架
- **[React 18](https://react.dev/)** - 用户界面库
- **[TypeScript](https://www.typescriptlang.org/)** - 类型安全的 JavaScript
- **[Vite](https://vitejs.dev/)** - 极速的前端构建工具
- **[Rust](https://www.rust-lang.org/)** - 系统级编程语言（后端）

## 项目结构

```
├── src/                   # 前端代码 (React + TypeScript)
│   ├── components/       # React 组件
│   ├── config/          # 预设供应商配置
│   ├── lib/             # Tauri API 封装
│   └── utils/           # 工具函数
├── src-tauri/            # 后端代码 (Rust)
│   ├── src/             # Rust 源代码
│   │   ├── commands.rs  # Tauri 命令定义
│   │   ├── config.rs    # 配置文件管理
│   │   ├── provider.rs  # 供应商管理逻辑
│   │   └── store.rs     # 状态管理
│   ├── capabilities/    # 权限配置
│   └── icons/           # 应用图标资源
└── screenshots/          # 界面截图
```

## 更新日志

查看 [CHANGELOG.md](CHANGELOG.md) 了解版本更新详情。

## 贡献

欢迎提交 Issue 和 Pull Request！

## License

MIT © Jason Young
