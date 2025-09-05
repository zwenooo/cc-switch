// unused imports removed
use std::path::PathBuf;

use crate::config::{
    delete_file, sanitize_provider_name,
};

/// 获取 Codex 配置目录路径
pub fn get_codex_config_dir() -> PathBuf {
    dirs::home_dir().expect("无法获取用户主目录").join(".codex")
}

/// 获取 Codex auth.json 路径
pub fn get_codex_auth_path() -> PathBuf {
    get_codex_config_dir().join("auth.json")
}

/// 获取 Codex config.toml 路径
pub fn get_codex_config_path() -> PathBuf {
    get_codex_config_dir().join("config.toml")
}

/// 获取 Codex 供应商配置文件路径
pub fn get_codex_provider_paths(
    provider_id: &str,
    provider_name: Option<&str>,
) -> (PathBuf, PathBuf) {
    let base_name = provider_name
        .map(|name| sanitize_provider_name(name))
        .unwrap_or_else(|| sanitize_provider_name(provider_id));

    let auth_path = get_codex_config_dir().join(format!("auth-{}.json", base_name));
    let config_path = get_codex_config_dir().join(format!("config-{}.toml", base_name));

    (auth_path, config_path)
}

/// 删除 Codex 供应商配置文件
pub fn delete_codex_provider_config(provider_id: &str, provider_name: &str) -> Result<(), String> {
    let (auth_path, config_path) = get_codex_provider_paths(provider_id, Some(provider_name));

    delete_file(&auth_path).ok();
    delete_file(&config_path).ok();

    Ok(())
}

//（移除未使用的备份/保存/恢复/导入函数，避免 dead_code 告警）
