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

从 [Releases](../../releases) 页面下载：

- **安装版 (推荐)**: `CC-Switch-Setup-x.x.x.exe`
  - 完整系统集成，正确显示应用图标
  - 自动创建桌面快捷方式和开始菜单项
- **便携版**: `CC-Switch-Portable-x.x.x.exe`
  - 无需安装，直接运行
  - 适合需要绿色软件的用户

### macOS 用户

从 [Releases](../../releases) 页面下载：

- **通用版本（推荐）**: `CC Switch-x.x.x-mac.zip` - Intel 版本，兼容所有 Mac（包括 M 系列芯片）

#### macOS 安装说明

**推荐使用通用版本**，它通过 Rosetta 2 在 M 系列 Mac 上运行良好，兼容性最佳。

由于作者没有苹果开发者账号，应用使用 ad-hoc 签名（未经苹果官方认证），首次打开时可能出现"未知开发者"警告。这是正常的安全提示，处理方法：

**方法 1 - 系统设置**：

1. 双击应用时选择"取消"
2. 打开"系统设置" → "隐私与安全性"
3. 在底部找到被阻止的应用，点击"仍要打开"
4. 确认后即可正常使用

**方法 2 - 自行编译**：

1. Clone 代码到本地：`git clone https://github.com/farion1231/cc-switch.git`
2. 安装依赖：`pnpm install`
3. 编译代码：`pnpm run build`
4. 打包应用：`pnpm run dist`
5. 在项目 release 目录找到编译好的应用包

**安全保障**：

- 应用已通过 ad-hoc 代码签名，确保文件完整性
- 源代码完全开源，可在 GitHub 审查
- 本地存储配置，无网络传输风险

**技术说明**：

- 使用 Intel x64 架构，通过 Rosetta 2 在 M 系列芯片上运行
- 兼容性和稳定性最佳，性能损失 minimal
- 避免了 ARM64 原生版本的签名复杂性问题

### Linux 用户

- **AppImage**: `CC Switch-x.x.x.AppImage`

下载后添加执行权限：

```bash
chmod +x CC-Switch-x.x.x.AppImage
```

## 使用说明

1. 点击"添加供应商"添加你的 API 配置
2. 选择要使用的供应商，点击单选按钮切换
3. 配置会自动保存到 Claude Code 的配置文件中
4. 重启或者新打开 Claude Code 终端以生效

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
