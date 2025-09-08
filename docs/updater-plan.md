# 更新功能开发计划（Tauri v2 Updater）

> 目标：基于 Tauri v2 官方 Updater，完成“检查更新 → 下载 → 安装 → 重启”的完整闭环；提供清晰的前后端接口、配置与测试/发布流程。

## 范围与目标
- 能力：静态 JSON 与动态接口两种更新源；可选稳定/测试通道；进度反馈与错误处理。
- 平台：macOS `.app` 优先；Windows 使用安装器（NSIS/MSI）。
- 安全：启用 Ed25519 更新签名校验；上线前建议平台代码签名与公证。

## 架构与依赖
- 插件：`tauri-plugin-updater`（更新）、`@tauri-apps/plugin-updater`（JS）；`tauri-plugin-process` 与 `@tauri-apps/plugin-process`（重启）。
- 签名与构建：`tauri signer generate` 生成密钥；CI/本机注入 `TAURI_SIGNING_PRIVATE_KEY`；`bundle.createUpdaterArtifacts: true` 生成签名制品。
- 权限：在 `src-tauri/capabilities/default.json` 启用 `updater:default` 与 `process:allow-restart`。
- 配置（`src-tauri/tauri.conf.json`）：
  - `plugins.updater.pubkey: "<PUBLICKEY.PEM>"`
  - `plugins.updater.endpoints: ["<更新源 URL 列表>"]`
  - Windows（可选）：`plugins.updater.windows.installMode: "passive|basicUi|quiet"`

## 前端接口设计（TypeScript）
- 类型
  - `type UpdateChannel = 'stable' | 'beta'`
  - `type UpdaterPhase = 'idle' | 'checking' | 'available' | 'downloading' | 'installing' | 'restarting' | 'upToDate' | 'error'`
  - `type UpdateInfo = { currentVersion: string; availableVersion: string; notes?: string; pubDate?: string }`
  - `type UpdateProgressEvent = { event: 'Started' | 'Progress' | 'Finished'; total?: number; downloaded?: number }`
  - `type UpdateError = { code: string; message: string; cause?: unknown }`
  - `type CheckOptions = { timeout?: number; channel?: UpdateChannel }`
- API（`src/lib/updater.ts`）
  - `getCurrentVersion(): Promise<string>` 读取当前版本。
  - `checkForUpdate(opts?: CheckOptions)` → `up-to-date` 或 `{ status: 'available', info, update }`。
  - `downloadAndInstall(update, onProgress?)` 下载并安装，进度回调映射 Started/Progress/Finished。
  - `relaunchApp()` 调用 `@tauri-apps/plugin-process.relaunch()`。
  - `runUpdateFlow(opts?)` 编排：检查 → 下载安装 → 重启；错误统一抛出 `UpdateError`。
  - `setUpdateChannel(channel)` 前端记录偏好；实际端点切换见“端点动态化”。
- Hook（可选 `useUpdater()`）
  - 返回 `{ phase, info?, progress?, error?, actions: { check, startUpdate, relaunch } }`。
- UI（组件建议）
  - `UpdateBanner`：发现新版本时展示；`UpdaterDialog`：显示说明、进度与错误/重试。

## Rust 集成与权限
- 插件注册（`src-tauri/src/main.rs`）：
  - `app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;`
  - `.plugin(tauri_plugin_process::init())` 用于重启。
- Windows 清理钩子（可选）：`UpdaterExt::on_before_exit(app.cleanup_before_exit)`，避免安装器启动前文件占用。
- 端点动态化（可选）：在 `setup` 根据配置/环境切换 `endpoints`、超时、代理或 headers。

## 更新源与格式
- 静态 JSON（latest.json）：字段 `version`、`platforms[target].url`、`platforms[target].signature`（`.sig` 内容）；可选 `notes`、`pub_date`。
- 动态接口：
  - 无更新：HTTP 204
  - 有更新：HTTP 200 → `{ version, url, signature, notes?, pub_date? }`
- 通道组织：`/stable/latest.json`、`/beta/latest.json`；CDN 缓存需可控，回滚可强制刷新。

## 用户流程与 UX
- 流程：检查 → 展示版本/日志 → 下载进度（累计/百分比）→ 安装 → 提示并重启。
- 错误：网络异常（超时/断网/证书）、签名不匹配、权限/文件占用（Win）。提供“重试/稍后更新”。
- 平台提示：
  - macOS：建议安装在 `~/Applications`，避免 `/Applications` 提权导致失败。
  - Windows：优先安装器分发，并选择合适 `installMode`。

## 测试计划
- 功能：有更新/无更新（204）/下载中断/重试/安装后重启成功与版本号提升。
- 安全：签名不匹配必须拒绝更新；端点不可用/被劫持有清晰提示。
- 网络：超时/断网/代理场景提示与恢复。
- 平台：
  - macOS：`/Applications` 与 `~/Applications` 的权限差异。
  - Windows：`passive|basicUi|quiet` 行为差异与成功率。
- 本地自测：以 v1.0.0 运行，构建 v1.0.1 制品+`.sig`，本地 HTTP 托管 `latest.json`，验证全链路。

## 发布与回滚
- 发布（CI 推荐）：注入 `TAURI_SIGNING_PRIVATE_KEY` → 构建生成各平台制品+签名 → 上传产物与 `latest.json` 至 Releases/CDN。
- 回滚：撤下问题版本或将 `latest.json` 指回上一个稳定版本；如需降级，Rust 侧可定制版本比较策略（可选）。

## 里程碑与验收
- D1：密钥与基础集成（插件/配置/权限）。
- D2：前端入口与进度 UI，静态 JSON 自测通过。
- D3：Releases/CDN 端到端验证，平台专项测试。
- D4：文档完善、回滚与异常流程演练。
- 验收：两平台完成“发现→下载→安装→重启→版本提升”；签名校验生效；异常有明确提示与可行恢复。

## 待确认
- 更新源托管（GitHub Releases 还是自有 CDN）。
- 是否需要 beta 通道与运行时切换。
- Windows 是否仅支持安装器分发；便携版兼容策略是否需要明确说明。
- UI 文案与样式偏好。

## 落地步骤（实施顺序）
1) 生成 Ed25519 密钥，将公钥写入 `plugins.updater.pubkey`，在构建环境配置 `TAURI_SIGNING_PRIVATE_KEY`。
2) `src-tauri` 注册 `tauri-plugin-updater` 与 `tauri-plugin-process`，补齐 `capabilities/default.json` 与 `tauri.conf.json`。
3) 前端新增 `src/lib/updater.ts` 封装与 `UpdateBanner`/`UpdaterDialog` 组件，接入入口按钮。
4) 本地静态 `latest.json` 自测全链路；完善错误与进度提示。
5) 配置 CI 发布产物与 `latest.json`；编写发布/回滚操作手册。
