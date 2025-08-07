# Claude Code 供应商切换器

一个用于管理和切换 Claude Code 不同供应商配置的桌面应用。

## 功能特性

- 一键切换不同供应商（Anthropic、packycode 等）
- 支持添加自定义供应商
- 简洁美观的图形界面

## 下载安装

### Windows 用户

从 [Releases](../../releases) 页面下载：

- **安装版 (推荐)**: `CC-Switch-Setup-x.x.x.exe`
  - 完整系统集成，正确显示应用图标
  - 自动创建桌面快捷方式和开始菜单项
- **便携版**: `CC-Switch-Portable-x.x.x.exe`
  - 无需安装，直接运行
  - 适合需要绿色软件的用户

### 其他平台

- **macOS**: `CC-Switch-x.x.x-mac.zip`
- **Linux**: `CC-Switch-x.x.x.AppImage`

## 使用说明

1. 点击"添加供应商"添加你的 API 配置
2. 选择要使用的供应商，点击单选按钮切换
3. 配置会自动保存到 Claude Code 的配置文件中

## 开发

```bash
# 安装依赖
pnpm install
# 或
npm install

# 开发模式
pnpm run dev

# 构建应用
pnpm run build

# 打包发布
pnpm run dist
```

## 技术栈

- Electron
- React
- TypeScript
- Vite
- electron-store

## 项目结构

```
├── src/
│   ├── main/          # 主进程代码
│   ├── renderer/      # 渲染进程代码
│   └── shared/        # 共享类型和工具
├── build/             # 应用图标资源
└── dist/              # 构建输出目录
```

## License

MIT
