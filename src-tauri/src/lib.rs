mod config;
mod provider;
mod store;
mod commands;

use store::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                // 设置 macOS 标题栏背景色为主界面蓝色
                if let Some(window) = app.get_webview_window("main") {
                    use objc2::runtime::AnyObject;
                    use objc2::rc::Retained;
                    use objc2_app_kit::NSColor;
                    
                    let ns_window_ptr = window.ns_window().unwrap();
                    let ns_window: Retained<AnyObject> = unsafe { 
                        Retained::retain(ns_window_ptr as *mut AnyObject).unwrap()
                    };
                    
                    // 使用与主界面 banner 相同的蓝色 #3498db
                    // #3498db = RGB(52, 152, 219)
                    let bg_color = unsafe {
                        NSColor::colorWithRed_green_blue_alpha(
                            52.0/255.0,   // R: 52
                            152.0/255.0,  // G: 152  
                            219.0/255.0,  // B: 219
                            1.0,          // Alpha: 1.0
                        )
                    };
                    
                    unsafe {
                        use objc2::msg_send;
                        let _: () = msg_send![&*ns_window, setBackgroundColor: &*bg_color];
                    }
                }
            }
            
            // 初始化日志
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            
            // 初始化应用状态（仅创建一次，并在本函数末尾注入 manage）
            let app_state = AppState::new();
            
            // 如果没有供应商且存在 Claude Code 配置，自动导入
            {
                let manager = app_state.provider_manager.lock().unwrap();
                if manager.providers.is_empty() {
                    drop(manager); // 释放锁
                    
                    let settings_path = config::get_claude_settings_path();
                    if settings_path.exists() {
                        log::info!("检测到 Claude Code 配置，自动导入为默认供应商");
                        
                        if let Ok(settings_config) = config::import_current_config_as_default() {
                            let mut manager = app_state.provider_manager.lock().unwrap();
                            let provider = provider::Provider::with_id(
                                "default".to_string(),
                                "default".to_string(),
                                settings_config,
                                None,
                            );
                            
                            if manager.add_provider(provider).is_ok() {
                                manager.current = "default".to_string();
                                drop(manager);
                                let _ = app_state.save();
                                log::info!("成功导入默认供应商");
                            }
                        }
                    }
                }
            }
            
            // 将同一个实例注入到全局状态，避免重复创建导致的不一致
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_providers,
            commands::get_current_provider,
            commands::add_provider,
            commands::update_provider,
            commands::delete_provider,
            commands::switch_provider,
            commands::import_default_config,
            commands::get_claude_config_status,
            commands::get_claude_code_config_path,
            commands::open_config_folder,
            commands::open_external,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
