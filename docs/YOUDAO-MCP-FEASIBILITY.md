# 有道云笔记 MCP 接入可行性评估

> 调研目标：确认有道云笔记 MCP 端点背后是否存在可直接调用的 REST API，评估在 Sensend 中接入有道云笔记的技术可行性与实现路径。
> 调研时间：2026-08-18
> 版本基线：Sensend v0.4.0

---

## 〇、结论先行

1. **MCP 端点背后没有 REST API**。新版 API Key（mopen.163.com 签发）只服务于 MCP SSE 协议，旧版 REST OpenAPI 已停止新增申请，两套鉴权体系互不兼容。
2. **接入有道云笔记只有一条官方路径**：在 Rust 后端实现 MCP SSE 客户端，连接 `https://open.mail.163.com/api/ynote/mcp/sse`。
3. **技术可行，架构有差异**。现有四个适配器（Notion/FlowUs/飞书/本地）都是无状态 REST 调用，MCP 适配器需要维护有状态的 SSE 长连接，这是最大的架构差异点。
4. **推荐方案**：用 `rust-mcp-sdk` crate（v1.0.0，100% 协议一致性测试通过），而非手搓协议。备选方案是手写轻量 SSE 客户端（依赖更少但需自行处理协议细节）。

---

## 一、调研背景

Sensend 当前支持 Notion / FlowUs / 飞书 / 本地文件夹四个平台，全部走 REST API。用户希望增加有道云笔记支持。

调研分两个阶段：
1. **第一阶段**（已完成）：确认 MCP 端点背后是否有 REST API → 结论：没有。
2. **第二阶段**（本文）：评估 MCP SSE 客户端方案在 Sensend 中的可行性与实现路径。

---

## 二、调研结论：MCP 端点背后没有 REST API

### 2.1 两套体系完全割裂

| 维度 | 旧 OpenAPI（REST） | 新 MCP 服务 |
|------|-------------------|-------------|
| 端点 | `note.youdao.com/yws/open/*.json` | `open.mail.163.com/api/ynote/mcp/sse` |
| 协议 | REST（HTTP GET/POST） | MCP（JSON-RPC 2.0 over SSE） |
| 鉴权 | OAuth 1.0a / 2.0（ConsumerKey + Secret） | `x-api-key` 请求头 |
| Key 来源 | note.youdao.com 申请 | mopen.163.com（网易智能开发者平台） |
| 状态 | **停止新增申请** | 唯一在用 |
| 能力覆盖 | 用户/笔记本/笔记/分享/附件全套 CRUD | 剪藏、创建、搜索、列表、读取、资讯推送 |

### 2.2 排查过的路径

| 路径 | 结果 |
|------|------|
| 旧版 OpenAPI 文档（note.youdao.com/open/apidoc.html） | 页面顶部明确标注「OpenAPI 已停止新增申请」 |
| OpenClaw 安装/使用指南 | 仅描述 MCP 端点与 Skill 用法，未提 REST |
| YoudaoNote CLI 指南 | CLI 与 MCP 共用同一套 API Key，但未暴露底层 REST 端点 |
| mopen.163.com 平台 | 该平台被文档称为「MCP 控制台」，专门为 MCP 协议签发 Key |
| 非官方脚本 youdaonote-pull | 走网页版 Cookie 爬内部接口，只能读不能写，Cookie 会过期，不适合桌面应用 |

### 2.3 关键判断

新版 API Key（`x-api-key`）是网易智能开发者平台专门为 MCP 协议签发的。没有任何文档或迹象表明旧的 REST 端点会接受这个 Key。官方的 YoudaoNote CLI 工具内部也是连 MCP SSE 端点，不是 REST。第三方集成（WorkBuddy、OpenClaw 等）全部走 MCP SSE。

**结论：没有捷径，只能实现 MCP SSE 客户端。**

---

## 三、MCP SSE 协议简介（给不熟悉 MCP 的人）

MCP（Model Context Protocol）是 Anthropic 提出的标准协议，用于 AI 工具与外部服务通信。有道云笔记的 MCP 服务采用 SSE（Server-Sent Events）传输模式。

### 3.1 通信模型

```
客户端                                          服务端
  │                                               │
  │─── GET /api/ynote/mcp/sse ──────────────────►│  (SSE 长连接，带 x-api-key 头)
  │◄── event: endpoint ──────────────────────────│  (服务端返回一个 POST URL)
  │                                               │
  │─── POST {endpoint_url} ─────────────────────►│  (发送 JSON-RPC 请求，带 x-api-key 头)
  │    {"method":"initialize","params":{...}}     │
  │◄── event: message ───────────────────────────│  (SSE 流返回 JSON-RPC 响应)
  │    {"result":{"protocolVersion":"2025-11-25"}}│
  │                                               │
  │─── POST {endpoint_url} ─────────────────────►│  (调用具体工具)
  │    {"method":"tools/call","params":{...}}     │
  │◄── event: message ───────────────────────────│  (返回工具执行结果)
  │                                               │
  │   (SSE 连接保持，后续请求复用)                  │
```

**两个通道**：
- **GET 通道**（SSE 长连接）：持续接收服务端推送的事件（响应、通知）
- **POST 通道**：发送 JSON-RPC 2.0 请求（initialize、tools/list、tools/call 等）

### 3.2 与 REST 的本质区别

| 维度 | REST（现有适配器） | MCP SSE |
|------|-------------------|---------|
| 连接 | 无状态，每次请求独立 | 有状态，需维护 SSE 长连接 |
| 调用方式 | `reqwest.post(url, body)` | JSON-RPC `tools/call` over POST |
| 响应 | 同步 HTTP 响应 | 通过 SSE 流异步推送 |
| 会话 | 不需要 | 需要 initialize 握手 |
| 鉴权 | 每次请求带 token | SSE 连接 + POST 请求都带 `x-api-key` |

---

## 四、rust-mcp-sdk 方案评估

### 4.1 SDK 概况

| 属性 | 值 |
|------|-----|
| crate 名 | `rust-mcp-sdk` |
| 版本 | v1.0.0（稳定版） |
| 协议版本 | MCP 2025-11-25（最新） |
| 一致性测试 | 100% 通过官方 MCP conformance tests |
| 传输模式 | Stdio、Streamable HTTP、SSE（向后兼容） |
| 框架集成 | Axum、Actix、BYO Server |
| GitHub | rust-mcp-stack/rust-mcp-sdk |
| 许可证 | 待确认（README 未明确标注） |

### 4.2 关键能力

- ✅ SSE 客户端传输（`SseClientTransport`）
- ✅ JSON-RPC 2.0 协议处理（初始化握手、会话管理）
- ✅ 多客户端并发
- ✅ 断线重连（Resumability）
- ✅ 消息观察器（Telemetry & Monitoring）
- ✅ OAuth 认证支持（但有道云笔记用的是 `x-api-key` 头，不是 OAuth）

### 4.3 待验证项

| 验证点 | 说明 | 风险等级 |
|--------|------|---------|
| **自定义 HTTP 头支持** | `SseClientConfig` 是否支持设置 `x-api-key` 请求头 | 🔴 高（核心阻断点） |
| **SSE 连接生命周期管理** | 连接建立后能否长期保持、断线后能否自动重连 | 🟡 中 |
| **编译体积影响** | 引入 SDK 后二进制体积增量 | 🟡 中 |
| **TLS 兼容性** | SDK 底层 TLS 是否与 Sensend 现有的 rustls 兼容 | 🟢 低 |
| **tokio 版本兼容** | SDK 的 tokio 版本是否与 Sensend 的 tokio 1.x 冲突 | 🟢 低 |

> **核心风险**：`rust-mcp-sdk` 的 SSE 客户端是否支持自定义 HTTP 头。MCP SSE 的 `x-api-key` 鉴权是标准模式，从 MCP 生态来看大概率支持，但开始写代码前需确认 `SseClientConfig` 的 API 文档。

### 4.4 备选方案：手写轻量 SSE 客户端

如果 `rust-mcp-sdk` 不支持自定义头或引入过重，可以手写一个最小 MCP SSE 客户端。MCP SSE 协议本身不复杂：

**需要的依赖**（Sensend 已有 `reqwest` + `tokio`）：
- `reqwest`（已有）— 用于 GET SSE 连接和 POST 请求
- `eventsource-stream` 或手动解析 SSE 事件流（新增 1 个小 crate）
- `tokio`（已有）— 异步运行时

**核心逻辑约 200 行 Rust**：

```rust
// 伪代码：最小 MCP SSE 客户端
pub struct McpSseClient {
    api_key: String,
    endpoint_url: String,  // 从 SSE 初始化事件中获得
    http: reqwest::Client,
}

impl McpSseClient {
    // 1. 建立 SSE 连接，获取 POST endpoint
    async fn connect(&mut self, sse_url: &str) -> Result<(), String> {
        let resp = self.http.get(sse_url)
            .header("x-api-key", &self.api_key)
            .header("Accept", "text/event-stream")
            .send().await.map_err(|e| e.to_string())?;
        
        // 解析 SSE 流，等待 endpoint 事件
        // event: endpoint
        // data: https://open.mail.163.com/api/ynote/mcp/post?session_id=xxx
        self.endpoint_url = parse_endpoint_event(resp).await?;
        Ok(())
    }

    // 2. 发送 JSON-RPC 请求
    async fn call_tool(&self, tool_name: &str, args: Value) -> Result<Value, String> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": next_id(),
            "method": "tools/call",
            "params": { "name": tool_name, "arguments": args }
        });
        self.http.post(&self.endpoint_url)
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send().await.map_err(|e| e.to_string())?
            .json::<Value>().await.map_err(|e| e.to_string())
    }
}
```

**手写方案 vs SDK 方案对比**：

| 维度 | rust-mcp-sdk | 手写 SSE 客户端 |
|------|-------------|----------------|
| 依赖增量 | 1 个大 crate（含子依赖） | 1 个小 crate（eventsource-stream） |
| 代码量 | 少（SDK 处理协议细节） | 约 200-300 行 |
| 协议合规性 | 100%（官方测试） | 需自行验证 |
| 维护成本 | 低（SDK 升级跟协议） | 中（协议变更需手动跟） |
| 灵活性 | 受 SDK API 约束 | 完全可控 |
| 自定义头 | 需验证 SDK 是否支持 | 天然支持（reqwest 直接加头） |

---

## 五、Sensend 集成路径

### 5.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Tauri 应用进程                          │
│                                                             │
│  ┌───────────────────────┐      ┌────────────────────────┐  │
│  │   前端 (WebView)      │      │   后端 (Rust)          │  │
│  │   Vue 3 + TipTap      │ IPC  │                        │  │
│  │                       │◀────▶│  commands/platform.rs  │  │
│  │  (配置窗口新增         │      │  get_adapter("youdao") │  │
│  │   "有道云笔记"选项)    │      │        │               │  │
│  └───────────────────────┘      │  ┌─────▼──────────────┐ │  │
│                                 │  │ adapters/youdao.rs │ │  │
│                                 │  │ YoudaoAdapter      │ │  │
│                                 │  │  ├─ McpSseClient   │ │  │
│                                 │  │  │  (SSE 连接管理)  │ │  │
│                                 │  │  ├─ publish()      │ │  │
│                                 │  │  ├─ test_conn()    │ │  │
│                                 │  │  └─ append_blocks()│ │  │
│                                 │  └─────┬──────────────┘ │  │
│                                 └────────┼────────────────┘  │
└──────────────────────────────────────────┼───────────────────┘
                                           │
                           ┌───────────────▼───────────────────┐
                           │  open.mail.163.com                │
                           │  /api/ynote/mcp/sse               │
                           │  (MCP SSE，x-api-key 鉴权)         │
                           └───────────────────────────────────┘
```

### 5.2 与现有适配器的架构对比

| 维度 | Notion / FlowUs / 飞书 | 有道云笔记（MCP） |
|------|----------------------|------------------|
| 协议 | REST | MCP SSE（JSON-RPC 2.0） |
| HTTP 客户端 | `http_client()`（全局 reqwest 单例） | 独立 reqwest 实例（需 SSE 流支持） |
| 连接模型 | 无状态（每次 POST/GET 独立） | 有状态（SSE 长连接 + POST 通道） |
| 鉴权 | Bearer token / tenant_access_token | `x-api-key` 请求头 |
| 内容转换 | TipTap JSON → 平台 blocks（各适配器独立实现） | TipTap → Markdown → MCP tool args（复用 `markdown.rs`） |
| 会话管理 | 不需要 | 需要 initialize 握手 + 会话保持 |
| 错误模型 | HTTP 状态码 + 平台错误码 | JSON-RPC error 对象 |
| 适配器 trait | `PlatformAdapter` | 同一个 `PlatformAdapter`（接口不变） |

### 5.3 后端改动清单

#### 5.3.1 新增文件：`src-tauri/src/adapters/youdao.rs`

```rust
use serde_json::Value;
use async_trait::async_trait;
use super::*;

/// MCP SSE 客户端（轻量手写版或封装 rust-mcp-sdk）
struct McpSseClient {
    api_key: String,
    post_endpoint: Option<String>,  // initialize 后获得
    http: reqwest::Client,
}

impl McpSseClient {
    fn new(api_key: &str) -> Self { /* ... */ }

    /// 建立 SSE 连接 + initialize 握手
    async fn ensure_connected(&mut self) -> Result<(), String> { /* ... */ }

    /// 列出可用工具（首次连接时调用，用于发现工具名）
    async fn list_tools(&mut self) -> Result<Vec<String>, String> { /* ... */ }

    /// 调用工具
    async fn call_tool(&mut self, name: &str, args: Value) -> Result<Value, String> { /* ... */ }
}

/// 有道云笔记适配器
pub struct YoudaoAdapter;

#[async_trait]
impl PlatformAdapter for YoudaoAdapter {
    async fn publish(&self, content: &Value, instance: &PlatformInstance)
        -> Result<PublishResult, String>
    {
        // 1. 复用 markdown.rs 转换 TipTap → Markdown
        let md = markdown::tiptap_to_markdown(content);
        let title = markdown::extract_title(content);

        // 2. 建立 MCP 连接
        let mut client = McpSseClient::new(&instance.token);
        client.ensure_connected().await?;

        // 3. 调用 create_note 工具（工具名需实际 discover 后确认）
        let result = client.call_tool("create_note", json!({
            "title": title,
            "content": md,
            "notebook_id": instance.target_id,  // 如果需要指定笔记本
        })).await?;

        // 4. 解析结果，返回 URL（如果有）
        Ok(PublishResult {
            success: true,
            message: "已创建笔记".into(),
            url: result.get("url").and_then(|v| v.as_str()).map(String::from),
        })
    }

    async fn test_connection(&self, instance: &PlatformInstance) -> Result<(), String> {
        let mut client = McpSseClient::new(&instance.token);
        client.ensure_connected().await?;
        let tools = client.list_tools().await?;
        if tools.is_empty() {
            return Err("连接成功但未发现可用工具".into());
        }
        Ok(())
    }
}
```

#### 5.3.2 注册模块：`adapters/mod.rs`

```rust
// ① 添加模块声明
pub mod youdao;

// ② 在 get_platform_types() 中添加
PlatformTypeInfo {
    key: "youdao".into(),
    name: "有道云笔记".into(),
    color: "#E63946".into(),  // 有道红
    fields: vec![
        ConfigField {
            key: "token".into(),
            label: "API Key".into(),
            hint: "在 mopen.163.com 获取".into(),
            secret: true, hidden: false, browse: false,
            default_value: None, optional: false,
        },
        ConfigField {
            key: "target_id".into(),
            label: "笔记本 ID".into(),
            hint: "有道云笔记的笔记本 ID（可选，留空则发到默认笔记本）".into(),
            secret: false, hidden: false, browse: false,
            default_value: None, optional: true,
        },
    ],
},

// ③ 在 resolve_target_id() 中添加（如果有 URL 解析需求）
"youdao" => youdao::YoudaoAdapter::resolve_youdao_id(trimmed),
```

#### 5.3.3 工厂分发：`commands/platform.rs`

```rust
"youdao" => Ok(Box::new(adapters::youdao::YoudaoAdapter)),
```

### 5.4 前端改动

**通常无需改动**。前端表单由后端 `get_platform_types()` 返回的 `ConfigField` 元数据驱动动态渲染，平台色点由 `color` 字段决定。新增「有道云笔记」选项后，配置窗口会自动显示对应表单。

### 5.5 依赖变更

**方案 A（rust-mcp-sdk）**：
```toml
# Cargo.toml [dependencies] 新增
rust-mcp-sdk = { version = "1.0", default-features = false, features = ["client", "sse"] }
```

**方案 B（手写 SSE 客户端）**：
```toml
# Cargo.toml [dependencies] 新增
eventsource-stream = "0.2"  # SSE 流解析
# reqwest 已有，tokio 已有，serde_json 已有
```

### 5.6 用户配置流程

1. 用户前往 [mopen.163.com](https://mopen.163.com)（网易智能开发者平台）
2. 手机号登录（需有道云笔记账号已绑定手机号）
3. 在「API 管理」中获取 API Key
4. 在 Sensend 配置窗口新增「有道云笔记」实例
5. 填入 API Key，点「测试连接」
6. 保存即可使用

---

## 六、风险与待验证项

### 6.1 技术风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| **rust-mcp-sdk 不支持自定义 HTTP 头** | 无法传 `x-api-key`，方案 A 不可用 | 改用方案 B（手写 SSE 客户端，reqwest 天然支持自定义头） |
| **MCP 工具名不确定** | 不知道 create_note 等工具的确切名称和参数格式 | 首次连接时调用 `tools/list` 发现工具，根据实际返回调整调用参数 |
| **SSE 连接稳定性** | 桌面应用长时间运行，SSE 连接可能断开 | 实现断线重连；或改为每次发送时新建连接（牺牲性能换简单） |
| **编译体积增长** | rust-mcp-sdk 可能引入大量子依赖 | 方案 B 只需 1 个小 crate（eventsource-stream）；或用 `cargo tree` 检查后决定 |
| **API Key 限流** | 高频发送可能被限流 | Sensend 是手动触发发送，频率天然很低，风险不大 |

### 6.2 产品风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| **用户获取 API Key 的门槛** | 比 Notion token 或飞书 App ID 复杂 | 在配置窗口 hint 中提供 mopen.163.com 链接和简要说明 |
| **MCP 协议变更** | 网易可能升级 MCP 版本导致不兼容 | rust-mcp-sdk 跟官方协议；手写方案关注 `protocolVersion` 字段 |
| **能力限制** | MCP 工具可能不支持所有操作（如追加内容到已有笔记） | 先 `tools/list` 发现能力，再决定 publish_mode 支持范围 |

### 6.3 验证清单

在开始正式开发前，建议先做以下验证：

- [ ] **V1**：确认 `rust-mcp-sdk` 的 `SseClientConfig` 是否支持自定义 HTTP 头（查 docs.rs 或源码）
- [ ] **V2**：用 API Key 手动连接 MCP 端点，`tools/list` 获取完整工具清单和参数 schema
- [ ] **V3**：确认 `create_note`（或等效工具）的参数格式——是否接受 Markdown 内容、是否需要指定笔记本
- [ ] **V4**：确认是否有「追加内容到已有笔记」的工具（决定是否支持 block 模式）
- [ ] **V5**：测试 SSE 连接的保活时长和断线行为
- [ ] **V6**：如选方案 A，检查 `cargo tree -d` 是否有依赖冲突

---

## 七、工作量预估

| 阶段 | 方案 A（SDK） | 方案 B（手写） | 说明 |
|------|-------------|---------------|------|
| 依赖调研与验证 | 0.5 天 | 0.5 天 | V1-V6 验证清单 |
| MCP 客户端实现 | 0.5 天 | 1 天 | SDK 封装 vs 手写 SSE |
| YoudaoAdapter 实现 | 0.5 天 | 0.5 天 | trait 实现 + TipTap→Markdown |
| 注册与集成 | 0.5 天 | 0.5 天 | mod.rs / platform.rs / 测试 |
| 联调与测试 | 0.5 天 | 1 天 | 实际发送验证 |
| **合计** | **约 2.5 天** | **约 3.5 天** | — |

---

## 八、结论与建议

### 8.1 可行性判断

**技术可行，推荐实施。** 理由：
1. 官方 MCP 路径稳定，API Key 获取流程清晰
2. `rust-mcp-sdk` 成熟稳定，或有轻量手写备选
3. Sensend 的 adapter 架构扩展性好，新增平台只需实现 trait + 注册
4. 前端零改动（元数据驱动）

### 8.2 方案选择建议

**优先尝试方案 A（rust-mcp-sdk），不通过则转方案 B（手写）。**

决策点：V1 验证（SDK 是否支持自定义 HTTP 头）。
- 通过 → 方案 A，省心省力
- 不通过 → 方案 B，多花 1 天但完全可控

### 8.3 注意事项

1. **先验证再开发**：V1-V2 是核心阻断点，确认后再动工
2. **MCP 工具名以实际 discover 为准**：文档未公开工具的精确名称和参数 schema，需运行时发现
3. **连接策略**：桌面应用发送频率低，建议每次发送时新建连接（简单可靠），而非维护长连接（复杂但高效）
4. **内容格式**：有道云笔记 MCP 工具大概率接受 Markdown（CLI 指南显示 create 命令用 Markdown），可复用 `markdown.rs` 的 `tiptap_to_markdown()`
