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
        // [ZC-01 配套] 复用窗口不会重新挂载，通知前端重拉数据（否则隐藏期间别处的改动看不到）
        let _ = app.emit_to("config", "config-window-shown", ());
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
        // [ZC-02] 先隐藏建窗 + 建窗时即上主题底色；由前端挂载完成后主动 show，
        // 消除「白窗 → 上色」和「窗口可见但内容未就绪」两段闪
        .visible(false)
        .background_color(stored_window_bg(&app))
        .build()
        .map_err(|e| e.to_string())?;

    // [ZC-02] 兜底：前端 500ms 内未 show（挂载报错等），后端强制显示，窗口绝不"消失"
    let bg_app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(win) = bg_app.get_webview_window("config") {
            if !win.is_visible().unwrap_or(true) {
                let _ = win.show();
            }
        }
    });

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

/// [M-02] 明文底稿名单。两个名字同属一处泄露面，新增备份名务必同步这里：
/// - `config.json.bak`：当前迁移逻辑写入的回滚底稿；
/// - `config.json.pre-encrypt-backup`：实测用户数据目录里存在、内容为明文，但全仓代码零引用
///   （来源已不可考，仓库历史也查不到）→ 没人会清它，只能靠这里兜住。
const PLAINTEXT_BACKUPS: [&str; 2] = ["config.json.bak", "config.json.pre-encrypt-backup"];

/// 删除明文备份。**只允许在凭证迁移已确认完成时调用。**
/// 拆成独立函数是为了让"启动清理"与"迁移后当场删"走同一条路径——
/// 两处各写一遍必然漂移（曾经就只删 .bak、留下孤儿文件多活一个启动周期）。
fn remove_plaintext_backups(dir: &std::path::Path) {
    for name in PLAINTEXT_BACKUPS {
        let path = dir.join(name);
        if !path.exists() {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => log::info!("凭证迁移已确认完成，清理明文备份 {}", path.display()),
            // exists() 与 remove 之间被外部（杀软/清理工具）删掉：不算失败，别刷假 warn
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            // 其余删不掉（占用/只读/ACL）必须留痕：否则用户以为清了，实际明文还躺在盘上
            Err(e) => log::warn!("清理明文备份失败 {}: {}", path.display(), e),
        }
    }
}

/// [M-02 修复] 清理已无必要的明文备份。
///
/// [补刀 1] 判定必须直接读文件 + 独立解析，不能用 store.get 判断：
/// tauri-plugin-store 加载时是 `let _ = store_inner.load()`（store.rs:222），
/// 文件半截/损坏时解析错误被吞掉、缓存留空，store.get 全返回 None
/// → 会被误判为"已无明文" → 删掉唯一可回滚的备份。
/// 而配置非原子写（fs::write 截断写）恰好能产生半截文件——最需要备份的场景反而丢备份。
///
/// [补刀 2] 判据必须与迁移后的回读校验**完全一致**（config_fully_encrypted = 可解析 +
/// 至少有一个凭证 + 每个凭证既能解密），否则迁移时因"解不开"而刻意保留的备份，
/// 会被这里按"有 enc1: 前缀"直接删掉，白留。跨机器/跨 Windows 账户拷来的配置正是
/// 这种情况：JSON 合法、全前缀，但 DPAPI 解不开——此时明文备份是唯一可恢复副本。
fn cleanup_stale_backup(app: &AppHandle) {
    let dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return,
    };
    if !PLAINTEXT_BACKUPS.iter().any(|name| dir.join(name).exists()) {
        return;
    }
    if config_fully_encrypted(app) {
        remove_plaintext_backups(&dir);
    } else {
        log::warn!(
            "config.json 未通过密文校验（损坏 / 跨机不可解 / 仍有明文 / 无凭证），保留明文备份 {:?}",
            PLAINTEXT_BACKUPS
        );
    }
}

/// [M-02 补刀] 回读校验：config.json 可解析，且文档里每个非空凭证都是密文、且真能解开。
/// 用于判断"密文是否已确实可用"——只有确认可用，才敢丢掉唯一的明文底稿。
///
/// [补刀 3] **零凭证不算通过**。原实现对空文档返回 true（`all()` 对空集合恒真），
/// 于是 store 读空 / 用户重置 / 重装残留导致 config.json 变成"合法但无凭证"时，
/// 就会把两份明文备份一起删掉——而那恰恰是唯一需要备份的灾难态。
/// 因此必须要求"至少存在一个非空的、能解开的凭证"。
fn config_fully_encrypted(app: &AppHandle) -> bool {
    let Ok(dir) = app.path().app_data_dir() else { return false };
    let Ok(bytes) = std::fs::read(dir.join("config.json")) else { return false };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return false };
    credentials_all_encrypted(&value)
}

/// 纯判定（拆出来是为了可测）：文档里至少存在一个非空凭证，且每个非空凭证都能解密。
/// 递归任意层级的 `token` / `token2`（platform_instances 数组、token_memory 映射都在其中）。
fn credentials_all_encrypted(value: &serde_json::Value) -> bool {
    fn walk(value: &serde_json::Value, seen: &mut usize, ok: &mut bool) {
        match value {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    if k == "token" || k == "token2" {
                        // 凭证字段不是字符串 = 形态未知，不能当作"合格"放行（这是删备份的闸门）
                        let Some(s) = v.as_str() else {
                            *ok = false;
                            continue;
                        };
                        if s.is_empty() {
                            continue; // 空串无需加密，也不计入"有凭证"
                        }
                        *seen += 1;
                        // 非空必须是「密文前缀 + 能成功解密」
                        if !(security::is_encrypted(s) && security::unprotect(s).is_ok()) {
                            *ok = false;
                        }
                    } else {
                        walk(v, seen, ok);
                    }
                }
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| walk(v, seen, ok)),
            _ => {}
        }
    }

    let (mut seen, mut ok) = (0usize, true);
    walk(value, &mut seen, &mut ok);
    seen > 0 && ok
}

pub fn migrate_credentials_at_startup(app: &AppHandle) {
    // [M-02 修复] 先清理历史明文备份，再做本次迁移检查
    cleanup_stale_backup(app);
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
            // [M-02 补刀] 写盘后立刻回读校验：文件能解析 + 至少一个凭证 + 每个凭证都可解密，
            // 才说明迁移真的落地了。校验通过就当场删掉**全部**明文备份——不给明文令牌留驻留窗口；
            // 校验不过则全部保留，等下个启动周期由 cleanup_stale_backup 再试 / 供人工回滚。
            if config_fully_encrypted(app) {
                if let Ok(dir) = app.path().app_data_dir() {
                    remove_plaintext_backups(&dir);
                }
            } else {
                log::warn!("密文回读校验未通过，保留明文备份以便回滚");
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 零凭证（空文档 / 只有主题）绝不能算"已全密文"：
    /// 它是 config.json 被读空或重置后的形态，恰恰最需要保留明文备份。
    /// 曾经的实现用 `all()` 对空集合恒真 → 会在灾难态下把备份一起删掉。
    #[test]
    fn empty_config_is_not_encrypted() {
        assert!(!credentials_all_encrypted(&json!({})));
        assert!(!credentials_all_encrypted(&json!({ "theme": "light" })));
        assert!(!credentials_all_encrypted(&json!({ "platform_instances": [] })));
    }

    /// 全空串凭证 = 没有凭证，同样不许当"已加密"放行
    #[test]
    fn all_blank_tokens_is_not_encrypted() {
        let v = json!({ "platform_instances": [{ "token": "", "token2": "" }] });
        assert!(!credentials_all_encrypted(&v));
    }

    /// 还有明文凭证 → 不通过（迁移尚未完成）
    #[test]
    fn leftover_plaintext_blocks() {
        let v = json!({ "platform_instances": [{ "token": "ntn_plain_token" }] });
        assert!(!credentials_all_encrypted(&v));
    }

    /// 凭证字段形态异常（数字/对象）→ 不通过，不能当作合格。
    /// 必须让"同文档里还有一个有效凭证"成立，否则用例会因为 seen=0 而恒 false，
    /// 测不出「非字符串分支有没有 correctly 置 ok=false」这一性质。
    #[test]
    #[cfg(windows)]
    fn non_string_token_blocks() {
        let enc = security::protect("real-token").unwrap();
        assert!(!credentials_all_encrypted(&json!({ "token": 123, "token2": enc })));
        let enc2 = security::protect("real-token").unwrap();
        assert!(!credentials_all_encrypted(&json!({ "token": { "nested": "x" }, "token2": enc2 })));
    }

    /// 嵌套的 token_memory 里混进明文也要被发现。
    /// 顶层必须是**能解开的**密文，否则 ok 早就 false，测不出递归遍历是否漏了 token_memory。
    #[test]
    #[cfg(windows)]
    fn nested_plaintext_blocks() {
        let v = json!({
            "platform_instances": [{ "token": security::protect("ok-token").unwrap() }],
            "token_memory": { "id1": { "token": "still-plaintext" } }
        });
        assert!(!credentials_all_encrypted(&v));
    }

    /// 密文全部可解 → 通过（依赖 DPAPI，仅 Windows 可跑）；空串不影响判定
    #[test]
    #[cfg(windows)]
    fn fully_encrypted_passes() {
        let enc = security::protect("real-token").unwrap();
        let v = json!({
            "platform_instances": [{ "token": enc, "token2": "" }],
            "token_memory": { "id1": { "token": security::protect("t2").unwrap() } }
        });
        assert!(credentials_all_encrypted(&v));
    }

    /// 跨机/换账户拷来的密文（前缀对但 DPAPI 解不开）→ 不通过，必须保留明文备份
    #[test]
    #[cfg(windows)]
    fn undecryptable_ciphertext_blocks() {
        let v = json!({ "platform_instances": [{ "token": "enc1:bm90LWEtcmVhbC1kcGFwaS1ibG9i" }] });
        assert!(!credentials_all_encrypted(&v));
    }
}