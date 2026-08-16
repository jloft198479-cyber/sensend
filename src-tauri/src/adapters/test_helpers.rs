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
/// 文件不存在则写入（首次运行），存在则断言相等。
pub fn assert_or_update_golden(adapter: &str, name: &str, extension: &str, output: &str) {
    let path = PathBuf::from(FIXTURES_DIR)
        .join("expected_current")
        .join(adapter)
        .join(format!("{}.{}", name, extension));

    // 确保父目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    if path.exists() {
        let expected = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("无法读取 golden 文件 {}: {}", path.display(), e));
        assert_eq!(
            expected.trim(),
            output.trim(),
            "[{}] {} 输出与 golden 文件不一致",
            adapter,
            name
        );
    } else {
        std::fs::write(&path, output)
            .unwrap_or_else(|e| panic!("无法写入 golden 文件 {}: {}", path.display(), e));
        panic!(
            "Golden 文件不存在，已写入: {}. 请重新运行测试以验证。",
            path.display()
        );
    }
}

/// 将 JSON 输出格式化为可比较的字符串（排序键，稳定序列化）
pub fn format_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}