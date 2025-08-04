# Claude Code 供应商切换器

一个用于管理和切换 Claude Code 不同供应商配置的桌面应用。

## 功能特性

- 🔄 一键切换不同供应商（Anthropic、OpenRouter 等）
- 🔍 实时监控供应商状态和响应时间
- ⚡ 支持添加自定义供应商
- 🎨 简洁美观的图形界面
- 🔒 安全存储 API 密钥

## 开发

```bash
# 安装依赖
npm install

# 开发模式
npm run dev

# 构建应用
npm run build

# 打包发布
npm run dist
```

## 使用说明

1. 点击"添加供应商"添加你的 API 配置
2. 系统会自动检测每个供应商的状态
3. 选择要使用的供应商，点击单选按钮切换
4. 配置会自动保存到 Claude Code 的配置文件中

## 技术栈

- Electron
- React
- TypeScript
- Vite

## License

MIT