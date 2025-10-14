use std::fs;
use std::path::PathBuf;

const CLAUDE_DIR: &str = ".claude";
const CLAUDE_CONFIG_FILE: &str = "config.json";

fn claude_dir() -> Result<PathBuf, String> {
    // 优先使用设置中的覆盖目录
    if let Some(dir) = crate::settings::get_claude_override_dir() {
        return Ok(dir);
    }
    let home = dirs::home_dir().ok_or_else(|| "无法获取用户主目录".to_string())?;
    Ok(home.join(CLAUDE_DIR))
}

pub fn claude_config_path() -> Result<PathBuf, String> {
    Ok(claude_dir()?.join(CLAUDE_CONFIG_FILE))
}

pub fn ensure_claude_dir_exists() -> Result<PathBuf, String> {
    let dir = claude_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("创建 Claude 配置目录失败: {}", e))?;
    }
    Ok(dir)
}

pub fn read_claude_config() -> Result<Option<String>, String> {
    let path = claude_config_path()?;
    if path.exists() {
        let content =
            fs::read_to_string(&path).map_err(|e| format!("读取 Claude 配置失败: {}", e))?;
        Ok(Some(content))
    } else {
        Ok(None)
    }
}

fn is_managed_config(content: &str) -> bool {
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(value) => value
            .get("primaryApiKey")
            .and_then(|v| v.as_str())
            .map(|val| val == "any")
            .unwrap_or(false),
        Err(_) => false,
    }
}

pub fn write_claude_config() -> Result<bool, String> {
    // 增量写入：仅设置 primaryApiKey = "any"，保留其它字段
    let path = claude_config_path()?;
    ensure_claude_dir_exists()?;

    // 尝试读取并解析为对象
    let mut obj = match read_claude_config()? {
        Some(existing) => match serde_json::from_str::<serde_json::Value>(&existing) {
            Ok(serde_json::Value::Object(map)) => serde_json::Value::Object(map),
            _ => serde_json::json!({}),
        },
        None => serde_json::json!({}),
    };

    let mut changed = false;
    if let Some(map) = obj.as_object_mut() {
        let cur = map
            .get("primaryApiKey")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if cur != "any" {
            map.insert(
                "primaryApiKey".to_string(),
                serde_json::Value::String("any".to_string()),
            );
            changed = true;
        }
    }

    if changed || !path.exists() {
        let serialized = serde_json::to_string_pretty(&obj)
            .map_err(|e| format!("序列化 Claude 配置失败: {}", e))?;
        fs::write(&path, format!("{}\n", serialized))
            .map_err(|e| format!("写入 Claude 配置失败: {}", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn clear_claude_config() -> Result<bool, String> {
    let path = claude_config_path()?;
    if !path.exists() {
        return Ok(false);
    }

    let content = match read_claude_config()? {
        Some(content) => content,
        None => return Ok(false),
    };

    let mut value = match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(value) => value,
        Err(_) => return Ok(false),
    };

    let obj = match value.as_object_mut() {
        Some(obj) => obj,
        None => return Ok(false),
    };

    if obj.remove("primaryApiKey").is_none() {
        return Ok(false);
    }

    let serialized = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("序列化 Claude 配置失败: {}", e))?;
    fs::write(&path, format!("{}\n", serialized))
        .map_err(|e| format!("写入 Claude 配置失败: {}", e))?;
    Ok(true)
}

pub fn claude_config_status() -> Result<(bool, PathBuf), String> {
    let path = claude_config_path()?;
    Ok((path.exists(), path))
}

pub fn is_claude_config_applied() -> Result<bool, String> {
    match read_claude_config()? {
        Some(content) => Ok(is_managed_config(&content)),
        None => Ok(false),
    }
}
