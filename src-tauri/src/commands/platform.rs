use tauri::{AppHandle, Emitter, Manager, WebviewUrl};
use tauri_plugin_store::StoreExt;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::adapters::{self, PlatformAdapter, PlatformInstance, PlatformTypeInfo, PublishResult};
use crate::security;

// ── Store 辅助 ──

/// 读取实例列表并解密凭证（旧版明文兼容：无 enc1: 前缀原样返回）
fn get_instances_from_store(app: &AppHandle) -> Result<Vec<PlatformInstance>, String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    match store.get("platform_instances") {
        Some(value) => {
            let mut instances: Vec<PlatformInstance> =
                serde_json::from_value(value).map_err(|e| e.to_string())?;
            for inst in &mut instances {
                inst.token = security::unprotect(&inst.token)?;
                inst.token2 = security::unprotect(&inst.token2)?;
            }
            Ok(instances)
        }
        None => Ok(vec![]),
    }
}

/// 递归检查存储原始值里是否还有未加密的 token/token2 字段。
/// 注意：判据是盘上原始值（未解密），不是内存明文——内存值经解密后
/// 全为明文形态，若用它判断会导致每次保存都触发备份。
fn value_has_plaintext_secret(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(k, v)| {
            if (k == "token" || k == "token2") && v.is_string() {
                let s = v.as_str().unwrap_or_default();
                !s.is_empty() && !security::is_encrypted(s)
            } else {
                value_has_plaintext_secret(v)
            }
        }),
        serde_json::Value::Array(items) => items.iter().any(value_has_plaintext_secret),
        _ => false,
    }
}

/// 明文→密文首次落盘前，把 config.json 复制为 config.json.bak（可回滚）。
/// 备份失败则中止写入，避免"没留底就覆盖明文"。
fn backup_config_before_migration(app: &AppHandle) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let src = dir.join("config.json");
    if src.exists() {
        std::fs::copy(&src, dir.join("config.json.bak"))
            .map_err(|e| format!("迁移前备份 config.json 失败: {e}"))?;
    }
    Ok(())
}

/// 保存实例列表：凭证加密落盘；盘上仍是明文（迁移未完成）时先备份再覆盖
fn save_instances_to_store(app: &AppHandle, instances: &[PlatformInstance]) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    if store.get("platform_instances").map_or(false, |v| value_has_plaintext_secret(&v)) {
        backup_config_before_migration(app)?;
    }
    let mut secured = instances.to_vec();
    for inst in &mut secured {
        inst.token = security::protect(&inst.token)?;
        inst.token2 = security::protect(&inst.token2)?;
    }
    store.set("platform_instances", serde_json::to_value(&secured).map_err(|e| e.to_string())?);
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
        .decorations(false)
        .always_on_top(true)
        .visible(true)
        .build()
        .map_err(|e| e.to_string())?;

    // 新窗口底色跟随当前主题
    apply_window_bg(&app, stored_window_bg(&app));

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
pub async fn publish_note(
    app: AppHandle,
    instance_id: String,
    content: Value,
) -> Result<PublishResult, String> {
    let instances = get_instances_from_store(&app)?;
    let instance = instances.iter().find(|i| i.id == instance_id)
        .ok_or_else(|| "未找到指定的平台实例".to_string())?;

    let adapter = get_adapter(&instance.platform_type)?;
    let result = if instance.publish_mode == "block" {
        adapter.append_blocks(&content, instance).await
    } else {
        adapter.publish(&content, instance).await
    };
    // 发送结果留痕（仅平台名/模式/结果消息，不含笔记内容与凭证），线上排障用
    match &result {
        Ok(r) => log::info!(
            "发送成功: 平台={} 模式={} 消息={}",
            instance.platform_type,
            instance.publish_mode,
            r.message
        ),
        Err(e) => log::warn!(
            "发送失败: 平台={} 模式={} 错误={}",
            instance.platform_type,
            instance.publish_mode,
            e
        ),
    }
    result
}

// ── 默认发送目标（存 config.json，比 localStorage 可靠）──

#[tauri::command]
pub async fn get_default_target(app: AppHandle) -> Result<Option<String>, String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    Ok(store.get("default_target").and_then(|v| v.as_str().map(|s| s.to_string())))
}

#[tauri::command]
pub async fn set_default_target(app: AppHandle, target_id: String) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set("default_target", serde_json::to_value(&target_id).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())
}

// ── 授权码记忆（存 config.json：token_memory { 平台: { token, token2 } }）──

/// 某平台最近一次填写的授权码，新增页面时预填，省去复制粘贴
#[derive(Serialize, serde::Deserialize, Clone, Default)]
pub struct TokenMemory {
    pub token: String,
    pub token2: String,
}

#[tauri::command]
pub async fn get_token_memory(app: AppHandle, platform_type: String) -> Result<Option<TokenMemory>, String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    match store.get("token_memory") {
        Some(value) => {
            let mut map: HashMap<String, TokenMemory> =
                serde_json::from_value(value).map_err(|e| e.to_string())?;
            if let Some(tm) = map.get_mut(&platform_type) {
                tm.token = security::unprotect(&tm.token)?;
                tm.token2 = security::unprotect(&tm.token2)?;
            }
            Ok(map.get(&platform_type).cloned())
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn set_token_memory(
    app: AppHandle,
    platform_type: String,
    token: String,
    token2: String,
) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    // 盘上仍是明文（迁移未完成）→ 先备份再覆盖
    if store.get("token_memory").map_or(false, |v| value_has_plaintext_secret(&v)) {
        backup_config_before_migration(&app)?;
    }
    let mut map: HashMap<String, TokenMemory> = store
        .get("token_memory")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    // 新值并入，全量统一为明文再加密落盘（顺带升级残留旧明文）
    map.insert(platform_type, TokenMemory { token, token2 });
    for tm in map.values_mut() {
        tm.token = security::unprotect(&tm.token)?;
        tm.token2 = security::unprotect(&tm.token2)?;
    }
    for tm in map.values_mut() {
        tm.token = security::protect(&tm.token)?;
        tm.token2 = security::protect(&tm.token2)?;
    }
    store.set("token_memory", serde_json::to_value(map).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())
}

// ── 启动迁移：把残留的明文凭证一次性加密落盘（幂等，无明文不写盘） ──

pub fn migrate_credentials_at_startup(app: &AppHandle) {
    let result = (|| -> Result<(), String> {
        let store = app.store("config.json").map_err(|e| e.to_string())?;
        let mut changed = false;

        // 1) platform_instances
        if let Some(value) = store.get("platform_instances") {
            let mut instances: Vec<PlatformInstance> =
                serde_json::from_value(value).map_err(|e| e.to_string())?;
            let mut touched = false;
            for inst in &mut instances {
                if !inst.token.is_empty() && !security::is_encrypted(&inst.token) {
                    inst.token = security::protect(&inst.token)?;
                    touched = true;
                }
                if !inst.token2.is_empty() && !security::is_encrypted(&inst.token2) {
                    inst.token2 = security::protect(&inst.token2)?;
                    touched = true;
                }
            }
            if touched {
                changed = true;
                store.set("platform_instances", serde_json::to_value(&instances).map_err(|e| e.to_string())?);
            }
        }

        // 2) token_memory
        if let Some(value) = store.get("token_memory") {
            let mut map: HashMap<String, TokenMemory> =
                serde_json::from_value(value).map_err(|e| e.to_string())?;
            let mut touched = false;
            for tm in map.values_mut() {
                if !tm.token.is_empty() && !security::is_encrypted(&tm.token) {
                    tm.token = security::protect(&tm.token)?;
                    touched = true;
                }
                if !tm.token2.is_empty() && !security::is_encrypted(&tm.token2) {
                    tm.token2 = security::protect(&tm.token2)?;
                    touched = true;
                }
            }
            if touched {
                changed = true;
                store.set("token_memory", serde_json::to_value(&map).map_err(|e| e.to_string())?);
            }
        }

        if changed {
            // 备份当前盘上的旧文件（此刻仍是明文），再落密文
            backup_config_before_migration(app)?;
            store.save().map_err(|e| e.to_string())?;
            log::info!("明文凭证已迁移为 DPAPI 密文（原文件备份为 config.json.bak）");
        }
        Ok(())
    })();
    if let Err(e) = result {
        // 迁移失败不阻塞启动：下次保存实例/启动时再试
        log::warn!("凭证加密迁移跳过: {e}");
    }
}

// ── 主题色（存 config.json，两窗口同步 + 窗口底色跟随）──
// 设计原则：主题注册表（含 windowBg 颜色）只有前端 useTheme.ts 一份（SSOT），
// 后端不存任何颜色映射，只负责"存值 / 涂色"，加主题无需改 Rust。

/// 读已存主题（无记录默认 light）
pub fn stored_theme(app: &AppHandle) -> String {
    app.store("config.json")
        .ok()
        .and_then(|s| s.get("theme").and_then(|v| v.as_str().map(|v| v.to_string())))
        .unwrap_or_else(|| "light".to_string())
}

/// 解析 "#rrggbb" 为原生 Color，失败返回 None
fn parse_hex(hex: &str) -> Option<tauri::window::Color> {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        let r = u8::from_str_radix(&h[0..2], 16).ok()?;
        let g = u8::from_str_radix(&h[2..4], 16).ok()?;
        let b = u8::from_str_radix(&h[4..6], 16).ok()?;
        Some(tauri::window::Color(r, g, b, 255))
    } else {
        None
    }
}

/// 读前端已存的窗口底色（无记录/坏值兜底白色）
pub fn stored_window_bg(app: &AppHandle) -> tauri::window::Color {
    app.store("config.json")
        .ok()
        .and_then(|s| s.get("theme_bg").and_then(|v| v.as_str().map(|v| v.to_string())))
        .and_then(|hex| parse_hex(&hex))
        .unwrap_or(tauri::window::Color(0xff, 0xff, 0xff, 255))
}

/// 全部窗口底色涂成指定颜色（防加载时闪白/闪黑）
pub fn apply_window_bg(app: &AppHandle, color: tauri::window::Color) {
    for win in app.webview_windows().values() {
        let _ = win.set_background_color(Some(color));
    }
}

#[tauri::command]
pub async fn get_theme(app: AppHandle) -> Result<String, String> {
    Ok(stored_theme(&app))
}

#[tauri::command]
pub async fn set_theme(app: AppHandle, theme: String, window_bg: String) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set("theme", serde_json::Value::String(theme.clone()));
    store.set("theme_bg", serde_json::Value::String(window_bg.clone()));
    store.save().map_err(|e| e.to_string())?;
    // 窗口底色跟随 + 通知所有窗口
    apply_window_bg(&app, parse_hex(&window_bg).unwrap_or(tauri::window::Color(0xff, 0xff, 0xff, 255)));
    app.emit("theme-updated", &theme).map_err(|e| e.to_string())
}