# 更新功能开发计划（Tauri v2 Updater）

> 目标：为桌面应用（macOS `.app`、Windows）集成并验证基于 Tauri v2 官方 Updater 插件的自动更新能力，覆盖版本检测、下载、安装与重启的完整闭环，并建立可复用的发布与测试流程。

## 一、范围与目标
- 核心能力：
  - 检查更新 → 下载 → 安装 → 应用重启。
  - 支持静态 JSON 与动态接口两种更新源。
  - 版本通道（stable/beta）与运行时切换端点（可选）。
- 平台覆盖：
  - macOS `.app`（优先级高）。
  - Windows（推荐安装器路径 NSIS/MSI；便携版说明限制）。
- 安全要求：
  - Updater 更新签名（Ed25519）强制开启：客户端 `pubkey` 校验服务端签名。
  - 不强制平台代码签名（macOS/Windows），但建议上线前完善。

## 二、方案概述
- 插件：`tauri-plugin-updater`（Rust/JS 双端 API），前端重启依赖 `@tauri-apps/plugin-process`。
- 核心配置：
  - `src-tauri/tauri.conf.json`
    - `bundle.createUpdaterArtifacts: true`
    - `plugins.updater.pubkey: "<PUBLICKEY.PEM 内容>"`
    - `plugins.updater.endpoints: ["<更新源 URL 列表>"]`
    - Windows 可选：`plugins.updater.windows.installMode: "passive|basicUi|quiet"`
  - `src-tauri/capabilities/default.json` 增加：`"updater:default"`
- 构建与签名：
  - 生成密钥：`tauri signer generate` → 注入构建环境变量 `TAURI_SIGNING_PRIVATE_KEY`（及可选密码）。
  - 构建产出带签名的更新制品（各平台包 + sig）。

## 三、交付物
- 可运行的自动更新能力（含前端触发入口与进度反馈）。
- 配置完善：`tauri.conf.json`、`capabilities/default.json`、插件初始化。
- 本地更新测试用 `latest.json` 模板与脚本（或说明）。
- 文档：
  - 更新源格式说明（静态/动态）。
  - 发布与回滚操作说明。
  - 常见问题排查清单。

## 四、里程碑与任务拆解
1) 准备与密钥
- 生成更新签名密钥对（Ed25519）。
- 将公钥填入 `plugins.updater.pubkey`，在构建环境配置 `TAURI_SIGNING_PRIVATE_KEY`。

2) 插件集成（Rust 侧）
- 注册插件：`tauri_plugin_updater::Builder::new().build()`。
- 可选：在 `setup` 中后台自动检查与安装；或暴露 `invoke` 命令由前端驱动。
- 可选：`updater_builder()` 覆盖网络参数（超时/代理/headers）与动态端点。

3) 前端接入（Renderer）
- 依赖：`@tauri-apps/plugin-updater`、`@tauri-apps/plugin-process`。
- 提供“检查更新”入口：显示当前版本、可用版本、更新日志。
- 进度反馈：`downloadAndInstall` 回调显示 Started/Progress/Finished。
- 成功后调用 `relaunch()`（或 Rust 侧 `app.restart()`）。

4) 配置与权限
- `tauri.conf.json`：`createUpdaterArtifacts`、`pubkey`、`endpoints`、Windows `installMode`（如需）。
- `capabilities/default.json`：加入 `"updater:default"`。

5) 更新源与产物
- 静态 JSON（`latest.json`）模板：
  - `version`、`notes`、`pub_date`（可选），`platforms[target].url` 与 `platforms[target].signature`（必填，signature 为 `.sig` 内容）。
- 动态 API：
  - 无更新返回 HTTP 204；有更新返回 200 + `{ version, url, signature, notes?, pub_date? }`。
- 产物托管：
  - 本地 HTTP 服务器（开发自测）或 GitHub Releases/CDN（准生产）。

6) 测试计划
- 基线用例：
  - 发现更新 → 下载 → 安装 → 重启 → 版本号提升。
  - 无更新（204）提示“已是最新”。
  - 签名不匹配拒绝更新（安全校验）。
  - 网络超时/失败的错误提示与恢复。
- 平台专项：
  - macOS：`~/Applications` 与 `/Applications` 两种放置；无苹果账号情况下的 Gatekeeper 行为提示。
  - Windows：安装器三种 `installMode`；便携版在用户可写目录的可行性验证与限制说明（文件锁/提权）。
- 自测步骤（本地静态 JSON）：
  - 用 v1.0.0 作为“已安装版本”，构建 v1.0.1 更新产物与 `.sig`。
  - 生成 `latest.json`，启动本地 HTTP（如 `npx http-server`）。
  - 旧版应用中点击“检查更新”并验证完整流程。

7) 发布与回滚
- 发布：
  - 通过 CI 生成更新产物与签名，上传到 Release/CDN。
  - 产物与 `latest.json` 上传至 Releases/CDN（或刷新动态接口数据）。
- 回滚：
  - 撤回最新产物；或将 `latest.json` 指向上一个稳定版本。
  - 如允许降级，Rust 侧定制 `version_comparator`。

8) 文档与移交
- 更新源格式说明、运维手册、常见问题排查（见下文附录）。

## 五、配置清单（示例）
- `src-tauri/tauri.conf.json` 关键片段：
```json
{
  "bundle": { "createUpdaterArtifacts": true },
  "plugins": {
    "updater": {
      "pubkey": "<PUBLICKEY.PEM 内容>",
      "endpoints": [
        "https://releases.example.com/{{target}}/{{arch}}/{{current_version}}",
        "https://github.com/org/repo/releases/latest/download/latest.json"
      ],
      "windows": { "installMode": "passive" }
    }
  }
}
```
- `src-tauri/capabilities/default.json`：
```json
{
  "permissions": [
    "updater:default"
  ]
}
```

## 六、前端最小用例（伪代码）
```ts
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

export async function runUpdateFlow() {
  const update = await check({ timeout: 30000 })
  if (!update) return { status: 'up-to-date' }

  let downloaded = 0
  let total = 0
  await update.downloadAndInstall((e) => {
    switch (e.event) {
      case 'Started': total = e.data.contentLength ?? 0; break
      case 'Progress': downloaded += e.data.chunkLength; break
    }
  })

  await relaunch()
}
```

## 七、更新源样例
- 静态 `latest.json`：
```json
{
  "version": "1.0.1",
  "notes": "Bug fixes and performance improvements",
  "pub_date": "2025-01-01T10:00:00Z",
  "platforms": {
    "darwin-aarch64": { "url": "https://cdn/app-1.0.1-darwin-aarch64.tar.gz", "signature": "<sig 内容>" },
    "windows-x86_64": { "url": "https://cdn/app-1.0.1-x86_64.zip", "signature": "<sig 内容>" }
  }
}
```
- 动态接口（有更新返回 200）：
```json
{ "version": "1.0.1", "url": "https://cdn/app-1.0.1.zip", "signature": "<sig>", "notes": "..." }
```
- 无更新返回：HTTP 204 No Content。

## 八、平台差异与限制
- macOS `.app`：
  - 支持直接替换更新；若位于 `/Applications` 且需要管理员权限可能失败（Updater 不主动提权）。建议用户安装到 `~/Applications`。
  - 无苹果开发者账号也可测试 Updater，但分发会触发 Gatekeeper 警告（建议正式版走代码签名+公证）。
- Windows：
  - 强烈建议安装器（NSIS/MSI），Updater 会下载并运行安装器，`installMode` 控制交互程度。
  - 便携版（绿色版）不稳定：运行中无法覆盖自身文件、缺乏提权与回滚；如必须使用，需将应用放到可写目录并设计辅助替换流程（本计划不默认实现）。

## 九、测试用例清单
- 功能流转：有更新/无更新/下载中断/重试/安装后重启成功。
- 安全校验：签名错误与端点被劫持时应拒绝更新。
- 网络异常：超时、代理、断网时的提示与恢复路径。
- 平台行为：
  - macOS：不同安装路径权限导致的成功/失败覆盖。
  - Windows：`passive|basicUi|quiet` 下安装器行为；便携版在用户可写目录的替换验证（若执行）。

## 十、发布与运维
- 发布流水线（建议 CI）：
  - 设置 `TAURI_SIGNING_PRIVATE_KEY`（机密变量）。
  - 构建生成各平台更新产物与签名。
  - 产物与 `latest.json` 上传至 Releases/CDN（或刷新动态接口数据）。
- 回滚策略：
  - 撤回最新产物；或将 `latest.json` 指向上一个稳定版本。
  - 如允许降级，Rust 侧定制 `version_comparator`。

## 十一、时间排期（参考）
- D1：密钥与基础集成（插件/配置/权限）。
- D2：前端入口与进度 UI、静态 JSON 本地自测通过。
- D3：GitHub Releases/CDN 端到端验证、平台专项测试。
- D4：文档完善、回滚与异常流程演练、准备上线。

## 十二、验收标准
- 基线流转：在两平台完成“发现→下载→安装→重启→版本提升”。
- 安全：签名校验生效，签名不匹配拒绝更新。
- 文档：更新源规范、操作手册、排障清单齐备。
- 稳定性：网络异常与常见权限问题有明确用户提示与可行恢复路径。

---

### 附录 A：常见问题排查
- “未发现更新”：确认 `version` 更大、端点可达、HTTP 返回码（204/200）。
- “签名校验失败”：`pubkey` 与私钥不匹配；`signature` 必须是 `.sig` 文件内容。
- “下载/超时失败”：增加 `timeout`、检查代理/证书、重试策略与提示。
- “macOS 更新失败”：检查安装路径是否需要管理员权限；建议 `~/Applications`。
- “Windows 便携版覆盖失败”：可写权限/进程占用/缺少提权，优先改为安装器分发。

### 附录 B：后续可选优化
- 渠道支持（stable/beta）与 UI 切换。
- 增量发布策略与 CDN 缓存优化。
- 远程开关与灰度比例控制（动态接口）。
- 统一 Telemetry：下载失败率、平均时延、更新成功率统计。
