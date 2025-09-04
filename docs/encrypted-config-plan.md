# CC Switch 加密配置与切换重构方案（V1）

## 1. 目标与范围

- 目标：将 `~/.cc-switch/config.json` 作为单一真实来源（SSOT），改为“加密落盘”；切换时从解密后的内存配置写入目标应用主配置（Claude/Codex）。
- 范围：
  - 后端（Rust/Tauri）新增加密模块与读写改造。
  - 调整切换逻辑为“内存 → 主配置”，切换前回填 live 配置到当前供应商，避免用户外部手改丢失。
  - 新增“旧文件清理与归档”能力：默认仅归档不删除，并在迁移成功后提醒用户执行；可在设置页手动触发。
  - 兼容旧明文配置（v1/v2），首次保存迁移为加密文件。

## 2. 背景现状（简述）

- 当前：
  - 全局配置：`~/.cc-switch/config.json`（v2：`MultiAppConfig`，含多个 `ProviderManager`）。
  - 切换：依赖“供应商副本文件”（Claude：`~/.claude/settings-<name>.json`；Codex：`~/.codex/auth-<name>.json`、`config-<name>.toml`）→ 恢复到主配置。
  - 启动：若检测到现有主配置，自动导入为 `default` 供应商。
- 问题：存在“副本 ↔ 总配置”双来源，可能不一致；明文落盘有泄露风险。

## 3. 总体方案

- 以加密文件 `~/.cc-switch/config.enc.json` 替代明文存储；进程启动时解密一次加载到内存，后续以内存为准；保存时加密写盘。
- 切换时：直接从内存 `Provider.settings_config` 写入目标应用主配置；切换前回填当前 live 配置到 `current` 供应商，保留外部修改。
- 明文兼容：若无加密文件，读取旧 `config.json`（含 v1→v2 迁移），首次保存写加密文件，并备份旧明文。
- 旧文件清理：提供“可回滚归档”而非删除。扫描 `~/.cc-switch/config.json`（v1/v2）与 Claude/Codex 的历史副本文件，用户确认后移动到 `~/.cc-switch/archive/<ts>/`，生成 `manifest.json` 以便恢复；默认不做静默清理。

## 4. 密钥管理

- 存储：系统级凭据管家（keyring crate）。
  - Service：`cc-switch`；Account：`config-key-v1`；内容：Base64 编码的 32 字节随机密钥（AES-256）。
- 首次运行：生成随机密钥，写入 Keychain。
- 进程内缓存：启动加载后缓存密钥，避免重复 IO。
- 轮换（后续）：支持命令触发“旧密钥解密 → 新密钥加密”的原子迁移。
- 回退策略：Keychain 不可用时进入“只读模式”并提示用户（不建议将密钥落盘）。

## 5. 加密封装格式

- 文件：`~/.cc-switch/config.enc.json`
- 结构（JSON 封装，便于演进）：
  ```json
  {
    "v": 1,
    "alg": "AES-256-GCM",
    "nonce": "<base64-nonce>",
    "ct": "<base64-ciphertext>"
  }
  ```
- 明文：`serde_json::to_vec(MultiAppConfig)`；加密：AES-GCM（12 字节随机 nonce）；每次保存生成新 nonce。

## 6. 模块与改造点

- 新增 `src-tauri/src/secure_store.rs`：
  - `get_or_create_key() -> Result<[u8;32], String>`：从 Keychain 获取/生成密钥。
  - `encrypt_bytes(key, plaintext) -> (nonce, ciphertext)`；`decrypt_bytes(key, nonce, ciphertext)`。
  - `read_encrypted_config() -> Result<MultiAppConfig, String>`：读取 `config.enc.json`、解析封装、解密、反序列化。
  - `write_encrypted_config(cfg: &MultiAppConfig) -> Result<(), String>`：序列化→加密→原子写入。
- 新增 `src-tauri/src/legacy_cleanup.rs`（旧文件清理/归档）：
  - `scan_legacy_files() -> LegacyScanReport`：扫描旧 `config.json`（v1/v2）与 Claude/Codex 副本文件（`settings-*.json`、`auth-*.json`、`config-*.toml`），返回分组清单、大小、mtime；永不将 live 文件（`settings.json`、`auth.json`、`config.toml`、`config.enc.json`）列为可归档。
  - `archive_legacy_files(selection) -> ArchiveResult`：将选中文件移动到 `~/.cc-switch/archive/<ts>/` 下对应子目录（`cc-switch/`、`claude/`、`codex/`），生成 `manifest.json`（记录原路径、归档路径、大小、mtime、sha256、类别）；同分区 `rename`，跨分区“copy + fsync + remove”。
  - `restore_from_archive(manifest_path, items?) -> RestoreResult`：从归档恢复选中文件；若原路径已有同名文件则中止并提示冲突。
  - 可选：`purge_archived(before_days)` 仅删除 `archive/` 内的过期归档；默认关闭。
  - 安全护栏：操作前后做 mtime/hash 复核（CAS）；发生变化中止并提示“外部已修改”。
- 调整 `src-tauri/src/app_config.rs`：
  - `MultiAppConfig::load()`：优先 `read_encrypted_config()`；若无则读旧明文：
    - 若检测到 v1（`ProviderManager`）→ 迁移到 v2（原有逻辑保留）。
  - `MultiAppConfig::save()`：统一调用 `write_encrypted_config()`；若检测到旧 `config.json`，首次保存时备份为 `config.v1.backup.<ts>.json`（或保留为只读，视实现选择）。
- 调整 `src-tauri/src/commands.rs::switch_provider`：
  - Claude：
    1. 回填：若 `~/.claude/settings.json` 存在且 `current` 非空 → 读取 JSON，写回 `manager.providers[current].settings_config`。
    2. 切换：从目标 `provider.settings_config` 直接写 `~/.claude/settings.json`（确保父目录存在）。
  - Codex：
    1. 回填：读取 `~/.codex/auth.json`（JSON）与 `~/.codex/config.toml`（字符串；非空做 TOML 校验）→ 合成为 `{auth, config}` → 写回 `manager.providers[current].settings_config`。
    2. 切换：从目标 `provider.settings_config` 中取 `auth`（必需）与 `config`（可空）写入对应主配置（非空 `config` 校验 TOML）。
  - 更新 `manager.current = id`，`state.save()` → 触发加密保存。
- 保留/清理：
  - 阶段一保留 `codex_config.rs` 与 `config.rs` 的副本读写函数（减少改动面），但切换不再依赖“副本恢复”。
  - 阶段二可移除 add/update 时的“副本写入”，转为仅更新内存并保存加密配置。

## 7. 数据流与时序

- 启动：`AppState::new()` → `MultiAppConfig::load()`（优先加密）→ 进程内持有解密后的配置。
- 添加/编辑/删除：更新内存中的 `ProviderManager` → `state.save()`（加密写盘）。
- 切换：回填 live → 以目标供应商内存配置写入主配置 → 更新 `current` → `state.save()`。
- 迁移后提醒：若首次从旧明文迁移成功，弹出“发现旧配置，可归档”提示；用户可进入“存储与清理”页面查看并执行归档。

## 8. 迁移策略

- 读取顺序：`config.enc.json`（新）→ `config.json`（旧）。
- 旧版支持：
  - v1 明文（单 `ProviderManager`）→ 自动迁移为 v2（已有逻辑）。
  - v2 明文 → 直接加载。
- 首次保存：写 `config.enc.json`；若存在旧 `config.json`，备份为 `config.v1.backup.<ts>.json`（或保留为只读）。
- 失败处理：解密失败/破损 → 明确提示并拒绝覆盖；允许用户手动回滚备份。
- 旧文件处理：默认不自动删除。提供“扫描→归档”的可选流程，将旧 `config.json` 与历史副本文件移动到 `~/.cc-switch/archive/<ts>/`，保留 `manifest.json` 以支持恢复。

## 9. 回滚策略

- 加密回滚：保留 `config.v1.backup.<ts>.json` 作为明文快照；必要时让 `load()` 回退到该备份（手动步骤）。
- 切换回退：临时切换回“副本恢复”路径（现有代码仍在，快速恢复可用）。

## 10. 安全与性能

- 算法：AES-256-GCM（AEAD）；随机 12 字节 nonce；每次保存新 nonce。
- 性能：对几十 KB 级别文件，加解密开销远低于磁盘 IO 和 JSON 处理；冷启动 Keychain 取密钥 1–20ms，可缓存。
- 可靠性：原子写入（临时文件 + rename）；写入失败不破坏现有文件。
- 可选增强：`zeroize` 清理密钥与明文；Claude 配置 JSON Schema 校验。
- 清理安全：归档而非删除；不触及 live 文件；归档/恢复采用 CAS 校验与错误回滚；归档路径冲突加后缀去重（如 `-2`、`-3`）。

## 11. API 与 UX 影响

- 前端 API：现有行为不变；新增清理相关命令（Tauri）供 UI 调用：`scan_legacy_files`、`archive_legacy_files`、`restore_from_archive`（`purge_archived` 可选）。
- UI 提示：在“配置文件位置”旁提示“已加密存储”。
- 清理入口：设置页新增“存储与清理”面板，展示扫描结果、支持归档与从归档恢复；首次迁移成功后弹出提醒（可稍后再说）。
- 文案约定：明确“仅归档、不删除；删除需二次确认且默认关闭自动删除”。

## 12. 开发任务拆解（阶段一为本次交付）

- 阶段一（核心改造 + 清理能力最小闭环）
  - 新增模块 `secure_store.rs`：Keychain 与加解密工具函数。
  - 改造 `app_config.rs`：`load()/save()` 支持加密文件与旧明文迁移、原子写入、备份。
  - 改造 `commands.rs::switch_provider`：
    - 回填 live 配置 → 写入目标主配置（Claude/Codex）。
    - 去除对“副本恢复”的依赖（保留函数以便回退）。
  - 旧文件清理：新增 `legacy_cleanup.rs` 与对应 Tauri 命令，完成“扫描→归档→恢复”；首次迁移成功后在 UI 弹提醒，指向“设置 > 存储与清理”。
  - 保持 `import_default_config`、`get_config_status` 行为不变。
- 阶段二（清理与增强）
  - 移除 add/update 对“副本文件”的写入，完全以内存+加密文件为中心。
  - Claude settings 的 JSON Schema 校验；导出明文快照；只读模式显式开关。
- 阶段三（安全升级）
  - 密钥轮换；可选 passphrase（KDF: Argon2id + salt）。

## 14. 验收标准

- 功能：
  - 无加密明文文件也能启动并正确读写；
  - 切换成功将内存配置写入主配置；
  - 外部手改在下一次切换前被回填保存；
  - 旧配置自动迁移并生成加密文件；
  - Keychain/解密异常时不损坏已有文件，给出可理解错误。
  - 清理：扫描能准确识别旧明文与副本文件；执行归档后原路径不再存在文件、归档目录生成 `manifest.json`；从归档恢复可还原到原路径（不覆盖已存在文件）。
- 质量：
  - 关键路径加错误处理与日志；
  - 写入采用原子替换；
  - 代码变更集中、最小侵入，与现有风格一致。
  - 清理操作具备 CAS 校验、错误回滚、绝不触及 live 文件与 `config.enc.json`。

## 15. 风险与对策

- Keychain 不可用或权限受限：
  - 对策：只读模式 + 明确提示；不覆盖落盘；允许手动恢复明文备份。
- 加密文件损坏：
  - 对策：严格校验与错误分支；保留旧文件；不做“盲目重置”。
- 与“副本文件”并存导致混淆：
  - 对策：阶段一保留但不依赖；阶段二移除写入，文档化行为变更。
- 清理误删或不可逆：
  - 对策：默认仅归档不删除；删除需二次确认且仅作用于 `archive/`；提供 `manifest.json` 恢复；归档/恢复全程 CAS 校验与回滚。

## 16. 发布与回退

- 发布：随 Tauri 应用正常发布，无需前端变更。
- 回退：保留旧明文备份；将切换逻辑临时改回“副本恢复”路径可快速回退。

## 17. 旧文件清理与归档（新增）

- 归档对象：
  - `~/.cc-switch/config.json`（v1/v2，迁移成功后）
  - `~/.claude/settings-*.json`（保留 `settings.json`）
  - `~/.codex/auth-*.json`、`~/.codex/config-*.toml`（保留 `auth.json`、`config.toml`）
- 归档位置与结构：`~/.cc-switch/archive/<timestamp>/{cc-switch,claude,codex}/...`
- `manifest.json`：记录原路径、归档路径、大小、mtime、sha256、类别（v1/v2/claude/codex）；用于恢复与可视化。
- 提醒策略：首次迁移成功后弹窗提醒；设置页“存储与清理”提供扫描、归档、恢复操作；默认不自动删除，可选“删除归档 >N 天”开关（默认关闭）。
- 护栏：永不移动/删除 live 文件与 `config.enc.json`；执行前后 CAS 校验；跨分区采用“copy+fsync+remove”；失败即时回滚并提示。

## 18. 变更点清单（代码）

- 新增：`src-tauri/src/secure_store.rs`
- 修改：
  - `src-tauri/src/app_config.rs`（load/save 加密化、迁移与原子写入）
  - `src-tauri/src/commands.rs`（switch_provider 改为内存 → 主配置，并回填 live）
  - `src-tauri/src/legacy_cleanup.rs`（扫描/归档/恢复旧文件）
- 保持：
  - `src-tauri/src/config.rs`、`src-tauri/src/codex_config.rs`（读写工具与校验，阶段一不大动）
  - 前端 `src/lib/tauri-api.ts` 与 UI 逻辑

## 19. 开放问题（待确认）

- Keychain 失败时是否提供“本地明文密钥文件（600 权限）”的应急模式（当前建议：不提供，保持只读）。
- 加密文件名固定为 `config.enc.json` 是否满足预期，或需隐藏（如 `.config.enc`）。
- 是否需要提供“自动删除归档 >N 天”的开关（默认关闭，建议 N=30）。

---

以上方案为“阶段一”可落地版本，能在保持前端无感的前提下完成“加密存储 + 内存驱动切换”的核心目标。如需，我可以继续补充任务看板（Issue 列表）与实施顺序的 PR 规划。
