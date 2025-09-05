mod app_config;
mod codex_config;
mod commands;
mod config;
mod provider;
mod store;
mod migration;

use store::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                // 设置 macOS 标题栏背景色为主界面蓝色
                if let Some(window) = app.get_webview_window("main") {
                    use objc2::rc::Retained;
                    use objc2::runtime::AnyObject;
                    use objc2_app_kit::NSColor;

                    let ns_window_ptr = window.ns_window().unwrap();
                    let ns_window: Retained<AnyObject> =
                        unsafe { Retained::retain(ns_window_ptr as *mut AnyObject).unwrap() };

                    // 使用与主界面 banner 相同的蓝色 #3498db
                    // #3498db = RGB(52, 152, 219)
                    let bg_color = unsafe {
                        NSColor::colorWithRed_green_blue_alpha(
                            52.0 / 255.0,  // R: 52
                            152.0 / 255.0, // G: 152
                            219.0 / 255.0, // B: 219
                            1.0,           // Alpha: 1.0
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

            // 首次启动迁移：扫描副本文件，合并到 config.json，并归档副本；旧 config.json 先归档
            {
                let mut config_guard = app_state.config.lock().unwrap();
                let migrated = migration::migrate_copies_into_config(&mut *config_guard)?;
                if migrated {
                    log::info!("已将副本文件导入到 config.json，并完成归档");
                }
                // 确保两个 App 条目存在
                config_guard.ensure_app(&app_config::AppType::Claude);
                config_guard.ensure_app(&app_config::AppType::Codex);
            }

            // 保存配置
            let _ = app_state.save();

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
            commands::get_config_status,
            commands::get_claude_code_config_path,
            commands::open_config_folder,
            commands::open_external,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
