# Sensend 平台适配器开发规范

本文档定义了 Sensend 平台适配器的开发标准，确保所有适配器风格统一、易于维护。

---

## 一、架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                        前端 (Vue)                            │
│  ConfigWindow.vue → 配置表单 → PlatformInstance             │
│  App.vue → 发送内容 → tiptap JSON                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Tauri Commands                          │
│  platform.rs → open_config_window / test / publish          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Adapters (Rust)                         │
│  mod.rs → PlatformAdapter trait + 字段配置                   │
│  notion.rs / flowus.rs / lark.rs / local.rs                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 二、前端配置字段规范

### 2.1 字段定义结构

在 `src-tauri/src/adapters/mod.rs` 的 `get_platform_types()` 中定义：

```rust
PlatformTypeInfo {
    key: "platform_key".into(),        // 平台唯一标识
    name: "显示名称".into(),             // 前端展示名称
    color: "#2CAF68".into(),            // 主题色
    fields: vec![                       // 配置字段列表
        ConfigField {
            key: "token".into(),        // 字段键名
            label: "标签".into(),        // 输入框标签
            hint: "提示文字".into(),      // 输入框下方提示
            secret: true,               // 是否为密钥（显示/隐藏切换）
            hidden: false,              // 是否隐藏整个字段
            browse: false,              // 是否显示"浏览"按钮
            default_value: None,        // 默认值
            optional: false,            // 是否可选
        },
    ],
}
```

### 2.2 字段属性说明

| 属性 | 类型 | 说明 |
|------|------|------|
| `key` | String | 存储键名，对应 `PlatformInstance` 字段 |
| `label` | String | 输入框标签文字 |
| `hint` | String | 输入框下方的提示文字（可为空） |
| `secret` | bool | true → 密码输入框，带显示/隐藏切换 |
| `hidden` | bool | true → 前端不显示该字段（如 local 的 token） |
| `browse` | bool | true → 显示"浏览"按钮（仅用于本地文件夹） |
| `default_value` | Option<String> | 用户未填时自动填充的值 |
| `optional` | bool | true → 该字段可以不填 |

### 2.3 特殊字段约定

| 字段键名 | 用途 | 平台示例 |
|---------|------|---------|
| `token` | 主凭证 | Notion Token、FlowUs 授权码、飞书 App ID |
| `token2` | 第二凭证 | 飞书 App Secret |
| `target_id` | 目标标识 | 页面链接、文档链接、文件夹路径 |

### 2.4 前端处理规则

1. **URL 解析**：前端不做任何解析，用户粘贴原始链接，后端 `resolve_target_id()` 统一处理
2. **写入模式**：`publish_mode` 字段控制
   - `page`：创建子页面（默认）
   - `block`：追加到现有页面
3. **字段隐藏**：`hidden: true` 的字段不显示，但会设置 `default_value`

---

## 三、后端适配器规范

### 3.1 文件结构

```
adapters/
├── mod.rs           # 模块入口、公共接口、字段配置
├── notion.rs        # Notion 适配器
├── flowus.rs        # FlowUs 适配器
├── lark.rs          # 飞书适配器
├── local.rs         # 本地文件适配器
└── markdown.rs      # Markdown 导出工具
```

### 3.2 必需常量

```rust
const API_BASE: &str = "https://api.example.com/v1";
const API_VERSION: &str = "2024-01-01";  // 如果平台需要版本号
```

### 3.3 版本标记

在 `new()` 方法中输出版本标记：

```rust
impl XxxAdapter {
    pub fn new() -> Self {
        log::info!("[Xxx] REST API 版本 - YYYY-MM-DD");
        Self
    }
}
```

### 3.4 统一请求方法

```rust
async fn request(
    &self,
    method: &str,
    path: &str,
    token: &str,
    body: Option<Value>,
) -> Result<Value, String> {
    let client = super::http_client();  // 复用全局客户端
    let url = format!("{}{}", API_BASE, path);

    log::debug!("[Xxx] {} {}", method, url);

    let mut req = match method {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PATCH" => client.patch(&url),
        "DELETE" => client.delete(&url),
        _ => return Err(format!("不支持的 HTTP 方法: {}", method)),
    };

    req = req.header("Authorization", format!("Bearer {}", token));

    if let Some(b) = body {
        req = req.json(&b);
        log::debug!("[Xxx] 请求体: {}", serde_json::to_string(&b).unwrap_or_default());
    }

    let res = req.send().await.map_err(|e| format!("请求失败: {}", e))?;

    let status = res.status();
    let body: Value = res.json().await.unwrap_or_default();

    log::debug!("[Xxx] 响应状态: {}", status);

    if !status.is_success() {
        let msg = body.get("message").and_then(|m| m.as_str())
            .unwrap_or(&format!("HTTP 错误 ({})", status));
        return Err(format!("Xxx API: {}", msg));
    }

    Ok(body)
}
```

**要点**：
- 使用 `super::http_client()` 复用连接池
- 统一的日志输出格式
- 统一的错误处理

---

## 四、PlatformAdapter Trait

### 4.1 接口定义

```rust
#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    /// 创建子页面
    async fn publish(&self, content: &Value, instance: &PlatformInstance) -> Result<PublishResult, String>;

    /// 测试连接
    async fn test_connection(&self, instance: &PlatformInstance) -> Result<(), String>;

    /// 探测目标类型（可选，默认返回 "page"）
    async fn probe_type(&self, _instance: &PlatformInstance) -> Result<String, String> {
        Ok("page".to_string())
    }

    /// 追加内容（可选，默认返回不支持）
    async fn append_blocks(&self, _content: &Value, _instance: &PlatformInstance) -> Result<PublishResult, String> {
        Err("该平台不支持追加写入".into())
    }
}
```

### 4.2 能力矩阵

| 平台 | publish | append_blocks | probe_type | 特殊说明 |
|------|---------|---------------|------------|---------|
| Notion | ✅ | ✅ | ✅ | 支持数据库/页面 |
| FlowUs | ✅ | ✅ | ✅ | 支持多维表/页面 |
| 飞书 | ❌ | ✅ | ❌ | 仅支持追加文档 |
| 本地 | ✅ | ❌ | ❌ | 创建 .md 文件 |

### 4.3 返回值规范

```rust
PublishResult {
    success: bool,           // 操作是否成功
    message: String,         // 结果消息
    url: Option<String>,     // 创建/追加的页面链接
}
```

---

## 五、格式转换规范

### 5.1 TipTap 节点类型

适配器应尽可能覆盖以下 TipTap 节点类型。各适配器的实际支持情况见 [4.2 能力矩阵](#42-能力矩阵)，真源是各适配器源码中的 `tiptap_to_xxx` 转换函数。

| TipTap 类型 | 说明 |
|------------|------|
| `paragraph` | 段落 |
| `heading` | 标题（level 1-3） |
| `bulletList` | 无序列表 |
| `orderedList` | 有序列表 |
| `blockquote` | 引用块 |
| `codeBlock` | 代码块 |
| `horizontalRule` | 分割线 |
| `taskList` / `taskItem` | 待办清单 |
| `table` | 表格 |

### 5.2 文本样式（marks）

| TipTap mark | 说明 |
|------------|------|
| `bold` | 粗体 |
| `italic` | 斜体 |
| `strike` | 删除线 |
| `code` | 行内代码 |
| `link` | 链接 |
| `underline` | 下划线（Markdown 导出走 `<u>` HTML 兜底） |

### 5.3 飞书特殊格式

飞书 block_type 使用数字枚举：

| 类型 | block_type |
|------|-----------|
| 段落 | 2 |
| 标题1 | 3 |
| 标题2 | 4 |
| 标题3 | 5 |
| 无序列表 | 12 |
| 有序列表 | 13 |
| 代码块 | 14 |
| 引用块 | 15 |
| 分割线 | 22 |

---

## 六、日志规范

### 6.1 日志级别

| 级别 | 使用场景 |
|------|---------|
| `info` | 版本标记、关键操作成功 |
| `debug` | 请求/响应详情、中间状态 |
| `warn` | 可恢复的失败、降级处理 |
| `error` | 不可恢复的错误 |

### 6.2 日志格式

```rust
log::info!("[Xxx] 操作描述");
log::debug!("[Xxx] 详细信息: {}", value);
log::warn!("[Xxx] 警告信息: {}", reason);
```

**格式**：`[平台名] 消息内容`

---

## 七、错误处理

### 7.1 错误消息格式

```rust
Err(format!("Xxx API: {}", api_error_message))
```

后端返回的原始错误消息直接透传到前端，前端仅做网络断开兜底判断（`usePlatform.ts` 的 `friendlyError`），其余错误消息原样展示给用户。

> 适配器无需实现 `friendly_error` 函数——错误消息中应包含足够的上下文（如 HTTP 状态码、API 返回的 message 字段）供用户理解。

---

## 八、平台特殊限制

### 8.1 飞书

| 限制 | 说明 |
|------|------|
| 认证方式 | 需要 App ID + App Secret 获取 tenant_access_token |
| 文件夹访问 | tenant_access_token 只能访问应用创建的文件夹 |
| 多维表 | 不适合存储长文本，仅写入标题 |
| URL 解析 | 需处理 wiki 空间 URL，调用 wiki API 获取 obj_token |

**推荐功能**：仅支持追加文档

### 8.2 Notion

| 限制 | 说明 |
|------|------|
| Token 类型 | Integration Token |
| 数据库 | 支持内嵌数据库，需提取 schema |
| 分批追加 | 每批最多 100 个 block |

### 8.3 FlowUs

| 限制 | 说明 |
|------|------|
| Token 类型 | MCP 授权码 |
| 多维表 | 支持内嵌多维表 |
| 分批追加 | 每批最多 100 个 block |

---

## 九、开发检查清单

新增或修改适配器时，检查以下项目：

### 9.1 后端

- [ ] 常量定义（API_BASE、API_VERSION）
- [ ] 版本标记日志
- [ ] 统一的 request() 方法
- [ ] 使用 `super::http_client()` 复用连接
- [ ] 目标类型判断（如支持数据库）
- [ ] 所有字段都被使用（无编译警告）
- [ ] 调试日志（请求/响应）
- [ ] 错误处理友好化
- [ ] 分批追加（每批 ≤ 100）
- [ ] URL 解析函数（如需要）

### 9.2 前端配置

- [ ] 在 `get_platform_types()` 中添加字段配置
- [ ] `hint` 提示文字清晰易懂
- [ ] 密钥字段设置 `secret: true`
- [ ] 测试连接提示可见（窗口高度足够）

### 9.3 测试

- [ ] 测试连接功能正常
- [ ] 创建页面功能正常
- [ ] 追加内容功能正常（如支持）
- [ ] 中文内容正确发送
- [ ] 错误提示友好

---

## 十、示例参考

| 平台 | 文件 | 特点 |
|------|------|------|
| FlowUs | `flowus.rs` | 标准实现，支持页面/多维表 |
| Notion | `notion.rs` | 标准实现，含数据库 schema 提取 |
| 飞书 | `lark.rs` | 特殊实现，仅追加文档 |
| 本地 | `local.rs` | 简单实现，创建 .md 文件 |

**新增适配器时，以 FlowUs 或 Notion 为模板。飞书因 API 差异较大，仅供参考。**

---

## 十一、常见问题

### Q1: 如何添加新平台？

1. 在 `adapters/` 下创建 `xxx.rs`
2. 实现 `PlatformAdapter` trait
3. 在 `mod.rs` 中添加模块导出和字段配置
4. 在 `get_adapter()` 中添加匹配分支
5. 在 `resolve_target_id()` 中添加 URL 解析（如需要）

### Q2: 如何处理平台特有的认证方式？

在适配器内部处理，对外统一使用 `token` 和 `token2` 字段。例如飞书：
- `token` = App ID
- `token2` = App Secret
- 适配器内部用这两个字段获取 tenant_access_token

### Q3: 如何支持数据库/多维表？

1. 实现 `probe_type()` 返回目标类型
2. 在 `resolve_target()` 中判断目标类型
3. 数据库写入时提取 schema，构造正确的字段值

### Q4: 分批追加的批次大小？

Notion/FlowUs：每批最多 100 个 block
飞书：每批最多 50 个 block

---

## 十二、版本历史

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-08-18 | 1.2 | 删除"必须支持"清单，改为指向能力矩阵；移除 Rust 侧 friendly_error 规范；补充 taskList/table/underline 节点说明 |
| 2026-04-29 | 1.1 | 添加前端配置字段规范、能力矩阵、平台特殊限制 |
| 2026-04-28 | 1.0 | 初始版本 |
