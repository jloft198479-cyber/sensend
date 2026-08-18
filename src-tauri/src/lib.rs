use std::fs;
use tauri::{Emitter, Manager};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};

mod adapters;
mod commands;
mod memory_trim;

/// 唤起主窗的统一入口（单实例 / 托盘菜单 / 托盘点击 / 快捷键共用）
/// S5 悬浮球上线后，在此追加隐藏悬浮球的逻辑，四处调用点无需再改
pub fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        // 使 pending 的内存修剪失效，保证唤起首下不被换页拖慢
        memory_trim::on_window_shown();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── 内存优化（乙）：禁用 GPU 进程 + V8 堆封顶 ──
    // WebView2 通过该环境变量追加 Chromium 启动参数（必须在 WebView2 环境创建前设置）：
    // - --disable-gpu：不启动 GPU 进程（省 ~40-60MB 物理内存），
    //   本应用为小白底纯文字窗口，CPU 软件渲染肉眼无差别
    // - --disable-software-rasterizer：连带禁用 SwiftShader 软件光栅化
    // - --memory-pressure-threshold=moderate：更早触发 Chromium 内存回收
    // - --js-flags=--max-old-space-size=96：V8 堆封顶 96MB，防 renderer 无界增长
    let base = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    let extra = "--disable-gpu --disable-software-rasterizer --memory-pressure-threshold=moderate --js-flags=--max-old-space-size=96";
    if base.is_empty() {
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", extra);
    } else if !base.contains("--disable-gpu") {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            format!("{base} {extra}"),
        );
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
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
            commands::platform::get_default_target,
            commands::platform::set_default_target,
            commands::platform::get_token_memory,
            commands::platform::set_token_memory,
            commands::platform::get_theme,
            commands::platform::set_theme,
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

            // 窗口底色跟随已存主题（防启动闪色）
            let bg = commands::platform::stored_window_bg(&app.handle());
            commands::platform::apply_window_bg(&app.handle(), bg);

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
                                show_main(app);
                            }
                            "quit" => {
                                let _ = app.emit("app-exit-request", ());
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                            show_main(tray.app_handle());
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
                    // 隐藏 10 秒后自动修剪进程树工作集（隐藏即瘦身）
                    memory_trim::on_window_hidden();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}