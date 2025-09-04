#![allow(non_snake_case)]

use std::collections::HashMap;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::app_config::AppType;
use crate::codex_config;
use crate::config::{ConfigStatus, get_claude_settings_path};
use crate::provider::Provider;
use crate::store::AppState;

/// 获取所有供应商
#[tauri::command]
pub async fn get_providers(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<HashMap<String, Provider>, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let config = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = config
        .get_manager(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    Ok(manager.get_all_providers().clone())
}

/// 获取当前供应商ID
#[tauri::command]
pub async fn get_current_provider(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<String, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let config = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = config
        .get_manager(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    Ok(manager.current.clone())
}

/// 添加供应商
#[tauri::command]
pub async fn add_provider(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    provider: Provider,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let mut config = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = config
        .get_manager_mut(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    // 根据应用类型保存配置文件
    match app_type {
        AppType::Codex => {
            // Codex: 保存两个文件
            codex_config::save_codex_provider_config(
                &provider.id,
                &provider.name,
                &provider.settings_config,
            )?;
        }
        AppType::Claude => {
            // Claude: 使用原有逻辑
            use crate::config::{get_provider_config_path, write_json_file};
            let config_path = get_provider_config_path(&provider.id, Some(&provider.name));
            write_json_file(&config_path, &provider.settings_config)?;
        }
    }

    manager.providers.insert(provider.id.clone(), provider);

    // 保存配置
    drop(config); // 释放锁
    state.save()?;

    Ok(true)
}

/// 更新供应商
#[tauri::command]
pub async fn update_provider(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    provider: Provider,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let mut config = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = config
        .get_manager_mut(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    // 检查供应商是否存在
    if !manager.providers.contains_key(&provider.id) {
        return Err(format!("供应商不存在: {}", provider.id));
    }

    // 如果名称改变了，需要处理配置文件
    if let Some(old_provider) = manager.providers.get(&provider.id) {
        if old_provider.name != provider.name {
            // 删除旧配置文件
            match app_type {
                AppType::Codex => {
                    codex_config::delete_codex_provider_config(&provider.id, &old_provider.name)
                        .ok();
                }
                AppType::Claude => {
                    use crate::config::{delete_file, get_provider_config_path};
                    let old_config_path =
                        get_provider_config_path(&provider.id, Some(&old_provider.name));
                    delete_file(&old_config_path).ok();
                }
            }
        }
    }

    // 保存新配置文件
    match app_type {
        AppType::Codex => {
            codex_config::save_codex_provider_config(
                &provider.id,
                &provider.name,
                &provider.settings_config,
            )?;
        }
        AppType::Claude => {
            use crate::config::{get_provider_config_path, write_json_file};
            let config_path = get_provider_config_path(&provider.id, Some(&provider.name));
            write_json_file(&config_path, &provider.settings_config)?;
        }
    }

    manager.providers.insert(provider.id.clone(), provider);

    // 保存配置
    drop(config); // 释放锁
    state.save()?;

    Ok(true)
}

/// 删除供应商
#[tauri::command]
pub async fn delete_provider(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    id: String,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let mut config = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = config
        .get_manager_mut(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    // 检查是否为当前供应商
    if manager.current == id {
        return Err("不能删除当前正在使用的供应商".to_string());
    }

    // 获取供应商信息
    let provider = manager
        .providers
        .get(&id)
        .ok_or_else(|| format!("供应商不存在: {}", id))?
        .clone();

    // 删除配置文件
    match app_type {
        AppType::Codex => {
            codex_config::delete_codex_provider_config(&id, &provider.name)?;
        }
        AppType::Claude => {
            use crate::config::{delete_file, get_provider_config_path};
            let config_path = get_provider_config_path(&id, Some(&provider.name));
            delete_file(&config_path)?;
        }
    }

    // 从管理器删除
    manager.providers.remove(&id);

    // 保存配置
    drop(config); // 释放锁
    state.save()?;

    Ok(true)
}

/// 切换供应商
#[tauri::command]
pub async fn switch_provider(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    id: String,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let mut config = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = config
        .get_manager_mut(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    // 检查供应商是否存在
    let provider = manager
        .providers
        .get(&id)
        .ok_or_else(|| format!("供应商不存在: {}", id))?
        .clone();

    // SSOT 切换：先回填 live 配置到当前供应商，然后从内存写入目标主配置
    match app_type {
        AppType::Codex => {
            use serde_json::Value;

            // 回填：读取 live（auth.json + config.toml）写回当前供应商 settings_config
            if !manager.current.is_empty() {
                let auth_path = codex_config::get_codex_auth_path();
                let config_path = codex_config::get_codex_config_path();
                if auth_path.exists() {
                    let auth: Value = crate::config::read_json_file(&auth_path)?;
                    let config_str = if config_path.exists() {
                        std::fs::read_to_string(&config_path)
                            .map_err(|e| format!("读取 config.toml 失败: {}", e))?
                    } else {
                        String::new()
                    };

                    let live = serde_json::json!({
                        "auth": auth,
                        "config": config_str,
                    });

                    if let Some(cur) = manager.providers.get_mut(&manager.current) {
                        cur.settings_config = live;
                    }
                }
            }

            // 切换：从目标供应商 settings_config 写入主配置
            let auth_path = codex_config::get_codex_auth_path();
            let config_path = codex_config::get_codex_config_path();
            if let Some(parent) = auth_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建 Codex 目录失败: {}", e))?;
            }

            // 写 auth.json（必需）
            let auth = provider
                .settings_config
                .get("auth")
                .ok_or_else(|| "目标供应商缺少 auth 配置".to_string())?;
            crate::config::write_json_file(&auth_path, auth)?;

            // 写 config.toml（可选）
            if let Some(cfg) = provider.settings_config.get("config") {
                if let Some(cfg_str) = cfg.as_str() {
                    if !cfg_str.trim().is_empty() {
                        toml::from_str::<toml::Table>(cfg_str)
                            .map_err(|e| format!("config.toml 格式错误: {}", e))?;
                    }
                    std::fs::write(&config_path, cfg_str)
                        .map_err(|e| format!("写入 config.toml 失败: {}", e))?;
                } else {
                    // 非字符串时，写空
                    std::fs::write(&config_path, "")
                        .map_err(|e| format!("写入空的 config.toml 失败: {}", e))?;
                }
            } else {
                // 缺失则写空
                std::fs::write(&config_path, "")
                    .map_err(|e| format!("写入空的 config.toml 失败: {}", e))?;
            }
        }
        AppType::Claude => {
            use crate::config::{read_json_file, write_json_file};

            let settings_path = get_claude_settings_path();

            // 回填：读取 live settings.json 写回当前供应商 settings_config
            if settings_path.exists() && !manager.current.is_empty() {
                if let Ok(live) = read_json_file::<serde_json::Value>(&settings_path) {
                    if let Some(cur) = manager.providers.get_mut(&manager.current) {
                        cur.settings_config = live;
                    }
                }
            }

            // 切换：从目标供应商 settings_config 写入主配置
            if let Some(parent) = settings_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
            }
            write_json_file(&settings_path, &provider.settings_config)?;
        }
    }

    // 更新当前供应商
    manager.current = id;

    log::info!("成功切换到供应商: {}", provider.name);

    // 保存配置
    drop(config); // 释放锁
    state.save()?;

    Ok(true)
}

/// 导入当前配置为默认供应商
#[tauri::command]
pub async fn import_default_config(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    // 若已存在 default 供应商，则直接返回，避免重复导入
    {
        let config = state
            .config
            .lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;

        if let Some(manager) = config.get_manager(&app_type) {
            if manager.get_all_providers().contains_key("default") {
                return Ok(true);
            }
        }
    }

    // 根据应用类型导入配置
    // 读取当前主配置为默认供应商（不再写入副本文件）
    let settings_config = match app_type {
        AppType::Codex => {
            let auth_path = codex_config::get_codex_auth_path();
            let config_path = codex_config::get_codex_config_path();
            if !auth_path.exists() {
                return Err("Codex 配置文件不存在".to_string());
            }
            let auth: serde_json::Value = crate::config::read_json_file::<serde_json::Value>(&auth_path)?;
            let config_str = if config_path.exists() {
                let s = std::fs::read_to_string(&config_path)
                    .map_err(|e| format!("读取 config.toml 失败: {}", e))?;
                if !s.trim().is_empty() {
                    toml::from_str::<toml::Table>(&s)
                        .map_err(|e| format!("config.toml 语法错误: {}", e))?;
                }
                s
            } else {
                String::new()
            };
            serde_json::json!({ "auth": auth, "config": config_str })
        }
        AppType::Claude => {
            let settings_path = get_claude_settings_path();
            if !settings_path.exists() {
                return Err("Claude Code 配置文件不存在".to_string());
            }
            crate::config::read_json_file::<serde_json::Value>(&settings_path)?
        }
    };

    // 创建默认供应商
    let provider = Provider::with_id(
        "default".to_string(),
        "default".to_string(),
        settings_config,
        None,
    );

    // 添加到管理器
    let mut config = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = config
        .get_manager_mut(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    // 不再写入副本文件，仅更新内存配置

    manager.providers.insert(provider.id.clone(), provider);

    // 如果没有当前供应商，设置为 default
    if manager.current.is_empty() {
        manager.current = "default".to_string();
    }

    // 保存配置
    drop(config); // 释放锁
    state.save()?;

    Ok(true)
}

/// 获取 Claude Code 配置状态
#[tauri::command]
pub async fn get_claude_config_status() -> Result<ConfigStatus, String> {
    Ok(crate::config::get_claude_config_status())
}

/// 获取应用配置状态（通用）
/// 兼容两种参数：`app_type`（推荐）或 `app`（字符串）
#[tauri::command]
pub async fn get_config_status(
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<ConfigStatus, String> {
    let app = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    match app {
        AppType::Claude => Ok(crate::config::get_claude_config_status()),
        AppType::Codex => {
            use crate::codex_config::{get_codex_auth_path, get_codex_config_dir};
            let auth_path = get_codex_auth_path();

            // 放宽：只要 auth.json 存在即可认为已配置；config.toml 允许为空
            let exists = auth_path.exists();
            let path = get_codex_config_dir().to_string_lossy().to_string();

            Ok(ConfigStatus { exists, path })
        }
    }
}

/// 获取 Claude Code 配置文件路径
#[tauri::command]
pub async fn get_claude_code_config_path() -> Result<String, String> {
    Ok(get_claude_settings_path().to_string_lossy().to_string())
}

/// 打开配置文件夹
/// 兼容两种参数：`app_type`（推荐）或 `app`（字符串）
#[tauri::command]
pub async fn open_config_folder(
    handle: tauri::AppHandle,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);
    
    let config_dir = match app_type {
        AppType::Claude => crate::config::get_claude_config_dir(),
        AppType::Codex => crate::codex_config::get_codex_config_dir(),
    };

    // 确保目录存在
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    // 使用 opener 插件打开文件夹
    handle.opener()
        .open_path(config_dir.to_string_lossy().to_string(), None::<String>)
        .map_err(|e| format!("打开文件夹失败: {}", e))?;

    Ok(true)
}

/// 打开外部链接
#[tauri::command]
pub async fn open_external(app: tauri::AppHandle, url: String) -> Result<bool, String> {
    // 规范化 URL，缺少协议时默认加 https://
    let url = if url.starts_with("http://") || url.starts_with("https://") {
        url
    } else {
        format!("https://{}", url)
    };

    // 使用 opener 插件打开链接
    app.opener()
        .open_url(&url, None::<String>)
        .map_err(|e| format!("打开链接失败: {}", e))?;

    Ok(true)
}
