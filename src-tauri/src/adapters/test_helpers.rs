//! 黄金测试共享工具：加载 fixture、比较 golden 文件。
//! 仅 `#[cfg(test)]` 编译。

use std::path::PathBuf;
use serde_json::Value;

const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");

/// 加载 fixture JSON
pub fn load_fixture(name: &str) -> Value {
    let path = PathBuf::from(FIXTURES_DIR).join(format!("{}.json", name));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("无法读取 fixture {}: {}", name, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("无法解析 fixture {}: {}", name, e))
}

/// 比较输出与 golden 文件。
///
/// 文件不存在时**不再自动写入**：自动重建会把"当前输出"直接当成正确答案，
/// 一旦解析逻辑改错，golden 会跟着一起错，门禁从"发现变化"退化成"永远通过"。
/// 需要生成新基准时，显式设环境变量 `SENSEND_UPDATE_GOLDEN=1` 跑一次，并人工复查产物。
pub fn assert_or_update_golden(adapter: &str, name: &str, extension: &str, output: &str) {
    let path = PathBuf::from(FIXTURES_DIR)
        .join("expected_current")
        .join(adapter)
        .join(format!("{}.{}", name, extension));

    // 确保父目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let update_allowed = std::env::var("SENSEND_UPDATE_GOLDEN").as_deref() == Ok("1");

    if path.exists() {
        let expected = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("无法读取 golden 文件 {}: {}", path.display(), e));
        if expected.trim() == output.trim() {
            return;
        }
        // 显式要求更新时才覆盖（用于「有意改变输出格式」的重构），平时一律报差异
        if update_allowed {
            std::fs::write(&path, output)
                .unwrap_or_else(|e| panic!("无法写入 golden 文件 {}: {}", path.display(), e));
            panic!(
                "已更新 golden 文件: {}。请人工确认新内容确实是对的，再跑一次测试。",
                path.display()
            );
        }
        assert_eq!(
            expected.trim(),
            output.trim(),
            "[{}] {} 输出与 golden 文件不一致",
            adapter,
            name
        );
        return;
    }

    if update_allowed {
        std::fs::write(&path, output)
            .unwrap_or_else(|e| panic!("无法写入 golden 文件 {}: {}", path.display(), e));
        panic!(
            "已生成 golden 文件: {}。请人工复查内容是否正确，然后去掉 SENSEND_UPDATE_GOLDEN 重新运行测试。",
            path.display()
        );
    }
    panic!(
        "golden 文件缺失: {}。若是新增用例，请设 SENSEND_UPDATE_GOLDEN=1 运行一次以生成基准（生成后必须人工复查，不要盲信自动产物）。",
        path.display()
    );
}

/// 将 JSON 输出格式化为可比较的字符串（排序键，稳定序列化）
pub fn format_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}