use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 获取 Claude Code 配置目录路径
pub fn get_claude_config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户主目录")
        .join(".claude")
}

/// 获取 Claude Code 主配置文件路径
pub fn get_claude_settings_path() -> PathBuf {
    let dir = get_claude_config_dir();
    let settings = dir.join("settings.json");
    if settings.exists() {
        return settings;
    }
    // 兼容旧版命名：claude.json
    dir.join("claude.json")
}

/// 获取应用配置目录路径 (~/.cc-switch)
pub fn get_app_config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户主目录")
        .join(".cc-switch")
}

/// 获取应用配置文件路径
pub fn get_app_config_path() -> PathBuf {
    get_app_config_dir().join("config.json")
}

/// 清理供应商名称，确保文件名安全
pub fn sanitize_provider_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// 获取供应商配置文件路径
pub fn get_provider_config_path(provider_id: &str, provider_name: Option<&str>) -> PathBuf {
    let base_name = provider_name
        .map(|name| sanitize_provider_name(name))
        .unwrap_or_else(|| sanitize_provider_name(provider_id));
    
    get_claude_config_dir().join(format!("settings-{}.json", base_name))
}

/// 读取 JSON 配置文件
pub fn read_json_file<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, String> {
    if !path.exists() {
        return Err(format!("文件不存在: {}", path.display()));
    }
    
    let content = fs::read_to_string(path)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("解析 JSON 失败: {}", e))
}

/// 写入 JSON 配置文件
pub fn write_json_file<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }
    
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| format!("序列化 JSON 失败: {}", e))?;
    
    fs::write(path, json)
        .map_err(|e| format!("写入文件失败: {}", e))
}

/// 复制文件
pub fn copy_file(from: &Path, to: &Path) -> Result<(), String> {
    fs::copy(from, to)
        .map_err(|e| format!("复制文件失败: {}", e))?;
    Ok(())
}

/// 删除文件
pub fn delete_file(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_file(path)
            .map_err(|e| format!("删除文件失败: {}", e))?;
    }
    Ok(())
}

/// 检查 Claude Code 配置状态
#[derive(Serialize, Deserialize)]
pub struct ConfigStatus {
    pub exists: bool,
    pub path: String,
}

/// 获取 Claude Code 配置状态
pub fn get_claude_config_status() -> ConfigStatus {
    let path = get_claude_settings_path();
    ConfigStatus {
        exists: path.exists(),
        path: path.to_string_lossy().to_string(),
    }
}

/// 备份配置文件
pub fn backup_config(from: &Path, to: &Path) -> Result<(), String> {
    if from.exists() {
        copy_file(from, to)?;
        log::info!("已备份配置文件: {} -> {}", from.display(), to.display());
    }
    Ok(())
}

/// 导入当前 Claude Code 配置为默认供应商
pub fn import_current_config_as_default() -> Result<Value, String> {
    let settings_path = get_claude_settings_path();
    
    if !settings_path.exists() {
        return Err("Claude Code 配置文件不存在".to_string());
    }
    
    // 读取当前配置
    let settings_config: Value = read_json_file(&settings_path)?;
    
    // 保存为 default 供应商
    let default_provider_path = get_provider_config_path("default", Some("default"));
    write_json_file(&default_provider_path, &settings_config)?;
    
    log::info!("已导入当前配置为默认供应商");
    Ok(settings_config)
}
