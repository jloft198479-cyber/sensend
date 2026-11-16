//! 隐藏即瘦身（内存优化·甲）
//!
//! 窗口隐藏 10 秒后，对 sensend.exe 及其全部 WebView2 子进程调用
//! SetProcessWorkingSetSizeEx(-1, -1) 修剪工作集，把闲置物理内存还给系统。
//! 唤起（窗口重新显示）会使 pending 的修剪失效，保证唤起首下不被拖慢。
//!
//! [n-05] 修剪前会核对进程映像名（本程序 / msedgewebview2.exe），
//! 防止「快照之后 pid 被系统复用给别的程序」时误修剪无关进程。
//!
//! 零新增依赖：直接声明 kernel32 FFI。

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// 可见性变化票号：hide/show 都递增。
/// 延迟线程只在「10 秒内无任何可见性变化」时执行修剪，
/// 避免用户隐藏后马上唤起、或连续 toggle 造成误修剪。
static VISIBILITY_TICKET: AtomicU64 = AtomicU64::new(0);

/// 延迟时长：隐藏后等 10 秒再修剪，给「隐藏后马上又唤起」留缓冲
const TRIM_DELAY: Duration = Duration::from_secs(10);

const TH32CS_SNAPPROCESS: u32 = 0x2;
const PROCESS_SET_QUOTA: u32 = 0x0100;
/// [n-05] 读进程映像名所需的最小权限（不要求 PROCESS_VM_READ）
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const INVALID_HANDLE: isize = -1;

#[repr(C)]
#[allow(non_snake_case)]
struct PROCESSENTRY32W {
    dwSize: u32,
    cntUsage: u32,
    th32ProcessID: u32,
    th32DefaultHeapID: usize,
    th32ModuleID: u32,
    cntThreads: u32,
    th32ParentProcessID: u32,
    pcPriClassBase: i32,
    dwFlags: u32,
    szExeFile: [u16; 260],
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> isize;
    fn Process32FirstW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
    fn Process32NextW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
    fn CloseHandle(hObject: isize) -> i32;
    fn GetCurrentProcessId() -> u32;
    fn SetProcessWorkingSetSizeEx(hProcess: isize, dwMin: usize, dwMax: usize, flags: u32) -> i32;
    fn QueryFullProcessImageNameW(
        hProcess: isize,
        dwFlags: u32,
        lpExeName: *mut u16,
        lpdwSize: *mut u32,
    ) -> i32;
}

/// [n-05] 取进程映像文件名（小写）。拿不到就返回 None（调用方按「不认识」处理，跳过不修剪）。
///
/// 为什么必须校验：快照只记 pid/ppid，而从「收集快照」到「OpenProcess」之间隔着几毫秒，
/// 目标进程可能已经退出、pid 被系统分给别的程序。旧代码直接按 pid 打开并修剪工作集，
/// 那个「别的程序」就会莫名被抽走物理内存（不至于损坏数据，但完全不该发生）。
/// 名字对不上就跳过，是这条链路唯一便宜的兜底。
unsafe fn process_image_name(handle: isize) -> Option<String> {
    let mut buf = vec![0u16; 1024];
    let mut size = buf.len() as u32;
    if QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) == 0 {
        return None;
    }
    let path = String::from_utf16_lossy(&buf[..size as usize]);
    let name = path.rsplit(|c| c == '\\' || c == '/').next().unwrap_or(&path);
    Some(name.to_ascii_lowercase())
}

/// 允许修剪的进程映像名：本程序自身 + WebView2 各子进程。
/// 取自身名而不是写死 "sensend.exe"：改名/带后缀的调试产物也能盖住。
fn trim_allowlist() -> HashSet<String> {
    let mut set = HashSet::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(name) = exe.file_name() {
            set.insert(name.to_string_lossy().to_ascii_lowercase());
        }
    }
    set.insert("msedgewebview2.exe".to_string());
    set
}

/// 窗口隐藏时调用：10 秒后若无可见性变化，则修剪整个进程树的工作集
pub fn on_window_hidden() {
    let ticket = VISIBILITY_TICKET.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        std::thread::sleep(TRIM_DELAY);
        if VISIBILITY_TICKET.load(Ordering::SeqCst) == ticket {
            trim_process_tree();
        }
    });
}

/// 窗口显示时调用：使所有 pending 的修剪失效（不延迟唤起首下）
pub fn on_window_shown() {
    VISIBILITY_TICKET.fetch_add(1, Ordering::SeqCst);
}

/// 遍历系统进程表，找出当前进程及全部后代（WebView2 全家），
/// 逐一修剪工作集：把可丢弃的物理内存页换出到页面文件。
fn trim_process_tree() {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE {
            return;
        }

        // 收集全系统 (pid, ppid)
        let mut entries: Vec<(u32, u32)> = Vec::new();
        let mut pe = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            cntUsage: 0,
            th32ProcessID: 0,
            th32DefaultHeapID: 0,
            th32ModuleID: 0,
            cntThreads: 0,
            th32ParentProcessID: 0,
            pcPriClassBase: 0,
            dwFlags: 0,
            szExeFile: [0; 260],
        };
        if Process32FirstW(snap, &mut pe) != 0 {
            loop {
                entries.push((pe.th32ProcessID, pe.th32ParentProcessID));
                if Process32NextW(snap, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);

        // 从自身 pid 出发收集整棵进程树（防环）
        let root = GetCurrentProcessId();
        let mut tree: Vec<u32> = Vec::new();
        let mut visited: HashSet<u32> = HashSet::new();
        visited.insert(root);
        let mut frontier: Vec<u32> = vec![root];
        while let Some(pid) = frontier.pop() {
            tree.push(pid);
            for &(child, parent) in &entries {
                if parent == pid && visited.insert(child) {
                    frontier.push(child);
                }
            }
        }

        // 逐一修剪工作集
        let allow = trim_allowlist();
        let mut trimmed = 0;
        let mut skipped = 0;
        for &pid in &tree {
            let handle = OpenProcess(
                PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            );
            if handle != 0 && handle != INVALID_HANDLE {
                // 名字校验：pid 可能已被系统复用给别的程序（见 process_image_name 注释）
                match process_image_name(handle) {
                    Some(name) if allow.contains(&name) => {
                        // (usize::MAX, usize::MAX) 即 (-1, -1)：把工作集修剪到最小
                        if SetProcessWorkingSetSizeEx(handle, usize::MAX, usize::MAX, 0) != 0 {
                            trimmed += 1;
                        }
                    }
                    _ => skipped += 1,
                }
                CloseHandle(handle);
            }
        }
        // 日志级别用 info/warn（release 只在 Info 及以上落盘）：
        // 用 debug 的话「全部被跳过 = 修剪静默失效」在用户机器上一条痕迹都不留
        if skipped > 0 {
            log::warn!(
                "memory_trim: 修剪 {}/{} 个进程，跳过 {} 个非本程序进程（pid 可能已被复用）",
                trimmed,
                tree.len(),
                skipped
            );
        } else {
            log::info!("memory_trim: 修剪 {}/{} 个进程的工作集", trimmed, tree.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 白名单必须含「自身映像名」和 WebView2 进程名，否则修剪永远打不到目标
    #[test]
    fn allowlist_covers_self_and_webview() {
        let allow = trim_allowlist();
        assert!(allow.contains("msedgewebview2.exe"), "白名单缺 WebView2: {:?}", allow);
        let exe = std::env::current_exe().expect("拿不到自身路径");
        let name = exe.file_name().unwrap().to_string_lossy().to_ascii_lowercase();
        assert!(allow.contains(&name), "白名单缺自身映像名 {}: {:?}", name, allow);
    }

    /// [n-05] FFI 自检：能从 pid 读出映像名。
    /// 这条测试的价值在于——如果 QueryFullProcessImageNameW 的参数或常量写错，
    /// 名字永远返回 None，白名单会把**所有**进程都跳过（修剪静默失效，且日志看不出来）。
    #[test]
    fn image_name_resolves_for_current_process() {
        unsafe {
            let pid = GetCurrentProcessId();
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            assert!(handle != 0 && handle != INVALID_HANDLE, "OpenProcess 失败");
            let name = process_image_name(handle);
            CloseHandle(handle);
            let name = name.expect("QueryFullProcessImageNameW 未取到映像名（FFI 可能写错）");
            assert!(name.ends_with(".exe"), "意料之外的映像名: {}", name);
        }
    }
}
