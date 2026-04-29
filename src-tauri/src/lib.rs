use std::fs;
use tauri::{Emitter, Manager};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};

mod adapters;
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            commands::note::read_note,
            commands::note::save_note,
            commands::note::hide_window,
            commands::note::open_data_dir,
            commands::note::request_quit,
            commands::platform::open_config_window,
            commands::platform::get_platform_types,
            commands::platform::list_platform_instances,
            commands::platform::save_platform_instance,
            commands::platform::delete_platform_instance,
            commands::platform::test_platform_connection,
            commands::platform::probe_target,
            commands::platform::publish_note,
            commands::hotkey::get_hotkeys,
            commands::hotkey::save_hotkeys,
            commands::font::scan_user_fonts,
            commands::font::open_fonts_dir,
            commands::font::delete_user_font,
        ])
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("无法获取应用数据目录");
            if !app_dir.exists() {
                fs::create_dir_all(&app_dir).expect("无法创建应用数据目录");
            }

            // 注册全局唤醒快捷键
            commands::hotkey::init_hotkeys(&app.handle().clone());

            // 系统托盘
            if app.tray_by_id("main").is_none() {
                let show_item = MenuItemBuilder::with_id("show", "显示窗口").build(app)?;
                let quit_item = MenuItemBuilder::with_id("quit", "退出 Sensend").build(app)?;
                let menu = MenuBuilder::new(app)
                    .items(&[&show_item, &quit_item])
                    .build()?;

                TrayIconBuilder::with_id("main")
                    .icon(app.default_window_icon().cloned().unwrap())
                    .menu(&menu)
                    .tooltip("Sensend")
                    .on_menu_event(|app, event| {
                        match event.id().as_ref() {
                            "show" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                            "quit" => {
                                let _ = app.emit("app-exit-request", ());
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}