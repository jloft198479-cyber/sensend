# Sensend v0.4.0 升级记录（单一信息源）

> 本文件是 v0.4.0 升级相关决策的**唯一权威来源**。原始方案文档（完整版 / 格式兼容性 / 审核结论 / 任务分配 / 交接说明）已归档至 `docs/archive/`，仅作历史查证，不再维护。
> 最后校正：2026-08-19（内存优化实测结论已回写本节）。

---

## 一、产品定位与减法声明（产品真源，所有升级决策以此为准）

**Sensend 是「速记 → 扔出去 → 在目的地改」的中转站。**

- 想到 → 记录 → @目标 → 发射 → 完事
- 不做档案馆（不建本地文档目录、不做双向同步、不做离线队列）
- 不做多目标同时分发（手动多次即可）
- 修改后形成新文档是产品语义的必然结果，解法是 UI 清晰反馈而非技术归档

**明确不做（减法声明）**：图片传输、双向同步、本地文档目录、离线队列/发送历史、多目标同时分发、图片粘贴/编辑、风格化主题（文艺/科技）、.docx 拖入解析、新平台适配器。

---

## 二、架构决策：IR 中间层（格式兼容性）

- **问题**：4 份平行转换实现（~400 行重复），同一缺陷要修四遍且没修齐。
- **方案**：TipTap JSON → `adapters/convert.rs`（唯一遍历点，纯函数）→ 规范化 IR（`Vec<IrBlock>`，块级 Paragraph/Heading/Bullet/Ordered/Todo/Quote/Code/Table/Divider + 内联 `Vec<Inline>`）→ 四平台薄映射层（无遍历、无递归）。
- **能力矩阵**：从代码 `PlatformCapabilities` 生成，杜绝文档滞后。
- **测试**：`fixtures/` + 双层断言（`expected_current` 锁现状 / `expected_target` 修后切换），先锁行为再重构。

---

## 三、审核关键纠正（执行前必读，原《审核结论》第三节）

1. **mention 检测优化原方案代码是错的**——原 `tr.steps.some(s => s.toJSON().stepType !== 'replace')` 会跳过唯一需检测的场景（打字 = ReplaceStep）。正确做法：用已有的 `mentionReplace` meta 快路径，仅「本事务是 mention 替换 / 删除了字符 / 当前存在 mention」三种情况才全量遍历，普通打字直接跳过（覆盖 99% 按键）。✅ 已随性能优化（S4）落地。
2. **vite manualChunks 单独无效**——静态 import 仍启动时加载。正确：`main.ts` 改动态 `import()`，Rollup 自动分包。
3. **visible:false + 前端 show() 缺权限**——须补 `core:window:allow-show` capability，否则白屏优化上线即报权限错误。
4. **悬浮球窗口三前置**：capabilities 的 windows 数组加新窗口 label；唤起主窗收口到 Rust `show_main()`（四处调用点统一）；内存预算自相矛盾（<80MB 与独立 WebView 球冲突）→ 球懒创建、预算放宽 <130MB。

---

## 四、执行状态

- 截至 2026-08-18：S0–S5 完成（IR 重构、测试骨架、性能高危项、格式兼容性首批）；B1（TaskList）随 S4 完成；B2–B9 待执行（详细任务单见 `docs/archive/升级任务分配方案-20260817.md`）。
- 2026-08-19 追加：内存优化（关 GPU 进程 + 隐藏即瘦身），改动见 `src-tauri/src/memory_trim.rs` 与 `lib.rs`，构建验证见 `docs/BUILD-GUIDE.md`。

---

## 五、相对原始方案的更正（本记录已采纳）

| 原方案说法 | 更正（实测 / 后续） | 来源 |
|---|---|---|
| 构建入口「按 HANDOVER 手动激活环境」、引用 `BUILD-ENV.md` | 唯一可靠入口为 `scripts/build-release.ps1`（已修复 MSVC PATH 大小写变体覆盖 bug）；`BUILD-ENV.md` 已并入 `docs/BUILD-GUIDE.md` 后删除 | BUILD-GUIDE |
| 内存目标「空闲 <80MB / 编辑 1h <120MB」 | WebView2 引擎地板约 100MB+；关 GPU + 瘦身后隐藏态可压至 ~15–30MB，使用中 ~60–70MB。原数字基于不完整认知，已更正 | 2026-08-19 内存实测 |
| 悬浮球独立 WebView 与 <80MB 预算并存 | 二者冲突，已明确球懒创建、预算放宽 <130MB | 审核结论 3.4 |

---

## 六、构建与验证纪律

- **构建唯一入口**：`powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"`（Rust/VS Build Tools 装在 M 盘非 PATH，普通终端直接 cargo 必失败）。
- **只改 `sensend/`**，严禁碰 `sensend-release/`（0.2.0 旧副本，改了白干）。
- **验证三件套**：`cargo check` / `cargo test`（Rust 改动）、`npm run build`（前端改动）、跨端互验（改后端必须看前端表现，反之亦然）。
- **阶段提交**：每阶段完成即 commit（阶段号作前缀），出问题单阶段回退。

---

> 原始方案全文见 `docs/archive/`：格式兼容性提升方案、升级方案完整版、升级方案审核结论、升级任务分配方案、HANDOVER 交接说明。
