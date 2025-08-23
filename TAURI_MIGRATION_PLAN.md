# Tauri 重构计划

## 项目概述

将 CC Switch 从 Electron 框架迁移到 Tauri，以大幅减少应用体积并提升性能。

### 目标收益

- **体积优化**: 77MB → ~8MB (减少 90%)
- **内存占用**: 减少 60-70%
- **启动速度**: 提升 3-5 倍
- **安全性**: Rust 内存安全 + 细粒度权限控制

## 技术栈对比

| 技术层   | Electron (当前)      | Tauri (目标)              |
| -------- | -------------------- | ------------------------- |
| 后端     | Node.js + TypeScript | Rust                      |
| 前端     | React + TypeScript   | React + TypeScript (不变) |
| IPC      | Electron IPC         | Tauri Commands            |
| 文件操作 | Node.js fs           | Rust std::fs              |
| 配置存储 | electron-store       | tauri-plugin-store        |
| 打包     | electron-builder     | Tauri CLI                 |
| WebView  | Chromium (内置)      | 系统 WebView              |

## 迁移计划

### Phase 1: 环境准备 (Day 1 上午)

- [x] 安装 Rust 开发环境
  ```bash
  # Windows: 下载 rustup-init.exe
  # https://www.rust-lang.org/tools/install
  ```
- [x] 安装 Tauri CLI
  ```bash
  pnpm add -g @tauri-apps/cli
  ```
- [x] 在现有项目中集成 Tauri

  ```bash
  # 安装 Tauri CLI 作为开发依赖
  pnpm add -D @tauri-apps/cli

  # 在现有项目中初始化 Tauri
  pnpm tauri init

  # 安装 Tauri API 包
  pnpm add @tauri-apps/api
  ```

### Phase 2: 项目结构调整 (Day 1 下午)

- [x] 创建 Tauri 项目配置
  - `src-tauri/` - Rust 后端代码
  - `src-tauri/tauri.conf.json` - Tauri 配置
  - `src-tauri/Cargo.toml` - Rust 依赖管理
- [x] 迁移前端构建配置
  - 调整 Vite 配置适配 Tauri ✅
  - 更新 package.json scripts ✅
- [x] 配置应用图标和元数据

### Phase 3: 后端功能迁移 (Day 2)

#### 3.1 核心功能模块 (上午)

- [x] **配置文件管理** (`src-tauri/src/config.rs`)
  - 读取 ~/.claude/settings.json
  - 写入配置文件
  - 备份/恢复配置
- [x] **供应商管理** (`src-tauri/src/provider.rs`)
  - 供应商列表的增删改查
  - 供应商配置切换逻辑
  - 配置文件命名规则 (settings-{name}.json)

#### 3.2 Tauri Commands 实现 (下午)

```rust
// 需要实现的命令列表 - 已完成
#[tauri::command]
async fn get_providers() -> Result<HashMap<String, Provider>, String>

#[tauri::command]
async fn get_current_provider() -> Result<String, String>

#[tauri::command]
async fn add_provider(provider: Provider) -> Result<bool, String>

#[tauri::command]
async fn update_provider(provider: Provider) -> Result<bool, String>

#[tauri::command]
async fn delete_provider(id: String) -> Result<bool, String>

#[tauri::command]
async fn switch_provider(id: String) -> Result<bool, String>

#[tauri::command]
async fn import_default_config() -> Result<bool, String>

#[tauri::command]
async fn get_claude_config_status() -> Result<ConfigStatus, String>
```

#### 3.3 数据存储 (`src-tauri/src/store.rs`)

- [x] 使用 tauri-plugin-store 替代 electron-store
- [x] 迁移配置存储逻辑 (~/.cc-switch/config.json)

### Phase 4: 前端适配 (Day 2 傍晚)

#### 4.1 API 层重构

- [ ] 创建 `src/lib/tauri-api.ts`
  - 替换 Electron IPC 调用为 Tauri invoke
  - 保持 API 接口一致，减少组件改动

```typescript
// 示例：迁移前后对比
// Electron (旧)
window.electronAPI.getProviders();

// Tauri (新)
import { invoke } from "@tauri-apps/api/tauri";
invoke("get_providers");
```

#### 4.2 最小化前端改动

- [ ] 更新 preload 桥接逻辑
- [ ] 调整窗口控制相关代码
- [ ] 处理文件路径差异

### Phase 5: 测试与优化 (Day 3 上午)

#### 5.1 功能测试清单

- [ ] 供应商列表显示
- [ ] 添加新供应商
- [ ] 编辑供应商信息
- [ ] 删除供应商
- [ ] 切换供应商配置
- [ ] 导入默认配置
- [ ] 预设模板功能
- [ ] API Key 快速输入

#### 5.2 跨平台测试

- [ ] Windows 10/11 测试
- [ ] 不考虑 Windows 7/8 兼容性
- [ ] macOS 测试 (如有条件)
- [ ] Linux 测试 (如有条件)

#### 5.3 性能优化

- [ ] Rust 代码优化 (release 模式)
- [ ] 减少不必要的文件 I/O
- [ ] 优化启动加载流程

### Phase 6: 构建与发布 (Day 3 下午)

#### 6.1 构建配置

- [ ] 配置 GitHub Actions CI/CD
- [ ] 设置代码签名 (Windows/macOS)
- [ ] 配置自动更新机制

#### 6.2 打包发布

- [ ] Windows NSIS 安装包
- [ ] Windows 便携版 (portable)
- [ ] macOS .app 包
- [ ] Linux AppImage

#### 6.3 版本发布

- [ ] 创建 3.0.0-beta.1 预发布
- [ ] 编写迁移说明文档
- [ ] 更新 README.md

## 风险与应对

### 技术风险

1. **Rust 学习曲线**

   - 风险：Rust 语法相对复杂
   - 应对：专注于基础文件 I/O，使用成熟库

2. **WebView2 兼容性**

- 不需要支持旧版 Windows

3. **跨平台差异**
   - 风险：不同系统的文件路径处理
   - 应对：使用 Tauri API 统一处理路径

### 用户体验风险

1. **界面渲染差异**

   - 风险：WebView 渲染可能与 Chromium 有细微差异
   - 应对：充分测试，必要时调整 CSS

2. **功能回归**
   - 风险：迁移过程中遗漏功能
   - 应对：严格按照测试清单验证

## 回滚方案

如果 Tauri 版本出现严重问题：

1. 立即从 electron-legacy 分支发布修复版本
2. 在 GitHub Release 页面提供两个版本下载
3. 明确标注版本差异和适用场景

## 时间线

- **Day 1**: 环境搭建 + 项目结构
- **Day 2**: 后端迁移 + 前端适配
- **Day 3**: 测试优化 + 构建发布
- **Total**: 3 个工作日完成迁移

## 成功标准

- ✅ 应用体积 < 10MB
- ✅ 冷启动时间 < 1 秒
- ✅ 所有现有功能正常工作
- ✅ 通过所有测试用例
- ✅ 成功构建三平台安装包

## 后续优化 (可选)

- 添加系统托盘功能
- 实现自动更新机制
- 添加快捷键支持
- 优化动画效果
- 支持深色模式跟随系统

---

_最后更新：2024-12-23_
_负责人：Jason Young_
_状态：进行中 - Phase 3 已完成_
