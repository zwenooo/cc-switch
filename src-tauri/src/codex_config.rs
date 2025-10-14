// unused imports removed
use std::path::PathBuf;

use crate::config::{
    atomic_write, delete_file, sanitize_provider_name, write_json_file, write_text_file,
};
use serde_json::Value;
use std::fs;
use std::path::Path;

/// 获取 Codex 配置目录路径
pub fn get_codex_config_dir() -> PathBuf {
    if let Some(custom) = crate::settings::get_codex_override_dir() {
        return custom;
    }

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
        .map(sanitize_provider_name)
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

/// 原子写 Codex 的 `auth.json` 与 `config.toml`，在第二步失败时回滚第一步
pub fn write_codex_live_atomic(auth: &Value, config_text_opt: Option<&str>) -> Result<(), String> {
    let auth_path = get_codex_auth_path();
    let config_path = get_codex_config_path();

    if let Some(parent) = auth_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建 Codex 目录失败: {}: {}", parent.display(), e))?;
    }

    // 读取旧内容用于回滚
    let old_auth = if auth_path.exists() {
        Some(
            fs::read(&auth_path)
                .map_err(|e| format!("读取旧 auth.json 失败: {}: {}", auth_path.display(), e))?,
        )
    } else {
        None
    };
    let _old_config =
        if config_path.exists() {
            Some(fs::read(&config_path).map_err(|e| {
                format!("读取旧 config.toml 失败: {}: {}", config_path.display(), e)
            })?)
        } else {
            None
        };

    // 准备写入内容
    let cfg_text = match config_text_opt {
        Some(s) => s.to_string(),
        None => String::new(),
    };
    if !cfg_text.trim().is_empty() {
        toml::from_str::<toml::Table>(&cfg_text).map_err(|e| {
            format!(
                "config.toml 语法错误: {} (路径: {})",
                e,
                config_path.display()
            )
        })?;
    }

    // 第一步：写 auth.json
    write_json_file(&auth_path, auth)?;

    // 第二步：写 config.toml（失败则回滚 auth.json）
    if let Err(e) = write_text_file(&config_path, &cfg_text) {
        // 回滚 auth.json
        if let Some(bytes) = old_auth {
            let _ = atomic_write(&auth_path, &bytes);
        } else {
            let _ = delete_file(&auth_path);
        }
        return Err(e);
    }

    Ok(())
}

/// 读取 `~/.codex/config.toml`，若不存在返回空字符串
pub fn read_codex_config_text() -> Result<String, String> {
    let path = get_codex_config_path();
    if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| format!("读取 config.toml 失败: {}", e))
    } else {
        Ok(String::new())
    }
}

/// 从给定路径读取 config.toml 文本（路径存在时）；路径不存在则返回空字符串
pub fn read_config_text_from_path(path: &Path) -> Result<String, String> {
    if path.exists() {
        std::fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {}", path.display(), e))
    } else {
        Ok(String::new())
    }
}

/// 对非空的 TOML 文本进行语法校验
pub fn validate_config_toml(text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    toml::from_str::<toml::Table>(text)
        .map(|_| ())
        .map_err(|e| format!("config.toml 语法错误: {}", e))
}

/// 读取并校验 `~/.codex/config.toml`，返回文本（可能为空）
pub fn read_and_validate_codex_config_text() -> Result<String, String> {
    let s = read_codex_config_text()?;
    validate_config_toml(&s)?;
    Ok(s)
}

/// 从指定路径读取并校验 config.toml，返回文本（可能为空）
pub fn read_and_validate_config_from_path(path: &Path) -> Result<String, String> {
    let s = read_config_text_from_path(path)?;
    validate_config_toml(&s)?;
    Ok(s)
}
