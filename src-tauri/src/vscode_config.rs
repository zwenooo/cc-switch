use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

use crate::config::write_json_file;

/// VS Code 默认用户配置子目录
const MAC_CODE_USER_DIR: &str = "Library/Application Support/Code/User";

/// 解析 VS Code 用户 settings.json 路径
pub fn get_vscode_settings_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        return dirs::home_dir()
            .expect("无法获取用户主目录")
            .join(MAC_CODE_USER_DIR)
            .join("settings.json");
    }

    #[cfg(target_os = "linux")]
    {
        return dirs::home_dir()
            .expect("无法获取用户主目录")
            .join(".config/Code/User")
            .join("settings.json");
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(data_dir) = dirs::data_dir() {
            return data_dir.join("Code/User").join("settings.json");
        }
        return dirs::home_dir()
            .expect("无法获取用户主目录")
            .join("AppData/Roaming")
            .join("Code/User")
            .join("settings.json");
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        dirs::home_dir()
            .expect("无法获取用户主目录")
            .join(".config/Code/User")
            .join("settings.json")
    }
}

fn load_settings(path: &Path) -> Result<Map<String, Value>, String> {
    if !path.exists() {
        return Ok(Map::new());
    }

    let content =
        std::fs::read_to_string(path).map_err(|e| format!("读取 VS Code 设置失败: {}", e))?;

    if content.trim().is_empty() {
        return Ok(Map::new());
    }

    match serde_json::from_str::<Value>(&content) {
        Ok(Value::Object(obj)) => Ok(obj),
        Ok(_) => Err("VS Code settings.json 必须为 JSON 对象".to_string()),
        Err(err) => Err(format!("解析 VS Code settings.json 失败: {}", err)),
    }
}

fn persist_settings(path: &Path, map: Map<String, Value>) -> Result<(), String> {
    let value = Value::Object(map);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 VS Code 配置目录失败: {}", e))?;
    }
    write_json_file(path, &value)
}

/// 写入或移除 chatgpt 相关 VS Code 配置
///
/// - `base_url` 为 Some 时更新/覆盖 `"chatgpt.apiBase"` 与 `"chatgpt.config"`
/// - `base_url` 为 None 时删除上述字段
pub fn write_vscode_settings(base_url: Option<&str>) -> Result<(), String> {
    let path = get_vscode_settings_path();
    let mut map = load_settings(&path)?;

    match base_url {
        Some(url) => {
            if url.trim().is_empty() {
                return Err("base_url 不能为空".into());
            }

            map.insert(
                "chatgpt.apiBase".to_string(),
                Value::String(url.to_string()),
            );

            let entry = map
                .entry("chatgpt.config".to_string())
                .or_insert_with(|| Value::Object(Map::new()));

            let obj = match entry {
                Value::Object(o) => o,
                _ => return Err("VS Code settings 中 chatgpt.config 必须是 JSON 对象".into()),
            };

            obj.insert(
                "preferred_auth_method".to_string(),
                Value::String("apikey".to_string()),
            );
        }
        None => {
            map.remove("chatgpt.apiBase");
            map.remove("chatgpt.config");
        }
    }

    persist_settings(&path, map)
}
