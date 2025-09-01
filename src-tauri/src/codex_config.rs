use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use crate::config::{
    copy_file, delete_file, read_json_file, sanitize_provider_name, write_json_file,
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

/// 备份 Codex 当前配置
pub fn backup_codex_config(provider_id: &str, provider_name: &str) -> Result<(), String> {
    let auth_path = get_codex_auth_path();
    let config_path = get_codex_config_path();
    let (backup_auth_path, backup_config_path) =
        get_codex_provider_paths(provider_id, Some(provider_name));

    // 备份 auth.json
    if auth_path.exists() {
        copy_file(&auth_path, &backup_auth_path)?;
        log::info!("已备份 Codex auth.json: {}", backup_auth_path.display());
    }

    // 备份 config.toml
    if config_path.exists() {
        copy_file(&config_path, &backup_config_path)?;
        log::info!("已备份 Codex config.toml: {}", backup_config_path.display());
    }

    Ok(())
}

/// 保存 Codex 供应商配置副本
pub fn save_codex_provider_config(
    provider_id: &str,
    provider_name: &str,
    settings_config: &Value,
) -> Result<(), String> {
    let (auth_path, config_path) = get_codex_provider_paths(provider_id, Some(provider_name));

    // 保存 auth.json
    if let Some(auth) = settings_config.get("auth") {
        write_json_file(&auth_path, auth)?;
    }

    // 保存 config.toml
    if let Some(config) = settings_config.get("config") {
        if let Some(config_str) = config.as_str() {
            // 若非空则进行 TOML 语法校验
            if !config_str.trim().is_empty() {
                toml::from_str::<toml::Table>(config_str)
                    .map_err(|e| format!("config.toml 格式错误: {}", e))?;
            }
            fs::write(&config_path, config_str)
                .map_err(|e| format!("写入供应商 config.toml 失败: {}", e))?;
        }
    }

    Ok(())
}

/// 删除 Codex 供应商配置文件
pub fn delete_codex_provider_config(provider_id: &str, provider_name: &str) -> Result<(), String> {
    let (auth_path, config_path) = get_codex_provider_paths(provider_id, Some(provider_name));

    delete_file(&auth_path).ok();
    delete_file(&config_path).ok();

    Ok(())
}

/// 从 Codex 供应商配置副本恢复到主配置
pub fn restore_codex_provider_config(provider_id: &str, provider_name: &str) -> Result<(), String> {
    let (provider_auth_path, provider_config_path) =
        get_codex_provider_paths(provider_id, Some(provider_name));
    let auth_path = get_codex_auth_path();
    let config_path = get_codex_config_path();

    // 确保目录存在
    if let Some(parent) = auth_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建 Codex 目录失败: {}", e))?;
    }

    // 复制 auth.json（必需）
    if provider_auth_path.exists() {
        copy_file(&provider_auth_path, &auth_path)?;
        log::info!("已恢复 Codex auth.json");
    } else {
        return Err(format!(
            "供应商 auth.json 不存在: {}",
            provider_auth_path.display()
        ));
    }

    // 复制 config.toml（可选，允许为空；不存在则创建空文件以保持一致性）
    if provider_config_path.exists() {
        copy_file(&provider_config_path, &config_path)?;
        log::info!("已恢复 Codex config.toml");
    } else {
        // 写入空文件
        fs::write(&config_path, "").map_err(|e| format!("创建空的 config.toml 失败: {}", e))?;
        log::info!("供应商 config.toml 缺失，已创建空文件");
    }

    Ok(())
}

/// 导入当前 Codex 配置为默认供应商
pub fn import_current_codex_config() -> Result<Value, String> {
    let auth_path = get_codex_auth_path();
    let config_path = get_codex_config_path();

    // 行为放宽：仅要求 auth.json 存在；config.toml 可缺失
    if !auth_path.exists() {
        return Err("Codex 配置文件不存在".to_string());
    }

    // 读取 auth.json
    let auth = read_json_file::<Value>(&auth_path)?;

    // 读取 config.toml（允许不存在或读取失败时为空）
    let config_str = if config_path.exists() {
        let s = fs::read_to_string(&config_path)
            .map_err(|e| format!("读取 config.toml 失败: {}", e))?;
        if !s.trim().is_empty() {
            toml::from_str::<toml::Table>(&s)
                .map_err(|e| format!("config.toml 语法错误: {}", e))?;
        }
        s
    } else {
        String::new()
    };

    // 组合成完整配置
    let settings_config = serde_json::json!({
        "auth": auth,
        "config": config_str
    });

    // 保存为默认供应商副本
    save_codex_provider_config("default", "default", &settings_config)?;

    Ok(settings_config)
}
