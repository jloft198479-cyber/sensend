use tauri::{AppHandle, Emitter, Manager, WebviewUrl};
use tauri_plugin_store::StoreExt;
use serde_json::Value;

use crate::adapters::{self, PlatformAdapter, PlatformInstance, PlatformTypeInfo, ProbeResult, PublishResult};

// ── Store 辅助 ──

fn get_instances_from_store(app: &AppHandle) -> Result<Vec<PlatformInstance>, String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    match store.get("platform_instances") {
        Some(value) => serde_json::from_value(value).map_err(|e| e.to_string()),
        None => Ok(vec![]),
    }
}

fn save_instances_to_store(app: &AppHandle, instances: &[PlatformInstance]) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set("platform_instances", serde_json::to_value(instances).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())
}

fn get_adapter(platform_type: &str) -> Result<Box<dyn PlatformAdapter>, String> {
    match platform_type {
        "local" => Ok(Box::new(adapters::local::LocalAdapter::new())),
        "notion" => Ok(Box::new(adapters::notion::NotionAdapter::new())),
        "flowus" => Ok(Box::new(adapters::flowus::FlowUsAdapter::new())),
        "lark" => Ok(Box::new(adapters::lark::LarkAdapter::new())),
        _ => Err(format!("不支持的平台类型: {}", platform_type)),
    }
}

// ── Commands ──

#[tauri::command]
pub async fn open_config_window(app: AppHandle) -> Result<(), String> {
    if let Some(config_win) = app.get_webview_window("config") {
        let _ = config_win.show();
        let _ = config_win.set_focus();
        return Ok(());
    }

    let main_win = app.get_webview_window("main").ok_or("主窗口不存在")?;
    let main_pos = main_win.outer_position().map_err(|e| e.to_string())?;

    let config_url = match cfg!(debug_assertions) {
        true => WebviewUrl::External("http://localhost:1420?page=config".parse().unwrap()),
        false => WebviewUrl::App("index.html?page=config".into()),
    };

    tauri::WebviewWindowBuilder::new(&app, "config", config_url)
        .title("Sensend - 平台管理")
        .inner_size(420.0, 580.0)
        .min_inner_size(420.0, 580.0)
        .position(main_pos.x as f64 + 40.0, main_pos.y as f64 + 60.0)
        .resizable(true)
        .decorations(true)
        .always_on_top(true)
        .visible(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_platform_types() -> Vec<PlatformTypeInfo> {
    adapters::get_platform_types()
}

#[tauri::command]
pub async fn list_platform_instances(app: AppHandle) -> Result<Vec<PlatformInstance>, String> {
    get_instances_from_store(&app)
}

#[tauri::command]
pub async fn save_platform_instance(
    app: AppHandle,
    instance: PlatformInstance,
) -> Result<(), String> {
    let mut instances = get_instances_from_store(&app)?;
    if let Some(pos) = instances.iter().position(|i| i.id == instance.id) {
        instances[pos] = instance;
    } else {
        instances.push(instance);
    }
    save_instances_to_store(&app, &instances)?;
    let _ = app.emit("instances-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn delete_platform_instance(
    app: AppHandle,
    instance_id: String,
) -> Result<(), String> {
    let mut instances = get_instances_from_store(&app)?;
    instances.retain(|i| i.id != instance_id);
    save_instances_to_store(&app, &instances)?;
    let _ = app.emit("instances-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn test_platform_connection(instance: PlatformInstance) -> Result<(), String> {
    let adapter = get_adapter(&instance.platform_type)?;
    adapter.test_connection(&instance).await
}

#[tauri::command]
pub async fn probe_target(instance: PlatformInstance) -> Result<ProbeResult, String> {
    let adapter = get_adapter(&instance.platform_type)?;
    let target_type = adapter.probe_type(&instance).await?;
    Ok(ProbeResult { target_type })
}

#[tauri::command]
pub async fn publish_note(
    app: AppHandle,
    instance_id: String,
    content: Value,
) -> Result<PublishResult, String> {
    let instances = get_instances_from_store(&app)?;
    let instance = instances.iter().find(|i| i.id == instance_id)
        .ok_or_else(|| "未找到指定的平台实例".to_string())?;

    let adapter = get_adapter(&instance.platform_type)?;
    if instance.publish_mode == "block" {
        adapter.append_blocks(&content, instance).await
    } else {
        adapter.publish(&content, instance).await
    }
}