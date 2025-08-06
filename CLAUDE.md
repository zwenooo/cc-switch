# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

这是一个 Electron + React + TypeScript 的桌面应用，用于管理和切换 Claude Code 的不同供应商配置（Anthropic、OpenRouter 等）。

## 开发命令

```bash
# 安装依赖
pnpm install

# 开发模式（同时启动 Vite 开发服务器和 Electron 主进程监听）
pnpm run dev

# 仅构建主进程
pnpm run build:main

# 仅构建渲染进程
pnpm run build:renderer

# 完整构建
pnpm run build

# 启动应用（需要先构建）
pnpm start
# 或
electron .

# 打包发布
pnpm run dist
```

## 代码架构

### 三层架构设计

1. **主进程 (src/main/)**
   - `index.ts`: Electron 主进程入口，处理窗口创建和 IPC 通信
   - `services.ts`: 核心业务逻辑，包含供应商状态检查和 Claude Code 配置文件操作
   - `preload.ts`: 预加载脚本，提供安全的 IPC API 接口

2. **渲染进程 (src/renderer/)**
   - `App.tsx`: 主应用组件，状态管理和业务逻辑协调
   - `components/`: React 组件
     - `ProviderList.tsx`: 供应商列表显示和操作
     - `AddProviderModal.tsx`: 添加供应商的模态框
     - `EditProviderModal.tsx`: 编辑供应商的模态框

3. **共享类型 (src/shared/types.ts)**
   - 定义主进程和渲染进程间的数据结构
   - 声明 `window.electronAPI` 接口用于类型安全的 IPC 调用

### 数据流

- 使用 `electron-store` 在主进程中持久化供应商配置
- 通过 IPC 在主进程和渲染进程间通信
- 供应商状态通过 HTTP 请求实时检测
- 切换供应商时直接修改 `~/.claude/settings.json` 文件

### 关键文件操作

- Claude Code 配置文件路径: `~/.claude/settings.json`
- 配置更新通过 `services.ts` 中的 `switchProvider()` 函数完成
- 应用配置存储在 `electron-store` 默认位置

## 构建配置

- 主进程使用 `tsconfig.main.json` 配置
- 渲染进程使用 `tsconfig.json` + Vite 构建
- 开发时渲染进程运行在 `localhost:3000`
- 生产时渲染进程文件位于 `dist/renderer/`

## 依赖管理

项目使用 npm 管理依赖，但检测到 `pnpm-lock.yaml`，建议使用 pnpm：

```bash
# 安装依赖
pnpm install

# 或使用 npm
npm install
```

## 核心 IPC 接口

主要的 IPC 接口定义在 `src/shared/types.ts` 中的 `window.electronAPI`：

- `getProviders()`: 获取所有供应商配置
- `switchProvider(providerId)`: 切换当前供应商
- `addProvider(provider)`: 添加新供应商
- `updateProvider(provider)`: 更新供应商配置
- `deleteProvider(id)`: 删除供应商
- `getClaudeCodeConfigPath()`: 获取 Claude Code 配置路径
- `selectConfigFile()`: 打开文件选择对话框

## 重要说明

- 供应商配置存储在 `electron-store` 中，自动持久化
- 切换供应商会直接修改 `~/.claude/settings.json` 中的 `ANTHROPIC_AUTH_TOKEN` 和 `ANTHROPIC_BASE_URL`
- 应用支持多平台打包（Windows NSIS、macOS DMG、Linux AppImage）