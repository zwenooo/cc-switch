use serde_json::{json, Value};
use std::collections::HashMap;

use crate::app_config::{AppType, McpConfig, MultiAppConfig};

/// 基础校验：允许 stdio/http；或省略 type（视为 stdio）。对应必填字段存在
fn validate_mcp_spec(spec: &Value) -> Result<(), String> {
    if !spec.is_object() {
        return Err("MCP 服务器定义必须为 JSON 对象".into());
    }
    let t_opt = spec.get("type").and_then(|x| x.as_str());
    // 支持两种：stdio/http；若缺省 type 则按 stdio 处理（与社区常见 .mcp.json 一致）
    let is_stdio = t_opt.map(|t| t == "stdio").unwrap_or(true);
    let is_http = t_opt.map(|t| t == "http").unwrap_or(false);
    
    if !(is_stdio || is_http) {
        return Err("MCP 服务器 type 必须是 'stdio' 或 'http'（或省略表示 stdio）".into());
    }

    if is_stdio {
        let cmd = spec.get("command").and_then(|x| x.as_str()).unwrap_or("");
        if cmd.trim().is_empty() {
            return Err("stdio 类型的 MCP 服务器缺少 command 字段".into());
        }
    }
    if is_http {
        let url = spec.get("url").and_then(|x| x.as_str()).unwrap_or("");
        if url.trim().is_empty() {
            return Err("http 类型的 MCP 服务器缺少 url 字段".into());
        }
    }
    Ok(())
}

/// 返回已启用的 MCP 服务器（过滤 enabled==true）
fn collect_enabled_servers(cfg: &McpConfig) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    for (id, spec) in cfg.servers.iter() {
        let enabled = spec
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if enabled {
            out.insert(id.clone(), spec.clone());
        }
    }
    out
}

pub fn get_servers_snapshot_for(config: &MultiAppConfig, app: &AppType) -> HashMap<String, Value> {
    config.mcp_for(app).servers.clone()
}

pub fn upsert_in_config_for(
    config: &mut MultiAppConfig,
    app: &AppType,
    id: &str,
    spec: Value,
) -> Result<bool, String> {
    if id.trim().is_empty() {
        return Err("MCP 服务器 ID 不能为空".into());
    }
    validate_mcp_spec(&spec)?;

    // 默认 enabled 不强制设值；若字段不存在则保持不变（或 UI 决定）
    if spec.get("enabled").is_none() {
        // 缺省不设，以便后续 set_enabled 独立控制
    }

    let servers = &mut config.mcp_for_mut(app).servers;
    let before = servers.get(id).cloned();
    servers.insert(id.to_string(), spec);

    Ok(before.is_none())
}

pub fn delete_in_config_for(config: &mut MultiAppConfig, app: &AppType, id: &str) -> Result<bool, String> {
    if id.trim().is_empty() {
        return Err("MCP 服务器 ID 不能为空".into());
    }
    let existed = config.mcp_for_mut(app).servers.remove(id).is_some();
    Ok(existed)
}

/// 设置启用状态并同步到 ~/.claude.json
pub fn set_enabled_and_sync_for(
    config: &mut MultiAppConfig,
    app: &AppType,
    id: &str,
    enabled: bool,
) -> Result<bool, String> {
    if id.trim().is_empty() {
        return Err("MCP 服务器 ID 不能为空".into());
    }
    if let Some(spec) = config.mcp_for_mut(app).servers.get_mut(id) {
        // 写入 enabled 字段
        let mut obj = spec.as_object().cloned().ok_or_else(|| "MCP 服务器定义必须为 JSON 对象".to_string())?;
        obj.insert("enabled".into(), json!(enabled));
        *spec = Value::Object(obj);
    } else {
        // 若不存在则直接返回 false
        return Ok(false);
    }

    // 同步启用项
    match app {
        AppType::Claude => {
            // 将启用项投影到 ~/.claude.json
            sync_enabled_to_claude(config)?;
        }
        AppType::Codex => {
            // Codex 的 MCP 写入尚未实现（TOML 结构未定），此处先跳过
        }
    }
    Ok(true)
}

/// 将 config.json 中 enabled==true 的项投影写入 ~/.claude.json
pub fn sync_enabled_to_claude(config: &MultiAppConfig) -> Result<(), String> {
    let enabled = collect_enabled_servers(&config.mcp.claude);
    crate::claude_mcp::set_mcp_servers_map(&enabled)
}

/// 从 ~/.claude.json 导入 mcpServers 到 config.json（设为 enabled=true）。
/// 已存在的项仅强制 enabled=true，不覆盖其他字段。
pub fn import_from_claude(config: &mut MultiAppConfig) -> Result<usize, String> {
    let text_opt = crate::claude_mcp::read_mcp_json()?;
    let Some(text) = text_opt else { return Ok(0) };
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("解析 ~/.claude.json 失败: {}", e))?;
    let Some(map) = v.get("mcpServers").and_then(|x| x.as_object()) else { return Ok(0) };

    let mut changed = 0usize;
    for (id, spec) in map.iter() {
        // 校验目标 spec
        validate_mcp_spec(spec)?;

        // 规范化为对象
        let mut obj = spec.as_object().cloned().ok_or_else(|| "MCP 服务器定义必须为 JSON 对象".to_string())?;
        obj.insert("enabled".into(), json!(true));

        let entry = config.mcp_for_mut(&AppType::Claude).servers.entry(id.clone());
        use std::collections::hash_map::Entry;
        match entry {
            Entry::Vacant(vac) => {
                vac.insert(Value::Object(obj));
                changed += 1;
            }
            Entry::Occupied(mut occ) => {
                // 只确保 enabled=true；不覆盖其他字段
                if let Some(mut existing) = occ.get().as_object().cloned() {
                    let prev = existing.get("enabled").and_then(|b| b.as_bool()).unwrap_or(false);
                    if !prev {
                        existing.insert("enabled".into(), json!(true));
                        occ.insert(Value::Object(existing));
                        changed += 1;
                    }
                }
            }
        }
    }
    Ok(changed)
}
