use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

use crate::config::{
    copy_file, delete_file, get_provider_config_path, read_json_file, write_json_file,
    get_claude_settings_path, backup_config
};

/// 供应商结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    #[serde(rename = "settingsConfig")]
    pub settings_config: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
}

impl Provider {
    /// 从现有ID创建供应商
    pub fn with_id(id: String, name: String, settings_config: Value, website_url: Option<String>) -> Self {
        Self {
            id,
            name,
            settings_config,
            website_url,
        }
    }
}

/// 供应商管理器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderManager {
    pub providers: HashMap<String, Provider>,
    pub current: String,
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
            current: String::new(),
        }
    }
}

impl ProviderManager {
    /// 加载供应商列表
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            log::info!("配置文件不存在，创建新的供应商管理器");
            return Ok(Self::default());
        }
        
        read_json_file(path)
    }
    
    /// 保存供应商列表
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        write_json_file(path, self)
    }
    
    /// 添加供应商
    pub fn add_provider(&mut self, provider: Provider) -> Result<(), String> {
        // 保存供应商配置到独立文件
        let config_path = get_provider_config_path(&provider.id, Some(&provider.name));
        write_json_file(&config_path, &provider.settings_config)?;
        
        // 添加到管理器
        self.providers.insert(provider.id.clone(), provider);
        Ok(())
    }
    
    /// 更新供应商
    pub fn update_provider(&mut self, provider: Provider) -> Result<(), String> {
        // 检查供应商是否存在
        if !self.providers.contains_key(&provider.id) {
            return Err(format!("供应商不存在: {}", provider.id));
        }
        
        // 如果名称改变了，需要处理配置文件
        if let Some(old_provider) = self.providers.get(&provider.id) {
            if old_provider.name != provider.name {
                // 删除旧配置文件
                let old_config_path = get_provider_config_path(&provider.id, Some(&old_provider.name));
                delete_file(&old_config_path).ok(); // 忽略删除错误
            }
        }
        
        // 保存新配置文件
        let config_path = get_provider_config_path(&provider.id, Some(&provider.name));
        write_json_file(&config_path, &provider.settings_config)?;
        
        // 更新管理器
        self.providers.insert(provider.id.clone(), provider);
        Ok(())
    }
    
    /// 删除供应商
    pub fn delete_provider(&mut self, provider_id: &str) -> Result<(), String> {
        // 检查是否为当前供应商
        if self.current == provider_id {
            return Err("不能删除当前正在使用的供应商".to_string());
        }
        
        // 获取供应商信息
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| format!("供应商不存在: {}", provider_id))?;
        
        // 删除配置文件
        let config_path = get_provider_config_path(provider_id, Some(&provider.name));
        delete_file(&config_path)?;
        
        // 从管理器删除
        self.providers.remove(provider_id);
        Ok(())
    }
    
    /// 切换供应商
    pub fn switch_provider(&mut self, provider_id: &str) -> Result<(), String> {
        // 检查供应商是否存在
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| format!("供应商不存在: {}", provider_id))?;
        
        let settings_path = get_claude_settings_path();
        let provider_config_path = get_provider_config_path(provider_id, Some(&provider.name));
        
        // 检查供应商配置文件是否存在
        if !provider_config_path.exists() {
            return Err(format!("供应商配置文件不存在: {}", provider_config_path.display()));
        }
        
        // 如果当前有配置，先备份到当前供应商
        if settings_path.exists() && !self.current.is_empty() {
            if let Some(current_provider) = self.providers.get(&self.current) {
                let current_provider_path = get_provider_config_path(&self.current, Some(&current_provider.name));
                backup_config(&settings_path, &current_provider_path)?;
                log::info!("已备份当前供应商配置: {}", current_provider.name);
            }
        }
        
        // 确保主配置父目录存在
        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败: {}", e))?;
        }

        // 复制新供应商配置到主配置
        copy_file(&provider_config_path, &settings_path)?;
        
        // 更新当前供应商
        self.current = provider_id.to_string();
        
        log::info!("成功切换到供应商: {}", provider.name);
        Ok(())
    }
    
    /// 获取所有供应商
    pub fn get_all_providers(&self) -> &HashMap<String, Provider> {
        &self.providers
    }
}
