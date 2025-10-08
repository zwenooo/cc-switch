use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{atomic_write, get_claude_config_dir};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub settings_local_path: String,
    pub settings_local_exists: bool,
    pub enable_all_project_mcp_servers: bool,
    pub mcp_json_path: String,
    pub mcp_json_exists: bool,
    pub server_count: usize,
}

fn claude_dir() -> PathBuf {
    get_claude_config_dir()
}

fn settings_local_path() -> PathBuf {
    claude_dir().join("settings.local.json")
}

fn mcp_json_path() -> PathBuf {
    claude_dir().join("mcp.json")
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content =
        fs::read_to_string(path).map_err(|e| format!("读取文件失败: {}: {}", path.display(), e))?;
    let value: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {}: {}", path.display(), e))?;
    Ok(value)
}

fn write_json_value(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}: {}", parent.display(), e))?;
    }
    let json = serde_json::to_string_pretty(value).map_err(|e| format!("序列化 JSON 失败: {}", e))?;
    atomic_write(path, json.as_bytes())
}

pub fn get_mcp_status() -> Result<McpStatus, String> {
    let settings_local = settings_local_path();
    let mcp_path = mcp_json_path();

    let mut enable = false;
    if settings_local.exists() {
        let v = read_json_value(&settings_local)?;
        enable = v
            .get("enableAllProjectMcpServers")
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
    }

    let (exists, count) = if mcp_path.exists() {
        let v = read_json_value(&mcp_path)?;
        let servers = v.get("mcpServers").and_then(|x| x.as_object());
        (true, servers.map(|m| m.len()).unwrap_or(0))
    } else {
        (false, 0)
    };

    Ok(McpStatus {
        settings_local_path: settings_local.to_string_lossy().to_string(),
        settings_local_exists: settings_local.exists(),
        enable_all_project_mcp_servers: enable,
        mcp_json_path: mcp_path.to_string_lossy().to_string(),
        mcp_json_exists: exists,
        server_count: count,
    })
}

pub fn read_mcp_json() -> Result<Option<String>, String> {
    let path = mcp_json_path();
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("读取 MCP 配置失败: {}", e))?;
    Ok(Some(content))
}

pub fn set_enable_all_projects(enable: bool) -> Result<bool, String> {
    let path = settings_local_path();
    let mut v = if path.exists() { read_json_value(&path)? } else { serde_json::json!({}) };

    let current = v
        .get("enableAllProjectMcpServers")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    if current == enable && path.exists() {
        return Ok(false);
    }

    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "enableAllProjectMcpServers".to_string(),
            Value::Bool(enable),
        );
    }
    write_json_value(&path, &v)?;
    Ok(true)
}

pub fn upsert_mcp_server(id: &str, spec: Value) -> Result<bool, String> {
    if id.trim().is_empty() {
        return Err("MCP 服务器 ID 不能为空".into());
    }
    // 基础字段校验（尽量宽松）
    if !spec.is_object() {
        return Err("MCP 服务器定义必须为 JSON 对象".into());
    }
    let t = spec
        .get("type")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    if t != "stdio" && t != "sse" {
        return Err("MCP 服务器 type 必须是 'stdio' 或 'sse'".into());
    }
    let cmd = spec.get("command").and_then(|x| x.as_str()).unwrap_or("");
    if cmd.is_empty() {
        return Err("MCP 服务器缺少 command".into());
    }

    let path = mcp_json_path();
    let mut root = if path.exists() { read_json_value(&path)? } else { serde_json::json!({}) };

    // 确保 mcpServers 对象存在
    {
        let obj = root.as_object_mut().ok_or_else(|| "mcp.json 根必须是对象".to_string())?;
        if !obj.contains_key("mcpServers") {
            obj.insert("mcpServers".into(), serde_json::json!({}));
        }
    }

    let before = root.clone();
    if let Some(servers) = root
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
    {
        servers.insert(id.to_string(), spec);
    }

    if before == root && path.exists() {
        return Ok(false);
    }

    write_json_value(&path, &root)?;
    Ok(true)
}

pub fn delete_mcp_server(id: &str) -> Result<bool, String> {
    if id.trim().is_empty() {
        return Err("MCP 服务器 ID 不能为空".into());
    }
    let path = mcp_json_path();
    if !path.exists() {
        return Ok(false);
    }
    let mut root = read_json_value(&path)?;
    let Some(servers) = root.get_mut("mcpServers").and_then(|v| v.as_object_mut()) else {
        return Ok(false);
    };
    let existed = servers.remove(id).is_some();
    if !existed {
        return Ok(false);
    }
    write_json_value(&path, &root)?;
    Ok(true)
}

pub fn validate_command_in_path(cmd: &str) -> Result<bool, String> {
    if cmd.trim().is_empty() {
        return Ok(false);
    }
    // 如果包含路径分隔符，直接判断是否存在可执行文件
    if cmd.contains('/') || cmd.contains('\\') {
        return Ok(Path::new(cmd).exists());
    }

    let path_var = env::var_os("PATH").unwrap_or_default();
    let paths = env::split_paths(&path_var);

    #[cfg(windows)]
    let exts: Vec<String> = env::var("PATHEXT")
        .unwrap_or(".COM;.EXE;.BAT;.CMD".into())
        .split(';')
        .map(|s| s.trim().to_uppercase())
        .collect();

    for p in paths {
        let candidate = p.join(cmd);
        if candidate.is_file() {
            return Ok(true);
        }
        #[cfg(windows)]
        {
            for ext in &exts {
                let cand = p.join(format!("{}{}", cmd, ext));
                if cand.is_file() {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

