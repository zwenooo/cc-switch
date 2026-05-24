#![allow(non_snake_case)]

use crate::app_config::AppType;
use crate::init_status::{InitErrorPayload, SkillsMigrationPayload};
use crate::services::ProviderService;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tauri::AppHandle;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 打开外部链接
#[tauri::command]
pub async fn open_external(app: AppHandle, url: String) -> Result<bool, String> {
    let url = if url.starts_with("http://") || url.starts_with("https://") {
        url
    } else {
        format!("https://{url}")
    };

    app.opener()
        .open_url(&url, None::<String>)
        .map_err(|e| format!("打开链接失败: {e}"))?;

    Ok(true)
}

#[tauri::command]
pub async fn copy_text_to_clipboard(text: String) -> Result<bool, String> {
    // Use spawn_blocking to avoid blocking the async runtime
    // Clipboard access can block on some platforms and may have thread/loop constraints
    tokio::task::spawn_blocking(move || {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("访问系统剪贴板失败: {e}"))?;
        clipboard
            .set_text(text)
            .map_err(|e| format!("写入系统剪贴板失败: {e}"))?;
        Ok(true)
    })
    .await
    .map_err(|e| format!("剪贴板任务执行失败: {e}"))?
}

/// 检查更新
#[tauri::command]
pub async fn check_for_updates(handle: AppHandle) -> Result<bool, String> {
    handle
        .opener()
        .open_url(
            "https://github.com/farion1231/cc-switch/releases/latest",
            None::<String>,
        )
        .map_err(|e| format!("打开更新页面失败: {e}"))?;

    Ok(true)
}

/// 判断是否为便携版（绿色版）运行
#[tauri::command]
pub async fn is_portable_mode() -> Result<bool, String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("获取可执行路径失败: {e}"))?;
    if let Some(dir) = exe_path.parent() {
        Ok(dir.join("portable.ini").is_file())
    } else {
        Ok(false)
    }
}

/// 获取应用启动阶段的初始化错误（若有）。
/// 用于前端在早期主动拉取，避免事件订阅竞态导致的提示缺失。
#[tauri::command]
pub async fn get_init_error() -> Result<Option<InitErrorPayload>, String> {
    Ok(crate::init_status::get_init_error())
}

/// 获取 JSON→SQLite 迁移结果（若有）。
/// 只返回一次 true，之后返回 false，用于前端显示一次性 Toast 通知。
#[tauri::command]
pub async fn get_migration_result() -> Result<bool, String> {
    Ok(crate::init_status::take_migration_success())
}

/// 获取 Skills 自动导入（SSOT）迁移结果（若有）。
/// 只返回一次 Some({count})，之后返回 None，用于前端显示一次性 Toast 通知。
#[tauri::command]
pub async fn get_skills_migration_result() -> Result<Option<SkillsMigrationPayload>, String> {
    Ok(crate::init_status::take_skills_migration_result())
}

#[derive(serde::Serialize)]
pub struct ToolVersion {
    name: String,
    version: Option<String>,
    latest_version: Option<String>, // 新增字段：最新版本
    error: Option<String>,
    /// 已定位到可执行文件、但 `--version` 报错退出（装了却跑不起来，如 Node 版本不达标）。
    /// 供前端区分"未安装"与"已安装·无法运行"，无需匹配 error 文案反推语义。
    installed_but_broken: bool,
    /// 工具运行环境: "windows", "wsl", "macos", "linux", "unknown"
    env_type: String,
    /// 当 env_type 为 "wsl" 时，返回该工具绑定的 WSL distro（用于按 distro 探测 shells）
    wsl_distro: Option<String>,
}

const VALID_TOOLS: [&str; 6] = [
    "claude", "codex", "gemini", "opencode", "openclaw", "hermes",
];

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslShellPreferenceInput {
    #[serde(default)]
    pub wsl_shell: Option<String>,
    #[serde(default)]
    pub wsl_shell_flag: Option<String>,
}

// Keep platform-specific env detection in one place to avoid repeating cfg blocks.
#[cfg(target_os = "windows")]
fn tool_env_type_and_wsl_distro(tool: &str) -> (String, Option<String>) {
    if let Some(distro) = wsl_distro_for_tool(tool) {
        ("wsl".to_string(), Some(distro))
    } else {
        ("windows".to_string(), None)
    }
}

#[cfg(target_os = "macos")]
fn tool_env_type_and_wsl_distro(_tool: &str) -> (String, Option<String>) {
    ("macos".to_string(), None)
}

#[cfg(target_os = "linux")]
fn tool_env_type_and_wsl_distro(_tool: &str) -> (String, Option<String>) {
    ("linux".to_string(), None)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn tool_env_type_and_wsl_distro(_tool: &str) -> (String, Option<String>) {
    ("unknown".to_string(), None)
}

#[tauri::command]
pub async fn get_tool_versions(
    tools: Option<Vec<String>>,
    wsl_shell_by_tool: Option<HashMap<String, WslShellPreferenceInput>>,
) -> Result<Vec<ToolVersion>, String> {
    let requested: Vec<&str> = if let Some(tools) = tools.as_ref() {
        let set: std::collections::HashSet<&str> = tools.iter().map(|s| s.as_str()).collect();
        VALID_TOOLS
            .iter()
            .copied()
            .filter(|t| set.contains(t))
            .collect()
    } else {
        VALID_TOOLS.to_vec()
    };
    let mut results = Vec::new();

    for tool in requested {
        let pref = wsl_shell_by_tool.as_ref().and_then(|m| m.get(tool));
        let tool_wsl_shell = pref.and_then(|p| p.wsl_shell.as_deref());
        let tool_wsl_shell_flag = pref.and_then(|p| p.wsl_shell_flag.as_deref());

        results.push(get_single_tool_version_impl(tool, tool_wsl_shell, tool_wsl_shell_flag).await);
    }

    Ok(results)
}

#[tauri::command]
pub async fn run_tool_lifecycle_action(
    tools: Vec<String>,
    action: String,
    wsl_shell_by_tool: Option<HashMap<String, WslShellPreferenceInput>>,
) -> Result<(), String> {
    let action = ToolLifecycleAction::from_str(&action)?;
    let requested = normalize_requested_tools(&tools);
    if requested.is_empty() {
        return Err("No supported tools selected".to_string());
    }

    let label = match action {
        ToolLifecycleAction::Install => "tool_install",
        ToolLifecycleAction::Update => "tool_update",
    };

    // build 阶段含锚定探测（对每个工具跑 `--version` 定位命令行实际命中那处），
    // 与执行一并放进 blocking 线程，避免阻塞 async runtime。
    tokio::task::spawn_blocking(move || {
        let command_line =
            build_tool_lifecycle_command(&requested, action, wsl_shell_by_tool.as_ref())?;
        run_tool_lifecycle_silently(&command_line, label)
    })
    .await
    .map_err(|e| format!("tool lifecycle task join error: {e}"))?
}

/// 静默执行工具安装/更新脚本：直接捕获子进程输出并阻塞到命令真正结束，
/// 不再弹出可见终端窗口（与 `launch_terminal_running` 的"开窗即返回"形成对比，
/// 后者仍保留给 provider 切换等需要交互式终端的场景）。
/// 失败时回传 stderr/stdout 末尾若干行，供前端 toast 提示。
#[cfg(not(target_os = "windows"))]
fn run_tool_lifecycle_silently(command_line: &str, _label: &str) -> Result<(), String> {
    use std::process::Command;
    // command_line 是 bash 风格脚本（含 `set -e` 与多行命令）；强制用 bash 执行，
    // 避免用户默认 shell 为 fish/zsh 时 `set -e` 等语义不一致。
    let output = Command::new("bash")
        .arg("-c")
        .arg(command_line)
        .output()
        .map_err(|e| format!("启动安装进程失败: {e}"))?;
    finish_lifecycle_output(&output)
}

/// Windows 静默执行：command_line 是 .bat 内容（@echo off + call/wsl 行，CRLF 分隔），
/// 写临时 .bat 后用 `cmd /C` 执行，`CREATE_NO_WINDOW` 抑制 console 窗口。
#[cfg(target_os = "windows")]
fn run_tool_lifecycle_silently(command_line: &str, label: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let bat_file =
        std::env::temp_dir().join(format!("cc_switch_{}_{}.bat", label, std::process::id()));
    std::fs::write(&bat_file, command_line).map_err(|e| format!("写入批处理文件失败: {e}"))?;

    let output = Command::new("cmd")
        .arg("/C")
        .arg(&bat_file)
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let _ = std::fs::remove_file(&bat_file);

    finish_lifecycle_output(&output.map_err(|e| format!("启动安装进程失败: {e}"))?)
}

/// 把子进程退出结果转成 `Result`：成功返回 `Ok`；失败提取 stderr（空则回退 stdout）
/// 的末尾若干行作为错误详情，避免把整段安装日志塞进 toast。
fn finish_lifecycle_output(output: &std::process::Output) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let raw = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    let detail = last_lines(raw, 8);
    Err(if detail.is_empty() {
        format!("命令执行失败 (exit code: {:?})", output.status.code())
    } else {
        detail
    })
}

/// 取文本末尾最多 `n` 行（npm / pip 的关键错误通常出现在输出尾部）。
fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

fn normalize_requested_tools(tools: &[String]) -> Vec<&'static str> {
    let set: std::collections::HashSet<&str> = tools.iter().map(|s| s.as_str()).collect();
    VALID_TOOLS
        .iter()
        .copied()
        .filter(|tool| set.contains(tool))
        .collect()
}

#[derive(Debug, Clone, Copy)]
enum ToolLifecycleAction {
    Install,
    Update,
}

impl FromStr for ToolLifecycleAction {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "install" => Ok(Self::Install),
            "update" => Ok(Self::Update),
            _ => Err(format!("Unsupported tool action: {value}")),
        }
    }
}

fn build_tool_lifecycle_command(
    tools: &[&str],
    action: ToolLifecycleAction,
    wsl_shell_by_tool: Option<&HashMap<String, WslShellPreferenceInput>>,
) -> Result<String, String> {
    let mut lines = Vec::new();

    #[cfg(not(target_os = "windows"))]
    {
        // set -e 让任一步失败即中止;set -o pipefail 让管道前段失败也参与判定——
        // install 的 `curl ... | bash` 路径下,curl 失败时 bash 收空 stdin 仍 exit 0,
        // 没有 pipefail 会"假成功"而绕过 `||` 兜底链,工具其实没装上。
        lines.push("set -e".to_string());
        lines.push("set -o pipefail".to_string());
    }

    #[cfg(target_os = "windows")]
    lines.push("@echo off".to_string());

    for tool in tools {
        let label = tool_display_name(tool);
        lines.push(format!("echo ========== {label} =========="));

        let pref = wsl_shell_by_tool.and_then(|m| m.get(*tool));
        let line = build_tool_action_line(
            tool,
            action,
            pref.and_then(|p| p.wsl_shell.as_deref()),
            pref.and_then(|p| p.wsl_shell_flag.as_deref()),
        )?;
        lines.push(line);

        #[cfg(target_os = "windows")]
        lines.push("if errorlevel 1 exit /b %errorlevel%".to_string());

        #[cfg(not(target_os = "windows"))]
        lines.push(String::new());
    }

    Ok(lines.join(if cfg!(target_os = "windows") {
        "\r\n"
    } else {
        "\n"
    }))
}

fn tool_display_name(tool: &str) -> &'static str {
    match tool {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "gemini" => "Gemini CLI",
        "opencode" => "OpenCode",
        "openclaw" => "OpenClaw",
        "hermes" => "Hermes",
        _ => "Unknown",
    }
}

/// hermes pip 升级命令的 Unix shell 版本:`python3` 优先 + `python` 兜底。
/// Non-Windows target 直接用;Windows target 上的 WSL 分支也用——WSL 内部跑的是
/// Linux,**不存在 Windows 的 `py` launcher**(Microsoft PEP 397 是 Windows 独有),
/// Ubuntu 24.04+ 默认连 `python` alias 都没有、只有 `python3`。
const HERMES_PIP_UPGRADE_UNIX: &str =
    "python3 -m pip install --upgrade \"hermes-agent[web]\" || python -m pip install --upgrade \"hermes-agent[web]\"";

fn tool_action_shell_command(tool: &str, _action: ToolLifecycleAction) -> Option<&'static str> {
    match tool {
        "claude" => Some("npm i -g @anthropic-ai/claude-code@latest"),
        "codex" => Some("npm i -g @openai/codex@latest"),
        "gemini" => Some("npm i -g @google/gemini-cli@latest"),
        "opencode" => Some("npm i -g opencode-ai@latest"),
        "openclaw" => Some("npm i -g openclaw@latest"),
        #[cfg(target_os = "windows")]
        "hermes" => Some(
            "py -m pip install --upgrade \"hermes-agent[web]\" || python -m pip install --upgrade \"hermes-agent[web]\"",
        ),
        #[cfg(not(target_os = "windows"))]
        "hermes" => Some(HERMES_PIP_UPGRADE_UNIX),
        _ => None,
    }
}

/// Windows host 上的 WSL 分支专用:`tool_action_shell_command` 在 Windows target 编译
/// 出的版本对 hermes 返回的是 Windows 的 `py -m pip ...`,但跨 `wsl.exe` 边界后跑的
/// 是 Linux——`py` launcher 是 Windows 独有,执行会 fail。这个 wrapper 把 hermes 替换
/// 为 unix 版命令,其余工具(npm 类)跨平台命令字符串相同、直接透传。
#[cfg(target_os = "windows")]
fn wsl_tool_action_shell_command(tool: &str, action: ToolLifecycleAction) -> Option<&'static str> {
    if tool == "hermes" {
        return Some(HERMES_PIP_UPGRADE_UNIX);
    }
    tool_action_shell_command(tool, action)
}

fn build_tool_action_line(
    tool: &str,
    action: ToolLifecycleAction,
    wsl_shell: Option<&str>,
    wsl_shell_flag: Option<&str>,
) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // ① WSL 工具(override 是 UNC `\\wsl$\<distro>\...`):锚定的绝对路径是 Windows
        //    主机路径,跨 wsl.exe 进入 distro 文件系统后无效;且 enumerate 不参与 WSL。
        //    始终走静态命令(npm i -g / pip)并通过 wsl.exe -d distro -- sh 包一层。
        //    **必须用 wsl_tool_action_shell_command 而非 tool_action_shell_command**:
        //    后者在 Windows target 给 hermes 返回 Windows 的 `py -m pip ...`,跨 wsl.exe
        //    后跑 Linux、py 不存在,要替换为 unix 版的 python3/python 兜底链。
        if let Some(distro) = wsl_distro_for_tool(tool) {
            let command = wsl_tool_action_shell_command(tool, action)
                .ok_or_else(|| format!("Unsupported tool action target: {tool}"))?;
            return build_wsl_tool_action_line(&distro, command, wsl_shell, wsl_shell_flag);
        }
        // ② Windows 原生 update 锚定;install 走静态(install.sh 是 bash 脚本,Windows
        //    无意义)。**`enumerate_tool_installations` 在这里 per-tool 重做、与前端
        //    probe 阶段算过的结果不共享是 by design**:run_tool_lifecycle_action 是
        //    独立 IPC 调用,不信任前端回传的命令字符串(避免命令注入面扩大);前端是
        //    逐工具触发 lifecycle,batch 化会破坏"逐工具独立成败"的 UX。
        let command = match action {
            ToolLifecycleAction::Update => {
                let installs = enumerate_tool_installations(tool);
                installs_anchored_command(tool, &installs)
                    .unwrap_or_else(|| static_fallback_command(tool))
            }
            ToolLifecycleAction::Install => static_fallback_command(tool),
        };
        if command.is_empty() {
            return Err(format!("Unsupported tool action target: {tool}"));
        }
        // .bat 调用 .cmd/.bat 必须用 `call` 否则当前脚本被替换、后续 `if errorlevel`
        // 行被跳过;对 .exe 加 call 无害(等同直接调用)。锚定命令头部可能是 .cmd
        // (npm/pnpm)或 .exe(volta),静态命令头部是 `npm`(也是 .cmd)、`py` 等——
        // 全部加 `call ` 前缀,风格统一且语义正确。含空格的头部已被 `win_quote_path_for_batch`
        // 加上双引号,call 对带引号的路径解析正常。
        return Ok(format!("call {command}"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (wsl_shell, wsl_shell_flag);
        // update 锚定到命令行实际命中的那处（写回同一个 node / brew / 原生安装器），
        // 而非裸 `npm` 落到 PATH 第一个 npm；install 走「上游推荐 || npm 兜底」短路链
        // （有 native installer 的工具如 claude/opencode），其余仍裸 npm/pip。
        let command = match action {
            ToolLifecycleAction::Update => {
                let installs = enumerate_tool_installations(tool);
                installs_anchored_command(tool, &installs)
                    .unwrap_or_else(|| static_fallback_command(tool))
            }
            ToolLifecycleAction::Install => install_command_for(tool),
        };
        if command.is_empty() {
            return Err(format!("Unsupported tool action target: {tool}"));
        }
        Ok(command)
    }
}

#[cfg(target_os = "windows")]
fn build_wsl_tool_action_line(
    distro: &str,
    command: &str,
    force_shell: Option<&str>,
    force_shell_flag: Option<&str>,
) -> Result<String, String> {
    if !is_valid_wsl_distro_name(distro) {
        return Err(format!("Invalid WSL distro name: {distro}"));
    }

    let shell = force_shell
        .map(|s| s.rsplit('/').next().unwrap_or(s))
        .unwrap_or("sh");
    if !is_valid_shell(shell) {
        return Err(format!("Invalid WSL shell: {shell}"));
    }

    let flag = if let Some(flag) = force_shell_flag {
        if !is_valid_shell_flag(flag) {
            return Err(format!("Invalid WSL shell flag: {flag}"));
        }
        flag
    } else {
        default_flag_for_shell(shell)
    };

    Ok(format!(
        "wsl.exe -d {distro} -- {shell} {flag} {}",
        windows_cmd_double_quote_arg(command)
    ))
}

/// Windows 双引号包裹基础原语:无条件加引号 + 内部 `"` 转义为 `\"`。
/// `windows_cmd_double_quote_arg`(给 wsl.exe 传 bash 命令字符串用)与
/// `win_quote_path_for_batch`(给锚定路径用)都基于它,避免两份 quoter 各自演化、
/// 未来对同一路径产生不一致引用形态。镜像 POSIX 侧 `shell_single_quote` 与
/// `quote_path_if_spaced` 的"重量基础 + 轻量条件包装"两层结构。
#[cfg(target_os = "windows")]
fn win_double_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

#[cfg(target_os = "windows")]
fn windows_cmd_double_quote_arg(value: &str) -> String {
    win_double_quote(value)
}

/// 获取单个工具的版本信息（内部实现）
async fn get_single_tool_version_impl(
    tool: &str,
    wsl_shell: Option<&str>,
    wsl_shell_flag: Option<&str>,
) -> ToolVersion {
    debug_assert!(
        VALID_TOOLS.contains(&tool),
        "unexpected tool name in get_single_tool_version_impl: {tool}"
    );

    // 判断该工具的运行环境 & WSL distro（如有）
    let (env_type, wsl_distro) = tool_env_type_and_wsl_distro(tool);

    // 使用全局 HTTP 客户端（已包含代理配置）
    let client = crate::proxy::http_client::get();

    // 1. 获取本地版本
    let probe = if let Some(distro) = wsl_distro.as_deref() {
        try_get_version_wsl(tool, distro, wsl_shell, wsl_shell_flag)
    } else {
        #[cfg(target_os = "windows")]
        {
            // Windows 上只执行已经定位到的真实可执行文件，避免 `cmd /C tool`
            // 误触发 App Execution Alias 或协议处理器。
            scan_cli_version(tool)
        }

        #[cfg(not(target_os = "windows"))]
        {
            // PATH 第一个命令优先；只有它确实没装(NotFound)才去常见目录兜底扫描。
            match try_get_version(tool) {
                ShellProbe::NotFound(_) => scan_cli_version(tool),
                found => found,
            }
        }
    };
    let (local_version, local_error, installed_but_broken) = match probe {
        ShellProbe::Found(v) => (Some(v), None, false),
        ShellProbe::FoundButFailed(e) => (None, Some(e), true),
        ShellProbe::NotFound(e) => (None, Some(e), false),
    };

    // 2. 获取远程最新版本
    let latest_version = match tool {
        "claude" => fetch_npm_latest_version(&client, "@anthropic-ai/claude-code").await,
        "codex" => fetch_npm_latest_version(&client, "@openai/codex").await,
        "gemini" => fetch_npm_latest_version(&client, "@google/gemini-cli").await,
        "opencode" => {
            if let Some(version) = fetch_npm_latest_version(&client, "opencode-ai").await {
                Some(version)
            } else {
                fetch_github_latest_version(&client, "anomalyco/opencode").await
            }
        }
        "openclaw" => fetch_npm_latest_version(&client, "openclaw").await,
        "hermes" => fetch_pypi_latest_version(&client, "hermes-agent").await,
        _ => None,
    };

    ToolVersion {
        name: tool.to_string(),
        version: local_version,
        latest_version,
        error: local_error,
        installed_but_broken,
        env_type,
        wsl_distro,
    }
}

/// Helper function to fetch latest version from npm registry
async fn fetch_npm_latest_version(client: &reqwest::Client, package: &str) -> Option<String> {
    let url = format!("https://registry.npmjs.org/{package}");
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json.get("dist-tags")
                    .and_then(|tags| tags.get("latest"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// Helper function to fetch latest version from GitHub releases
async fn fetch_github_latest_version(client: &reqwest::Client, repo: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    match client
        .get(&url)
        .header("User-Agent", "cc-switch")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json.get("tag_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.strip_prefix('v').unwrap_or(s).to_string())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// Helper function to fetch latest version from PyPI
async fn fetch_pypi_latest_version(client: &reqwest::Client, package: &str) -> Option<String> {
    let url = format!("https://pypi.org/pypi/{package}/json");
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json.get("info")
                    .and_then(|info| info.get("version"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// 预编译的版本号正则表达式
static VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\d+\.\d+\.\d+(-[\w.]+)?").expect("Invalid version regex"));

/// 从版本输出中提取纯版本号
fn extract_version(raw: &str) -> String {
    VERSION_RE
        .find(raw)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| raw.to_string())
}

/// 工具未安装时的统一错误文案；WSL 路径会再拼上 `[WSL:{distro}] ` 前缀。
const NOT_INSTALLED: &str = "not installed or not executable";

/// CLI 版本探测的三态结果，跨平台统一各 probe（`try_get_version` /
/// `try_get_version_wsl` / `scan_cli_version`）的返回，进而在 `ToolVersion` 上给出
/// 结构化的 `installed_but_broken` 信号——避免前端靠匹配错误文案反推语义。
///
/// 关键区分"没装"与"装了但 `--version` 自身报错退出"（如工具要求更高的 Node 版本）：
/// 后者必须如实上报、不去别处捞旧版掩盖，否则"升级到新版却跑不起来"会被旧版盖住，
/// 表现为"升级成功但版本号不变"。
enum ShellProbe {
    /// 成功拿到版本号
    Found(String),
    /// 可执行存在、但 `--version` 非零退出（携带诊断信息，如 stderr 末尾若干行）
    FoundButFailed(String),
    /// 没找到该命令（携带描述性消息，供 UI 展示）
    NotFound(String),
}

/// 在非 Windows 平台用用户 shell 执行 `{tool} --version` 探测版本。
///
/// Windows 不走此路径：`cmd /C {tool}` 可能误触发 App Execution Alias /
/// 协议处理器（曾导致 Windows 版整体被禁用），那里改由 `scan_cli_version`
/// 只执行已定位到的真实可执行文件。
#[cfg(not(target_os = "windows"))]
fn try_get_version(tool: &str) -> ShellProbe {
    use std::process::Command;

    let output = {
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| is_valid_shell(s))
            .unwrap_or_else(|| "sh".to_string());
        let flag = default_flag_for_shell(&shell);
        Command::new(shell)
            .arg(flag)
            .arg(format!("{tool} --version"))
            .output()
    };

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if out.status.success() {
                let raw = if stdout.is_empty() { &stderr } else { &stdout };
                if raw.is_empty() {
                    ShellProbe::NotFound(NOT_INSTALLED.to_string())
                } else {
                    ShellProbe::Found(extract_version(raw))
                }
            } else {
                // exit 127 = shell 找不到命令（可放心 fallback 到搜索路径）；其它非零码
                // = 命令存在但 --version 自身报错退出，须如实上报、不 fallback 掩盖。
                let err = if stderr.is_empty() { stdout } else { stderr };
                if out.status.code() == Some(127) || err.is_empty() {
                    ShellProbe::NotFound(NOT_INSTALLED.to_string())
                } else {
                    ShellProbe::FoundButFailed(last_lines(err.trim(), 4))
                }
            }
        }
        Err(_) => ShellProbe::NotFound(NOT_INSTALLED.to_string()),
    }
}

/// 校验 WSL 发行版名称是否合法
/// WSL 发行版名称只允许字母、数字、连字符和下划线
#[cfg(target_os = "windows")]
fn is_valid_wsl_distro_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Validate that the given shell name is one of the allowed shells.
fn is_valid_shell(shell: &str) -> bool {
    matches!(
        shell.rsplit('/').next().unwrap_or(shell),
        "sh" | "bash" | "zsh" | "fish" | "dash"
    )
}

/// Validate that the given shell flag is one of the allowed flags.
#[cfg(target_os = "windows")]
fn is_valid_shell_flag(flag: &str) -> bool {
    matches!(flag, "-c" | "-lc" | "-lic")
}

/// Return the default invocation flag for the given shell.
fn default_flag_for_shell(shell: &str) -> &'static str {
    match shell.rsplit('/').next().unwrap_or(shell) {
        "dash" | "sh" => "-c",
        "fish" => "-lc",
        _ => "-lic",
    }
}

#[cfg(target_os = "windows")]
fn try_get_version_wsl(
    tool: &str,
    distro: &str,
    force_shell: Option<&str>,
    force_shell_flag: Option<&str>,
) -> ShellProbe {
    use std::process::Command;

    // 防御性断言：tool 只能是预定义的值
    debug_assert!(VALID_TOOLS.contains(&tool), "unexpected tool name: {tool}");

    // 校验 distro 名称，防止命令注入
    if !is_valid_wsl_distro_name(distro) {
        return ShellProbe::NotFound(format!("[WSL:{distro}] invalid distro name"));
    }

    // 构建 Shell 脚本检测逻辑
    let (shell, flag, cmd) = if let Some(shell) = force_shell {
        // Defensive validation: never allow an arbitrary executable name here.
        if !is_valid_shell(shell) {
            return ShellProbe::NotFound(format!("[WSL:{distro}] invalid shell: {shell}"));
        }
        let shell = shell.rsplit('/').next().unwrap_or(shell);
        let flag = if let Some(flag) = force_shell_flag {
            if !is_valid_shell_flag(flag) {
                return ShellProbe::NotFound(format!("[WSL:{distro}] invalid shell flag: {flag}"));
            }
            flag
        } else {
            default_flag_for_shell(shell)
        };

        (shell.to_string(), flag, format!("{tool} --version"))
    } else {
        let cmd = if let Some(flag) = force_shell_flag {
            if !is_valid_shell_flag(flag) {
                return ShellProbe::NotFound(format!("[WSL:{distro}] invalid shell flag: {flag}"));
            }
            format!("\"${{SHELL:-sh}}\" {flag} '{tool} --version'")
        } else {
            // 兜底：自动尝试 -lic, -lc, -c
            format!(
                "\"${{SHELL:-sh}}\" -lic '{tool} --version' 2>/dev/null || \"${{SHELL:-sh}}\" -lc '{tool} --version' 2>/dev/null || \"${{SHELL:-sh}}\" -c '{tool} --version'"
            )
        };

        ("sh".to_string(), "-c", cmd)
    };

    let output = Command::new("wsl.exe")
        .args(["-d", distro, "--", &shell, flag, &cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if out.status.success() {
                let raw = if stdout.is_empty() { &stderr } else { &stdout };
                if raw.is_empty() {
                    ShellProbe::NotFound(format!("[WSL:{distro}] {NOT_INSTALLED}"))
                } else {
                    ShellProbe::Found(extract_version(raw))
                }
            } else {
                let err = if stderr.is_empty() { stdout } else { stderr };
                // wsl.exe 透传的退出码不总可靠，故同时用 exit 127 与 "command not found"
                // 文本兜底判别"没装"；其余非零退出视作"装了但 --version 报错"。
                let not_found = err.is_empty()
                    || out.status.code() == Some(127)
                    || err.contains("command not found")
                    || err.contains("not found");
                if not_found {
                    ShellProbe::NotFound(format!("[WSL:{distro}] {NOT_INSTALLED}"))
                } else {
                    ShellProbe::FoundButFailed(format!(
                        "[WSL:{distro}] {}",
                        last_lines(err.trim(), 4)
                    ))
                }
            }
        }
        Err(e) => ShellProbe::NotFound(format!("[WSL:{distro}] exec failed: {e}")),
    }
}

/// 非 Windows 平台的 WSL 版本检测存根
/// 注意：此函数实际上不会被调用，因为 `wsl_distro_from_path` 在非 Windows 平台总是返回 None。
/// 保留此函数是为了保持 API 一致性，防止未来重构时遗漏。
#[cfg(not(target_os = "windows"))]
fn try_get_version_wsl(
    _tool: &str,
    _distro: &str,
    _force_shell: Option<&str>,
    _force_shell_flag: Option<&str>,
) -> ShellProbe {
    ShellProbe::NotFound("WSL check not supported on this platform".to_string())
}

fn push_unique_path(paths: &mut Vec<std::path::PathBuf>, path: std::path::PathBuf) {
    if path.as_os_str().is_empty() {
        return;
    }

    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn push_env_single_dir(paths: &mut Vec<std::path::PathBuf>, value: Option<std::ffi::OsString>) {
    if let Some(raw) = value {
        push_unique_path(paths, std::path::PathBuf::from(raw));
    }
}

fn extend_from_path_list(
    paths: &mut Vec<std::path::PathBuf>,
    value: Option<std::ffi::OsString>,
    suffix: Option<&str>,
) {
    if let Some(raw) = value {
        for p in std::env::split_paths(&raw) {
            let dir = match suffix {
                Some(s) => p.join(s),
                None => p,
            };
            push_unique_path(paths, dir);
        }
    }
}

fn extend_from_cli_path_env(
    paths: &mut Vec<std::path::PathBuf>,
    value: Option<std::ffi::OsString>,
) {
    if let Some(raw) = value {
        for p in std::env::split_paths(&raw) {
            if should_skip_cli_path_env_dir(&p) {
                continue;
            }
            push_unique_path(paths, p);
        }
    }
}

fn should_skip_cli_path_env_dir(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        is_windows_app_execution_alias_dir(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        false
    }
}

#[cfg(target_os = "windows")]
fn is_windows_app_execution_alias_dir(path: &Path) -> bool {
    let normalized = path
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    normalized
        .trim_end_matches('\\')
        .ends_with("\\microsoft\\windowsapps")
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn push_env_child_dir(
    paths: &mut Vec<std::path::PathBuf>,
    value: Option<std::ffi::OsString>,
    child: &str,
) {
    if let Some(raw) = value {
        push_unique_path(paths, std::path::PathBuf::from(raw).join(child));
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn extend_existing_child_search_paths(
    paths: &mut Vec<std::path::PathBuf>,
    base: &Path,
    suffix: Option<&str>,
) {
    if !base.exists() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = match suffix {
                Some(suffix) => entry.path().join(suffix),
                None => entry.path(),
            };
            if path.exists() {
                push_unique_path(paths, path);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn extend_windows_cli_manager_search_paths(paths: &mut Vec<std::path::PathBuf>, home: &Path) {
    push_env_single_dir(paths, std::env::var_os("PNPM_HOME"));
    push_env_child_dir(paths, std::env::var_os("VOLTA_HOME"), "bin");
    push_env_single_dir(paths, std::env::var_os("NVM_SYMLINK"));
    push_env_child_dir(paths, std::env::var_os("SCOOP"), "shims");
    push_env_child_dir(paths, std::env::var_os("SCOOP_GLOBAL"), "shims");

    if let Some(nvm_home) = std::env::var_os("NVM_HOME") {
        let nvm_home = std::path::PathBuf::from(nvm_home);
        push_unique_path(paths, nvm_home.clone());
        extend_existing_child_search_paths(paths, &nvm_home, None);
    }

    if let Some(appdata) = dirs::data_dir() {
        let nvm_home = appdata.join("nvm");
        push_unique_path(paths, nvm_home.clone());
        extend_existing_child_search_paths(paths, &nvm_home, None);
    }

    if !home.as_os_str().is_empty() {
        push_unique_path(paths, home.join("scoop").join("shims"));
    }

    if let Some(local_data) = dirs::data_local_dir() {
        push_unique_path(paths, local_data.join("pnpm"));
        push_unique_path(paths, local_data.join("Volta").join("bin"));
        push_unique_path(paths, local_data.join("Yarn").join("bin"));
    }

    let program_data = std::env::var_os("ProgramData")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("C:\\ProgramData"));
    push_unique_path(paths, program_data.join("scoop").join("shims"));
}

/// OpenCode install.sh 路径优先级（见 https://github.com/anomalyco/opencode README）:
///   $OPENCODE_INSTALL_DIR > $XDG_BIN_DIR > $HOME/bin > $HOME/.opencode/bin
/// 额外扫描 Bun 默认全局安装路径（~/.bun/bin）
/// 和 Go 安装路径（~/go/bin、$GOPATH/*/bin）。
fn opencode_extra_search_paths(
    home: &Path,
    opencode_install_dir: Option<std::ffi::OsString>,
    xdg_bin_dir: Option<std::ffi::OsString>,
    gopath: Option<std::ffi::OsString>,
) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    push_env_single_dir(&mut paths, opencode_install_dir);
    push_env_single_dir(&mut paths, xdg_bin_dir);

    if !home.as_os_str().is_empty() {
        push_unique_path(&mut paths, home.join("bin"));
        push_unique_path(&mut paths, home.join(".opencode").join("bin"));
        push_unique_path(&mut paths, home.join(".bun").join("bin"));
        push_unique_path(&mut paths, home.join("go").join("bin"));
    }

    extend_from_path_list(&mut paths, gopath, Some("bin"));

    paths
}

fn tool_executable_candidates(tool: &str, dir: &Path) -> Vec<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        vec![
            dir.join(format!("{tool}.cmd")),
            dir.join(format!("{tool}.exe")),
            dir.join(tool),
        ]
    }

    #[cfg(not(target_os = "windows"))]
    {
        vec![dir.join(tool)]
    }
}

fn extend_mise_node_search_paths(paths: &mut Vec<std::path::PathBuf>, home: &Path) {
    if home.as_os_str().is_empty() {
        return;
    }

    let mise_base = home.join(".local/share/mise");
    push_unique_path(paths, mise_base.join("shims"));

    let node_installs = mise_base.join("installs").join("node");
    if node_installs.exists() {
        if let Ok(entries) = std::fs::read_dir(&node_installs) {
            for entry in entries.flatten() {
                let bin_path = entry.path().join("bin");
                if bin_path.exists() {
                    push_unique_path(paths, bin_path);
                }
            }
        }
    }
}

/// 构建某工具的候选搜索目录（原生安装优先，PATH 兜底）。
/// 单探兜底 (`scan_cli_version`) 与全量枚举 (`enumerate_tool_installations`) 共用，
/// 确保两条路径看到的是同一组安装位置。
fn build_tool_search_paths(tool: &str) -> Vec<std::path::PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();

    // 常见的安装路径（原生安装优先）
    let mut search_paths: Vec<std::path::PathBuf> = Vec::new();
    if !home.as_os_str().is_empty() {
        push_unique_path(&mut search_paths, home.join(".local/bin"));
        push_unique_path(&mut search_paths, home.join(".npm-global/bin"));
        push_unique_path(&mut search_paths, home.join("n/bin"));
        push_unique_path(&mut search_paths, home.join(".volta/bin"));
        extend_mise_node_search_paths(&mut search_paths, &home);
    }

    #[cfg(target_os = "macos")]
    {
        push_unique_path(
            &mut search_paths,
            std::path::PathBuf::from("/opt/homebrew/bin"),
        );
        push_unique_path(
            &mut search_paths,
            std::path::PathBuf::from("/usr/local/bin"),
        );
        if tool == "hermes" {
            let python_base = home.join("Library").join("Python");
            if python_base.exists() {
                if let Ok(entries) = std::fs::read_dir(&python_base) {
                    for entry in entries.flatten() {
                        let bin_path = entry.path().join("bin");
                        if bin_path.exists() {
                            push_unique_path(&mut search_paths, bin_path);
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        push_unique_path(
            &mut search_paths,
            std::path::PathBuf::from("/usr/local/bin"),
        );
        push_unique_path(&mut search_paths, std::path::PathBuf::from("/usr/bin"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = dirs::data_dir() {
            push_unique_path(&mut search_paths, appdata.join("npm"));
            if tool == "hermes" {
                let python_base = appdata.join("Python");
                if python_base.exists() {
                    if let Ok(entries) = std::fs::read_dir(&python_base) {
                        for entry in entries.flatten() {
                            let scripts_path = entry.path().join("Scripts");
                            if scripts_path.exists() {
                                push_unique_path(&mut search_paths, scripts_path);
                            }
                        }
                    }
                }
            }
        }
        if tool == "hermes" {
            if let Some(local_data) = dirs::data_local_dir() {
                let programs_python = local_data.join("Programs").join("Python");
                if programs_python.exists() {
                    if let Ok(entries) = std::fs::read_dir(&programs_python) {
                        for entry in entries.flatten() {
                            let scripts_path = entry.path().join("Scripts");
                            if scripts_path.exists() {
                                push_unique_path(&mut search_paths, scripts_path);
                            }
                        }
                    }
                }
            }
        }
        push_unique_path(
            &mut search_paths,
            std::path::PathBuf::from("C:\\Program Files\\nodejs"),
        );
        extend_windows_cli_manager_search_paths(&mut search_paths, &home);
    }

    let fnm_base = home.join(".local/state/fnm_multishells");
    if fnm_base.exists() {
        if let Ok(entries) = std::fs::read_dir(&fnm_base) {
            for entry in entries.flatten() {
                let bin_path = entry.path().join("bin");
                if bin_path.exists() {
                    push_unique_path(&mut search_paths, bin_path);
                }
            }
        }
    }

    let nvm_base = home.join(".nvm/versions/node");
    if nvm_base.exists() {
        if let Ok(entries) = std::fs::read_dir(&nvm_base) {
            for entry in entries.flatten() {
                let bin_path = entry.path().join("bin");
                if bin_path.exists() {
                    push_unique_path(&mut search_paths, bin_path);
                }
            }
        }
    }

    if tool == "opencode" {
        let extra_paths = opencode_extra_search_paths(
            &home,
            std::env::var_os("OPENCODE_INSTALL_DIR"),
            std::env::var_os("XDG_BIN_DIR"),
            std::env::var_os("GOPATH"),
        );

        for path in extra_paths {
            push_unique_path(&mut search_paths, path);
        }
    }

    let path_env = std::env::var_os("PATH");
    extend_from_cli_path_env(&mut search_paths, path_env);
    search_paths
}

/// 扫描常见路径查找 CLI（PATH 主命令未命中时的兜底单探）。
fn scan_cli_version(tool: &str) -> ShellProbe {
    use std::process::Command;

    let search_paths = build_tool_search_paths(tool);
    let current_path = std::env::var_os("PATH")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();

    // 记录"可执行文件存在、但 `--version` 非零退出"时的首个诊断信息。
    // 典型场景：工具已安装但当前环境跑不起来（如 openclaw 要求 Node v22.19+）。
    // 这类信息比笼统的 "not installed" 有用得多，循环结束未探到版本时回传。
    let mut exec_diagnostic: Option<String> = None;

    for path in &search_paths {
        #[cfg(target_os = "windows")]
        let new_path = format!("{};{}", path.display(), current_path);

        #[cfg(not(target_os = "windows"))]
        let new_path = format!("{}:{}", path.display(), current_path);

        for tool_path in tool_executable_candidates(tool, path) {
            if !tool_path.exists() {
                continue;
            }

            #[cfg(target_os = "windows")]
            let output = {
                Command::new("cmd")
                    .args(["/C", &format!("\"{}\" --version", tool_path.display())])
                    .env("PATH", &new_path)
                    .creation_flags(CREATE_NO_WINDOW)
                    .output()
            };

            #[cfg(not(target_os = "windows"))]
            let output = {
                Command::new(&tool_path)
                    .arg("--version")
                    .env("PATH", &new_path)
                    .output()
            };

            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                if out.status.success() {
                    let raw = if stdout.is_empty() { &stderr } else { &stdout };
                    if !raw.is_empty() {
                        return ShellProbe::Found(extract_version(raw));
                    }
                } else if exec_diagnostic.is_none() {
                    let detail = if stderr.is_empty() { stdout } else { stderr };
                    let detail = detail.trim();
                    if !detail.is_empty() {
                        exec_diagnostic = Some(last_lines(detail, 4));
                    }
                }
            }
        }
    }

    // 有诊断 = 找到了可执行文件但 --version 报错（装了跑不起来）；否则视作未安装。
    match exec_diagnostic {
        Some(detail) => ShellProbe::FoundButFailed(detail),
        None => ShellProbe::NotFound(NOT_INSTALLED.to_string()),
    }
}

/// 单个工具在系统中的一处安装，用于"多处安装互相打架"的冲突诊断。
/// 字段保持 snake_case（与 `ToolVersion` 一致），前端按同名字段读取。
#[derive(Debug, serde::Serialize)]
pub struct ToolInstallation {
    /// 候选入口路径（用户实际在 PATH 里看到/输入的那个，未解析软链）。
    path: String,
    /// `--version` 成功时解析出的版本号。
    version: Option<String>,
    /// `--version` 是否 exit 0（装了且能在当前环境跑起来）。
    runnable: bool,
    /// 跑不起来时的诊断信息末尾若干行。
    error: Option<String>,
    /// 由路径前缀推断的安装来源（nvm/homebrew/...），驱动 UI 徽章。
    source: String,
    /// 是否为 PATH 解析到的那处（= 命令行默认，也是升级会作用的目标）。
    is_path_default: bool,
    /// 卸载该处的命令(仅展示给用户复制,UI 不代执行)。源感知 + 真身感知:
    /// brew formula → `brew uninstall <formula>`、volta → `volta uninstall`、bun →
    /// `bun rm -g`、pnpm → `pnpm rm -g`、其余 npm 全局 → 锚定那处同目录 npm 的
    /// `npm rm -g`(附物理删除兜底)。复用 `brew_formula_from_path`、`infer_install_source`、
    /// `sibling_bin`,与升级路径(`anchored_command_from_paths`)共享真身判定——避免
    /// 前端按 `source` 字符串拼命令时把"homebrew formula"和"homebrew node 的 npm 全局包"
    /// 混为一谈。
    ///
    /// **eager 计算(与升级命令按需算的不对称是 by design)**:升级命令在
    /// `installs_anchored_command` 里只为 default install 算一次;卸载命令则**每处
    /// install 都预计算**进字段,因为冲突诊断 UI 需要为每处展示对应的卸载 hint——
    /// 延后到二次 IPC 会增加"诊断 → 显示命令"的端到端延迟。计算成本是纯字符串
    /// 扫描(评估:8 install × ~2μs ≈ 16μs,可忽略),不影响 probe 1-3s 主时延。
    uninstall_command: String,
    /// `uninstall_command` 是否是 Windows cmd 兼容形式(含 quoted 路径,PowerShell
    /// 用户需 `&` call operator 前缀)。前端据此**条件渲染**一行 PowerShell 限制提示。
    ///
    /// **必须由后端提供结构化信号、前端不能 string match**:POSIX 的
    /// `shell_single_quote` 转义形式是 `'"'"'`(关单引号 + 双引号包字面单引号 +
    /// 重开)——含双引号!所以 macOS 用户名带 `'`(如 `o'leary`)时,POSIX 卸载命令
    /// **巧合地也含 `"`**,前端 `cmd.includes('"')` 会**误判为 Windows cmd 形式**,
    /// 给 POSIX 用户错误显示 PowerShell 提示。string-based 平台推断在这里 false-positive。
    ///
    /// 后端用 `cfg!(target_os = "windows") && uninstall_command.contains('"')` 算:
    /// `cfg!` 是**编译时 ground truth**(不是字符串猜测),POSIX 编译时整体短路为
    /// false,与 POSIX 单引号 escape 含 `"` 的形态完全无关。
    uninstall_command_needs_cmd_hint: bool,
    /// canonicalize 解析后的真身路径(brew 形如 `Cellar/<formula>/...`、claude 原生形如
    /// `~/.local/share/claude/versions/...`),用于 `anchored_command_from_paths` 的真身
    /// 判定。`enumerate_tool_installations` 已经为去重算过一次,这里复用避免上游
    /// `installs_anchored_command` 再 canonicalize 一遍——消除冗余 syscall + 闭合
    /// "enumerate 与 anchor 看到同一真身"的一致性边界(否则两次 canonicalize 之间
    /// symlink 被换会让锚定指向不同真身)。`#[serde(skip)]` 不外露给前端。
    #[serde(skip)]
    real: std::path::PathBuf,
}

/// 由可执行文件路径前缀推断安装来源。纯字符串匹配、无副作用。
/// 顺序敏感：Homebrew 的 Cellar 真身要先于通用规则命中。
fn infer_install_source(path: &Path) -> &'static str {
    let s = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    if s.contains("/.nvm/") {
        "nvm"
    } else if s.contains("/homebrew/") || s.contains("/cellar/") {
        "homebrew"
    // `.volta` 是 macOS/Linux 默认安装(`~/.volta/bin`),`/volta/` 兜底覆盖
    // Windows 的 `%LOCALAPPDATA%\Volta\bin` / `%VOLTA_HOME%\bin`(无前导点)。
    } else if s.contains("/.volta/") || s.contains("/volta/") {
        "volta"
    } else if s.contains("fnm_multishells") {
        "fnm"
    } else if s.contains("/mise/") {
        "mise"
    } else if s.contains("/.bun/") {
        "bun"
    // pnpm 全局包目录: macOS 一般 `~/.local/share/pnpm`(已 normalize 到 `/pnpm/`)
    // 与 Windows `%LOCALAPPDATA%\pnpm` / `%PNPM_HOME%` 都命中 `/pnpm/`。
    } else if s.contains("/pnpm/") {
        "pnpm"
    } else if s.contains("/scoop/") {
        "scoop"
    } else if s.contains("/library/python")
        || s.contains("/scripts/")
        || s.contains("/site-packages/")
    {
        "pip"
    } else {
        "system"
    }
}

/// 从 shell 输出里挑出第一个绝对路径行（trim 后以 `/` 开头），跳过交互式登录 shell
/// （`-lic`）里 .zshrc 打印的欢迎语/提示符等噪音。canonicalize 由调用方做（碰 FS）。
#[cfg(not(target_os = "windows"))]
fn first_abs_path_line(raw: &str) -> Option<&str> {
    raw.lines().map(str::trim).find(|l| l.starts_with('/'))
}

/// 用与 `try_get_version` 相同的登录 shell 解析 PATH 默认命中的可执行文件路径，
/// canonicalize 后作为"命令行默认 / 升级目标"的锚点（与升级会作用的那处对齐）。
#[cfg(not(target_os = "windows"))]
fn resolve_path_default(tool: &str) -> Option<std::path::PathBuf> {
    use std::process::Command;
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| is_valid_shell(s))
        .unwrap_or_else(|| "sh".to_string());
    let flag = default_flag_for_shell(&shell);
    let out = Command::new(shell)
        .arg(flag)
        .arg(format!("command -v {tool}"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    // 不能死取第一行：交互式 .zshrc 可能先打印欢迎语（如 "🚀 Welcome back"），
    // command -v 的真实路径在其后；取第一个 `/` 开头的行才稳。
    let first = first_abs_path_line(&raw)?;
    std::fs::canonicalize(first).ok()
}

#[cfg(target_os = "windows")]
fn resolve_path_default(tool: &str) -> Option<std::path::PathBuf> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    let out = Command::new("cmd")
        .args(["/C", &format!("where {tool}")])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    let first = raw.lines().next()?.trim();
    if first.is_empty() {
        return None;
    }
    std::fs::canonicalize(first).ok()
}

/// 枚举工具在系统中的所有安装（不短路）。与 `scan_cli_version` 共用
/// `build_tool_search_paths`，但不在首个命中处停止——而是对每个去重后的真实
/// 可执行文件都跑一次 `--version`，从而能发现"升级写入 A 处、PATH 实际用 B 处"。
fn enumerate_tool_installations(tool: &str) -> Vec<ToolInstallation> {
    use std::process::Command;

    let search_paths = build_tool_search_paths(tool);
    let current_path = std::env::var_os("PATH")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let path_default = resolve_path_default(tool);

    let mut seen: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();
    let mut installs: Vec<ToolInstallation> = Vec::new();

    for dir in &search_paths {
        #[cfg(target_os = "windows")]
        let new_path = format!("{};{}", dir.display(), current_path);
        #[cfg(not(target_os = "windows"))]
        let new_path = format!("{}:{}", dir.display(), current_path);

        for tool_path in tool_executable_candidates(tool, dir) {
            if !tool_path.exists() {
                continue;
            }
            // canonicalize 解析软链后去重：/opt/homebrew/bin/x → Cellar/...、nvm shim 等
            // 多个入口可能指向同一真实文件，只算一处安装。
            let real = std::fs::canonicalize(&tool_path).unwrap_or_else(|_| tool_path.clone());
            if !seen.insert(real.clone()) {
                continue;
            }

            #[cfg(target_os = "windows")]
            let output = {
                use std::os::windows::process::CommandExt;
                Command::new("cmd")
                    .args(["/C", &format!("\"{}\" --version", tool_path.display())])
                    .env("PATH", &new_path)
                    .creation_flags(CREATE_NO_WINDOW)
                    .output()
            };
            #[cfg(not(target_os = "windows"))]
            let output = Command::new(&tool_path)
                .arg("--version")
                .env("PATH", &new_path)
                .output();

            let (version, runnable, error) = match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    let raw = if stdout.is_empty() { stderr } else { stdout };
                    (Some(extract_version(&raw)), true, None)
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    let detail = if stderr.is_empty() { stdout } else { stderr };
                    let detail = detail.trim();
                    let error = if detail.is_empty() {
                        None
                    } else {
                        Some(last_lines(detail, 4))
                    };
                    (None, false, error)
                }
                Err(e) => (None, false, Some(e.to_string())),
            };

            let is_path_default = path_default.as_ref() == Some(&real);
            let path_str = tool_path.display().to_string();
            // **source 在这里算一次,同时供 ToolInstallation.source 字段和
            // uninstall_command_from_paths 的 match 用**(review N1):函数签名上
            // source 作为参数传入,避免函数内部再 `infer_install_source(Path::new(bin_path))`
            // 二次扫描 + 重复 lower-case 分配。同时把"路径分类"的契约外化——上下游
            // 都以同一标签作判定,不会因函数内外重算意外漂移。
            let source = infer_install_source(&tool_path);
            // 卸载提示与升级锚定共用同一份 (bin_path, real) —— 见
            // `uninstall_command_from_paths` 的 doc:**前端按 source 字符串拼无法
            // 区分 brew formula vs homebrew npm 全局包**,只能在后端拿真身判定。
            let uninstall_command =
                uninstall_command_from_paths(tool, &path_str, &real.to_string_lossy(), source);
            // PowerShell hint 触发位:Windows + 命令含 quoted 路径。`cfg!` 是编译时
            // 常量,POSIX 编译时整体短路为 false——这就把 POSIX 单引号 escape `'"'"'`
            // 也含 `"` 的字面巧合从触发域里排除掉(见字段 doc)。
            let uninstall_command_needs_cmd_hint =
                cfg!(target_os = "windows") && uninstall_command.contains('"');

            installs.push(ToolInstallation {
                path: path_str,
                version,
                runnable,
                error,
                source: source.to_string(),
                is_path_default,
                uninstall_command,
                uninstall_command_needs_cmd_hint,
                // 复用上面 line ~1357 已 canonicalize 的真身,避免下游
                // installs_anchored_command 再 canonicalize 一遍同一文件。
                real: real.clone(),
            });
        }
    }

    // PATH 默认那处排最前，UI 一眼看到"命令行默认用的是哪处"。
    installs.sort_by_key(|i| std::cmp::Reverse(i.is_path_default));
    installs
}

/// 工具对应的 npm 包名（hermes 走 pip，不在此表）。锚定升级据此拼 `npm i -g`。
/// 全平台共用一张表——Windows 锚定层(`anchored_command_from_paths` 的 windows 版)也读这里。
fn npm_package_for(tool: &str) -> Option<&'static str> {
    match tool {
        "claude" => Some("@anthropic-ai/claude-code"),
        "codex" => Some("@openai/codex"),
        "gemini" => Some("@google/gemini-cli"),
        "opencode" => Some("opencode-ai"),
        "openclaw" => Some("openclaw"),
        _ => None,
    }
}

/// 取路径的父目录(纯字符串截断,不碰 fs):`/a/b/npm` → `/a/b`、`C:\a\b\npm.cmd`
/// → `C:\a\b`、混合分隔符 `C:\a/b\npm` → `C:\a/b`。无父目录返回空串。
///
/// 平台无关:`\` 和 `/` 都识别,取两者最右出现位置。`Option<usize>` 的 Ord 让
/// `None < Some(_)`,所以 `rfind('\\').max(rfind('/'))` 自动取存在的那个、两者都
/// 存在时取靠右的——比 `or_else` 优先取一种正确(混合分隔符不会拿错父目录)。
/// 跨平台 fs separator 在两侧均接受,使 macOS/Linux 上的 cargo test 也能跑 Windows
/// 路径用例(`parent_dir_cases::mixed_separators_takes_rightmost`)。空串语义由上游
/// `sibling_bin` 的 `is_empty()` 检查转成 None → 锚定整体退化到静态兜底。
fn parent_dir(p: &str) -> String {
    match p.rfind('\\').max(p.rfind('/')) {
        Some(i) if i > 0 => p[..i].to_string(),
        _ => String::new(),
    }
}

/// 从 canonicalize 后的真身路径提取 Homebrew formula 名：
/// `/opt/homebrew/Cellar/gemini-cli/0.13.0/...` → `Some("gemini-cli")`。
/// 非 Cellar 路径（= 不是 formula，可能是 Homebrew 的 node 装的 npm 全局包）返回 None。
/// 关键区分：formula 即便内部用 node，真身也落在 `Cellar/<formula>/` 下；而 Homebrew
/// npm 全局包落在 `/opt/homebrew/lib/node_modules`（不含 Cellar）。两者升级命令不同。
#[cfg(not(target_os = "windows"))]
fn brew_formula_from_path(real: &str) -> Option<String> {
    let mut segs = real.split('/');
    while let Some(seg) = segs.next() {
        if seg.eq_ignore_ascii_case("Cellar") {
            return segs.next().filter(|s| !s.is_empty()).map(|s| s.to_string());
        }
    }
    None
}

/// 含空格才用 POSIX 单引号包一层,否则保持裸路径——命令展示更干净。
/// claude / brew / volta / bun / npm 五个锚定分支共用,避免"含空格"判定漂移。
///
/// **仅按空格判定,不防其他 shell 元字符**(`$` / `` ` `` / `'` / `"` / `;` 等)。
/// 调用方传入的是探测得到的可执行路径(`enumerate_tool_installations` 里来源于
/// `Path::display()`),实际 macOS/Linux 上 home dir 名几乎不允许这类字符、
/// npm/brew/volta/bun 也不会装到含这类字符的路径,与 diff 前内联在 npm 分支里的
/// `if npm.contains(' ')` 实现等价。若未来要扩广,改成 `shell_single_quote` 无条件
/// 包裹即可,但会失去"无空格时的清洁展示"。
#[cfg(not(target_os = "windows"))]
fn quote_path_if_spaced(p: &str) -> String {
    if p.contains(' ') {
        shell_single_quote(p)
    } else {
        p.to_string()
    }
}

/// **卸载 hint 专用 POSIX quoting**——白名单判定:全字符都在
/// `[A-Za-z0-9._/+=:@-]` 安全集合内 → 保持裸路径(常规 nvm/brew/volta/bun 路径
/// 以及 npm scope 包名 `@scope/pkg` 都命中,命令展示干净);任何不在集合的字符
/// (包括空格、`;`、`$`、`` ` ``、`'`、`"`、`&`、`|`、`<`、`>`、`(`、`)`、`*`、
/// `?`、`[`、`]`、`{`、`}`、`!`、`#`、换行、tab 等)→ 走 `shell_single_quote`
/// 无条件强 quote。
///
/// **白名单含 `@`** :npm scope 包名形如 `@anthropic-ai/claude-code` /
/// `@google/gemini-cli`,pkg_dir 必然含 `@`;且 `@` 在 bash/zsh/dash/sh 都无特殊
/// 含义(zsh 的 `${var@P}` 是 parameter expansion 的修饰符,需要前缀 `$`,单独 `@`
/// 字符在路径里无害),纳入白名单不引入风险。
///
/// **为什么不复用 `quote_path_if_spaced`**(P2 修复):后者只查空格,但卸载命令
/// 是 destructive(`rm -rf <pkg_dir>`)——含 `;` / `` ` `` / `$(...)` 的路径会让
/// shell 把卸载链拆成多条命令、把 rm 落到错的路径。升级命令"出错"是 inconvenient,
/// 但卸载侧的 quoting 漏洞是 destructive risk,**安全门槛必须更高**。
///
/// **为什么用白名单而非黑名单**:shell 元字符列表长且 shell-specific(bash vs zsh
/// vs dash 略有差异),黑名单容易漏。白名单只允许"绝对安全字符",其他一律 quote
/// ——default-quoting 比 default-bare 更适合 destructive 命令。常规 POSIX 安装
/// 路径 + npm 包名只用安全字符,白名单不影响展示美观。
///
/// 路径含安全集合外字符(如 home dir 名带 `;` `'` `$`)时,`shell_single_quote`
/// 把单引号 escape 为 `'"'"'`(关引号 + 双引号包单引号 + 重开引号),跨
/// bash/zsh/dash/sh 全部 POSIX 兼容。
#[cfg(not(target_os = "windows"))]
fn posix_quote_for_user_shell(p: &str) -> String {
    let is_safe = |c: char| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '+' | '=' | ':' | '-' | '@')
    };
    if p.chars().all(is_safe) {
        p.to_string()
    } else {
        shell_single_quote(p)
    }
}

/// 锚定路径走 `.bat` 文件且**被 `call` 调用**,需要为 batch 特殊字符做两层防御:
///
/// **(1) `%` 经历两轮 percent expansion → 用 4 个 `%` 转义**。.bat 中字面 `%` 的
/// 标准转义是 `%%`,但 `call` 命令(Microsoft `call /?`:"percent (%) expansion is
/// performed on each parameter")**在 batch parser 处理完 `%%` → `%` 后自己再做一轮**。
/// 所以源 .bat 里写 `%%FOO%%`,batch 一轮变 `%FOO%`,call 二轮当成 variable reference
/// 又展开一次——要让最终 call 看到字面 `%FOO%` 必须写 `%%%%FOO%%%%`(一轮 → `%%FOO%%`,
/// 二轮 → `%FOO%` 字面)。这是 cmd 唯一**引号无法保护**的字符:引号内的 `%` 仍参与
/// 两轮 expansion。
///
/// **(2) token 边界 / escape 字符触发外层双引号**:`' '` `'&'` `'('` `')'` `'^'`
/// `';'` `'<'` `'>'` `'|'` `','` 任一出现即包引号。NTFS 允许这些字符出现在路径中,
/// 不包会让 cmd 把路径切成多 token、`^` 又会触发 escape;引号内它们是字面意义,
/// 而且 call 二次解析对引号内的它们也不会做特殊处理(`^` 在引号内失去 escape 作用,
/// token 边界字符在引号内是字面)。
///
/// `!`(delayed expansion)只在 `setlocal enabledelayedexpansion` 下生效——我们
/// .bat 头只有 `@echo off`、没开,所以不需要处理。`'` 在 cmd 中无特殊意义。
///
/// 镜像 POSIX `quote_path_if_spaced` 的"轻量条件包装"语义:不含任何特殊字符就保持
/// 裸路径(命令展示更干净),否则用 `win_double_quote` 包并做必要转义。
#[cfg(target_os = "windows")]
fn win_quote_path_for_batch(p: &str) -> String {
    // `%` 经历两轮 expansion:.bat parser 一轮 + `call` 二轮(Microsoft `call /?`:
    // "percent (%) expansion is performed on each parameter")。要让 call 最终看到
    // 字面 `%` 需要 4 个 → `%%%%`(batch 一轮 → `%%`,call 二轮 → `%` 字面)。
    // 引号内仍参与两轮 expansion,所以这一步独立于外层引号、必须无条件做。
    let escaped = if p.contains('%') {
        p.replace('%', "%%%%")
    } else {
        p.to_string()
    };
    // 注:`needs_quote` 基于**原路径** `p` 判断,不能用 `escaped`——后者引入的 `%`
    // 字符不算"特殊触发字符",否则含 `%` 的路径会被错误地额外加引号。
    let needs_quote = p
        .chars()
        .any(|c| matches!(c, ' ' | '&' | '(' | ')' | '^' | ';' | '<' | '>' | '|' | ','));
    if needs_quote {
        win_double_quote(&escaped)
    } else {
        escaped
    }
}

/// **复制给用户的命令**专用 Windows quoting,**目标语法为 cmd**——与
/// `win_quote_path_for_batch` 的关键差异是**不做 `%` 转义**(本 helper 服务的
/// 命令不进 .bat + call,只一轮 expansion 不需要 4 倍)。
///
/// **PowerShell 用户的已知限制**(P3, 2026-05-24):本 helper 生成的 quoted 路径
/// (含空格时 `"C:\Program Files\nodejs\npm.cmd"`)在 PowerShell 下**当字符串字面
/// 值,不会执行**——PowerShell 对含引号的可执行路径需要 call operator `&` 前缀:
///
/// ```text
///     cmd:        "C:\Program Files\nodejs\npm.cmd" rm -g pkg  ← OK
///     PowerShell: "C:\Program Files\nodejs\npm.cmd" rm -g pkg  ← 当字符串字面
///     PowerShell: & "C:\Program Files\nodejs\npm.cmd" rm -g pkg  ← OK
///     cmd:        & "C:\Program Files\nodejs\npm.cmd" rm -g pkg  ← `&` 是命令分隔符,报错
/// ```
///
/// 两个 shell 在引号路径上语法完全不兼容,无法用一份 quoting 同时满足。本 helper
/// 选 cmd 兼容形式(Windows 历史默认 shell;路径不含空格时 PowerShell 也能直接跑,
/// 因为无引号路径在 PowerShell 下走 unquoted invocation)。**含空格路径的 PowerShell
/// 用户需手动在命令前加 `&`** ——前端 UI 文案应明示此限制(否则用户照 hint 复制
/// 粘到 PowerShell 会看到"字符串回显但什么都没执行"的迷惑现象)。
///
/// **`%` 字符的语义对比**:cmd 一轮展开会把 `%foo%` 当变量;PowerShell / bash on
/// Windows / WSL 都无 % 展开(字面识别)。三套语义无法共存,**保持原始 `%`**
/// 是最诚实的兜底——cmd 用户在含 `%` 路径下仍会被变量展开困住(cmd 固有限制,
/// 双引号内 `%` 仍展开,任何 quoting 救不了);若用 batch quoter 的 4 倍 `%`,
/// 则三个 shell 全部得到错路径(cmd 一轮变 `%%`、PowerShell/bash 字面 `%%%%`)。
///
/// **本 helper 与 `win_quote_path_for_batch` 必须保持镜像的部分**:token 边界字符
/// (空格 & ( ) ^ ; < > | ,)的双引号包裹规则一致——这些字符在三个 shell 下都
/// 用引号包裹保持字面,跨 shell 一致。
///
/// 使用场景仅限"卸载提示之类只展示不代执行的命令";.bat + call 链路坚决用
/// `win_quote_path_for_batch`,否则两轮 expansion 下 % 字面会丢失。
#[cfg(target_os = "windows")]
fn win_quote_path_for_user_shell(p: &str) -> String {
    let needs_quote = p
        .chars()
        .any(|c| matches!(c, ' ' | '&' | '(' | ')' | '^' | ';' | '<' | '>' | '|' | ','));
    if needs_quote {
        win_double_quote(p)
    } else {
        p.to_string()
    }
}

/// Windows 版 sibling 推导:在 `<bin_path 父目录>` 下按 `ext_candidates` 顺序找
/// 第一个存在的 `<exe_basename>.<ext>` 文件,返回该绝对路径。
///
/// **与 POSIX `sibling_bin` 的关键区别:这里碰 fs**——Windows 上 npm/pnpm 的入口
/// 实际扩展名可能是 `.cmd` 也可能是 `.exe`(Node.js installer 装的是 `npm.cmd`、
/// 部分 pnpm 是 `pnpm.exe`),纯字符串拼接无法知道哪个真的存在,猜错会拼出
/// "GUI 执行时 file not found" 的命令。fs 检查放进 helper、单测用 tempdir 覆盖,
/// 让上层 `anchored_command_from_paths` 仍保持"接收已锚定路径"的接口形态。
///
/// **TOCTOU 是 by design**:预检 `is_file` 是为了让确认对话框展示真实命令字符串;
/// 检查到执行之间被外部进程(卸载器 / nvm switch / 杀软隔离)移走文件 → cmd /C
/// 报 ENOENT,toast 显示错误。不要在执行前再做二次预检——双重 syscall 也解决不了 race。
///
/// 候选扩展名顺序按工具 idiom:npm/pnpm 优先 `.cmd`(node 装的),volta 优先 `.exe`
/// (Volta 是 Rust 写的 native binary)。
///
/// **不用 `which::which_in` 的理由**:per-tool 扩展名优先级(volta 偏 `.exe`、npm/pnpm
/// 偏 `.cmd`)与 PATHEXT 的固定顺序不一致,而且只为这一处加 `which` 依赖收益不抵 audit
/// surface。`PathBuf::join` 让 separator 选择交给 std,避免 `format!("{dir}\\...")`
/// 硬编码 `\\` 在混合分隔符 bin_path 下产出丑陋路径。
///
/// 空 dir 或所有候选都不存在 → None,上游退化到静态命令,与 POSIX 路径同款语义。
#[cfg(target_os = "windows")]
fn sibling_bin_with_ext(
    bin_path: &str,
    exe_basename: &str,
    ext_candidates: &[&str],
) -> Option<String> {
    let dir = parent_dir(bin_path);
    if dir.is_empty() {
        return None;
    }
    let dir = std::path::PathBuf::from(dir);
    for ext in ext_candidates {
        let candidate = dir.join(format!("{exe_basename}.{ext}"));
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

/// 返回 `<bin_path 同目录>/<exe>` 的绝对路径。bin_path 是命令行命中的入口
/// (如 `/opt/homebrew/bin/gemini`、`~/.volta/bin/codex`),`exe` 是与之共处一个
/// bin 目录的另一个可执行(`brew` / `volta` / `bun` / `npm`)——这些包管理器
/// 都把自己的 cli 跟它们安装的命令并列放在同一个 bin 目录,所以"同目录推导"
/// 是可靠的绝对路径来源。
///
/// **dir 为空(bin_path 不含 `/`) → 返回 None**:此时无法推导出绝对路径,让上游
/// `anchored_command_from_paths` 整体退化为 None,调用方落到静态命令兜底——而非
/// 悄悄拼出 `npm i -g <pkg>` 这种依赖 PATH 的指令,违背"必须绝对路径"不变量。
/// 实际从 `enumerate_tool_installations` 走的 bin_path 都是 `Path::display()` 出
/// 来的绝对路径,这条防线不期望被触发,但闭合了 helper 与函数文档的语义一致。
#[cfg(not(target_os = "windows"))]
fn sibling_bin(bin_path: &str, exe: &str) -> Option<String> {
    let dir = parent_dir(bin_path);
    if dir.is_empty() {
        None
    } else {
        Some(format!("{dir}/{exe}"))
    }
}

/// 卸载 hint 专用:取 sibling exe 绝对路径并 quote;推不出 sibling 时退化为**裸命令名**
/// (让用户终端 PATH 默认那个工具处理)。
///
/// **与 anchored upgrade 不返 None 的差异**:`anchored_command_from_paths` 的 POSIX 版
/// 用 `?` 短路返 `None`,让上游退化到静态命令并标 `anchored=false`;卸载 hint 是直接
/// 给用户复制粘贴的——返 None 等于"不展示命令",诚实但 UX 差。退化为裸命令(无路径
/// 前缀)反而是"明确意图 + 让用户终端自决"的合理兜底,与 `static_fallback_command` 同款。
///
/// **quoting 用 `posix_quote_for_user_shell` 而非 `quote_path_if_spaced`**(P2 修复):
/// 卸载命令是 destructive(部分分支带 `rm -rf` 兜底),含 `;` / `` ` `` / `$` 的路径
/// 必须强 quote 防止 shell 拆词出错——detail 见 `posix_quote_for_user_shell` doc。
///
/// 仅在 POSIX uninstall 的 brew/volta/bun/pnpm/npm 五条分支共用,消除"sibling_bin →
/// map → unwrap_or_else 裸字面" 三行 boilerplate 的重复出现。
#[cfg(not(target_os = "windows"))]
fn quoted_sibling_or_bare(bin_path: &str, exe: &str) -> String {
    sibling_bin(bin_path, exe)
        .map(|b| posix_quote_for_user_shell(&b))
        .unwrap_or_else(|| exe.to_string())
}

/// Windows 版镜像 `quoted_sibling_or_bare`,quoter 用 `win_quote_path_for_user_shell`
/// (不做 4× % 转义,目标是复制到用户 shell 而非 .bat + call)。fallback 是 exe_basename
/// 裸名(`"npm"`/`"pnpm"`/`"volta"`),与原 `format!("npm rm -g {pkg}")` 兜底一致——
/// 不返回带扩展名的 `npm.cmd`,因为用户 shell 上 `npm` 已被 PATH 解析时自动加扩展。
#[cfg(target_os = "windows")]
fn quoted_sibling_or_bare_with_ext(
    bin_path: &str,
    exe_basename: &str,
    ext_candidates: &[&str],
) -> String {
    sibling_bin_with_ext(bin_path, exe_basename, ext_candidates)
        .map(|p| win_quote_path_for_user_shell(&p))
        .unwrap_or_else(|| exe_basename.to_string())
}

/// 给定工具、原始 bin 路径（命令行命中的入口）、canonicalize 后的真身路径，
/// 推断"写回同一处"的锚定升级命令。**POSIX 版是纯函数（不碰 FS）**——真实 canonicalize
/// 由调用方做（`installs_anchored_command` 复用 enumerate 时算出的 `inst.real`),
/// 便于单测覆盖各包管理器分支。Windows 版同名函数因 sibling 扩展名歧义必须读 fs,
/// 是刻意保留的平台差异(详见 Windows 版本 doc)。hermes（pip）不在此处:
/// `npm_package_for` 返回 None → None。
///
/// **对偶函数**:`uninstall_command_from_paths` 是本函数在卸载侧的镜像,共享 brew
/// formula 真身判定 + sibling 锚定 + source 等价类。任一边新增/移除等价类都得
/// 同步另一边——卸载侧 doc 对此点有更详细的不变量说明。
///
/// **关键不变量：返回的命令必须用绝对路径调用执行体，不依赖 PATH**。
/// 这条命令最终在 `run_tool_lifecycle_silently` 的非登录 `bash -c` 里执行——
/// GUI App 启动的进程 PATH 由 launchd / Windows Service / systemd 给,通常**不含**
/// `~/.local/bin` / `/opt/homebrew/bin` / `~/.volta/bin` 等用户级 bin 目录;而探测
/// 阶段 `try_get_version` 用的是 `$SHELL -lic`(登录+交互式,会读 .zshrc/.zprofile),
/// 两者 PATH 不对称。裸 `claude update` / `brew upgrade ...` 在 GUI 进程里大概率
/// `command not found`(exit 127)→ `set -e` 中止 → 用户看到失败 toast,锚定决策却
/// 已展示给用户"将写回原生那处"——欺骗性故障。
///
/// 判定顺序（命中即返回）：
/// ① Claude 原生安装器（`~/.local/share/claude/versions/`）→ `<bin_path 绝对> update`；
///    bin_path 指向 launcher,launcher 内部 dispatch update 子命令。它不归 npm 管,
///    且在 PATH 里比 nvm/homebrew 更靠前,用 npm 升级会装到别处且被原生那份遮蔽。
/// ② Homebrew formula（真身在 `Cellar/<formula>/`）→ `<bin_path 同目录>/brew upgrade <formula>`;
///    formula 名 ≠ npm 包名（gemini-cli ≠ @google/gemini-cli）。
/// ③ volta → `<bin_path 同目录>/volta install <pkg>`;bun → `<bin_path 同目录>/bun add -g <pkg>@latest`。
///    `~/.volta/bin` / `~/.bun/bin` 几乎不在 GUI 进程的 PATH 里,且用户可能 PATH
///    上有另一份 volta/bun → 必须绝对路径锚定到命令行实际命中的那一份。
/// ④ 其余 npm 全局包（nvm / homebrew-npm / mise / fnm / system）→ 锚定到"那处 bin
///    目录的 npm"，确保升级写回命令行实际在用的那个 node，而非 PATH 第一个 npm。
#[cfg(not(target_os = "windows"))]
fn anchored_command_from_paths(tool: &str, bin_path: &str, real_target: &str) -> Option<String> {
    let pkg = npm_package_for(tool)?;
    let real_lower = real_target.to_ascii_lowercase();

    if tool == "claude"
        && (real_lower.contains("/.local/share/claude/")
            || real_lower.contains("/claude/versions/"))
    {
        return Some(format!("{} update", quote_path_if_spaced(bin_path)));
    }
    if let Some(formula) = brew_formula_from_path(real_target) {
        let brew = sibling_bin(bin_path, "brew")?;
        return Some(format!("{} upgrade {formula}", quote_path_if_spaced(&brew)));
    }
    match infer_install_source(Path::new(bin_path)) {
        "volta" => {
            let volta = sibling_bin(bin_path, "volta")?;
            return Some(format!("{} install {pkg}", quote_path_if_spaced(&volta)));
        }
        "bun" => {
            let bun = sibling_bin(bin_path, "bun")?;
            return Some(format!(
                "{} add -g {pkg}@latest",
                quote_path_if_spaced(&bun)
            ));
        }
        // 自带同级 npm 的 node 管理器：落到下面锚定到那处的 npm。
        "nvm" | "fnm" | "mise" | "homebrew" => {}
        // system / 未知来源（如 opencode install.sh 装的 ~/.opencode/bin、~/go/bin）
        // 通常没有同级 npm，锚定会拼出 `<dir>/npm` 这种必失败命令；返回 None 让调用方
        // 回退静态裸命令（至少能跑），并由前端据 anchored=false 给出诚实文案。
        _ => return None,
    }
    let npm = sibling_bin(bin_path, "npm")?;
    Some(format!("{} i -g {pkg}@latest", quote_path_if_spaced(&npm)))
}

/// 给定一处具体安装的 bin 路径、真身路径与 source 标签,推断**卸载该处**的命令
/// (纯函数、不碰 FS)。`source` 由调用方传入(`enumerate_tool_installations` 复用
/// 已为 `ToolInstallation.source` 字段算过的那次 `infer_install_source` 结果,本函数
/// 不再二次调用,见 review N1)。与 `anchored_command_from_paths` 共享真身判定:
/// formula 走 `brew uninstall`、volta/bun/pnpm 走各自 uninstaller、其余 npm 全局走
/// sibling npm 的 `rm -g`。
///
/// **为什么不让前端拼**:前端只拿到 `source` 字符串,但 `infer_install_source` 把
/// `/Cellar/` 与 `/opt/homebrew/lib/node_modules/` 都归为 `"homebrew"`,**有损分类**——
/// 前端区分不出 formula 和 npm 全局包,只能统一拼成 `npm rm -g`,对 brew formula
/// 会 `no such package` 失败(实测案例:`/opt/homebrew/bin/gemini` → `Cellar/gemini-cli/`,
/// 前端给的 `npm rm -g @google/gemini-cli` 跑出来报"not installed")。把卸载命令
/// 搬到后端、复用 `brew_formula_from_path(real)` 即可正确区分。
///
/// **不变量**:与升级路径一致,尽量绝对路径锚定(避免命令拷贝到非登录 shell 里
/// PATH 缺 `/opt/homebrew/bin` 等用户级目录而 `command not found`)。同目录推不出
/// brew/volta/bun/pnpm/npm 时退化到裸命令(`brew uninstall <formula>` 等),与
/// `static_fallback_command` 同款"诚实兜底"语义。`quoted_sibling_or_bare` 封装此模式。
///
/// 判定顺序(命中即返回):
/// ① hermes → pip(包名固定,不参与锚定)
/// ② brew formula(真身 `Cellar/<formula>/`)→ sibling `brew uninstall <formula>`;
///    formula 名 ≠ npm 包名(gemini-cli ≠ @google/gemini-cli),用 `npm_package_for(pkg)`
///    会卸错对象。
/// ③ volta → sibling `volta uninstall <pkg>`;bun → sibling `bun rm -g <pkg>`。
/// ④ pnpm → sibling `pnpm rm -g <pkg>`。**pnpm 必须独占分支**,不能落到 ⑤ npm 兜底:
///    pnpm 全局包**不在 npm prefix 下**,`npm rm -g` 会报"not installed"必失败。
///    POSIX 上 `~/.local/share/pnpm/<cli>` 是常见形态(review M1 实际命中)。
/// ⑤ nvm/fnm/mise/homebrew(npm 全局)→ sibling `npm rm -g <pkg>` + 物理删除兜底;
///    "homebrew npm 全局"在 ② 之后落到这里——它是 `/opt/homebrew/lib/node_modules/`
///    下的包(真身**不含 Cellar**),正确卸载就是 `npm rm`,与 nvm 共享同一分支。
/// ⑥ 其余(system / unknown / scoop / pip 等无可靠锚定的来源)→ 无前缀 `npm rm -g`,
///    让用户终端 PATH 默认那个 npm 处理。
#[cfg(not(target_os = "windows"))]
fn uninstall_command_from_paths(
    tool: &str,
    bin_path: &str,
    real_target: &str,
    source: &str,
) -> String {
    if tool == "hermes" {
        return "python3 -m pip uninstall hermes-agent".to_string();
    }
    let Some(pkg) = npm_package_for(tool) else {
        return String::new();
    };
    if let Some(formula) = brew_formula_from_path(real_target) {
        let brew = quoted_sibling_or_bare(bin_path, "brew");
        return format!("{brew} uninstall {formula}");
    }
    match source {
        "volta" => {
            let volta = quoted_sibling_or_bare(bin_path, "volta");
            format!("{volta} uninstall {pkg}")
        }
        "bun" => {
            let bun = quoted_sibling_or_bare(bin_path, "bun");
            format!("{bun} rm -g {pkg}")
        }
        "pnpm" => {
            // 与 ⑤ 分支故意不共享 npm 路径:pnpm 全局 prefix 与 npm 不同
            // (`~/.local/share/pnpm/global` vs `<node_root>/lib/node_modules`),
            // 用 `npm rm` 卸 pnpm 包会"package not installed"。Windows 版 anchored
            // upgrade 已有 pnpm 分支,POSIX 卸载端补齐对偶。
            let pnpm = quoted_sibling_or_bare(bin_path, "pnpm");
            format!("{pnpm} rm -g {pkg}")
        }
        // 自带同级 npm 的 node 管理器:锚定到那处的 npm,保证卸载作用于命令行
        // 实际命中的那个 node。等价类与 anchored_command_from_paths 的 POSIX 版
        // 一一对应,任何一边新增/移除等价类都要同步另一边。
        //
        // **附加物理删除兜底(Jason 2026-05-23 实测 bug 修复)**:nvm node v18.20.8
        // 上 `npm rm -g <pkg>` 静默 no-op——返回 `up to date in 91ms`、exit 0、
        // 文案像 `npm install`、bin 软链与 `lib/node_modules/<pkg>` 都没动,用户
        // 照 hint 复制后"以为成功了"但版本没变。可能根因:老 node 的 npm 全局
        // 元数据/package-lock 损坏。
        //
        // 兜底形态:`<npm> rm -g <pkg>; [ -e <bin> ] && rm -f <bin> && rm -rf <pkg_dir>`
        // - npm rm 真生效 → bin 已不存在 → `[ -e bin ]` 失败 → 短路,物理删除不触发(幂等)
        // - npm rm 静默 no-op → bin 仍在 → 物理删除接管,与"卸载意图"对齐
        // - 命令变长 2-3× 是可接受成本,换用户复制粘贴一次到位
        //
        // **rm -rf 红线**:目标是 `<node_root>/lib/node_modules/<pkg>` 这一具体
        // 包目录,**不是** `<node_root>/lib/node_modules` 整个 node_modules。`<pkg>`
        // 是 `@anthropic-ai/claude-code`/`opencode-ai` 等 npm 包全名(含 scope),
        // npm 包名规则严格无注入面;但 pkg_dir 算不出(parent_dir 返空)时**不加
        // 兜底**,保留原 `npm rm -g`,避免拼出 `/lib/node_modules/<pkg>` 这种
        // 误指根目录的命令。
        //
        // **本分支专用 `<node_root>/{bin,lib}` 标准 layout 假设**:nvm/fnm/mise/
        // homebrew(npm 全局)四者都把 node 装到 `<node_root>/{bin,lib,...}` 标准
        // 布局下,所以 `<bin_path>` 的祖父就是 node 安装根。volta/bun/scoop 的
        // 包目录布局完全不同(volta `~/.volta/tools/image/packages/`、bun
        // `~/.bun/install/global/node_modules/`),它们走各自分支不进这里。未来
        // 扩广本分支到新来源时,务必先校验 `<bin_path>/../../lib/node_modules/<pkg>`
        // 假设是否成立。
        "nvm" | "fnm" | "mise" | "homebrew" => {
            let npm = quoted_sibling_or_bare(bin_path, "npm");
            // <bin_dir> = parent(bin_path),<node_root> = parent(bin_dir)
            // 即 nvm:`~/.nvm/versions/node/v18/bin/gemini` → bin_dir `~/.nvm/.../v18/bin`
            //    → node_root `~/.nvm/.../v18` → pkg_dir `~/.nvm/.../v18/lib/node_modules/@google/gemini-cli`
            let bin_dir = parent_dir(bin_path);
            let node_root = parent_dir(&bin_dir);
            if node_root.is_empty() {
                // parent_dir 推不出 node_root(理论不会发生:bin_path 是绝对路径) →
                // 不加兜底,与原行为一致,避免拼错 pkg_dir。
                format!("{npm} rm -g {pkg}")
            } else {
                let pkg_dir = format!("{node_root}/lib/node_modules/{pkg}");
                // **强 quote 而非 quote_path_if_spaced**(P2 修复):兜底链含 `;` 和
                // `&&` 多语句分隔,bin/pkg_dir 路径若含 `;` / `` ` `` / `$` 等元字符,
                // 弱 quoting 会让 shell 拆出额外命令——destructive `rm -rf` 落到错的
                // 路径。`posix_quote_for_user_shell` 白名单判定:常规路径不加引号
                // (展示干净),特殊字符触发 `shell_single_quote` 无条件强 quote。
                let bin_q = posix_quote_for_user_shell(bin_path);
                let pkg_dir_q = posix_quote_for_user_shell(&pkg_dir);
                format!("{npm} rm -g {pkg}; [ -e {bin_q} ] && rm -f {bin_q} && rm -rf {pkg_dir_q}")
            }
        }
        // system / unknown / scoop / pip 等不期望锚定 npm 的来源:
        // **不要拼 `<dir>/npm`** (该处通常无 sibling npm,会拼出必失败的命令——
        // 与 anchored_command_from_paths 在同类来源返回 None 的语义对偶)。
        // 退化到无前缀的 `npm rm -g <pkg>`,让用户终端 PATH 默认的 npm 处理;
        // 卸载语义下这比"返回 None 让 UI 不显示命令"更友好,因为这条 hint 本来
        // 就是给用户复制粘贴的——明确意图 + 用户自决比沉默更好。
        _ => format!("npm rm -g {pkg}"),
    }
}

/// Windows 版卸载命令生成。等价类与 anchored upgrade 一致(volta/pnpm/npm 三分支),
/// 无 brew/bun/claude-native(平台不存在)、无 formula 判定路径——`real_target` 占位
/// 保持与 POSIX 版签名对称;`source` 由调用方传入(见 POSIX 版 doc)。
/// `quoted_sibling_or_bare_with_ext` 探不到时退化到 exe_basename 裸名。
///
/// **quoting 用 `win_quote_path_for_user_shell` 而非 `win_quote_path_for_batch`**:
/// 卸载命令在前端只展示给用户复制到自己的 shell,不经过 .bat + call 的两轮 %
/// expansion——后者把 `%` 转 `%%%%` 是为了让 call 看到字面 `%`,但用户的 cmd /
/// PowerShell / bash 都只一轮甚至零轮展开,直接复制 `%%%%` 会得到错的路径。
/// 与升级路径(`anchored_command_from_paths` Windows 版用 `win_quote_path_for_batch`,
/// 因为它落 .bat + call)形成"目标终端决定 quoter"的对照,doc 在 helper 一侧
/// 已经写明,这里不重复。
///
/// **目标语法是 cmd / PowerShell 限制**(P3, 2026-05-24):本函数生成的命令是
/// cmd 兼容形式——含空格路径用双引号包裹(`"C:\Program Files\..."`),PowerShell
/// 对这种引号路径**当字符串字面值不执行**,需要 call operator `&` 前缀。两个
/// shell 在引号路径上语法不兼容,无法用一份命令同时满足;详见
/// `win_quote_path_for_user_shell` doc 的 PowerShell 限制段。
///
/// **前端 UI 通过 `ToolInstallation.uninstall_command_needs_cmd_hint` 结构化字段
/// 条件渲染提示**(AboutSection.tsx 卸载行下方, i18n key `toolUninstallPwshHint`)。
/// 该 bool 由 enumerate 用 `cfg!(target_os = "windows") && contains('"')` 算出,
/// `cfg!` 是编译时 ground truth——POSIX 编译时整体短路为 false,与 POSIX
/// `shell_single_quote` 的 `'"'"'` 转义形态含 `"` 的字面巧合**完全无关**。
/// **前端不能 string match `uninstall_command`**:POSIX 路径带 `'`(如
/// macOS `o'leary`)的命令也含 `"`,前端 includes('"') 会误判为 Windows cmd 形式。
///
/// **POSIX 兜底物理删除链在 Windows 版**有意不镜像**:实测未观察到 nvm-windows /
/// Scoop 上 `npm.cmd rm -g` 静默 no-op(POSIX 的 npm rm 静默是 nvm node v18.20.8
/// 老 node 的特定 bug),而 Windows 的物理删除链需要 `del` / `rmdir /S /Q` + `if
/// exist` 三段语义,跨 cmd/PowerShell shell 不一致(`del` 在 PowerShell 是 Remove-
/// Item 别名、参数不同),收益不抵复杂度。若未来在 Windows 上观察到同款 bug,可
/// 在本函数加 cmd-only 兜底链。
///
/// hermes 走 `py -m pip uninstall`,与 `tool_action_shell_command` 的 Windows 分支
/// 用同一 py launcher 入口。
#[cfg(target_os = "windows")]
fn uninstall_command_from_paths(
    tool: &str,
    bin_path: &str,
    _real_target: &str,
    source: &str,
) -> String {
    if tool == "hermes" {
        return "py -m pip uninstall hermes-agent".to_string();
    }
    let Some(pkg) = npm_package_for(tool) else {
        return String::new();
    };
    match source {
        "volta" => {
            let volta = quoted_sibling_or_bare_with_ext(bin_path, "volta", &["exe", "cmd"]);
            format!("{volta} uninstall {pkg}")
        }
        "pnpm" => {
            let pnpm = quoted_sibling_or_bare_with_ext(bin_path, "pnpm", &["cmd", "exe"]);
            format!("{pnpm} rm -g {pkg}")
        }
        _ => {
            let npm = quoted_sibling_or_bare_with_ext(bin_path, "npm", &["cmd", "exe"]);
            format!("{npm} rm -g {pkg}")
        }
    }
}

/// Windows 版锚定命令生成。等价类压缩到 3 种(volta/pnpm/npm),不存在 brew/bun/
/// claude-native(Windows 没 Homebrew、Bun for Windows 仍 preview、claude.ai/install.sh
/// 是 bash 脚本)。Scoop/Chocolatey/winget/nvm-windows/MS Store node 都归 npm 类——
/// 它们都只是"如何装 node"的不同入口,全局包真正的 idiom 仍是 sibling `npm.cmd`。
///
/// **与 POSIX 版的语义差异**:POSIX 版是纯函数(不碰 fs),Windows 版通过
/// `sibling_bin_with_ext` 读 fs 来探明扩展名(`.cmd` vs `.exe`)——Node installer
/// 装 `.cmd`、Volta 装 `.exe`,纯字符串拼接无法消歧。这一平台差异**被刻意保留**:
/// 测试用 tempdir 隔离 fs,生产侧 TOCTOU 是 by design(见 `sibling_bin_with_ext` doc)。
///
/// `_real_target` 占位维持与 POSIX 版的签名对称——Windows 上未观测到需要真身路径
/// 区分的等价类(无 Cellar、无 claude-native installer)。若未来加 Scoop persist 锚定
/// (scoop 装的工具真身在 `<scoop_root>/persist/<app>/...`),从这里启用 `_real_target`。
///
/// **关键不变量同 POSIX 版:返回的命令必须用绝对路径,不依赖 PATH**。Windows GUI
/// 进程 PATH 由 Service Control Manager / explorer.exe 给,通常不含用户 `%LOCALAPPDATA%`
/// 下的 Volta/pnpm 路径;`$SHELL -lic` 的探测时 PATH 与执行时 PATH 不对称。
///
/// 判定顺序(命中即返回):
/// ① volta(`infer_install_source == "volta"`)→ sibling `volta.exe`/`.cmd` install <pkg>
/// ② pnpm(`infer_install_source == "pnpm"`)→ sibling `pnpm.cmd`/`.exe` add -g <pkg>@latest
///    (用 add+@latest 而非 update:语义明确、且对"之前没在 pnpm 全局装过"也幂等)
/// ③ 其余 → sibling `npm.cmd`/`.exe` i -g <pkg>@latest
///
/// hermes(pip)在 `npm_package_for` 返回 None → 整体返 None,落静态 pip 兜底。
/// 所有 sibling 探测都通过 `sibling_bin_with_ext`(碰 fs):该处无候选扩展名存在 →
/// 返 None,上游 `plan_command_for` 兜回静态命令、`anchored=false`。
#[cfg(target_os = "windows")]
fn anchored_command_from_paths(tool: &str, bin_path: &str, _real_target: &str) -> Option<String> {
    let pkg = npm_package_for(tool)?;

    match infer_install_source(Path::new(bin_path)) {
        "volta" => {
            let volta = sibling_bin_with_ext(bin_path, "volta", &["exe", "cmd"])?;
            Some(format!(
                "{} install {pkg}",
                win_quote_path_for_batch(&volta)
            ))
        }
        "pnpm" => {
            let pnpm = sibling_bin_with_ext(bin_path, "pnpm", &["cmd", "exe"])?;
            Some(format!(
                "{} add -g {pkg}@latest",
                win_quote_path_for_batch(&pnpm)
            ))
        }
        // 兜底 = npm 类:Scoop / Chocolatey / winget / nvm-windows / MS Store nodejs /
        // system / 任何识别不到专属来源的 → sibling npm.cmd。
        _ => {
            let npm = sibling_bin_with_ext(bin_path, "npm", &["cmd", "exe"])?;
            Some(format!(
                "{} i -g {pkg}@latest",
                win_quote_path_for_batch(&npm)
            ))
        }
    }
}

/// 从枚举结果里取"命令行实际命中的那处"：优先 `is_path_default`；否则（解析不到
/// PATH 默认、但只有一处）取唯一那处；多处且无默认标记 → None（无从锚定）。
///
/// 全平台共用——POSIX 和 Windows 版的 `anchored_command_from_paths` 都通过
/// `installs_anchored_command` 调它,取默认那处再 canonicalize 拿真身。
fn default_install(installs: &[ToolInstallation]) -> Option<&ToolInstallation> {
    installs.iter().find(|i| i.is_path_default).or_else(|| {
        if installs.len() == 1 {
            installs.first()
        } else {
            None
        }
    })
}

/// 基于已枚举的安装列表生成锚定升级命令（复用 enumerate 结果，避免二次探测）。
/// 读取 enumerate 时已 canonicalize 写入的 `inst.real`,**不再二次 canonicalize**——
/// 既消除冗余 syscall,也闭合"enumerate 与 anchor 看到同一真身"的一致性边界
/// (两次 canonicalize 之间 symlink 被换会让锚定指向不同真身)。
///
/// 全平台共用——`anchored_command_from_paths` 自身是 cfg 二选一(POSIX 五分支 /
/// Windows 三分支),这里只负责取默认那处 + 转发。
fn installs_anchored_command(tool: &str, installs: &[ToolInstallation]) -> Option<String> {
    let inst = default_install(installs)?;
    let real = inst.real.to_string_lossy();
    anchored_command_from_paths(tool, &inst.path, &real)
}

/// 静态裸命令（= 现有的 `npm i -g <pkg>@latest` / pip 升级）。
/// 锚定探不到默认安装时回退到它，等同于"装到 PATH 第一个 npm"的旧行为。
fn static_fallback_command(tool: &str) -> String {
    tool_action_shell_command(tool, ToolLifecycleAction::Update)
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// 新装(install)的命令:对有官方 installer 的工具走「上游推荐 || npm 兜底」短路链,
/// 其余工具透传到 `static_fallback_command`(= 与 update fallback 同一张 npm/pip 表)。
///
/// 设计理由:
/// - install 没有锚点可言(从无到有),但**有"上游推荐方式"这一事实** ——
///   Anthropic 和 SST(OpenCode)都已将自家 native installer 列为首推、把 npm 列为传统方式。
///   把这层认知补进来,让 install 表与 update 端的锚定决策树共用同一份"上游事实"。
/// - 短路链(POSIX `||`)保证 URL 不可达/防火墙拦截时仍能装上,降级到 npm。
/// - 仅非 Windows 启用:claude.ai/install.sh、opencode.ai/install 都是 bash 脚本,
///   Windows(及不带 bash 的环境)继续走原 `tool_action_shell_command` 的 npm 命令。
/// - 配套要求:`build_tool_lifecycle_command` 头部已开 `set -o pipefail`,确保
///   `curl ... | bash` 中 curl 失败能让管道整体非零、`||` 兜底真正触发。
#[cfg(not(target_os = "windows"))]
fn install_command_for(tool: &str) -> String {
    match tool {
        "claude" => "curl -fsSL https://claude.ai/install.sh | bash || npm i -g @anthropic-ai/claude-code@latest".to_string(),
        "opencode" => "curl -fsSL https://opencode.ai/install | bash || npm i -g opencode-ai@latest".to_string(),
        _ => static_fallback_command(tool),
    }
}

/// 计算某工具的升级命令与"是否需确认"。全平台共用一份:
/// - **Windows + WSL 工具**(override 是 `\\wsl$\<distro>\...` UNC 路径)始终走静态、
///   不锚定:锚定命令是 Windows 主机绝对路径,跨 `wsl.exe` 边界进入 distro 文件
///   系统后完全无效;且 `enumerate_tool_installations` 不参与 WSL 文件系统、锚定
///   无锚点。这一类显式短路到 `(unix_static, false, false)`,前端不会弹确认。
///   **必须用 `wsl_tool_action_shell_command`(unix 版)而非 `static_fallback_command`**
///   ——后者读 `tool_action_shell_command`,Windows target 给 hermes 返回 `py -m pip ...`,
///   跨 wsl.exe 后 py launcher 不存在;`build_tool_action_line` 的 WSL 分支也用同一
///   wrapper,保证 plan 展示给前端的命令与实际执行落 .bat 的命令一致。
/// - 其他平台与 Windows 原生工具走 `installs_anchored_command`:命中 → 锚定;
///   None(无默认 / sibling 不存在 / hermes pip 等)→ 静态兜底、`anchored=false`,
///   前端据此给"默认入口无法确定"诚实文案。
fn plan_command_for(tool: &str, installs: &[ToolInstallation]) -> (String, bool, bool) {
    #[cfg(target_os = "windows")]
    {
        if wsl_distro_for_tool(tool).is_some() {
            let cmd = wsl_tool_action_shell_command(tool, ToolLifecycleAction::Update)
                .map(|s| s.to_string())
                .unwrap_or_default();
            return (cmd, false, false);
        }
    }
    match installs_anchored_command(tool, installs) {
        Some(command) => (command, installs.len() >= 2, true),
        None => (static_fallback_command(tool), installs.len() >= 2, false),
    }
}

/// 多处安装是否构成"真冲突"：≥2 处，且(版本分歧 或 有的能跑有的跑不起来)。
/// 同版本装两份且都能跑不算冲突（不打扰用户）。诊断展示据此判定。
fn is_conflicting(installs: &[ToolInstallation]) -> bool {
    if installs.len() < 2 {
        return false;
    }
    let distinct_versions: std::collections::HashSet<&Option<String>> =
        installs.iter().map(|i| &i.version).collect();
    let runnable_mixed =
        installs.iter().any(|i| i.runnable) && installs.iter().any(|i| !i.runnable);
    distinct_versions.len() > 1 || runnable_mixed
}

/// 一次"探测工具安装分布"的结果：枚举到的所有安装 + 各项衍生判定。同时服务两条
/// 路径——诊断展示（`is_conflict`）与升级确认（`needs_confirmation`/`command`/`anchored`）。
/// 字段保持 snake_case（与 `ToolInstallation` 一致），前端按同名读取。
#[derive(Debug, serde::Serialize)]
pub struct ToolInstallationReport {
    tool: String,
    /// 该工具枚举到的所有安装。
    installs: Vec<ToolInstallation>,
    /// 严阈值：≥2 且(版本分歧或运行态混合)。诊断按钮/自动补诊据此展示冲突。
    is_conflict: bool,
    /// 宽阈值：≥2 处。升级确认据此弹窗（升级只动一处，任何多处都该让用户知情）。
    needs_confirmation: bool,
    /// 锚定后将执行的升级命令（仅展示；真正执行时后端会重新生成，不信任前端回传）。
    command: String,
    /// 是否成功锚定到某处具体安装。false = 退到裸 fallback 命令（无法确定命令行实际
    /// 命中哪处，或该处无同级 npm）；前端据此给出"默认入口无法确定"的诚实文案。
    anchored: bool,
}

/// 探测各工具的安装分布：枚举所有安装、标记冲突、生成锚定升级命令。只读、无副作用。
/// 诊断按钮、升级前确认、升级后补诊共用此命令，各取所需字段——避免对同一份枚举结果
/// 散落多套下游判定。
#[tauri::command]
pub async fn probe_tool_installations(
    tools: Vec<String>,
) -> Result<Vec<ToolInstallationReport>, String> {
    let requested = normalize_requested_tools(&tools);
    if requested.is_empty() {
        return Err("No supported tools selected".to_string());
    }
    tokio::task::spawn_blocking(move || {
        requested
            .into_iter()
            .map(|tool| {
                let installs = enumerate_tool_installations(tool);
                let (command, needs_confirmation, anchored) = plan_command_for(tool, &installs);
                let is_conflict = is_conflicting(&installs);
                ToolInstallationReport {
                    tool: tool.to_string(),
                    installs,
                    is_conflict,
                    needs_confirmation,
                    command,
                    anchored,
                }
            })
            .collect()
    })
    .await
    .map_err(|e| format!("probe task join error: {e}"))
}

#[cfg(target_os = "windows")]
fn wsl_distro_for_tool(tool: &str) -> Option<String> {
    let override_dir = match tool {
        "claude" => crate::settings::get_claude_override_dir(),
        "codex" => crate::settings::get_codex_override_dir(),
        "gemini" => crate::settings::get_gemini_override_dir(),
        "opencode" => crate::settings::get_opencode_override_dir(),
        "openclaw" => crate::settings::get_openclaw_override_dir(),
        "hermes" => crate::settings::get_hermes_override_dir(),
        _ => None,
    }?;

    wsl_distro_from_path(&override_dir)
}

/// 从 UNC 路径中提取 WSL 发行版名称
/// 支持 `\\wsl$\Ubuntu\...` 和 `\\wsl.localhost\Ubuntu\...` 两种格式
#[cfg(target_os = "windows")]
fn wsl_distro_from_path(path: &Path) -> Option<String> {
    use std::path::{Component, Prefix};
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return None;
    };
    match prefix.kind() {
        Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
            let server_name = server.to_string_lossy();
            if server_name.eq_ignore_ascii_case("wsl$")
                || server_name.eq_ignore_ascii_case("wsl.localhost")
            {
                let distro = share.to_string_lossy().to_string();
                if !distro.is_empty() {
                    return Some(distro);
                }
            }
            None
        }
        _ => None,
    }
}

/// 打开指定提供商的终端
///
/// 根据提供商配置的环境变量启动一个带有该提供商特定设置的终端
/// 无需检查是否为当前激活的提供商，任何提供商都可以打开终端
#[allow(non_snake_case)]
#[tauri::command]
pub async fn open_provider_terminal(
    state: State<'_, crate::store::AppState>,
    app: String,
    #[allow(non_snake_case)] providerId: String,
    cwd: Option<String>,
) -> Result<bool, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    let launch_cwd = resolve_launch_cwd(cwd)?;

    // 获取提供商配置
    let providers = ProviderService::list(state.inner(), app_type.clone())
        .map_err(|e| format!("获取提供商列表失败: {e}"))?;

    let provider = providers
        .get(&providerId)
        .ok_or_else(|| format!("提供商 {providerId} 不存在"))?;

    // 从提供商配置中提取环境变量
    let config = &provider.settings_config;
    let env_vars = extract_env_vars_from_config(config, &app_type);

    // 根据平台启动终端，传入提供商ID用于生成唯一的配置文件名
    launch_terminal_with_env(env_vars, &providerId, launch_cwd.as_deref())
        .map_err(|e| format!("启动终端失败: {e}"))?;

    Ok(true)
}

/// 从提供商配置中提取环境变量
fn extract_env_vars_from_config(
    config: &serde_json::Value,
    app_type: &AppType,
) -> Vec<(String, String)> {
    let mut env_vars = Vec::new();

    let Some(obj) = config.as_object() else {
        return env_vars;
    };

    // 处理 env 字段（Claude/Gemini 通用）
    if let Some(env) = obj.get("env").and_then(|v| v.as_object()) {
        for (key, value) in env {
            if let Some(str_val) = value.as_str() {
                env_vars.push((key.clone(), str_val.to_string()));
            }
        }

        // 处理 base_url: 根据应用类型添加对应的环境变量
        let base_url_key = match app_type {
            AppType::Claude | AppType::ClaudeDesktop => Some("ANTHROPIC_BASE_URL"),
            AppType::Gemini => Some("GOOGLE_GEMINI_BASE_URL"),
            _ => None,
        };

        if let Some(key) = base_url_key {
            if let Some(url_str) = env.get(key).and_then(|v| v.as_str()) {
                env_vars.push((key.to_string(), url_str.to_string()));
            }
        }
    }

    // Codex 使用 auth 字段转换为 OPENAI_API_KEY
    if *app_type == AppType::Codex {
        if let Some(auth) = obj.get("auth").and_then(|v| v.as_str()) {
            env_vars.push(("OPENAI_API_KEY".to_string(), auth.to_string()));
        }
    }

    // Gemini 使用 api_key 字段转换为 GEMINI_API_KEY
    if *app_type == AppType::Gemini {
        if let Some(api_key) = obj.get("api_key").and_then(|v| v.as_str()) {
            env_vars.push(("GEMINI_API_KEY".to_string(), api_key.to_string()));
        }
    }

    env_vars
}

fn resolve_launch_cwd(cwd: Option<String>) -> Result<Option<PathBuf>, String> {
    let Some(raw_path) = cwd.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };

    if raw_path.contains('\n') || raw_path.contains('\r') {
        return Err("目录路径包含非法换行符".to_string());
    }

    let path = Path::new(&raw_path);
    if !path.exists() {
        return Err(format!("目录不存在: {raw_path}"));
    }

    let resolved = std::fs::canonicalize(path).map_err(|e| format!("解析目录失败: {e}"))?;
    if !resolved.is_dir() {
        return Err(format!("选择的路径不是文件夹: {}", resolved.display()));
    }

    // Strip Windows extended-length prefix that canonicalize produces,
    // as it can break batch scripts and other shell commands.
    // Special-case \\?\UNC\server\share -> \\server\share for network/WSL paths.
    #[cfg(target_os = "windows")]
    let resolved = {
        let s = resolved.to_string_lossy();
        if let Some(unc) = s.strip_prefix(r"\\?\UNC\") {
            PathBuf::from(format!(r"\\{unc}"))
        } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
            PathBuf::from(stripped)
        } else {
            resolved
        }
    };

    Ok(Some(resolved))
}

/// 创建临时配置文件并启动 claude 终端
/// 使用 --settings 参数传入提供商特定的 API 配置
fn launch_terminal_with_env(
    env_vars: Vec<(String, String)>,
    provider_id: &str,
    cwd: Option<&Path>,
) -> Result<(), String> {
    let temp_dir = std::env::temp_dir();
    let config_file = temp_dir.join(format!(
        "claude_{}_{}.json",
        provider_id,
        std::process::id()
    ));

    // 创建并写入配置文件
    write_claude_config(&config_file, &env_vars)?;

    #[cfg(target_os = "macos")]
    {
        launch_macos_terminal(&config_file, cwd)?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        launch_linux_terminal(&config_file, cwd)?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        launch_windows_terminal(&temp_dir, &config_file, cwd)?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    Err("不支持的操作系统".to_string())
}

/// 写入 claude 配置文件
fn write_claude_config(
    config_file: &std::path::Path,
    env_vars: &[(String, String)],
) -> Result<(), String> {
    let mut config_obj = serde_json::Map::new();
    let mut env_obj = serde_json::Map::new();

    for (key, value) in env_vars {
        env_obj.insert(key.clone(), serde_json::Value::String(value.clone()));
    }

    config_obj.insert("env".to_string(), serde_json::Value::Object(env_obj));

    let config_json =
        serde_json::to_string_pretty(&config_obj).map_err(|e| format!("序列化配置失败: {e}"))?;

    std::fs::write(config_file, config_json).map_err(|e| format!("写入配置文件失败: {e}"))
}

/// macOS: 根据用户首选终端启动
#[cfg(target_os = "macos")]
fn launch_macos_terminal(config_file: &std::path::Path, cwd: Option<&Path>) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let preferred = crate::settings::get_preferred_terminal();
    let terminal = preferred.as_deref().unwrap_or("terminal");

    let temp_dir = std::env::temp_dir();
    let script_file = temp_dir.join(format!("cc_switch_launcher_{}.sh", std::process::id()));
    let config_path = config_file.to_string_lossy();
    let cd_command = build_shell_cd_command(cwd);

    // Write the shell script to a temp file
    let script_content = format!(
        r#"#!/bin/bash
trap 'rm -f "{config_path}" "{script_file}"' EXIT
{cd_command}
echo "Using provider-specific claude config:"
echo "{config_path}"
claude --settings "{config_path}"
exec bash --norc --noprofile
"#,
        config_path = config_path,
        script_file = script_file.display(),
        cd_command = cd_command,
    );

    std::fs::write(&script_file, &script_content).map_err(|e| format!("写入启动脚本失败: {e}"))?;

    // Make script executable
    std::fs::set_permissions(&script_file, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("设置脚本权限失败: {e}"))?;

    // Try the preferred terminal first, fall back to Terminal.app if it fails
    // Note: Kitty doesn't need the -e flag, others do
    let result = match terminal {
        "iterm2" => launch_macos_iterm2(&script_file),
        "warp" => launch_macos_warp(&script_file),
        "alacritty" => launch_macos_open_app("Alacritty", &script_file, true),
        "kitty" => launch_macos_open_app("kitty", &script_file, false),
        "ghostty" => launch_macos_ghostty(&script_file),
        "wezterm" => launch_macos_open_app("WezTerm", &script_file, true),
        "kaku" => launch_macos_open_app("Kaku", &script_file, true),
        _ => launch_macos_terminal_app(&script_file), // "terminal" or default
    };

    // If preferred terminal fails and it's not the default, try Terminal.app as fallback
    if result.is_err() && terminal != "terminal" {
        log::warn!(
            "首选终端 {} 启动失败，回退到 Terminal.app: {:?}",
            terminal,
            result.as_ref().err()
        );
        return launch_macos_terminal_app(&script_file);
    }

    result
}

/// macOS: Terminal.app
#[cfg(target_os = "macos")]
fn launch_macos_terminal_app(script_file: &std::path::Path) -> Result<(), String> {
    use std::process::Command;

    let applescript = format!(
        r#"tell application "Terminal"
    activate
    do script "bash '{}'"
end tell"#,
        script_file.display()
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&applescript)
        .output()
        .map_err(|e| format!("执行 osascript 失败: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Terminal.app 执行失败 (exit code: {:?}): {}",
            output.status.code(),
            stderr
        ));
    }

    Ok(())
}

/// macOS: iTerm2
#[cfg(target_os = "macos")]
fn build_macos_iterm2_applescript(script_file: &std::path::Path) -> String {
    format!(
        r#"set launcher_script to "bash '{}'"
set was_running to application "iTerm" is running
tell application "iTerm"
    if was_running then
        activate
        if (count of windows) = 0 then
            create window with default profile
        else
            tell current window
                create tab with default profile
            end tell
        end if
    else
        activate
        set waited to 0
        repeat while (count of windows) = 0
            delay 0.1
            set waited to waited + 1
            if waited >= 30 then exit repeat
        end repeat
        if (count of windows) = 0 then
            create window with default profile
        end if
    end if
    tell current session of current window
        write text launcher_script
    end tell
end tell"#,
        script_file.display()
    )
}

/// macOS: iTerm2
#[cfg(target_os = "macos")]
fn launch_macos_iterm2(script_file: &std::path::Path) -> Result<(), String> {
    use std::process::Command;

    let applescript = build_macos_iterm2_applescript(script_file);

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&applescript)
        .output()
        .map_err(|e| format!("执行 osascript 失败: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "iTerm2 执行失败 (exit code: {:?}): {}",
            output.status.code(),
            stderr
        ));
    }

    Ok(())
}

/// macOS: Ghostty — use --quit-after-last-window-closed to avoid cloning existing tabs
#[cfg(target_os = "macos")]
fn launch_macos_ghostty(script_file: &std::path::Path) -> Result<(), String> {
    use std::process::Command;

    let output = Command::new("open")
        .args([
            "-na",
            "Ghostty",
            "--args",
            "--quit-after-last-window-closed=true",
            "-e",
            "bash",
        ])
        .arg(script_file)
        .output()
        .map_err(|e| format!("启动 Ghostty 失败: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Ghostty 启动失败 (exit code: {:?}): {}",
            output.status.code(),
            stderr
        ));
    }

    Ok(())
}

/// macOS: 使用 open -na 启动支持 --args 参数的终端（Alacritty/Kitty/WezTerm/Kaku）
#[cfg(target_os = "macos")]
fn launch_macos_open_app(
    app_name: &str,
    script_file: &std::path::Path,
    use_e_flag: bool,
) -> Result<(), String> {
    use std::process::Command;

    let mut cmd = Command::new("open");
    cmd.arg("-na").arg(app_name).arg("--args");

    if use_e_flag {
        cmd.arg("-e");
    }
    cmd.arg("bash").arg(script_file);

    let output = cmd
        .output()
        .map_err(|e| format!("启动 {app_name} 失败: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{} 启动失败 (exit code: {:?}): {}",
            app_name,
            output.status.code(),
            stderr
        ));
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_macos_warp(script_file: &std::path::Path) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let mut cmd = Command::new("open");
    cmd.arg("-a").arg("Warp");

    // Warp URI scheme cannot work well with script_file, because:
    //
    // 1. script_file's name ends up with .sh, so Warp would open the file rather than execute it
    // 2. script_file has no execution permission, so we need to add one more indirection
    let mut second_script_file = tempfile::Builder::new()
        .disable_cleanup(true)
        .permissions(std::fs::Permissions::from_mode(0o755))
        .tempfile()
        .map_err(|e| format!("Failed to create temporary script file: {e}"))?;

    writeln!(
        &mut second_script_file,
        r#"#!/usr/bin/env sh

        rm -- "$0"

        exec bash {}
        "#,
        script_file.display(),
    )
    .map_err(|e| format!("Failed to write to temporary script file for Warp: {e}"))?;

    let mut warp_url = url::Url::parse("warp://action/new_tab").unwrap();
    warp_url
        .query_pairs_mut()
        .append_pair("path", &second_script_file.path().to_string_lossy());
    let warp_url = warp_url.to_string();
    cmd.arg(warp_url);

    let output = cmd.output().map_err(|e| format!("启动 Warp 失败: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Warp 启动失败 (exit code: {:?}): {}",
            output.status.code(),
            stderr
        ));
    }

    Ok(())
}

/// Linux: 根据用户首选终端启动
#[cfg(target_os = "linux")]
fn launch_linux_terminal(config_file: &std::path::Path, cwd: Option<&Path>) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let preferred = crate::settings::get_preferred_terminal();

    // Default terminal list with their arguments
    let default_terminals = [
        ("gnome-terminal", vec!["--"]),
        ("konsole", vec!["-e"]),
        ("xfce4-terminal", vec!["-e"]),
        ("mate-terminal", vec!["--"]),
        ("lxterminal", vec!["-e"]),
        ("alacritty", vec!["-e"]),
        ("kitty", vec!["-e"]),
        ("ghostty", vec!["-e"]),
    ];

    // Create temp script file
    let temp_dir = std::env::temp_dir();
    let script_file = temp_dir.join(format!("cc_switch_launcher_{}.sh", std::process::id()));
    let config_path = config_file.to_string_lossy();
    let cd_command = build_shell_cd_command(cwd);

    let script_content = format!(
        r#"#!/bin/bash
trap 'rm -f "{config_path}" "{script_file}"' EXIT
{cd_command}
echo "Using provider-specific claude config:"
echo "{config_path}"
claude --settings "{config_path}"
exec bash --norc --noprofile
"#,
        config_path = config_path,
        script_file = script_file.display(),
        cd_command = cd_command,
    );

    std::fs::write(&script_file, &script_content).map_err(|e| format!("写入启动脚本失败: {e}"))?;

    std::fs::set_permissions(&script_file, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("设置脚本权限失败: {e}"))?;

    // Build terminal list: preferred terminal first (if specified), then defaults
    let terminals_to_try: Vec<(&str, Vec<&str>)> = if let Some(ref pref) = preferred {
        // Find the preferred terminal's args from default list
        let pref_args = default_terminals
            .iter()
            .find(|(name, _)| *name == pref.as_str())
            .map(|(_, args)| args.to_vec())
            .unwrap_or_else(|| vec!["-e"]); // Default args for unknown terminals

        let mut list = vec![(pref.as_str(), pref_args)];
        // Add remaining terminals as fallbacks
        for (name, args) in &default_terminals {
            if *name != pref.as_str() {
                list.push((*name, args.to_vec()));
            }
        }
        list
    } else {
        default_terminals
            .iter()
            .map(|(name, args)| (*name, args.to_vec()))
            .collect()
    };

    let mut last_error = String::from("未找到可用的终端");

    for (terminal, args) in terminals_to_try {
        // Check if terminal exists in common paths
        let terminal_exists = std::path::Path::new(&format!("/usr/bin/{}", terminal)).exists()
            || std::path::Path::new(&format!("/bin/{}", terminal)).exists()
            || std::path::Path::new(&format!("/usr/local/bin/{}", terminal)).exists()
            || which_command(terminal);

        if terminal_exists {
            let result = Command::new(terminal)
                .args(&args)
                .arg("bash")
                .arg(script_file.to_string_lossy().as_ref())
                .spawn();

            match result {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = format!("执行 {} 失败: {}", terminal, e);
                }
            }
        }
    }

    // Clean up on failure
    let _ = std::fs::remove_file(&script_file);
    let _ = std::fs::remove_file(config_file);
    Err(last_error)
}

/// Check if a command exists using `which`
#[cfg(target_os = "linux")]
fn which_command(cmd: &str) -> bool {
    use std::process::Command;
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Windows: 根据用户首选终端启动
#[cfg(target_os = "windows")]
fn launch_windows_terminal(
    temp_dir: &std::path::Path,
    config_file: &std::path::Path,
    cwd: Option<&Path>,
) -> Result<(), String> {
    let preferred = crate::settings::get_preferred_terminal();
    let terminal = preferred.as_deref().unwrap_or("cmd");

    let bat_file = temp_dir.join(format!("cc_switch_claude_{}.bat", std::process::id()));
    let config_path_for_batch = escape_windows_batch_value(&config_file.to_string_lossy());
    let cwd_command = build_windows_cwd_command(cwd);

    let content = format!(
        "@echo off
{cwd_command}
echo Using provider-specific claude config:
echo {}
claude --settings \"{}\"
del \"{}\" >nul 2>&1
del \"%~f0\" >nul 2>&1
",
        config_path_for_batch,
        config_path_for_batch,
        config_path_for_batch,
        cwd_command = cwd_command,
    );

    std::fs::write(&bat_file, &content).map_err(|e| format!("写入批处理文件失败: {e}"))?;

    let bat_path = bat_file.to_string_lossy();
    let ps_cmd = format!("& '{}'", bat_path);

    // Try the preferred terminal first
    let result = match terminal {
        "powershell" => run_windows_start_command(
            &["powershell", "-NoExit", "-Command", &ps_cmd],
            "PowerShell",
        ),
        "wt" => run_windows_start_command(&["wt", "cmd", "/K", &bat_path], "Windows Terminal"),
        _ => run_windows_start_command(&["cmd", "/K", &bat_path], "cmd"), // "cmd" or default
    };

    // If preferred terminal fails and it's not the default, try cmd as fallback
    if result.is_err() && terminal != "cmd" {
        log::warn!(
            "首选终端 {} 启动失败，回退到 cmd: {:?}",
            terminal,
            result.as_ref().err()
        );
        return run_windows_start_command(&["cmd", "/K", &bat_path], "cmd");
    }

    result
}

fn build_shell_cd_command(cwd: Option<&Path>) -> String {
    cwd.map(|dir| {
        format!(
            "cd {} || exit 1\n",
            shell_single_quote(&dir.to_string_lossy())
        )
    })
    .unwrap_or_default()
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn is_windows_unc_path(path: &str) -> bool {
    path.starts_with(r"\\")
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn build_windows_cwd_command_str(path: &str) -> String {
    let escaped = escape_windows_batch_value(path);

    if is_windows_unc_path(path) {
        // `cmd.exe` cannot make a UNC path current via `cd`; `pushd` maps it first.
        format!("pushd \"{escaped}\" || exit /b 1\r\n")
    } else {
        format!("cd /d \"{escaped}\" || exit /b 1\r\n")
    }
}

#[cfg(target_os = "windows")]
fn build_windows_cwd_command(cwd: Option<&Path>) -> String {
    cwd.map(|dir| build_windows_cwd_command_str(&dir.to_string_lossy()))
        .unwrap_or_default()
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn escape_windows_batch_value(value: &str) -> String {
    value
        .replace('^', "^^")
        .replace('%', "%%")
        .replace('&', "^&")
        .replace('|', "^|")
        .replace('<', "^<")
        .replace('>', "^>")
        .replace('(', "^(")
        .replace(')', "^)")
}
/// Windows: Run a start command with common error handling
#[cfg(target_os = "windows")]
fn run_windows_start_command(args: &[&str], terminal_name: &str) -> Result<(), String> {
    use std::process::Command;

    let mut full_args = vec!["/C", "start"];
    full_args.extend(args);

    let output = Command::new("cmd")
        .args(&full_args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("启动 {} 失败: {e}", terminal_name))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{} 启动失败 (exit code: {:?}): {}",
            terminal_name,
            output.status.code(),
            stderr
        ));
    }

    Ok(())
}

/// 打开用户首选终端并在其中执行一段可信命令脚本。脚本尾部 `read -n 1` / `pause`
/// 是刻意设计的——让命令退出后窗口不要瞬间关闭，用户才看得到 `command
/// not found` / `ModuleNotFoundError` 这类诊断信息。
///
/// **Security**：`command_line` 会被原样拼进 shell/batch 脚本，调用方必须
/// 保证它是可信字符串（当前只由后端硬编码调用）。
pub(crate) fn launch_terminal_running(command_line: &str, label: &str) -> Result<(), String> {
    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let (script_file, script_content) = {
        let file = temp_dir.join(format!("cc_switch_{}_{}.sh", label, pid));
        let content = format!(
            r#"#!/bin/bash
trap 'rm -f "{script_path}"' EXIT
echo "[cc-switch] Starting: {label}"
echo ""
{cmd}
echo ""
echo "[cc-switch] Command exited. Press any key to close."
read -n 1 -s
"#,
            script_path = file.display(),
            label = label,
            cmd = command_line,
        );
        (file, content)
    };

    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(&script_file, &script_content)
            .map_err(|e| format!("写入启动脚本失败: {e}"))?;
        std::fs::set_permissions(&script_file, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("设置脚本权限失败: {e}"))?;

        let preferred = crate::settings::get_preferred_terminal();
        let terminal = preferred.as_deref().unwrap_or("terminal");

        let result = match terminal {
            "iterm2" => launch_macos_iterm2(&script_file),
            "warp" => launch_macos_warp(&script_file),
            "alacritty" => launch_macos_open_app("Alacritty", &script_file, true),
            "kitty" => launch_macos_open_app("kitty", &script_file, false),
            "ghostty" => launch_macos_ghostty(&script_file),
            "wezterm" => launch_macos_open_app("WezTerm", &script_file, true),
            "kaku" => launch_macos_open_app("Kaku", &script_file, true),
            _ => launch_macos_terminal_app(&script_file),
        };

        if result.is_err() && terminal != "terminal" {
            log::warn!(
                "首选终端 {} 启动失败，回退到 Terminal.app: {:?}",
                terminal,
                result.as_ref().err()
            );
            return launch_macos_terminal_app(&script_file);
        }
        result
    }

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        std::fs::write(&script_file, &script_content)
            .map_err(|e| format!("写入启动脚本失败: {e}"))?;
        std::fs::set_permissions(&script_file, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("设置脚本权限失败: {e}"))?;

        let preferred = crate::settings::get_preferred_terminal();
        let default_terminals = [
            ("gnome-terminal", vec!["--"]),
            ("konsole", vec!["-e"]),
            ("xfce4-terminal", vec!["-e"]),
            ("mate-terminal", vec!["--"]),
            ("lxterminal", vec!["-e"]),
            ("alacritty", vec!["-e"]),
            ("kitty", vec!["-e"]),
            ("ghostty", vec!["-e"]),
        ];

        let terminals_to_try: Vec<(&str, Vec<&str>)> = if let Some(ref pref) = preferred {
            let pref_args = default_terminals
                .iter()
                .find(|(name, _)| *name == pref.as_str())
                .map(|(_, args)| args.to_vec())
                .unwrap_or_else(|| vec!["-e"]);
            let mut list = vec![(pref.as_str(), pref_args)];
            for (name, args) in &default_terminals {
                if *name != pref.as_str() {
                    list.push((*name, args.to_vec()));
                }
            }
            list
        } else {
            default_terminals
                .iter()
                .map(|(name, args)| (*name, args.to_vec()))
                .collect()
        };

        let mut last_error = String::from("未找到可用的终端");

        for (terminal, args) in terminals_to_try {
            let terminal_exists = which_command(terminal)
                || ["/usr/bin", "/bin", "/usr/local/bin"]
                    .iter()
                    .any(|dir| std::path::Path::new(&format!("{}/{}", dir, terminal)).exists());

            if terminal_exists {
                let spawn_result = Command::new(terminal)
                    .args(&args)
                    .arg("bash")
                    .arg(script_file.to_string_lossy().as_ref())
                    .spawn();
                match spawn_result {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        last_error = format!("执行 {} 失败: {}", terminal, e);
                    }
                }
            }
        }

        let _ = std::fs::remove_file(&script_file);
        Err(last_error)
    }

    #[cfg(target_os = "windows")]
    {
        let preferred = crate::settings::get_preferred_terminal();
        let terminal = preferred.as_deref().unwrap_or("cmd");

        let bat_file = temp_dir.join(format!("cc_switch_{}_{}.bat", label, pid));
        let content = format!(
            "@echo off\r\necho [cc-switch] Starting: {label}\r\necho.\r\n{cmd}\r\necho.\r\necho [cc-switch] Command exited. Press any key to close.\r\npause >nul\r\ndel \"%~f0\" >nul 2>&1\r\n",
            label = label,
            cmd = command_line,
        );
        std::fs::write(&bat_file, &content).map_err(|e| format!("写入批处理文件失败: {e}"))?;

        let bat_path = bat_file.to_string_lossy();
        let ps_cmd = format!("& '{}'", bat_path);

        let result = match terminal {
            "powershell" => run_windows_start_command(
                &["powershell", "-NoExit", "-Command", &ps_cmd],
                "PowerShell",
            ),
            "wt" => run_windows_start_command(&["wt", "cmd", "/K", &bat_path], "Windows Terminal"),
            _ => run_windows_start_command(&["cmd", "/K", &bat_path], "cmd"),
        };

        let final_result = if result.is_err() && terminal != "cmd" {
            log::warn!(
                "首选终端 {} 启动失败，回退到 cmd: {:?}",
                terminal,
                result.as_ref().err()
            );
            run_windows_start_command(&["cmd", "/K", &bat_path], "cmd")
        } else {
            result
        };

        // The .bat self-deletes (`del "%~f0"`) after it runs, but that only
        // fires if *some* terminal actually launched it. If every attempt
        // failed, sweep the temp file ourselves to avoid pollution.
        if final_result.is_err() {
            let _ = std::fs::remove_file(&bat_file);
        }
        final_result
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = (temp_dir, pid, command_line, label);
        Err("不支持的操作系统".to_string())
    }
}

/// 设置窗口主题（Windows/macOS 标题栏颜色）
/// theme: "dark" | "light" | "system"
#[tauri::command]
pub async fn set_window_theme(window: tauri::Window, theme: String) -> Result<(), String> {
    use tauri::Theme;

    let tauri_theme = match theme.as_str() {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None, // system default
    };

    window.set_theme(tauri_theme).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_extract_version() {
        assert_eq!(extract_version("claude 1.0.20"), "1.0.20");
        assert_eq!(extract_version("v2.3.4-beta.1"), "2.3.4-beta.1");
        assert_eq!(extract_version("no version here"), "no version here");
    }

    /// `parent_dir` 是锚定层"由 bin 路径推导同目录绝对路径"的基石,跨平台共用——
    /// 这里固化 `\`/`/`/混合分隔符/根边界四种情况,避免未来重构悄悄改语义。
    mod parent_dir_cases {
        use super::super::*;

        #[test]
        fn unix_path() {
            assert_eq!(
                parent_dir("/Users/me/.volta/bin/codex"),
                "/Users/me/.volta/bin"
            );
        }

        #[test]
        fn windows_backslash() {
            assert_eq!(
                parent_dir("C:\\Users\\me\\AppData\\Local\\Volta\\bin\\codex.exe"),
                "C:\\Users\\me\\AppData\\Local\\Volta\\bin"
            );
        }

        #[test]
        fn mixed_separators_takes_rightmost() {
            // Windows 上 `Path::join` 与字符串拼接可能产出混合分隔符;取**两种之中最右
            // 出现**的位置,而非"优先 `\`"——后者在混合时会取错父目录。
            assert_eq!(
                parent_dir("C:\\Users\\me/Code/openclaw\\codex.cmd"),
                "C:\\Users\\me/Code/openclaw"
            );
        }

        #[test]
        fn no_separator_returns_empty() {
            // 无父目录 → 空串,锚定层据此返 None、回退静态命令。
            assert_eq!(parent_dir("codex"), "");
        }

        #[test]
        fn separator_at_root_returns_empty() {
            // `/codex`:根目录是 index 0,`i > 0` 不满足 → 空串。同款行为对 Windows
            // 上的 `\codex` 也成立(实际不会出现,但语义对齐)。
            assert_eq!(parent_dir("/codex"), "");
            assert_eq!(parent_dir("\\codex"), "");
        }
    }

    /// Windows-only 锚定升级回归(等价类压缩到 3 种 idiom:volta/pnpm/npm)。整块通过
    /// `cfg(target_os = "windows")` gate,在 macOS/Linux 上不参与 cargo test;Windows
    /// CI 跑全套验证。tempdir 模拟 sibling 入口存在/不存在,锁定"扩展名顺序优先级 +
    /// 含空格路径自动加双引号 + 探不到 sibling → None 退静态"三件事。
    #[cfg(target_os = "windows")]
    mod anchored_upgrade_windows {
        use super::super::*;

        /// 在 tempdir 下创建子目录 `subdir`(空字符串则用 tempdir 根),放入 `entry`
        /// 与若干 `siblings` 假文件。返回 `(TempDir, 子目录, 入口绝对路径)`——TempDir
        /// 必须保活,否则析构后 fs 文件消失、`is_file()` 失败,测试假绿。
        fn setup_sibling(
            subdir: &str,
            entry: &str,
            siblings: &[&str],
        ) -> (tempfile::TempDir, std::path::PathBuf, String) {
            let dir = tempfile::tempdir().unwrap();
            let sub = if subdir.is_empty() {
                dir.path().to_path_buf()
            } else {
                dir.path().join(subdir)
            };
            std::fs::create_dir_all(&sub).unwrap();
            std::fs::write(sub.join(entry), "").unwrap();
            for s in siblings {
                std::fs::write(sub.join(s), "").unwrap();
            }
            let bin_path = sub.join(entry).to_string_lossy().to_string();
            (dir, sub, bin_path)
        }

        /// **必须与 `win_quote_path_for_batch` 主体保持镜像**——给 anchored 测试动态算
        /// expected,让用例在 temp 根目录含空格 / `&` / `(` / `%` 等特殊字符的开发机上
        /// 也能通过(默认 Windows `%TEMP%` = `C:\Users\<user>\AppData\Local\Temp`,
        /// 用户名带空格的机器整条 path 含空格、生产代码会正确加引号、测试硬编码无引号
        /// expected 会假失败)。
        ///
        /// 镜像引入"两边必须同步"的隐性依赖——回归防护层是 `win_quote_*` 那 7 个独立
        /// 单测,它们用硬编码字面值锁住 quoting 规则本身,即便此镜像漂移也会被那一组
        /// 测试 catch;反之亦然。
        fn expect_quoted_path(p: &str) -> String {
            let escaped = p.replace('%', "%%%%");
            let needs_quote = p
                .chars()
                .any(|c| matches!(c, ' ' | '&' | '(' | ')' | '^' | ';' | '<' | '>' | '|' | ','));
            if needs_quote {
                format!("\"{escaped}\"")
            } else {
                escaped
            }
        }

        /// `win_quote_path_for_user_shell` 的镜像——卸载命令是给用户复制到自己 shell
        /// 跑的,**不做** `%` → `%%%%` 转义(那是 .bat + call 两轮 expansion 的需要,
        /// 用户的 cmd 一轮、PowerShell/bash 零轮展开,4 倍转义会让三个 shell 都拿不到
        /// 原路径)。其他 token 边界字符的引号包裹规则与 `expect_quoted_path` 一致,
        /// 因为引号在三个 shell 下都保持字面。
        ///
        /// 与 `expect_quoted_path` 物理相邻并排放,提醒读者两个 helper **同步**反映
        /// 生产端 `win_quote_path_for_batch` / `win_quote_path_for_user_shell` 那对函数
        /// 的"目标终端决定 quoter"边界——任一边改了,另一边也得跟。
        fn expect_user_shell_quoted_path(p: &str) -> String {
            // 不替换 `%`——这正是与 expect_quoted_path 的唯一差异。
            let needs_quote = p
                .chars()
                .any(|c| matches!(c, ' ' | '&' | '(' | ')' | '^' | ';' | '<' | '>' | '|' | ','));
            if needs_quote {
                format!("\"{p}\"")
            } else {
                p.to_string()
            }
        }

        #[test]
        fn volta_windows_uses_volta_install() {
            // tempdir 路径里不含 "volta" 子串,所以在 tempdir 下手建一个 `Volta` 子目录
            // 才能让 `infer_install_source` 通过路径 normalize 后命中 `/volta/` 分支。
            // sibling 候选顺序 `[exe, cmd]`——Volta 是 Rust 写的 native binary,首选 .exe。
            // expected 通过 `expect_quoted_path` 算出,以适应 temp 根目录含特殊字符的环境。
            let (_dir, sub, bin_path) = setup_sibling("Volta", "codex.cmd", &["volta.exe"]);
            let cmd = anchored_command_from_paths("codex", &bin_path, &bin_path);
            let volta_full = format!("{}\\volta.exe", sub.to_string_lossy());
            let expected = format!("{} install @openai/codex", expect_quoted_path(&volta_full));
            assert_eq!(cmd.as_deref(), Some(expected.as_str()));
        }

        #[test]
        fn pnpm_windows_uses_pnpm_add() {
            // bin_path 落 `%LOCALAPPDATA%\pnpm\codex.cmd`,sibling 有 `pnpm.cmd` → 锚定到
            // `<dir>\pnpm.cmd add -g @openai/codex@latest`。用 add+@latest 而非 update,
            // 兼容"之前没通过 pnpm 装过"的幂等性场景。
            let (_dir, sub, bin_path) = setup_sibling("pnpm", "codex.cmd", &["pnpm.cmd"]);
            let cmd = anchored_command_from_paths("codex", &bin_path, &bin_path);
            let pnpm_full = format!("{}\\pnpm.cmd", sub.to_string_lossy());
            let expected = format!(
                "{} add -g @openai/codex@latest",
                expect_quoted_path(&pnpm_full)
            );
            assert_eq!(cmd.as_deref(), Some(expected.as_str()));
        }

        #[test]
        fn npm_windows_default_branch() {
            // 任意 system 类路径(不命中 volta/pnpm)→ 兜底 sibling npm.cmd 锚定。
            // 模拟 nvm-windows 的实际形态:`<NVM_HOME>\v22.0.0\codex.cmd`。
            let (_dir, sub, bin_path) = setup_sibling("v22.0.0", "codex.cmd", &["npm.cmd"]);
            let cmd = anchored_command_from_paths("codex", &bin_path, &bin_path);
            let npm_full = format!("{}\\npm.cmd", sub.to_string_lossy());
            let expected = format!(
                "{} i -g @openai/codex@latest",
                expect_quoted_path(&npm_full)
            );
            assert_eq!(cmd.as_deref(), Some(expected.as_str()));
        }

        #[test]
        fn npm_windows_no_sibling_returns_none() {
            // sibling npm.cmd 不存在(纯独立二进制,如 opencode install.sh 装的)→ 返 None,
            // 上游兜回静态 `call npm i -g`,前端据 anchored=false 给"默认入口无法确定"提示。
            let (_dir, _sub, bin_path) = setup_sibling("", "codex.cmd", &[]);
            let cmd = anchored_command_from_paths("codex", &bin_path, &bin_path);
            assert!(cmd.is_none());
        }

        #[test]
        fn hermes_windows_returns_none_for_pip() {
            // hermes 不在 npm_package_for 表中 → 返 None,上游退回静态 pip 命令
            // (`py -m pip install --upgrade hermes-agent[web] || python -m pip ...`)。
            // 故意把 npm.cmd 也放进 sibling 验证"npm 在但 pkg 不在"也是 None,不靠运气。
            let (_dir, _sub, bin_path) = setup_sibling("", "hermes.exe", &["npm.cmd"]);
            let cmd = anchored_command_from_paths("hermes", &bin_path, &bin_path);
            assert!(cmd.is_none());
        }

        #[test]
        fn windows_path_with_space_is_double_quoted() {
            // 含空格的路径(`C:\Program Files\...`)在生成命令时必须用双引号包,否则
            // bat / cmd /C 解析会把第一个空格当 token 分隔符,后续参数串错。**精确等值断言
            // 锁定引号位置**(starts_with+contains 会放过"双引号位置错了但仍能命中"的回归)。
            let (_dir, sub, bin_path) = setup_sibling("Program Files", "codex.cmd", &["npm.cmd"]);
            let cmd = anchored_command_from_paths("codex", &bin_path, &bin_path);
            let expected = format!(
                "\"{}\\npm.cmd\" i -g @openai/codex@latest",
                sub.to_string_lossy()
            );
            assert_eq!(cmd.as_deref(), Some(expected.as_str()));
        }

        #[test]
        fn windows_full_batch_line_for_percent_path_uses_quadruple_escape() {
            // **完整生成的 batch 行**(`call ` + anchored cmd)对含字面 `%` 的路径必须
            // 4 倍转义 `%foo%` → `%%%%foo%%%%`:.bat parser 一轮还原为 `%%foo%%`,call
            // 二轮再还原为 `%foo%` 字面。helper 单测验证的是 `win_quote_path_for_batch`
            // 内部转义,这条 integration 测验证 anchored_command_from_paths 输出 + call
            // 包装后,**最终落到 .bat 的字符串**仍然闭合两轮 expansion。
            let (_dir, sub, bin_path) = setup_sibling("path%foo%", "codex.cmd", &["npm.cmd"]);
            let anchored = anchored_command_from_paths("codex", &bin_path, &bin_path).unwrap();
            // build_tool_action_line Windows 分支最终拼的就是 `call <anchored>`(中间
            // 没有其他变换),这里直接用 format! 复刻那一步,无需暴露内部 API。
            let batch_line = format!("call {anchored}");
            // 用 `expect_quoted_path` 算 npm 全路径的期望 quoting,**同时覆盖 temp 根
            // 含空格的环境**(否则 sub 本身含空格 + 子目录 `path%foo%` 触发 4 倍 `%` 转义
            // 会让 expected 漏引号、假失败)。
            let npm_full = format!("{}\\npm.cmd", sub.to_string_lossy());
            let expected = format!(
                "call {} i -g @openai/codex@latest",
                expect_quoted_path(&npm_full)
            );
            assert_eq!(batch_line, expected);
            // 双重锁定:确认 4 倍转义子串存在 + 不出现"残留的二倍转义或字面 `%foo%`"。
            assert!(
                batch_line.contains("%%%%foo%%%%"),
                "batch 行应含 4 倍转义 `%%%%foo%%%%`: {batch_line}"
            );
            assert!(
                !batch_line.contains("path%foo%"),
                "batch 行不应含未转义的字面 `%foo%`(会被 call 二次解析展开): {batch_line}"
            );
        }

        #[test]
        fn windows_uninstall_for_percent_path_keeps_literal_percent() {
            // **P3 回归用例**(2026-05-23):卸载命令是给用户复制到自己 shell 跑的,
            // **不能复用 anchored upgrade 的 `win_quote_path_for_batch`**——后者把
            // `%` 转 `%%%%` 是因为命令落 .bat + call(两轮 expansion 后还原 `%foo%`),
            // 而卸载命令直接渲染给用户复制,cmd 只一轮、PowerShell/bash 零轮展开,
            // `%%%%foo%%%%` 在三个 shell 下都拿不到原路径。
            //
            // 这条 integration 测试是上面那条 windows_full_batch_line_for_percent_path...
            // 的精确对照:同样 `path%foo%` 子目录,升级路径生成 `%%%%foo%%%%`、
            // 卸载路径应保持字面 `%foo%`。两条用例并排锁住"目标终端决定 quoter"边界。
            let (_dir, sub, bin_path) = setup_sibling("path%foo%", "codex.cmd", &["npm.cmd"]);
            // tempdir 路径不命中 nvm/homebrew/volta/... 任何前缀 → source = "system",
            // 落到 Windows 版 uninstall 的 `_ => npm` 兜底分支(这正是本用例要验证的:
            // npm.cmd 同目录被 sibling 探到、anchored 到那处)。
            let source = infer_install_source(std::path::Path::new(&bin_path));
            let uninstall = uninstall_command_from_paths("codex", &bin_path, &bin_path, source);
            // 含字面 `%foo%`,**不含** `%%%%`(那是 batch quoter 的痕迹)。
            assert!(
                uninstall.contains("path%foo%"),
                "卸载命令应保持字面 `path%foo%`(cmd 用户字面识别;PowerShell/bash 也字面但 PowerShell 还需 `&` 前缀执行含引号命令——见 win_quote_path_for_user_shell doc): {uninstall}"
            );
            assert!(
                !uninstall.contains("%%%%"),
                "卸载命令不应含 4 倍 `%` 转义(那是 .bat+call 链路的 quoting): {uninstall}"
            );
            // 端到端形态:`<npm 路径> rm -g @openai/codex`,npm 路径在 sub 下。
            // sub 可能含空格(取决于 temp 根目录),用 user-shell quoter 镜像算 expected——
            // 与 expect_quoted_path 对偶,锁住"卸载侧不做 % 转义、其他字符引号规则同款"。
            let npm_full = format!("{}\\npm.cmd", sub.to_string_lossy());
            let expected_npm = expect_user_shell_quoted_path(&npm_full);
            assert_eq!(uninstall, format!("{expected_npm} rm -g @openai/codex"));
        }
    }

    /// Windows-only helpers 单测——在 macOS/Linux 上整块通过 cfg 排除,不参与 `cargo test`。
    /// Windows CI(或本机 Windows 跑 cargo test)会激活这些用例。覆盖:①双引号
    /// quoting 镜像 POSIX 版;②sibling_bin_with_ext 在 fs 上按 ext 顺序探到第一个存在的、
    /// 全部不存在/空 dir 时返 None。tempdir 提供干净 fs 沙盒。
    #[cfg(target_os = "windows")]
    mod windows_helpers {
        use super::super::*;

        #[test]
        fn win_quote_clean_path_stays_bare() {
            // 普通路径不含特殊字符 → 不加引号,命令展示干净。
            assert_eq!(
                win_quote_path_for_batch("C:\\Users\\me\\npm.cmd"),
                "C:\\Users\\me\\npm.cmd"
            );
        }

        #[test]
        fn win_quote_spaced_path_gets_quoted() {
            assert_eq!(
                win_quote_path_for_batch("C:\\Program Files\\nodejs\\npm.cmd"),
                "\"C:\\Program Files\\nodejs\\npm.cmd\""
            );
        }

        #[test]
        fn win_quote_ampersand_path_gets_quoted() {
            // `&` 是 cmd 命令分隔符,NTFS 允许在路径中出现;没有引号会让 `call C:\A&B\npm.cmd`
            // 被解析为 `call C:\A` + `B\npm.cmd` 两条命令,执行错乱。
            assert_eq!(
                win_quote_path_for_batch("C:\\Tools&Dev\\npm.cmd"),
                "\"C:\\Tools&Dev\\npm.cmd\""
            );
        }

        #[test]
        fn win_quote_parens_path_gets_quoted() {
            // `(` / `)` 在 .bat 中是代码块语义,引号内才是字面意义。
            assert_eq!(
                win_quote_path_for_batch("C:\\Foo(x86)\\npm.cmd"),
                "\"C:\\Foo(x86)\\npm.cmd\""
            );
        }

        #[test]
        fn win_quote_caret_path_gets_quoted() {
            // `^` 是 cmd 的 escape character;包引号后是字面意义。
            assert_eq!(
                win_quote_path_for_batch("C:\\foo^bar\\npm.cmd"),
                "\"C:\\foo^bar\\npm.cmd\""
            );
        }

        #[test]
        fn win_quote_percent_is_escaped_to_quadruple_percent() {
            // `%` 经历 .bat 一轮 + call 二轮 expansion,要让 call 最终看到字面 `%FOO%`
            // 需要源 .bat 里写 `%%%%FOO%%%%`(一轮 → `%%FOO%%`,二轮 → `%FOO%` 字面)。
            // 用 `%%` 二倍转义只在 echo / 直接执行场景对,call 调用时会被还原成 variable
            // reference 进而被替换。**这一条用例锁住"call 二次解析"必须被 4 倍转义闭合**。
            assert_eq!(
                win_quote_path_for_batch("C:\\path%foo%\\npm.cmd"),
                "C:\\path%%%%foo%%%%\\npm.cmd"
            );
        }

        #[test]
        fn win_quote_percent_with_space_gets_both() {
            // `%` 4 倍转义与外层引号正交——含空格触发引号、含 `%` 触发 `%%%%` 转义,叠加。
            assert_eq!(
                win_quote_path_for_batch("C:\\my %dir%\\npm.cmd"),
                "\"C:\\my %%%%dir%%%%\\npm.cmd\""
            );
        }

        #[test]
        fn win_quote_needs_quote_uses_original_path() {
            // 回归 guard:`needs_quote` 判定基于**原路径**,不能用 escape 后字符串——
            // 否则原本无 token 边界字符的路径(如 `C:\path%foo%\npm.cmd`)在 escape
            // 引入更多 `%` 后被错误识别成"需要引号"。这是实现 bug 的隐性入口。
            // 入参不含任何 token 边界字符 → 不应加外层引号、只做 `%` 4 倍转义。
            let out = win_quote_path_for_batch("C:\\foo%bar%\\npm.cmd");
            assert!(!out.starts_with('"'), "纯 `%` 路径不应加外层引号: {out}");
        }

        // ─── win_quote_path_for_user_shell:对照 win_quote_path_for_batch ───
        // 关键差异:**不做 `%` 转义**(目标是用户复制到 cmd/PowerShell/bash,
        // 不进 .bat + call 的两轮 expansion)。token 边界字符的引号包裹规则
        // 与 batch quoter 一致(三个 shell 下引号都保持字面)。

        #[test]
        fn user_shell_quote_clean_path_stays_bare() {
            // 与 batch quoter 同样的"无特殊字符不加引号"语义。
            assert_eq!(
                win_quote_path_for_user_shell("C:\\Users\\me\\npm.cmd"),
                "C:\\Users\\me\\npm.cmd"
            );
        }

        #[test]
        fn user_shell_quote_spaced_path_gets_quoted() {
            // 空格触发外层引号(防止 shell 拆 token),与 batch quoter 同款。
            assert_eq!(
                win_quote_path_for_user_shell("C:\\Program Files\\nodejs\\npm.cmd"),
                "\"C:\\Program Files\\nodejs\\npm.cmd\""
            );
        }

        #[test]
        fn user_shell_quote_percent_stays_literal_unlike_batch() {
            // **回归用例**(P3 反馈):卸载命令是用户复制到自己 shell 跑的,
            // **不能用 batch 的 4 倍 % 转义**。对比:
            // - win_quote_path_for_batch("C:\\path%foo%\\npm.cmd")
            //     → "C:\\path%%%%foo%%%%\\npm.cmd"  (供 .bat+call 两轮展开后还原 `%foo%`)
            // - win_quote_path_for_user_shell("C:\\path%foo%\\npm.cmd")
            //     → "C:\\path%foo%\\npm.cmd"        (保持原 `%`,让用户自决)
            // 若用 batch 那个 quoter,粘到 cmd 一轮展开变 `%%`、粘到 PowerShell/bash
            // 直接字面 `%%%%`,三个 shell 都拿不到原路径。
            assert_eq!(
                win_quote_path_for_user_shell("C:\\path%foo%\\npm.cmd"),
                "C:\\path%foo%\\npm.cmd"
            );
        }

        #[test]
        fn user_shell_quote_percent_with_space_keeps_percent_literal() {
            // 与 batch quoter 的 win_quote_percent_with_space_gets_both 形成精确对照:
            // - batch: "C:\\my %dir%\\npm.cmd"  → "\"C:\\my %%%%dir%%%%\\npm.cmd\""
            // - user_shell: same input         → "\"C:\\my %dir%\\npm.cmd\""
            // 空格仍触发引号(token 边界、跨 shell 一致),`%` 保持字面(目标 shell 不确定)。
            assert_eq!(
                win_quote_path_for_user_shell("C:\\my %dir%\\npm.cmd"),
                "\"C:\\my %dir%\\npm.cmd\""
            );
        }

        #[test]
        fn sibling_bin_picks_first_existing_extension() {
            // 同目录同时存在 `npm.cmd` 和 `npm.exe` 时,候选顺序 `[cmd, exe]` 应取 .cmd——
            // 这是 Node.js 官方 installer 装出来的 idiom(.cmd 是入口、.exe 是 wrapper)。
            let dir = tempfile::tempdir().unwrap();
            let cmd_path = dir.path().join("npm.cmd");
            let exe_path = dir.path().join("npm.exe");
            std::fs::write(&cmd_path, "").unwrap();
            std::fs::write(&exe_path, "").unwrap();

            let codex = dir.path().join("codex.cmd").to_string_lossy().to_string();
            let found = sibling_bin_with_ext(&codex, "npm", &["cmd", "exe"]).unwrap();
            assert_eq!(found, cmd_path.to_string_lossy());
        }

        #[test]
        fn sibling_bin_volta_prefers_exe() {
            // Volta 是 Rust 写的 native binary,扩展名顺序应是 [exe, cmd]——若只有 .exe
            // 存在(常见情形),探到的就是它。
            let dir = tempfile::tempdir().unwrap();
            let exe_path = dir.path().join("volta.exe");
            std::fs::write(&exe_path, "").unwrap();

            let codex = dir.path().join("codex.exe").to_string_lossy().to_string();
            let found = sibling_bin_with_ext(&codex, "volta", &["exe", "cmd"]).unwrap();
            assert_eq!(found, exe_path.to_string_lossy());
        }

        #[test]
        fn sibling_bin_returns_none_when_none_exist() {
            // 同目录下没有任何候选 → None,锚定层据此退到静态命令。
            let dir = tempfile::tempdir().unwrap();
            let codex = dir.path().join("codex.cmd").to_string_lossy().to_string();
            assert!(sibling_bin_with_ext(&codex, "npm", &["cmd", "exe"]).is_none());
        }

        #[test]
        fn sibling_bin_returns_none_when_no_parent() {
            // bin_path 没有目录部分(纯文件名) → parent_dir 空串 → 返 None。
            assert!(sibling_bin_with_ext("codex.cmd", "npm", &["cmd"]).is_none());
        }

        #[test]
        fn wsl_hermes_command_uses_python3_not_py_launcher() {
            // 跨 wsl.exe 边界后跑的是 Linux,Windows 的 `py` launcher 不存在(PEP 397
            // 是 Windows 独有)、Ubuntu 24.04+ 连 `python` alias 也没有。
            // `tool_action_shell_command` 在 Windows target 给 hermes 返回 `py -m pip ...`
            // 是给 Windows 原生跑用的,WSL 分支必须改走 unix 版命令——这一切由
            // `wsl_tool_action_shell_command` 替换 hermes case 实现。
            let cmd = wsl_tool_action_shell_command("hermes", ToolLifecycleAction::Update).unwrap();
            assert!(
                cmd.starts_with("python3 -m pip"),
                "WSL hermes 必须用 python3 起手,得到: {cmd}"
            );
            assert!(
                !cmd.contains("py -m pip"),
                "WSL hermes 不能含 Windows 独有的 py launcher 命令,得到: {cmd}"
            );
        }

        #[test]
        fn wsl_npm_tools_passthrough_to_windows_target_command() {
            // npm 类工具的命令在 Windows/Unix 上字符串相同 → wrapper 直接透传给
            // `tool_action_shell_command`,无需额外替换。
            let cmd = wsl_tool_action_shell_command("claude", ToolLifecycleAction::Update).unwrap();
            assert_eq!(cmd, "npm i -g @anthropic-ai/claude-code@latest");
        }
    }

    /// `infer_install_source` 是判定锚定 idiom 的入口——nvm/homebrew/volta/pnpm/...
    /// 各对应不同的升级命令形态。函数内部已 `replace('\\','/').to_ascii_lowercase()`
    /// 归一化,Windows 反斜杠 + 大小写差异在此处不需要分平台。这里固化"哪条路径
    /// 算哪种来源"的归类断言,避免未来调整子串顺序时静默改变分类。
    mod install_source_classification {
        use super::super::*;
        use std::path::Path;

        #[test]
        fn macos_volta_with_dot_prefix() {
            assert_eq!(
                infer_install_source(Path::new("/Users/me/.volta/bin/codex")),
                "volta"
            );
        }

        #[test]
        fn windows_volta_localappdata_no_dot() {
            // `%LOCALAPPDATA%\Volta\bin\codex.exe` —— 没有前导点,靠兜底的 `/volta/`
            // 命中(归一化后小写)。如果只识别 `/.volta/`,Windows 这一类会落到 system。
            assert_eq!(
                infer_install_source(Path::new(
                    "C:\\Users\\me\\AppData\\Local\\Volta\\bin\\codex.exe"
                )),
                "volta"
            );
        }

        #[test]
        fn windows_pnpm_localappdata() {
            // `%LOCALAPPDATA%\pnpm\codex.cmd` —— pnpm 全局 bin 目录,识别为 pnpm 后
            // 锚定命令走 `pnpm add -g <pkg>@latest`,而不是 sibling npm。
            assert_eq!(
                infer_install_source(Path::new("C:\\Users\\me\\AppData\\Local\\pnpm\\codex.cmd")),
                "pnpm"
            );
        }

        #[test]
        fn windows_nvm_falls_back_to_system() {
            // nvm-windows 安装的工具路径不含 `.nvm`(它通常装在 `%APPDATA%\nvm` 或
            // `C:\Program Files\nodejs` symlink),刻意不识别成专属 source——锚定层
            // 会按 system → sibling npm.cmd 处理,跟 nvm-windows 的实际 idiom 一致
            // (它的全局包就是当前选中的 node 的 npm 装的)。
            assert_eq!(
                infer_install_source(Path::new(
                    "C:\\Users\\me\\AppData\\Roaming\\nvm\\v22.0.0\\codex.cmd"
                )),
                "system"
            );
        }

        #[test]
        fn windows_scoop_still_identified() {
            // Scoop 已有 `/scoop/` 分支;我们的 6 个工具都不是 scoop formula,所以这条
            // 实际不影响锚定决策(锚定层会用 sibling npm.cmd),但归类保留方便未来。
            assert_eq!(
                infer_install_source(Path::new("C:\\Users\\me\\scoop\\shims\\codex.cmd")),
                "scoop"
            );
        }
    }

    /// 锚定升级命令生成：用真实勘察到的安装路径固化为回归断言——
    /// 一台机器上 4 个工具恰好对应 4 种升级方式（原生 self-update / brew / nvm npm /
    /// homebrew npm），任何改动若打破其中一种都会立刻被这些用例拦下。
    #[cfg(not(target_os = "windows"))]
    mod anchored_upgrade {
        use super::super::*;
        use std::path::Path;

        /// 测试 helper:跟生产代码 `enumerate_tool_installations` 同款契约——从 bin 路径
        /// 推 source 标签,再传给 `uninstall_command_from_paths`。封一层让 uninstall_*
        /// 用例的 4 参数调用形态简洁。
        fn src(bin: &str) -> &'static str {
            infer_install_source(Path::new(bin))
        }

        fn inst(path: &str, is_default: bool) -> ToolInstallation {
            ToolInstallation {
                path: path.to_string(),
                version: None,
                runnable: true,
                error: None,
                source: infer_install_source(Path::new(path)).to_string(),
                is_path_default: is_default,
                // 升级锚定测试不关心 uninstall_command(那条由 uninstall_* 用例覆盖);
                // 填空串避免与本组用例的"升级命令应该是什么"互相干扰。
                uninstall_command: String::new(),
                uninstall_command_needs_cmd_hint: false,
                // 测试场景下不需要走 fs canonicalize——POSIX 锚定测试关心的是
                // path/real 都被传给 anchored_command_from_paths 的纯字符串判定,
                // 已有用例(brew_formula_extraction / claude_native_*)是直接
                // 调 anchored_command_from_paths,不通过 installs_anchored_command,
                // 这里 real 是给上层 default_install + read 用,填同值即可。
                real: std::path::PathBuf::from(path),
            }
        }

        #[test]
        fn claude_native_installer_uses_self_update() {
            // ~/.local/bin/claude → 真身在 ~/.local/share/claude/versions/,自带 self-update;
            // 它不归 npm 管,且在 PATH 里比 nvm/homebrew 更靠前,用 npm 升级纯属白装。
            // **绝对路径调用 launcher** 避免 GUI 非登录 `bash -c` 时 PATH 没有
            // ~/.local/bin 导致 `claude: not found`(exit 127)而失败。
            let cmd = anchored_command_from_paths(
                "claude",
                "/Users/me/.local/bin/claude",
                "/Users/me/.local/share/claude/versions/2.1.146",
            );
            assert_eq!(cmd.as_deref(), Some("/Users/me/.local/bin/claude update"));
        }

        #[test]
        fn gemini_homebrew_formula_uses_brew_upgrade() {
            // /opt/homebrew/bin/gemini → Cellar/gemini-cli/...:是 brew formula 而非 npm 全局包,
            // 且 formula 名(gemini-cli) ≠ npm 包名(@google/gemini-cli)。
            // **brew 与 formula 入口同目录**,用 `<dir>/brew` 绝对路径调用,避免 GUI
            // 非登录 `bash -c` 时 PATH 没有 /opt/homebrew/bin 导致 `brew: not found`。
            let cmd = anchored_command_from_paths(
                "gemini",
                "/opt/homebrew/bin/gemini",
                "/opt/homebrew/Cellar/gemini-cli/0.13.0/libexec/lib/node_modules/@google/gemini-cli/dist/index.js",
            );
            assert_eq!(
                cmd.as_deref(),
                Some("/opt/homebrew/bin/brew upgrade gemini-cli")
            );
        }

        #[test]
        fn codex_nvm_anchors_to_that_npm() {
            // 命令行命中 nvm 那处 → 升级写回同一个 node 的 npm，而非 PATH 第一个 npm。
            let cmd = anchored_command_from_paths(
                "codex",
                "/Users/me/.nvm/versions/node/v22.14.0/bin/codex",
                "/Users/me/.nvm/versions/node/v22.14.0/lib/node_modules/@openai/codex/bin/codex.js",
            );
            assert_eq!(
                cmd.as_deref(),
                Some("/Users/me/.nvm/versions/node/v22.14.0/bin/npm i -g @openai/codex@latest")
            );
        }

        #[test]
        fn homebrew_npm_global_package_anchors_not_brew() {
            // openclaw 装在 Homebrew node 的全局目录(lib/node_modules，非 Cellar)：
            // 是 npm 全局包，走 npm 锚定而非 brew upgrade。
            let cmd = anchored_command_from_paths(
                "openclaw",
                "/opt/homebrew/bin/openclaw",
                "/opt/homebrew/lib/node_modules/openclaw/openclaw.mjs",
            );
            assert_eq!(
                cmd.as_deref(),
                Some("/opt/homebrew/bin/npm i -g openclaw@latest")
            );
        }

        #[test]
        fn volta_uses_volta_install() {
            // `~/.volta/bin` 通常不在 GUI 非登录 `bash -c` 的 PATH 里,且用户可能
            // PATH 上还有另一份 volta → 必须绝对路径锚定到命令行命中的这一份。
            let cmd = anchored_command_from_paths(
                "codex",
                "/Users/me/.volta/bin/codex",
                "/Users/me/.volta/tools/image/packages/codex/lib/node_modules/@openai/codex",
            );
            assert_eq!(
                cmd.as_deref(),
                Some("/Users/me/.volta/bin/volta install @openai/codex")
            );
        }

        #[test]
        fn bun_uses_bun_add() {
            // bun 同 volta:`~/.bun/bin` 几乎不在 GUI PATH 里,绝对路径锚定。
            let cmd = anchored_command_from_paths(
                "opencode",
                "/Users/me/.bun/bin/opencode",
                "/Users/me/.bun/install/global/node_modules/opencode-ai/bin/opencode",
            );
            assert_eq!(
                cmd.as_deref(),
                Some("/Users/me/.bun/bin/bun add -g opencode-ai@latest")
            );
        }

        #[test]
        fn volta_path_with_space_is_quoted() {
            // volta 分支用 `<dir>/volta`,目录含空格时同样要 POSIX 引号包裹。
            let cmd = anchored_command_from_paths(
                "codex",
                "/Users/my name/.volta/bin/codex",
                "/Users/my name/.volta/tools/image/packages/codex/lib/node_modules/@openai/codex",
            );
            assert_eq!(
                cmd.as_deref(),
                Some("'/Users/my name/.volta/bin/volta' install @openai/codex")
            );
        }

        #[test]
        fn bun_path_with_space_is_quoted() {
            // bun 分支与 volta 共享 sibling_bin + quote_path_if_spaced,
            // 这条用例锁住 `bun add -g` 命令头部的引号包裹形态。
            let cmd = anchored_command_from_paths(
                "opencode",
                "/Users/my name/.bun/bin/opencode",
                "/Users/my name/.bun/install/global/node_modules/opencode-ai/bin/opencode",
            );
            assert_eq!(
                cmd.as_deref(),
                Some("'/Users/my name/.bun/bin/bun' add -g opencode-ai@latest")
            );
        }

        #[test]
        fn hermes_has_no_npm_anchor() {
            // hermes 走 pip，不在 npm 包表 → 锚定返回 None，调用方回退到静态 pip 升级命令。
            let cmd = anchored_command_from_paths(
                "hermes",
                "/usr/local/bin/hermes",
                "/usr/local/bin/hermes",
            );
            assert_eq!(cmd, None);
        }

        #[test]
        fn system_install_without_sibling_npm_is_not_anchored() {
            // opencode install.sh 装到 ~/.opencode/bin（独立二进制、无同级 npm）：
            // 不能锚定到 `<dir>/npm`（必失败），返回 None 让调用方回退静态命令。
            let cmd = anchored_command_from_paths(
                "opencode",
                "/Users/me/.opencode/bin/opencode",
                "/Users/me/.opencode/bin/opencode",
            );
            assert_eq!(cmd, None);
        }

        #[test]
        fn go_bin_install_is_not_anchored() {
            // ~/go/bin 同理：无同级 npm。
            let cmd = anchored_command_from_paths(
                "opencode",
                "/Users/me/go/bin/opencode",
                "/Users/me/go/bin/opencode",
            );
            assert_eq!(cmd, None);
        }

        #[test]
        fn fnm_install_anchors_to_that_npm() {
            // fnm 是自带同级 npm 的 node 管理器 → 锚定到那处的 npm。
            let cmd = anchored_command_from_paths(
                "codex",
                "/Users/me/.local/share/fnm_multishells/12345_abc/bin/codex",
                "/Users/me/.local/share/fnm_multishells/12345_abc/lib/node_modules/@openai/codex/bin/codex.js",
            );
            assert_eq!(
                cmd.as_deref(),
                Some(
                    "/Users/me/.local/share/fnm_multishells/12345_abc/bin/npm i -g @openai/codex@latest"
                )
            );
        }

        #[test]
        fn path_with_space_is_quoted() {
            let cmd = anchored_command_from_paths(
                "codex",
                "/Users/my name/.nvm/versions/node/v22/bin/codex",
                "/Users/my name/.nvm/versions/node/v22/lib/node_modules/@openai/codex/bin/codex.js",
            );
            assert_eq!(
                cmd.as_deref(),
                Some("'/Users/my name/.nvm/versions/node/v22/bin/npm' i -g @openai/codex@latest")
            );
        }

        #[test]
        fn claude_native_path_with_space_is_quoted() {
            // claude 分支同样要 POSIX 引号包裹含空格的 bin_path,
            // 否则 `/Users/my name/.local/bin/claude update` 会被 shell 拆词。
            let cmd = anchored_command_from_paths(
                "claude",
                "/Users/my name/.local/bin/claude",
                "/Users/my name/.local/share/claude/versions/2.1.146",
            );
            assert_eq!(
                cmd.as_deref(),
                Some("'/Users/my name/.local/bin/claude' update")
            );
        }

        #[test]
        fn brew_path_with_space_is_quoted() {
            // brew 分支用 `<bin_path 同目录>/brew`,目录含空格时同样要引号包裹。
            let cmd = anchored_command_from_paths(
                "gemini",
                "/opt/my brew/bin/gemini",
                "/opt/my brew/Cellar/gemini-cli/0.13.0/libexec/lib/node_modules/@google/gemini-cli/dist/index.js",
            );
            assert_eq!(
                cmd.as_deref(),
                Some("'/opt/my brew/bin/brew' upgrade gemini-cli")
            );
        }

        #[test]
        fn brew_formula_extraction() {
            assert_eq!(
                brew_formula_from_path("/opt/homebrew/Cellar/gemini-cli/0.13.0/bin/gemini")
                    .as_deref(),
                Some("gemini-cli")
            );
            // node 全局包不在 Cellar 下 → 不是 formula。
            assert_eq!(
                brew_formula_from_path("/opt/homebrew/lib/node_modules/openclaw/openclaw.mjs"),
                None
            );
            assert_eq!(
                brew_formula_from_path("/Users/me/.nvm/versions/node/v22/lib/node_modules/x"),
                None
            );
        }

        // ─── uninstall command (bug fix: 卸载命令的 source-aware 锚定) ───
        // 这组测试与上面 anchored upgrade 是对偶关系——同一组真身判定要在卸载侧
        // 也产出正确命令。先前卸载命令在前端拼,前端只拿到 `source` 字符串,无法
        // 区分 brew formula vs homebrew npm 全局包,把所有 `/opt/homebrew/bin/*`
        // 默认拼成 `npm rm -g`,对 brew formula 必然 `no such package` 失败。
        // 修复后卸载在后端拼,复用 brew_formula_from_path + sibling_bin。

        #[test]
        fn brew_formula_uninstall_uses_brew_uninstall() {
            // 关键回归用例:Jason 2026-05-23 实测的 bug 现场——
            // /opt/homebrew/bin/gemini 真身在 /Cellar/gemini-cli/,前端给出
            // `npm rm -g @google/gemini-cli` 会失败。修复后应该是 brew uninstall。
            // formula 名 ≠ npm 包名(gemini-cli ≠ @google/gemini-cli),
            // 用 npm 包名做参数会卸错对象。
            let bin = "/opt/homebrew/bin/gemini";
            let cmd = uninstall_command_from_paths(
                "gemini",
                bin,
                "/opt/homebrew/Cellar/gemini-cli/0.43.0/libexec/lib/node_modules/@google/gemini-cli/dist/index.js",
                src(bin),
            );
            assert_eq!(cmd, "/opt/homebrew/bin/brew uninstall gemini-cli");
        }

        #[test]
        fn homebrew_npm_global_uninstall_uses_npm_rm() {
            // openclaw 装在 Homebrew node 的全局目录(lib/node_modules,**非 Cellar**)
            // 是 npm 全局包,应该走 npm rm -g 而不是 brew uninstall。
            // 这条 case 锁住"homebrew formula vs homebrew npm 全局"的边界——
            // 两者前端 source 都是 "homebrew",必须靠后端真身判定区分。
            //
            // 命令尾巴附加物理删除兜底:`npm rm -g` 在某些环境(老 node 损坏 npm
            // 元数据)静默 no-op,`[ -e bin ] && rm -f bin && rm -rf pkg_dir` 兜底
            // 让真正不生效时仍能清理。详见 uninstall_command_from_paths doc。
            let bin = "/opt/homebrew/bin/openclaw";
            let cmd = uninstall_command_from_paths(
                "openclaw",
                bin,
                "/opt/homebrew/lib/node_modules/openclaw/dist/cli.js",
                src(bin),
            );
            assert_eq!(
                cmd,
                concat!(
                    "/opt/homebrew/bin/npm rm -g openclaw",
                    "; [ -e /opt/homebrew/bin/openclaw ]",
                    " && rm -f /opt/homebrew/bin/openclaw",
                    " && rm -rf /opt/homebrew/lib/node_modules/openclaw",
                )
            );
        }

        #[test]
        fn nvm_uninstall_anchors_to_that_npm() {
            // nvm node18 v18.20.8 装的 @google/gemini-cli 0.1.7——Jason 实测的
            // 第二处安装。卸载必须锚定到 v18.20.8 下的 npm,避免误删 v22.14.0 的。
            //
            // **这条用例是 npm rm 静默 no-op bug 的关键回归现场**:Jason 实测
            // 此机型上 `npm rm -g @google/gemini-cli` 返回 `up to date in 91ms`
            // exit 0 不动包目录。命令尾巴的物理删除兜底专门防御此 case。
            let bin = "/Users/me/.nvm/versions/node/v18.20.8/bin/gemini";
            let cmd = uninstall_command_from_paths(
                "gemini",
                bin,
                "/Users/me/.nvm/versions/node/v18.20.8/lib/node_modules/@google/gemini-cli/dist/index.js",
                src(bin),
            );
            assert_eq!(
                cmd,
                concat!(
                    "/Users/me/.nvm/versions/node/v18.20.8/bin/npm rm -g @google/gemini-cli",
                    "; [ -e /Users/me/.nvm/versions/node/v18.20.8/bin/gemini ]",
                    " && rm -f /Users/me/.nvm/versions/node/v18.20.8/bin/gemini",
                    " && rm -rf /Users/me/.nvm/versions/node/v18.20.8/lib/node_modules/@google/gemini-cli",
                )
            );
        }

        #[test]
        fn npm_uninstall_fallback_quotes_spaced_paths() {
            // 兜底链里有 3 处路径(npm / bin / pkg_dir),含空格时 3 处都要单引号包裹,
            // 否则 `[ -e /Users/my name/.../bin/gemini ]` 会被 shell 拆成多 token。
            // 锁住 quote_path_if_spaced 应用在兜底所有路径上,而非只 npm 头部。
            let bin = "/Users/my name/.nvm/versions/node/v22/bin/gemini";
            let cmd = uninstall_command_from_paths(
                "gemini",
                bin,
                "/Users/my name/.nvm/versions/node/v22/lib/node_modules/@google/gemini-cli/dist/index.js",
                src(bin),
            );
            assert_eq!(
                cmd,
                concat!(
                    "'/Users/my name/.nvm/versions/node/v22/bin/npm' rm -g @google/gemini-cli",
                    "; [ -e '/Users/my name/.nvm/versions/node/v22/bin/gemini' ]",
                    " && rm -f '/Users/my name/.nvm/versions/node/v22/bin/gemini'",
                    " && rm -rf '/Users/my name/.nvm/versions/node/v22/lib/node_modules/@google/gemini-cli'",
                )
            );
        }

        #[test]
        fn npm_uninstall_fallback_strong_quotes_shell_metachars() {
            // **P2 回归用例**:卸载命令是 destructive(`rm -rf <pkg_dir>`),路径含
            // `;` / `'` / `$` / `` ` `` 等 shell 元字符时必须强 quote——否则
            // `[ -e /Users/foo;bad/.../gemini ]` 会被 shell 拆成 `[ -e /Users/foo]`
            // + `bad/.../gemini ] && rm -rf ...`,destructive 命令落到错的路径。
            // `quote_path_if_spaced` 只查空格,本用例确保升级到
            // `posix_quote_for_user_shell`(白名单)后,危险字符触发 `shell_single_quote`。
            //
            // 用 `;` 作为代表性元字符——所有非 `[A-Za-z0-9._/+=:-]` 字符都走同一
            // shell_single_quote 路径,锁一个就够覆盖白名单切换语义。
            let bin = "/Users/foo;bar/.nvm/versions/node/v22/bin/gemini";
            let cmd = uninstall_command_from_paths(
                "gemini",
                bin,
                "/Users/foo;bar/.nvm/versions/node/v22/lib/node_modules/@google/gemini-cli/dist/index.js",
                src(bin),
            );
            // npm/bin/pkg_dir 三处都被单引号包裹(shell_single_quote 的形态)。
            // 这条 assert_eq 锁住"含 `;` 路径**所有**位置都强 quote",任一漏掉
            // 都会让 shell 拆词、可能 destructive。
            assert_eq!(
                cmd,
                concat!(
                    "'/Users/foo;bar/.nvm/versions/node/v22/bin/npm' rm -g @google/gemini-cli",
                    "; [ -e '/Users/foo;bar/.nvm/versions/node/v22/bin/gemini' ]",
                    " && rm -f '/Users/foo;bar/.nvm/versions/node/v22/bin/gemini'",
                    " && rm -rf '/Users/foo;bar/.nvm/versions/node/v22/lib/node_modules/@google/gemini-cli'",
                )
            );
        }

        #[test]
        fn npm_uninstall_fallback_quotes_single_quote_in_path() {
            // 路径含字面 `'`——`shell_single_quote` 的 `'"'"'` 转义形式必须能 round-trip。
            // 形态:`o'leary` → `'o'"'"'leary'`(关单引号 + 双引号包字面单引号 + 重开
            // 单引号),跨 bash/zsh/dash/sh 全部 POSIX 兼容。本用例在卸载兜底链上下文
            // 验证 npm/bin/pkg_dir 三处都正确 round-trip,防止未来"换 quoter"重构时
            // 漏掉单引号场景(单引号是最棘手的 shell quoting 边界)。
            //
            // **关键 ToolInstallation.uninstall_command_needs_cmd_hint 边界**(reviewer
            // 2026-05-24 抓的 false-positive):此命令含 `"`(escape 的一部分),但
            // 它是 **POSIX shell_single_quote 形态**,**不是** Windows cmd 引号路径。
            // 前端 `cmd.includes('"')` 在此场景会**误判**为 Windows cmd 形式,
            // 给 macOS 用户名带 `'`(如 `o'leary`)的用户错误显示 PowerShell 提示。
            // 这就是为什么 `uninstall_command_needs_cmd_hint` 必须由后端用
            // `cfg!(target_os = "windows") && contains('"')` 算——`cfg!` 是编译时
            // ground truth, POSIX 编译时整体短路 false, 与含 `"` 的字面巧合无关。
            // 本测试的存在 = 字面 contains('"') 永远 true, cfg 短路是唯一防线。
            let bin = "/Users/o'leary/.nvm/versions/node/v22/bin/gemini";
            let cmd = uninstall_command_from_paths(
                "gemini",
                bin,
                "/Users/o'leary/.nvm/versions/node/v22/lib/node_modules/@google/gemini-cli/dist/index.js",
                src(bin),
            );
            // 验证三处都得到同款 `'"'"'` 转义形态。
            assert!(
                cmd.contains("'/Users/o'\"'\"'leary/.nvm/versions/node/v22/bin/npm'"),
                "npm 路径单引号转义不对: {cmd}"
            );
            assert!(
                cmd.contains("'/Users/o'\"'\"'leary/.nvm/versions/node/v22/bin/gemini'"),
                "bin 路径单引号转义不对: {cmd}"
            );
            assert!(
                cmd.contains("'/Users/o'\"'\"'leary/.nvm/versions/node/v22/lib/node_modules/@google/gemini-cli'"),
                "pkg_dir 路径单引号转义不对: {cmd}"
            );
            // 显式断言:命令字面**确实**含 `"`——这是上面 doc 关键边界的存在证明。
            // 此 assert 让"前端 string match 是 false-positive"的论据有了不可绕过的
            // 测试支撑(未来若 shell_single_quote 改用别的 escape 形式,本断言会失败、
            // 触发 reviewer 评估是否还需要 cfg 防御层)。
            assert!(
                cmd.contains('"'),
                "POSIX shell_single_quote 转义形态必须含 `\"`(本用例的存在前提): {cmd}"
            );
        }

        #[test]
        fn npm_uninstall_fallback_only_for_anchored_branches() {
            // 兜底链**只在 nvm/fnm/mise/homebrew npm 全局四个分支出现**(那里 bin 与
            // pkg_dir 都能可靠推导)。其他分支不应该带兜底:
            // - brew formula: brew uninstall 没观察到 no-op
            // - volta: volta uninstall 没观察到 no-op
            // - bun: bun rm 没观察到 no-op
            // - system/unknown: 路径不可靠,**不要拼出可能误指根目录的 rm -rf**
            // 这条用例反向锁住——任意分支若误带兜底,会被这里 catch。
            //
            // **不包含 hermes** :hermes 在函数开头就 early return pip uninstall,根本
            // 不进 source match 分支,断言对 pip uninstall 是 vacuously true。hermes
            // 的"不进 npm 分支"已由 hermes_uninstall_uses_pip 用例独立锁住,这里再放
            // 一行只会误导读者(让人以为本用例在验证"hermes 走 npm 分支也不带兜底")。
            for (tool, bin, real) in [
                ("gemini", "/opt/homebrew/bin/gemini",
                 "/opt/homebrew/Cellar/gemini-cli/0.43.0/libexec/lib/node_modules/@google/gemini-cli/dist/index.js"),
                ("codex", "/Users/me/.volta/bin/codex",
                 "/Users/me/.volta/tools/image/packages/@openai/codex/lib/cli.js"),
                ("opencode", "/Users/me/.bun/bin/opencode",
                 "/Users/me/.bun/install/global/node_modules/opencode-ai/bin/opencode"),
                ("opencode", "/Users/me/.opencode/bin/opencode", "/Users/me/.opencode/bin/opencode"),
            ] {
                let cmd = uninstall_command_from_paths(tool, bin, real, src(bin));
                assert!(
                    !cmd.contains("rm -rf"),
                    "tool={tool} bin={bin}: 该分支不应含 rm -rf 兜底,实际生成: {cmd}"
                );
                assert!(
                    !cmd.contains("[ -e "),
                    "tool={tool} bin={bin}: 该分支不应含 `[ -e ]` 测试,实际生成: {cmd}"
                );
            }
        }

        #[test]
        fn volta_uninstall_uses_volta_uninstall() {
            let bin = "/Users/me/.volta/bin/codex";
            let cmd = uninstall_command_from_paths(
                "codex",
                bin,
                "/Users/me/.volta/tools/image/packages/@openai/codex/lib/cli.js",
                src(bin),
            );
            assert_eq!(cmd, "/Users/me/.volta/bin/volta uninstall @openai/codex");
        }

        #[test]
        fn bun_uninstall_uses_bun_rm() {
            let bin = "/Users/me/.bun/bin/opencode";
            let cmd = uninstall_command_from_paths(
                "opencode",
                bin,
                "/Users/me/.bun/install/global/node_modules/opencode-ai/bin/opencode",
                src(bin),
            );
            assert_eq!(cmd, "/Users/me/.bun/bin/bun rm -g opencode-ai");
        }

        #[test]
        fn pnpm_uninstall_uses_pnpm_rm() {
            // **review M1 回归用例**:pnpm 全局包不在 npm prefix 下,落到 npm rm -g
            // 会"not installed"必失败。Windows anchored upgrade 早已有 pnpm 分支,
            // POSIX 卸载补齐对偶——`~/.local/share/pnpm/<cli>` 是 pnpm 在 macOS/Linux
            // 上的标准全局 bin 形态,被 infer_install_source 归为 "pnpm" 类。
            let bin = "/Users/me/.local/share/pnpm/codex";
            let cmd = uninstall_command_from_paths(
                "codex",
                bin,
                "/Users/me/.local/share/pnpm/global/5/node_modules/@openai/codex/dist/cli.js",
                src(bin),
            );
            assert_eq!(cmd, "/Users/me/.local/share/pnpm/pnpm rm -g @openai/codex");
        }

        #[test]
        fn hermes_uninstall_uses_pip() {
            // hermes 走 pip,与锚定无关——包名固定 hermes-agent。
            let bin = "/Users/me/.local/bin/hermes";
            let cmd = uninstall_command_from_paths(
                "hermes",
                bin,
                "/Users/me/.local/bin/hermes",
                src(bin),
            );
            assert_eq!(cmd, "python3 -m pip uninstall hermes-agent");
        }

        #[test]
        fn brew_uninstall_path_with_space_is_quoted() {
            // brew 分支用 `<bin_path 同目录>/brew`,目录含空格时同样要引号包裹,
            // 否则 `/opt/my brew/bin/brew uninstall gemini-cli` 会被 shell 拆词
            // 当成两个命令。镜像 anchored upgrade 的 brew_path_with_space_is_quoted。
            let bin = "/opt/my brew/bin/gemini";
            let cmd = uninstall_command_from_paths(
                "gemini",
                bin,
                "/opt/my brew/Cellar/gemini-cli/0.13.0/libexec/lib/node_modules/@google/gemini-cli/dist/index.js",
                src(bin),
            );
            assert_eq!(cmd, "'/opt/my brew/bin/brew' uninstall gemini-cli");
        }

        #[test]
        fn system_install_uninstall_falls_back_to_bare_npm_rm() {
            // ~/.opencode/bin / ~/go/bin / ~/.local/bin 等独立安装无 sibling npm
            // (与 anchored upgrade 在同类来源返回 None 的语义对偶)。
            // **关键**:不要拼 `<bin_dir>/npm rm -g <pkg>`——该处无 npm 文件,
            // 拼出来必失败。退化到**无前缀** `npm rm -g <pkg>`,让用户终端 PATH
            // 默认的 npm 处理;卸载语义下"不锚定但明确意图"比"返回 None 让 UI
            // 不显示命令"更友好(这条 hint 本来就是给用户复制粘贴的)。
            let bin = "/Users/me/.opencode/bin/opencode";
            let cmd = uninstall_command_from_paths(
                "opencode",
                bin,
                "/Users/me/.opencode/bin/opencode",
                src(bin),
            );
            assert_eq!(cmd, "npm rm -g opencode-ai");
        }

        #[test]
        fn sibling_bin_returns_none_when_bin_path_has_no_directory() {
            // bin_path 不含 `/` → parent_dir 返回空 → sibling_bin 不能拼出绝对路径
            // → None,让上游 anchored_command_from_paths 整体退化为静态命令兜底,
            // 而不是悄悄拼出 `npm i -g <pkg>` 这种依赖 PATH 的指令(违背"必须绝对路径"
            // 不变量)。实际从 enumerate_tool_installations 走的 bin_path 都是绝对路径,
            // 这条防线不期望被触发,但闭合了 helper 与函数文档的语义一致。
            assert_eq!(sibling_bin("codex", "npm"), None);
            assert_eq!(sibling_bin("", "brew"), None);
            // 含 `/` 即可拼出绝对路径——这是常规路径。
            assert_eq!(
                sibling_bin("/opt/homebrew/bin/gemini", "brew").as_deref(),
                Some("/opt/homebrew/bin/brew")
            );
        }

        #[test]
        fn default_install_prefers_path_default() {
            let installs = vec![
                inst("/opt/homebrew/bin/openclaw", false),
                inst("/Users/me/.nvm/versions/node/v22/bin/openclaw", true),
            ];
            assert_eq!(
                default_install(&installs).map(|i| i.path.as_str()),
                Some("/Users/me/.nvm/versions/node/v22/bin/openclaw")
            );
        }

        #[test]
        fn default_install_falls_back_to_sole_entry() {
            let installs = vec![inst("/opt/homebrew/bin/gemini", false)];
            assert_eq!(
                default_install(&installs).map(|i| i.path.as_str()),
                Some("/opt/homebrew/bin/gemini")
            );
        }

        #[test]
        fn default_install_none_when_ambiguous() {
            let installs = vec![
                inst("/opt/homebrew/bin/openclaw", false),
                inst("/Users/me/.nvm/versions/node/v22/bin/openclaw", false),
            ];
            assert!(default_install(&installs).is_none());
        }

        #[test]
        fn first_abs_path_line_skips_shell_noise() {
            // 交互式 .zshrc 先打印欢迎语（如 powerlevel10k / 自定义提示），
            // command -v 的真实路径在其后 → 跳过噪音取真路径。
            assert_eq!(
                first_abs_path_line("🚀 Welcome back!\n/Users/me/.local/bin/claude\n"),
                Some("/Users/me/.local/bin/claude")
            );
            // 无噪音时取第一行。
            assert_eq!(
                first_abs_path_line("/opt/homebrew/bin/gemini\n"),
                Some("/opt/homebrew/bin/gemini")
            );
            // 输出里没有任何绝对路径 → None。
            assert_eq!(first_abs_path_line("welcome\nbye\n"), None);
        }

        #[test]
        fn is_conflicting_thresholds() {
            let make = |version: Option<&str>, runnable: bool| ToolInstallation {
                path: "/x".to_string(),
                version: version.map(str::to_string),
                runnable,
                error: None,
                source: "nvm".to_string(),
                is_path_default: false,
                uninstall_command: String::new(),
                uninstall_command_needs_cmd_hint: false,
                real: std::path::PathBuf::from("/x"),
            };
            // 单处 → 不冲突。
            assert!(!is_conflicting(&[make(Some("1.0.0"), true)]));
            // 两处同版本、都能跑 → 不冲突（同版本装两遍不打扰）。
            assert!(!is_conflicting(&[
                make(Some("1.0.0"), true),
                make(Some("1.0.0"), true)
            ]));
            // 版本分歧 → 冲突。
            assert!(is_conflicting(&[
                make(Some("1.0.0"), true),
                make(Some("2.0.0"), true)
            ]));
            // 同版本但运行态混合（一个能跑、一个跑不起来）→ 冲突。
            assert!(is_conflicting(&[
                make(Some("1.0.0"), true),
                make(Some("1.0.0"), false)
            ]));
        }
    }

    /// install 端的"上游推荐 || npm 兜底"短路链:把工具→官方安装方式这一上游事实
    /// 固化为回归断言。任何方案改动若打破短路链结构或 URL,都会被这些用例拦下。
    #[cfg(not(target_os = "windows"))]
    mod install_strategy {
        use super::super::*;

        #[test]
        fn claude_install_prefers_native_with_npm_fallback() {
            // Anthropic 现在主推 native installer(claude.ai/install.sh),
            // 网络不通时短路到 npm 仍能装上;两段都得在,顺序也得对。
            let cmd = install_command_for("claude");
            assert!(
                cmd.contains("https://claude.ai/install.sh"),
                "should include official installer URL: {cmd}"
            );
            assert!(
                cmd.contains("@anthropic-ai/claude-code@latest"),
                "should keep npm package as fallback: {cmd}"
            );
            let parts: Vec<&str> = cmd.split("||").collect();
            assert_eq!(parts.len(), 2, "should be a two-step short-circuit chain");
            assert!(parts[0].contains("install.sh"), "native first: {cmd}");
            assert!(parts[1].contains("npm i -g"), "npm second: {cmd}");
        }

        #[test]
        fn opencode_install_prefers_native_with_npm_fallback() {
            // SST 自家 install.sh 与 claude 同形态:bash 脚本、网络下载、装到 ~/.opencode/bin。
            let cmd = install_command_for("opencode");
            assert!(
                cmd.contains("https://opencode.ai/install"),
                "should include official installer URL: {cmd}"
            );
            assert!(
                cmd.contains("opencode-ai@latest"),
                "should keep npm package as fallback: {cmd}"
            );
            assert!(cmd.contains("||"), "should chain fallback: {cmd}");
        }

        #[test]
        fn codex_install_keeps_static_npm() {
            // OpenAI 暂无独立 native installer,保持原裸 npm,不引入兜底链(无东西可兜底)。
            let cmd = install_command_for("codex");
            assert_eq!(cmd, "npm i -g @openai/codex@latest");
            assert!(!cmd.contains("||"));
        }

        #[test]
        fn gemini_install_keeps_static_npm() {
            // Google 文档同时支持 brew/npm,但本表保持与 update fallback 一致的 npm。
            // 用户若已装 brew gemini-cli,update 路径的锚定会识别 formula → brew upgrade,
            // 所以 install 端不强行替用户决策"用 brew 还是 npm"。
            let cmd = install_command_for("gemini");
            assert_eq!(cmd, "npm i -g @google/gemini-cli@latest");
        }

        #[test]
        fn openclaw_install_keeps_static_npm() {
            let cmd = install_command_for("openclaw");
            assert_eq!(cmd, "npm i -g openclaw@latest");
        }

        #[test]
        fn hermes_install_keeps_static_pip_chain() {
            // hermes 已经有 python3 || python 兜底,install_command_for 透传到 fallback。
            let cmd = install_command_for("hermes");
            assert!(cmd.contains("hermes-agent[web]"), "pip package: {cmd}");
            assert!(
                cmd.contains("python3") && cmd.contains("python "),
                "should keep python3→python fallback chain: {cmd}"
            );
        }
    }

    #[cfg(target_os = "windows")]
    mod wsl_helpers {
        use super::super::*;

        #[test]
        fn test_is_valid_shell() {
            assert!(is_valid_shell("bash"));
            assert!(is_valid_shell("zsh"));
            assert!(is_valid_shell("sh"));
            assert!(is_valid_shell("fish"));
            assert!(is_valid_shell("dash"));
            assert!(is_valid_shell("/usr/bin/bash"));
            assert!(is_valid_shell("/bin/zsh"));
            assert!(!is_valid_shell("powershell"));
            assert!(!is_valid_shell("cmd"));
            assert!(!is_valid_shell(""));
        }

        #[test]
        fn test_is_valid_shell_flag() {
            assert!(is_valid_shell_flag("-c"));
            assert!(is_valid_shell_flag("-lc"));
            assert!(is_valid_shell_flag("-lic"));
            assert!(!is_valid_shell_flag("-x"));
            assert!(!is_valid_shell_flag(""));
            assert!(!is_valid_shell_flag("--login"));
        }

        #[test]
        fn test_default_flag_for_shell() {
            assert_eq!(default_flag_for_shell("sh"), "-c");
            assert_eq!(default_flag_for_shell("dash"), "-c");
            assert_eq!(default_flag_for_shell("/bin/dash"), "-c");
            assert_eq!(default_flag_for_shell("fish"), "-lc");
            assert_eq!(default_flag_for_shell("bash"), "-lic");
            assert_eq!(default_flag_for_shell("zsh"), "-lic");
            assert_eq!(default_flag_for_shell("/usr/bin/zsh"), "-lic");
        }

        #[test]
        fn test_is_valid_wsl_distro_name() {
            assert!(is_valid_wsl_distro_name("Ubuntu"));
            assert!(is_valid_wsl_distro_name("Ubuntu-22.04"));
            assert!(is_valid_wsl_distro_name("my_distro"));
            assert!(!is_valid_wsl_distro_name(""));
            assert!(!is_valid_wsl_distro_name("distro with spaces"));
            assert!(!is_valid_wsl_distro_name(&"a".repeat(65)));
        }
    }

    #[test]
    fn opencode_extra_search_paths_includes_install_and_fallback_dirs() {
        let home = PathBuf::from("/home/tester");
        let install_dir = Some(std::ffi::OsString::from("/custom/opencode/bin"));
        let xdg_bin_dir = Some(std::ffi::OsString::from("/xdg/bin"));
        let gopath =
            std::env::join_paths([PathBuf::from("/go/path1"), PathBuf::from("/go/path2")]).ok();

        let paths = opencode_extra_search_paths(&home, install_dir, xdg_bin_dir, gopath);

        assert_eq!(paths[0], PathBuf::from("/custom/opencode/bin"));
        assert_eq!(paths[1], PathBuf::from("/xdg/bin"));
        assert!(paths.contains(&PathBuf::from("/home/tester/bin")));
        assert!(paths.contains(&PathBuf::from("/home/tester/.opencode/bin")));
        assert!(paths.contains(&PathBuf::from("/home/tester/.bun/bin")));
        assert!(paths.contains(&PathBuf::from("/home/tester/go/bin")));
        assert!(paths.contains(&PathBuf::from("/go/path1/bin")));
        assert!(paths.contains(&PathBuf::from("/go/path2/bin")));
    }

    #[test]
    fn opencode_extra_search_paths_deduplicates_repeated_entries() {
        let home = PathBuf::from("/home/tester");
        let same_dir = Some(std::ffi::OsString::from("/same/path"));

        let paths = opencode_extra_search_paths(&home, same_dir.clone(), same_dir, None);

        let count = paths
            .iter()
            .filter(|path| path.as_path() == Path::new("/same/path"))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn opencode_extra_search_paths_deduplicates_bun_default_dir() {
        let home = PathBuf::from("/home/tester");
        let paths = opencode_extra_search_paths(&home, None, None, None);

        let count = paths
            .iter()
            .filter(|path| path.as_path() == Path::new("/home/tester/.bun/bin"))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn cli_path_env_search_paths_include_path_entries_and_dedupe() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        std::fs::create_dir_all(&first).expect("first dir should be created");
        std::fs::create_dir_all(&second).expect("second dir should be created");

        let path_env = std::env::join_paths([first.clone(), second.clone(), first.clone()])
            .expect("test path env should be joinable");
        let mut paths = vec![first.clone()];

        extend_from_cli_path_env(&mut paths, Some(path_env));

        assert!(paths.contains(&second));
        assert_eq!(paths.iter().filter(|path| *path == &first).count(), 1);
    }

    #[test]
    fn child_search_paths_include_existing_children_with_suffix() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let base = temp.path().join("node");
        let bin = base.join("25.8.0").join("bin");
        std::fs::create_dir_all(&bin).expect("version bin should be created");

        let mut paths = Vec::new();
        extend_existing_child_search_paths(&mut paths, &base, Some("bin"));

        assert!(paths.contains(&bin));
    }

    #[test]
    fn env_child_dir_appends_child_and_dedupes() {
        let base = std::ffi::OsString::from("/custom/toolchain");
        let mut paths = Vec::new();

        push_env_child_dir(&mut paths, Some(base.clone()), "bin");
        push_env_child_dir(&mut paths, Some(base), "bin");

        assert_eq!(paths, vec![PathBuf::from("/custom/toolchain").join("bin")]);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn cli_path_env_skips_windows_apps_alias_dir() {
        assert!(is_windows_app_execution_alias_dir(Path::new(
            r"C:\Users\tester\AppData\Local\Microsoft\WindowsApps"
        )));
        assert!(!is_windows_app_execution_alias_dir(Path::new(
            r"C:\Users\tester\AppData\Roaming\npm"
        )));
    }

    #[test]
    fn mise_node_search_paths_include_shims_and_installed_node_bins() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let home = temp.path();
        let node_bin = home
            .join(".local/share/mise/installs/node/25.8.0")
            .join("bin");
        std::fs::create_dir_all(&node_bin).expect("node bin should be created");

        let mut paths = Vec::new();
        extend_mise_node_search_paths(&mut paths, home);

        assert!(paths.contains(&home.join(".local/share/mise/shims")));
        assert!(paths.contains(&node_bin));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn tool_executable_candidates_non_windows_uses_plain_binary_name() {
        let dir = PathBuf::from("/usr/local/bin");
        let candidates = tool_executable_candidates("opencode", &dir);

        assert_eq!(candidates, vec![PathBuf::from("/usr/local/bin/opencode")]);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn tool_executable_candidates_windows_includes_cmd_exe_and_plain_name() {
        let dir = PathBuf::from("C:\\tools");
        let candidates = tool_executable_candidates("opencode", &dir);

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("C:\\tools\\opencode.cmd"),
                PathBuf::from("C:\\tools\\opencode.exe"),
                PathBuf::from("C:\\tools\\opencode"),
            ]
        );
    }

    #[test]
    fn resolve_launch_cwd_accepts_existing_directory() {
        let resolved =
            resolve_launch_cwd(Some(std::env::temp_dir().to_string_lossy().into_owned()))
                .expect("temp dir should resolve")
                .expect("temp dir should be present");

        assert!(resolved.is_dir());
    }

    #[test]
    fn resolve_launch_cwd_rejects_missing_directory() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let missing = std::env::temp_dir().join(format!("cc-switch-missing-{unique}"));

        let error = resolve_launch_cwd(Some(missing.to_string_lossy().into_owned()))
            .expect_err("missing directory should fail");

        assert!(error.contains("目录不存在"));
    }

    #[test]
    fn build_shell_cd_command_quotes_spaces_and_single_quotes() {
        let command = build_shell_cd_command(Some(Path::new("/tmp/project O'Brien")));

        assert_eq!(command, "cd '/tmp/project O'\"'\"'Brien' || exit 1\n");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn iterm2_applescript_cold_start_avoids_current_window_before_one_exists() {
        let script = build_macos_iterm2_applescript(Path::new("/tmp/cc_switch_launcher.sh"));

        let cold_start_branch = script
            .split("else\n        activate")
            .nth(1)
            .expect("cold start branch should be present")
            .split("    end if\n    tell current session")
            .next()
            .expect("cold start branch should end before writing command");

        assert!(cold_start_branch.contains("repeat while (count of windows) = 0"));
        assert!(cold_start_branch.contains("create window with default profile"));
        assert!(!cold_start_branch.contains("tell current window"));
        assert!(!cold_start_branch.contains("create tab with default profile"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn iterm2_applescript_keeps_new_tab_behavior_for_existing_windows() {
        let script = build_macos_iterm2_applescript(Path::new("/tmp/cc_switch_launcher.sh"));

        let running_branch = script
            .split("if was_running then")
            .nth(1)
            .expect("already-running branch should be present")
            .split("else\n        activate")
            .next()
            .expect("already-running branch should end before cold start branch");

        assert!(running_branch.contains("if (count of windows) = 0 then"));
        assert!(running_branch.contains("create window with default profile"));
        assert!(running_branch.contains("create tab with default profile"));
    }

    #[test]
    fn build_windows_cwd_command_str_uses_cd_for_drive_paths() {
        let command = build_windows_cwd_command_str(r"C:\work\repo");

        assert_eq!(command, "cd /d \"C:\\work\\repo\" || exit /b 1\r\n");
    }

    #[test]
    fn build_windows_cwd_command_str_uses_pushd_for_unc_paths() {
        let command = build_windows_cwd_command_str(r"\\wsl$\Ubuntu\home\coder\repo");

        assert_eq!(
            command,
            "pushd \"\\\\wsl$\\Ubuntu\\home\\coder\\repo\" || exit /b 1\r\n"
        );
    }

    #[test]
    fn build_windows_cwd_command_str_escapes_batch_metacharacters() {
        let command = build_windows_cwd_command_str(r"\\server\share\100%&(test)");

        assert_eq!(
            command,
            "pushd \"\\\\server\\share\\100%%^&^(test^)\" || exit /b 1\r\n"
        );
    }
}
