//! 隐藏即瘦身（内存优化·甲）
//!
//! 窗口隐藏 10 秒后，对 sensend.exe 及其全部 WebView2 子进程调用
//! SetProcessWorkingSetSizeEx(-1, -1) 修剪工作集，把闲置物理内存还给系统。
//! 唤起（窗口重新显示）会使 pending 的修剪失效，保证唤起首下不被拖慢。
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
        let mut trimmed = 0;
        for &pid in &tree {
            let handle = OpenProcess(PROCESS_SET_QUOTA, 0, pid);
            if handle != 0 && handle != INVALID_HANDLE {
                // (usize::MAX, usize::MAX) 即 (-1, -1)：把工作集修剪到最小
                if SetProcessWorkingSetSizeEx(handle, usize::MAX, usize::MAX, 0) != 0 {
                    trimmed += 1;
                }
                CloseHandle(handle);
            }
        }
        log::debug!(
            "memory_trim: 修剪 {}/{} 个进程的工作集",
            trimmed,
            tree.len()
        );
    }
}
