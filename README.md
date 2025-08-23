# Claude Code 供应商切换器

一个用于管理和切换 Claude Code 不同供应商配置的桌面应用。

## 功能特性

- 一键切换不同供应商
- 智谱 GLM、Qwen coder、DeepSeek v3.1、packycode 等预设供应商只需要填写 key 即可一键配置
- 支持添加自定义供应商
- 简洁美观的图形界面
- 信息存储在本地 ~/.cc-switch/config.json，无隐私风险

## 界面预览

### 主界面

![主界面](screenshots/main.png)

### 添加供应商

![添加供应商](screenshots/add.png)

## 下载安装

### Windows 用户

从 [Releases](../../releases) 页面下载最新版本的 Windows 安装包。

### macOS 用户

从 [Releases](../../releases) 页面下载最新版本的 macOS 应用包。

### Linux 用户

从 [Releases](../../releases) 页面下载最新版本的 Linux 应用。

## 使用说明

1. 点击"添加供应商"添加你的 API 配置
2. 选择要使用的供应商，点击单选按钮切换
3. 配置会自动保存到 Claude Code 的配置文件中
4. 重启或者新打开终端以生效

## 开发

```bash
# 安装依赖
pnpm install

# 开发模式
pnpm run dev

# 构建应用
pnpm run build
```

## 技术栈

- Tauri 2.0
- React
- TypeScript
- Vite
- Rust

## 项目结构

```
├── src/                   # 前端代码 (React)
│   ├── components/       # React 组件
│   ├── config/          # 配置文件
│   ├── lib/             # 工具库
│   └── utils/           # 工具函数
├── src-tauri/             # Tauri 后端代码 (Rust)
│   ├── src/             # Rust 源代码
│   └── icons/           # 应用图标资源
└── screenshots/           # 截图资源
```

## License

MIT
