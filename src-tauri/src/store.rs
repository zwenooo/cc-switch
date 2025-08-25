use std::sync::Mutex;
use crate::config::get_app_config_path;
use crate::provider::ProviderManager;

/// 全局应用状态
pub struct AppState {
    pub provider_manager: Mutex<ProviderManager>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new() -> Self {
        let config_path = get_app_config_path();
        let provider_manager = ProviderManager::load_from_file(&config_path)
            .unwrap_or_else(|e| {
                log::warn!("加载配置失败: {}, 使用默认配置", e);
                ProviderManager::default()
            });
        
        Self {
            provider_manager: Mutex::new(provider_manager),
        }
    }
    
    /// 保存配置到文件
    pub fn save(&self) -> Result<(), String> {
        let config_path = get_app_config_path();
        let manager = self.provider_manager.lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;
        
        manager.save_to_file(&config_path)
    }
    
    // 保留按需扩展：若未来需要热加载，可在此实现
}
