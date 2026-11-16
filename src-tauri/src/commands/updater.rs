/// 按需代理检测：仅当更新检查直连失败需要走 GitHub 兜底时才运行，零启动开销。
/// 检测优先级：HTTPS_PROXY 环境变量 → Windows 系统代理 → 本地常见代理端口。
/// 移植自 solo（03-solo/src-tauri/src/updater.rs，生产验证过的同款实现）。
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

/// 串行化检测：set_var/remove_var 非线程安全（Windows 并发读写环境变量是 UB），
/// 且启动静默检查与手动检查可能并发触发
static DETECT_LOCK: Mutex<()> = Mutex::const_new(());

/// [m-13 修复] 记住设置前的 HTTPS_PROXY 原值，未检测到代理时恢复原值而非直接抹掉，
/// 避免破坏用户系统里本来就配好的代理。
static ORIGINAL_HTTPS_PROXY: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();

/// 检测 GitHub 更新所需的代理，返回代理 URL。
/// 未检测到时返回 None（表示直连）。
pub async fn detect_github_proxy() -> Option<String> {
    // 1. 环境变量检查（即时，优先级最高）
    if let Some(proxy) = check_env_proxy() {
        return Some(proxy);
    }

    // 2. Windows 系统代理（即时，读注册表）
    #[cfg(target_os = "windows")]
    if let Some(proxy) = check_system_proxy() {
        return Some(proxy);
    }

    // 3. 本地代理端口扫描（仅前两步失败时）
    //    只扫 Clash(7890) 和 V2Ray(10809)，并真发 CONNECT 探针确认对端是代理
    //    （最坏 2 端口 × (300ms 连接 + 300ms 写 + 800ms 读)，命中即返回）
    check_local_ports().await
}

/// 检查环境变量 HTTPS_PROXY / HTTP_PROXY
fn check_env_proxy() -> Option<String> {
    for var in &["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"] {
        if let Ok(val) = std::env::var(var) {
            let val = val.trim().to_string();
            if !val.is_empty() && !val.starts_with("socks") {
                return Some(val);
            }
        }
    }
    None
}

/// 读取 Windows 系统代理设置（HKCU\Internet Settings）
#[cfg(target_os = "windows")]
fn check_system_proxy() -> Option<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings")
        .ok()?;

    let enabled: u32 = settings.get_value("ProxyEnable").ok()?;
    if enabled == 0 {
        return None;
    }

    let server: String = settings.get_value("ProxyServer").ok()?;
    if server.is_empty() {
        return None;
    }

    // 格式: "host:port" 或 "http=host:port;https=host:port;..."
    for entry in server.split(';') {
        let addr = entry
            .strip_prefix("https=")
            .or_else(|| entry.strip_prefix("http="))
            .unwrap_or(entry)
            .trim();
        if !addr.is_empty() {
            return Some(format!("http://{}", addr));
        }
    }
    None
}

/// 扫描本地代理端口：仅 Clash(7890) 和 V2Ray(10809)
async fn check_local_ports() -> Option<String> {
    for &port in &[7890u16, 10809] {
        if probe_http_proxy(port).await {
            return Some(format!("http://127.0.0.1:{}", port));
        }
    }
    None
}

/// [m-13 补刀] "端口能连上"不等于"它是个代理"——7890 完全可能被别的程序占着。
/// 真发一次 CONNECT 探针，只认 `200` 响应（代理隧道已建立）。
/// - 非 HTTP 服务 / 拒绝 CONNECT / 无响应 → 判否
/// - `407`（要求代理认证）也判否：reqwest 不会带代理凭据，认下来等于注入一个必然失败的代理
/// 把 HTTPS_PROXY 指到不可用的端口，比直连更糟，所以宁可漏也不误判。
async fn probe_http_proxy(port: u16) -> bool {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let Ok(Ok(mut stream)) = tokio::time::timeout(
        Duration::from_millis(300),
        TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    else {
        return false;
    };

    let probe = b"CONNECT github.com:443 HTTP/1.1\r\nHost: github.com:443\r\n\r\n";
    let sent = tokio::time::timeout(Duration::from_millis(300), stream.write_all(probe)).await;
    if !matches!(sent, Ok(Ok(()))) {
        return false;
    }

    // 响应头可能分多个包到达：读到空行（\r\n\r\n）或超时为止，别只读一次首包
    let mut buf: Vec<u8> = Vec::with_capacity(128);
    let deadline = tokio::time::Instant::now() + Duration::from_millis(800);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let mut chunk = [0u8; 128];
        match tokio::time::timeout(remaining, stream.read(&mut chunk)).await {
            Ok(Ok(0)) => break, // 对端关闭
            Ok(Ok(n)) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() >= 1024 {
                    break;
                }
            }
            _ => break, // 超时或读错误
        }
    }

    let head = String::from_utf8_lossy(&buf);
    head.starts_with("HTTP/") && head.contains(" 200")
}

/// 手动检查更新时触发的代理检测。检测到代理时设 HTTPS_PROXY，否则清空。
/// 前端调用此命令后再调 @tauri-apps/plugin-updater 的 check()——
/// 插件内部的 reqwest 客户端会读取该环境变量走代理。
/// [m-13 修复] 更新流程结束后把 HTTPS_PROXY 还原成用户原本的值。
/// 前端在「没发现新版本」「下载安装结束（成功/失败）」时调用。
/// 幂等：从未检测过代理（OnceLock 未初始化）时什么都不做。
#[tauri::command]
pub async fn restore_update_proxy() {
    let _guard = DETECT_LOCK.lock().await;
    let Some(original) = ORIGINAL_HTTPS_PROXY.get() else {
        return;
    };
    match original {
        Some(v) => std::env::set_var("HTTPS_PROXY", v),
        None => std::env::remove_var("HTTPS_PROXY"),
    }
}

#[tauri::command]
pub async fn detect_proxy_for_update() -> Result<String, String> {
    let _guard = DETECT_LOCK.lock().await;
    // [m-13 修复] 首次进入时记下原始值，后续只在本进程内改动，unset 时恢复原值
    let original = ORIGINAL_HTTPS_PROXY.get_or_init(|| std::env::var("HTTPS_PROXY").ok());
    match detect_github_proxy().await {
        Some(proxy) => {
            std::env::set_var("HTTPS_PROXY", &proxy);
            Ok(proxy)
        }
        None => {
            // 恢复原值：原本有代理就还原，原本没有才移除
            match original {
                Some(v) => std::env::set_var("HTTPS_PROXY", v),
                None => std::env::remove_var("HTTPS_PROXY"),
            }
            Err("未检测到可用代理，将使用直连。若直连不上请检查网络配置。".into())
        }
    }
}
