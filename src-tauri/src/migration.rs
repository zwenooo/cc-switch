use crate::app_config::{AppType, MultiAppConfig};
use crate::config::{
    archive_file, get_app_config_dir, get_app_config_path, get_claude_config_dir,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn get_marker_path() -> PathBuf {
    get_app_config_dir().join("migrated.copies.v1")
}

fn sanitized_id(base: &str) -> String {
    crate::config::sanitize_provider_name(base)
}

fn next_unique_id(existing: &HashSet<String>, base: &str) -> String {
    let base = sanitized_id(base);
    if !existing.contains(&base) {
        return base;
    }
    for i in 2..1000 {
        let candidate = format!("{}-{}", base, i);
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    format!("{}-dup", base)
}

fn extract_claude_api_key(value: &Value) -> Option<String> {
    value
        .get("env")
        .and_then(|env| env.get("ANTHROPIC_AUTH_TOKEN"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn extract_codex_api_key(value: &Value) -> Option<String> {
    value
        .get("auth")
        .and_then(|auth| auth.get("OPENAI_API_KEY").or_else(|| auth.get("openai_api_key")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// 去重策略：name + 原始 key 直接比较（不做哈希）

fn scan_claude_copies() -> Vec<(String, PathBuf, Value)> {
    let mut items = Vec::new();
    let dir = get_claude_config_dir();
    if !dir.exists() {
        return items;
    }
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let fname = match p.file_name().and_then(|s| s.to_str()) {
                Some(s) => s,
                None => continue,
            };
            if fname == "settings.json" || fname == "claude.json" {
                continue;
            }
            if !fname.starts_with("settings-") || !fname.ends_with(".json") {
                continue;
            }
            let name = fname.trim_start_matches("settings-").trim_end_matches(".json");
            if let Ok(val) = crate::config::read_json_file::<Value>(&p) {
                items.push((name.to_string(), p, val));
            }
        }
    }
    items
}

fn scan_codex_copies() -> Vec<(String, Option<PathBuf>, Option<PathBuf>, Value)> {
    let mut by_name: HashMap<String, (Option<PathBuf>, Option<PathBuf>)> = HashMap::new();
    let dir = crate::codex_config::get_codex_config_dir();
    if !dir.exists() {
        return Vec::new();
    }
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let fname = match p.file_name().and_then(|s| s.to_str()) {
                Some(s) => s,
                None => continue,
            };
            if fname.starts_with("auth-") && fname.ends_with(".json") {
                let name = fname.trim_start_matches("auth-").trim_end_matches(".json");
                let entry = by_name.entry(name.to_string()).or_default();
                entry.0 = Some(p);
            } else if fname.starts_with("config-") && fname.ends_with(".toml") {
                let name = fname.trim_start_matches("config-").trim_end_matches(".toml");
                let entry = by_name.entry(name.to_string()).or_default();
                entry.1 = Some(p);
            }
        }
    }

    let mut items = Vec::new();
    for (name, (auth_path, config_path)) in by_name {
        if let Some(authp) = auth_path {
            if let Ok(auth) = crate::config::read_json_file::<Value>(&authp) {
                let config_str = if let Some(cfgp) = &config_path {
                    fs::read_to_string(cfgp).unwrap_or_default()
                } else {
                    String::new()
                };
                // 校验 TOML（若非空）
                if !config_str.trim().is_empty() {
                    if let Err(e) = toml::from_str::<toml::Table>(&config_str) {
                        log::warn!("跳过无效 Codex config-{}.toml: {}", name, e);
                    }
                }
                let settings = serde_json::json!({
                    "auth": auth,
                    "config": config_str,
                });
                items.push((name, Some(authp), config_path, settings));
            }
        }
    }
    items
}

pub fn migrate_copies_into_config(config: &mut MultiAppConfig) -> Result<bool, String> {
    // 如果已迁移过则跳过
    let marker = get_marker_path();
    if marker.exists() {
        return Ok(false);
    }

    let claude_items = scan_claude_copies();
    let codex_items = scan_codex_copies();
    if claude_items.is_empty() && codex_items.is_empty() {
        // 即便没有可迁移项，也写入标记避免每次扫描
        fs::write(&marker, b"no-copies").map_err(|e| format!("写入迁移标记失败: {}", e))?;
        return Ok(false);
    }

    // 备份旧的 config.json
    let ts = now_ts();
    let app_cfg_path = get_app_config_path();
    if app_cfg_path.exists() {
        let _ = archive_file(ts, "cc-switch", &app_cfg_path);
    }

    // 读取 live：Claude（settings.json / claude.json）
    let live_claude: Option<(String, Value)> = {
        let settings_path = crate::config::get_claude_settings_path();
        if settings_path.exists() {
            match crate::config::read_json_file::<Value>(&settings_path) {
                Ok(val) => Some(("current".to_string(), val)),
                Err(e) => {
                    log::warn!("读取 Claude live 配置失败: {}", e);
                    None
                }
            }
        } else {
            None
        }
    };

    // 合并：Claude（优先 live，然后副本） - 去重键: name + apiKey（直接比较）
    config.ensure_app(&AppType::Claude);
    let manager = config.get_manager_mut(&AppType::Claude).unwrap();
    let mut ids: HashSet<String> = manager.providers.keys().cloned().collect();
    let mut live_claude_id: Option<String> = None;

    if let Some((name, value)) = &live_claude {
        let cand_key = extract_claude_api_key(value);
        let exist_id = manager
            .providers
            .iter()
            .find_map(|(id, p)| {
                let pk = extract_claude_api_key(&p.settings_config);
                if p.name == *name && pk == cand_key { Some(id.clone()) } else { None }
            });
        if let Some(exist_id) = exist_id {
            if let Some(prov) = manager.providers.get_mut(&exist_id) {
                log::info!("合并到已存在 Claude 供应商 '{}' (by name+key)", name);
                prov.settings_config = value.clone();
                live_claude_id = Some(exist_id);
            }
        } else {
            let id = next_unique_id(&ids, name);
            ids.insert(id.clone());
            let provider = crate::provider::Provider::with_id(
                id.clone(),
                name.clone(),
                value.clone(),
                None,
            );
            manager.providers.insert(provider.id.clone(), provider);
            live_claude_id = Some(id);
        }
    }
    for (name, path, value) in claude_items.iter() {
        let cand_key = extract_claude_api_key(value);
        let exist_id = manager
            .providers
            .iter()
            .find_map(|(id, p)| {
                let pk = extract_claude_api_key(&p.settings_config);
                if p.name == *name && pk == cand_key { Some(id.clone()) } else { None }
            });
        if let Some(exist_id) = exist_id {
            if let Some(prov) = manager.providers.get_mut(&exist_id) {
                log::info!("覆盖 Claude 供应商 '{}' 来自 {} (by name+key)", name, path.display());
                prov.settings_config = value.clone();
            }
        } else {
            let id = next_unique_id(&ids, name);
            ids.insert(id.clone());
            let provider = crate::provider::Provider::with_id(
                id.clone(),
                name.clone(),
                value.clone(),
                None,
            );
            manager.providers.insert(provider.id.clone(), provider);
        }
    }

    // 读取 live：Codex（auth.json 必需，config.toml 可空）
    let live_codex: Option<(String, Value)> = {
        let auth_path = crate::codex_config::get_codex_auth_path();
        let config_path = crate::codex_config::get_codex_config_path();
        if auth_path.exists() {
            match crate::config::read_json_file::<Value>(&auth_path) {
                Ok(auth) => {
                    let cfg = if config_path.exists() {
                        match std::fs::read_to_string(&config_path) {
                            Ok(s) => {
                                if !s.trim().is_empty() {
                                    if let Err(e) = toml::from_str::<toml::Table>(&s) {
                                        log::warn!("Codex live config.toml 语法错误: {}", e);
                                    }
                                }
                                s
                            }
                            Err(e) => {
                                log::warn!("读取 Codex live config.toml 失败: {}", e);
                                String::new()
                            }
                        }
                    } else {
                        String::new()
                    };
                    Some(("current".to_string(), serde_json::json!({"auth": auth, "config": cfg})))
                }
                Err(e) => {
                    log::warn!("读取 Codex live auth.json 失败: {}", e);
                    None
                }
            }
        } else {
            None
        }
    };

    // 合并：Codex（优先 live，然后副本） - 去重键: name + OPENAI_API_KEY（直接比较）
    config.ensure_app(&AppType::Codex);
    let manager = config.get_manager_mut(&AppType::Codex).unwrap();
    let mut ids: HashSet<String> = manager.providers.keys().cloned().collect();
    let mut live_codex_id: Option<String> = None;

    if let Some((name, value)) = &live_codex {
        let cand_key = extract_codex_api_key(value);
        let exist_id = manager
            .providers
            .iter()
            .find_map(|(id, p)| {
                let pk = extract_codex_api_key(&p.settings_config);
                if p.name == *name && pk == cand_key { Some(id.clone()) } else { None }
            });
        if let Some(exist_id) = exist_id {
            if let Some(prov) = manager.providers.get_mut(&exist_id) {
                log::info!("合并到已存在 Codex 供应商 '{}' (by name+key)", name);
                prov.settings_config = value.clone();
                live_codex_id = Some(exist_id);
            }
        } else {
            let id = next_unique_id(&ids, name);
            ids.insert(id.clone());
            let provider = crate::provider::Provider::with_id(
                id.clone(),
                name.clone(),
                value.clone(),
                None,
            );
            manager.providers.insert(provider.id.clone(), provider);
            live_codex_id = Some(id);
        }
    }
    for (name, authp, cfgp, value) in codex_items.iter() {
        let cand_key = extract_codex_api_key(value);
        let exist_id = manager
            .providers
            .iter()
            .find_map(|(id, p)| {
                let pk = extract_codex_api_key(&p.settings_config);
                if p.name == *name && pk == cand_key { Some(id.clone()) } else { None }
            });
        if let Some(exist_id) = exist_id {
            if let Some(prov) = manager.providers.get_mut(&exist_id) {
                log::info!("覆盖 Codex 供应商 '{}' 来自 {:?}/{:?} (by name+key)", name, authp, cfgp);
                prov.settings_config = value.clone();
            }
        } else {
            let id = next_unique_id(&ids, name);
            ids.insert(id.clone());
            let provider = crate::provider::Provider::with_id(
                id.clone(),
                name.clone(),
                value.clone(),
                None,
            );
            manager.providers.insert(provider.id.clone(), provider);
        }
    }

    // 若 current 为空，将 live 导入项设为 current
    {
        let manager = config.get_manager_mut(&AppType::Claude).unwrap();
        if manager.current.is_empty() {
            if let Some(id) = live_claude_id {
                manager.current = id;
            }
        }
    }
    {
        let manager = config.get_manager_mut(&AppType::Codex).unwrap();
        if manager.current.is_empty() {
            if let Some(id) = live_codex_id {
                manager.current = id;
            }
        }
    }

    // 归档副本文件
    for (_, p, _) in claude_items.into_iter() {
        let _ = archive_file(ts, "claude", &p);
    }
    for (_, ap, cp, _) in codex_items.into_iter() {
        if let Some(ap) = ap {
            let _ = archive_file(ts, "codex", &ap);
        }
        if let Some(cp) = cp {
            let _ = archive_file(ts, "codex", &cp);
        }
    }

    // 标记完成
    fs::write(&marker, b"done").map_err(|e| format!("写入迁移标记失败: {}", e))?;
    Ok(true)
}
