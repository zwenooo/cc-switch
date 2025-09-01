use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::config::{get_provider_config_path, write_json_file};

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
    pub fn with_id(
        id: String,
        name: String,
        settings_config: Value,
        website_url: Option<String>,
    ) -> Self {
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
    /// 添加供应商
    pub fn add_provider(&mut self, provider: Provider) -> Result<(), String> {
        // 保存供应商配置到独立文件
        let config_path = get_provider_config_path(&provider.id, Some(&provider.name));
        write_json_file(&config_path, &provider.settings_config)?;

        // 添加到管理器
        self.providers.insert(provider.id.clone(), provider);
        Ok(())
    }

    /// 获取所有供应商
    pub fn get_all_providers(&self) -> &HashMap<String, Provider> {
        &self.providers
    }
}
