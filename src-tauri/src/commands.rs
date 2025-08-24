use std::collections::HashMap;
use tauri::State;

use crate::config::{
    import_current_config_as_default, get_claude_settings_path,
    ConfigStatus,
};
use crate::provider::Provider;
use crate::store::AppState;

/// 获取所有供应商
#[tauri::command]
pub async fn get_providers(state: State<'_, AppState>) -> Result<HashMap<String, Provider>, String> {
    let manager = state.provider_manager.lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    
    Ok(manager.get_all_providers().clone())
}

/// 获取当前供应商ID
#[tauri::command]
pub async fn get_current_provider(state: State<'_, AppState>) -> Result<String, String> {
    let manager = state.provider_manager.lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    
    Ok(manager.current.clone())
}

/// 添加供应商
#[tauri::command]
pub async fn add_provider(
    state: State<'_, AppState>,
    provider: Provider,
) -> Result<bool, String> {
    let mut manager = state.provider_manager.lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    
    manager.add_provider(provider)?;
    
    // 保存配置
    drop(manager); // 释放锁
    state.save()?;
    
    Ok(true)
}

/// 更新供应商
#[tauri::command]
pub async fn update_provider(
    state: State<'_, AppState>,
    provider: Provider,
) -> Result<bool, String> {
    let mut manager = state.provider_manager.lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    
    manager.update_provider(provider)?;
    
    // 保存配置
    drop(manager); // 释放锁
    state.save()?;
    
    Ok(true)
}

/// 删除供应商
#[tauri::command]
pub async fn delete_provider(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let mut manager = state.provider_manager.lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    
    manager.delete_provider(&id)?;
    
    // 保存配置
    drop(manager); // 释放锁
    state.save()?;
    
    Ok(true)
}

/// 切换供应商
#[tauri::command]
pub async fn switch_provider(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let mut manager = state.provider_manager.lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    
    manager.switch_provider(&id)?;
    
    // 保存配置
    drop(manager); // 释放锁
    state.save()?;
    
    Ok(true)
}

/// 导入当前配置为默认供应商
#[tauri::command]
pub async fn import_default_config(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    // 若已存在 default 供应商，则直接返回，避免重复导入
    {
        let manager = state
            .provider_manager
            .lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;
        if manager.get_all_providers().contains_key("default") {
            return Ok(true);
        }
    }

    // 导入配置
    let settings_config = import_current_config_as_default()?;
    
    // 创建默认供应商
    let provider = Provider::with_id(
        "default".to_string(),
        "default".to_string(),
        settings_config,
        None,
    );
    
    // 添加到管理器
    let mut manager = state.provider_manager.lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    
    manager.add_provider(provider)?;
    
    // 如果没有当前供应商，设置为 default
    if manager.current.is_empty() {
        manager.current = "default".to_string();
    }
    
    // 保存配置
    drop(manager); // 释放锁
    state.save()?;
    
    Ok(true)
}

/// 获取 Claude Code 配置状态
#[tauri::command]
pub async fn get_claude_config_status() -> Result<ConfigStatus, String> {
    Ok(crate::config::get_claude_config_status())
}

/// 获取 Claude Code 配置文件路径
#[tauri::command]
pub async fn get_claude_code_config_path() -> Result<String, String> {
    Ok(get_claude_settings_path().to_string_lossy().to_string())
}

/// 打开配置文件夹
#[tauri::command]
pub async fn open_config_folder() -> Result<bool, String> {
    let config_dir = crate::config::get_claude_config_dir();
    
    // 确保目录存在
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }
    
    // 在不同平台上打开文件夹
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&config_dir)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&config_dir)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&config_dir)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {}", e))?;
    }
    
    Ok(true)
}

/// 打开外部链接
#[tauri::command]
pub async fn open_external(url: String) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", "", &url])
            .spawn()
            .map_err(|e| format!("打开链接失败: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("打开链接失败: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("打开链接失败: {}", e))?;
    }
    
    Ok(true)
}
