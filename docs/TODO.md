# Sensend 待办事项

> 本文件记录已发现但暂未处理的问题与改进点，便于后续统一处理。
> 新发现的问题追加到对应分类末尾即可，处理完成后移至「已处理」归档区。

---

## 待处理

### 界面/工程 · 复核后决定「保持现状」（2026-11-16 第 4 批）

三条都是评审提过、但按当前证据判断「改了收益小于风险」，记下来防下次重新纠结：

| 项 | 结论 | 原因（不是没空做，是想清楚了） |
|---|---|---|
| 长文档里只要文中存在 @mention，每个按键都全量遍历一次文档（m-28） | 保持现状 | 看起来能改成「按 `tr.mapping` 映射上次 mention 位置、只查那一个点」，但 `setContent` / 整段替换时映射会给出失效位置，一旦误判成「mention 已删除」，底栏发送目标就被静默切走。要动必须先有长文档卡顿的实测证据。代码注释已写明取舍（`useEditor.ts` onUpdate 处）。 |
| 编辑区可能「先空后满」（窗口立刻显示，笔记稍后读回，m-16） | 保持现状 | 数据风险已由 C-01 闸门消除（读取完成前编辑器 `setEditable(false)`，用户根本打不进字，不存在「输入被 setContent 覆盖」）。剩下的只是视觉上的瞬间空白，属「先响应再填充」的有意取舍；若改成等读完再显示，读盘卡住时窗口就永远不出现。 |
| GitHub Actions CI（m-24 只要求 build 串门禁，CI 属额外） | 未做 | `npm run build` 已串 `check:tokens` + `check:contrast` + `vue-tsc`（本地构建即门禁）。上 CI 会改发布链路，按约定等明确指令再做。 |

### 出站格式的已知限制（2026-11-16 第 3 批修复时确认）

三条都是「平台侧限制 + 修了会引入更大风险」，故按已知限制记录，不做代码改动：

1. **超长代码块仍可能超平台 block 总量上限**。单元素超限已解决（Notion/FlowUs/飞书都按 2000 字符切片），但飞书 block 级约 50000 字符、Notion 单 block rich_text 数组最多 100 段——20 万字符以上的代码块仍会被平台拒。真要解决得把超长代码块拆成多个 block，属功能变更。
2. **飞书表格单元格内的软换行可能不显示**。单元格内多段落现在会生成 `\n` 文本，但飞书对表格单元格的软换行支持有限（见 FEISHU-GUIDE 5.2.4），可能仍显示成一行。拆 block 会破坏单元格结构，不做。
3. **分片写入中途失败无法幂等**。三个平台的批量写入 API 都不支持按内容去重，失败后已写入的分片留在平台上。已做：429 限流自动重试（避免最常见的分片中断）、失败信息带上「已写入 N/M 块」提示先删再试。根治需要平台提供幂等键。

---

### 弹层工程债与命令范围备忘（2026-11-17 第三方评审留痕）

| 项 | 内容 | 状态 |
|---|---|---|
| render 工厂 | mention 与斜杠的 suggestion render（VueRenderer + tippy）已两份；第三个弹层出现前抽 `createSuggestionPopup(Component, opts)`，几何策略作参数传 | 记而不做 |
| 命令范围 | 斜杠现列 10 条（表格 6 + 引用/无序/有序/待办 4）；codeBlock/分割线/标题**刻意未列**——无需求输入不膨胀命令表，需要时 COMMANDS 添一项即可 | 记而不做 |
| 规范 docx | 内容字阶（editor.css `--text-editor` 16px / `--lh-editor` 1.9，独立于界面字阶）尚未写进设计规范文档；门禁白名单已加 16px 留痕，下次文档体检一并补章节 | 待补 |

---

### 表格悬浮把手（行业标准入口）· 记而不做

- **背景**：2026-11-17 表格结构操作（加/删行列、删表格）以斜杠命令落地（见「已处理」区）。行列操作的行业主流入口是**悬浮把手**（Notion/飞书/语雀：悬停行/列边缘出 +/⠿）或**右键菜单**（Word/Google Docs/Obsidian）；斜杠的行业定位是「插入新内容块」。
- **现状**：主人实测斜杠交互「挺顺手」，接受现状；把手不立项，仅记录防丢失。
- **升级路径**：若未来觉得「敲 / 加行烦」，做把手/右键菜单时命令执行层直接复用 `useSlashCommand.ts` 的 COMMANDS 注册表——届时把注册表提升为多入口共享真源（约半小时小重构），机制零改动。真正的遗留是可发现性（新人不知道格子里能敲 `/`），属体验升级非架构债。

---

### 前端空检查正则对带空格的 @实例名失效

- **发现时间**：2026-08-18（第三方复查 ir.rs 尾部剥除逻辑时发现）
- **现象**：实例名含空格（如 `我的 Notion`）时，发送前空检查的正则 `/(?<!\S)@\S+/g` 只剥到第一个空格，残留 ` Notion` 使检查误判为有内容而放行；后端 stripMentions 后全文只剩空白 → 空 IR → 适配器兜底发出一条空内容并提示成功（"成功的空发送"）
- **位置**：`src/composables/usePlatform.ts#L76`
- **修复方向**：空检查改用与 stripMentions 同源的判定（遍历 JSON 剔除 mention 节点后再判空），而非文本正则
- **严重度**：低（需实例名含空格且正文只有该 mention 才触发），暂不修，记此备查

---

### 平台发送健壮性（2026-08-19 从 EXPERIENCE §四 并入，集中维护）

以下为各平台已识别但暂不修复的发送健壮性问题，统一在此排队，不在其他文档重复维护。

| 平台 | 问题 | 严重度 | 状态 | 出处 |
|------|------|:---:|------|------|
| Notion | 429 限流无重试 | P3 | ✅ 已修复（退避 2 次；未读 Retry-After） | 2026-11-16 |
| 飞书 | 429/99991400 限流无重试 | P1 | ✅ 已修复（HTTP 429 + 业务码 99991400 各退避 2 次） | 2026-11-16 |
| 飞书 | 引用未用 quote_container 嵌套（段落间失去引用归属） | P1 | ✅ 已修复（quote_container + 子 quote，2026-11-18 实测） | FEISHU-GUIDE.md §5.6 P1 |
| 飞书 | text_run 无长度上限检查 | P1 | 🟡 单元素已按 2000 切片；block 总量上限未处理（已知限制） | 2026-11-16 |
| 飞书 | 嵌套列表拍平丢层级 | P2 | ✅ 已修复（子项挂父项 children，2026-11-18 实测） | FEISHU-GUIDE.md §5.6 P2 |
| 飞书 | 代码块 wrap 未设置（长代码行不自动换行） | P2 | ✅ 已修复（style.wrap=true，2026-11-18 实测） | FEISHU-GUIDE.md §5.6 P2 |

---

### 外部借鉴 · wandao 万能导（2026-09-02 评估，待展开）

- **来源**：`G:\skills\wandao-main`（开源项目源码，**不是** WorkBuddy 技能，无 SKILL.md）
- **它是什么**：多平台知识库批量导入导出搬迁工具（飞书 / 语雀 / Notion / 印象笔记等），Tauri 2 + HTML/JS + Python sidecar，1.4.x
- **与 sensend 关系**：方向相反 —— 它是「批量搬迁」，我们是「单条速记分发」。仅局部可借鉴，不可整体照抄

**待办清单**：四项已全部完成（#1 随 v0.5.1 上线，#2/#3/#4 于 2026-11-16 完成），明细移至「已处理」归档区。

> 注：wandao 参考源 `G:\skills\wandao-main` 已不在原位置（2026-11-16 实测），其思路记录仍有效；如需再对照实现须先重新定位源码。

**明确不做**（记此防下次重提）：

| 项 | 原因 |
|---|---|
| 插件中心 / Provider manifest / 任务中心 / checkpoint 断点续跑 / 增量跳过 / 内嵌 Python runtime | 属「批量搬迁」的重型机制，套在单条速记上即过度工程 |

> ⚠️ **AGPL-3.0 红线（长期有效，待办处理完也不删除）**：wandao 为 AGPL-3.0，sensend 为 MIT。直接复制其 Rust 源码会触发协议传染，法律上要求 sensend 整体以 AGPL 开源。**只能借鉴思路、自行重新实现，不得搬运文件。**

---

## 已处理（归档）

### 「Escape 盲触发」核实（2026-11-17）· 不成立，防重提

- 前轮复审 WARN：「Esc 只 hide 弹层，Suggestion 建议态仍活跃，Enter 会盲触发命令」
- 核实：@tiptap/suggestion 3.28.0 源码（dist/index.js:139-152）——Escape 分支即调 `dispatchExit` 退出建议态并触发 onExit，@mention 与斜杠命令均吃这条上游逻辑
- 结论：不成立，无需修。防下一轮当成遗漏重新讨论

### 表格功能前端入口（2026-11-17，斜杠命令方案闭环）

- **原状**：内核 TableKit / 粘贴链路 / 四平台适配全通，唯缺「从零新建表格」入口（2026-07-31 记录于本文件）。
- **第一版（气泡按钮）被否**：须先选中文字菜单才弹出，与「先建表再填字」的真实动作相反——主人指出逻辑不通（按钮版未保留，已 amend 掉）。
- **终版（斜杠命令）**：单笔提交 `08c1556`（未 push）：`useSlashCommand.ts`（Suggestion 插件，与 @mention 同构）+ `SlashList.vue`（弹层，InstanceMenu 同款 token 配方）+ useEditor.ts 一行挂载。六条命令（插入表格 + 表内加行/删行/加列/删列/删表）全走 TipTap 官方命令；注册表 `when` 上下文限位——表格命令仅光标在表内出现、插入表格在表内不出现（防套娃）；startOfLine 限行首防日期/网址误弹；代码块内不触发。
- **架构约定（主人拍板「不孤立成套、可插拔」）**：接入 = useEditor.ts 挂一行；加命令 = COMMANDS 添一项（when 可选）；Tab 到末格自动加行为内核自带。
- **验证**：npm run build（tokens+contrast+vue-tsc）全绿；无新依赖、无 Rust 改动。
- **经验**：① 给「插入类」功能定入口，先想用户真实动作的先后（先结构后内容），别套现有交互的壳；② 主人实测顺手的交互不必硬追行业标准——机制通用、入口特殊，可接受的组合。

**起因**：`设计评审-列表与管理页-2026-11-17.html` 诊断出管理页打开 0.3–0.55 秒的根因（每次冷启动新建窗口 + 被迫解析 689 KB 编辑器主包 + 等主题 IPC 才挂载）＋ @ 列表 8 条、管理页 10 条交互问题 ＋ 3 条逻辑隐患。施工单见 `执行任务书-zcode-2026-11-17.md`（ZC-01~13，分 B1 止血 / B2 结构 / B3 精修）。

**执行结果**：四个提交（`74fbc05` B1、`740b038` B2、`d00a030` B3a、`a517ac0` B3b），16 文件 +694/−171。

**实测达成**：管理页加载从 689 KB 降到约 118 KB（入口 84 + ConfigWindow 16 + ToastLayer 17）；配置窗关闭改隐藏（二次打开不再冷启动）；建窗 `visible(false)` + builder 上底色（白闪清零）；主题挂载前同步上色；@ 列表可按中文平台名搜索、零实例不再弹空壳、键鼠只有一套高亮。

**复查后补修的 5 处**（外部执行留的尾巴，2026-11-17 当场修掉）：
1. `TargetBadge.vue` 零引用死组件（42 行）→ 删除。图标与类型徽标的单一化已由 `TargetIcon` + 全局 `.type-badge` 达成，未接上的半成品不留。
2. 关闭按钮确认文案写「确认放弃修改并关闭」，但关闭=隐藏、组件不销毁、填了一半的内容下次打开还在 → 文案改为「未保存的修改会保留」，与行为一致（**UI 不许说假话**）。
3. `zeroInstanceTipShown` 是模块级一次性 flag，删光目标后再按 `@` 既不弹列表也不出提示（像按键坏了）→ 实例数 > 0 时复位。
4. 删除默认目标只清了前端 `defaultTargetId`，后端 `config.json` 留着幽灵 id → 下次启动主窗 D-02 分支会弹「上次默认发送目标已被删除」（用户刚删完却被通知"被删除"）→ 删除时一并 `set_default_target('')`。
5. `docs/CODE-WIKI.md` 零同步（新组件、新事件、入口拆分、窗口事件全未记）→ 六处补齐。

**流程观察**：任务书 D-03 明写「只做 B1，验收通过后停下等确认」，实际四批一次性做完，中间无人工验收点。代码质量未因此受损（门禁全绿），但意味着一旦 B1 出错，B2/B3 会叠在错误基础上——交付前约定确认点时要更硬。

> 验证：`cargo test` 114 全绿；`vue-tsc --noEmit` 0；`check-tokens` / `check-contrast` 通过（五主题各 40 变量同构）；反向校准（把 `--focus-ring` 改回低透明度）`check-contrast` 仍 exit 1，门禁有效。

### 代码评审第 4 批 · 界面小毛病 + 工程规范（2026-11-16，16 项闭环）

| 项 | 问题 | 处置 |
|---|---|---|
| M-06 | @ 里的目标实例被删后，发送静默回落到「本地文件夹」且 toast 报成功 | `App.vue` 发送前校验 mention 目标是否仍存在，不存在则明确报错并停止，不再回落 |
| M-12 | `instances-updated` 监听器泄漏（`onBeforeUnmount` 在 `await` 之后注册，Vue 实例已复位） | 卸载清理改成**同步**注册 + `disposed` 标志；若卸载发生在 `await` 期间，拿到句柄就地销毁 |
| m-08 | 本地发送后点「查看」没反应（本地返回磁盘路径，前端却走 `openUrl`） | 新增后端命令 `open_path`（只收存在的绝对路径）；前端按 `http(s)` 分流 |
| m-09 | 快捷键录制把空格写成 `Space`，匹配时却比 `'space'` / `'keyspace'`，含空格的组合键永远不生效；且 `Shift+1` 被记成 `Shift+!`、`Shift+/` 记成 `Shift+?`，后端 parse_key 不认 | 录制与匹配**共用** `codeNameFromEvent()`，一律按物理键位 `e.code` 取名（`A` / `1` / `F5` / `Space` / `Slash` / `Enter` / `ArrowUp`），两边只比一处；匹配时另加 `legacyKey()` 兜底，兼容升级前按 `e.key` 存下的老配置（`Shift+?` 之类），否则老用户的发送快捷键会静默失效 |
| m-17 | 静默检查「先记时间戳再检查」，网络一断就白扔 24 小时额度；手动检查忙时静默 return | 成功才计 24h，失败退避 1h；忙时给 toast 提示 |
| m-20 | Toast 无 `role`/`aria-live`、关闭按钮无 `aria-label`、叉号 `opacity:0.4`（约 2.3:1） | 每条自带语义：成功/提示 `role="status"`、错误/警告 `role="alert"`（这两个角色本身就是 live region，外层不再叠 `aria-live` 以免重复播报）；按钮补 `aria-label`；叉号改实色 `--muted`（≥4.5:1） |
| m-21 | 目标列表有键盘逻辑但没人调用；`outline:none` 压掉全局 `:focus-visible`（scoped 特异性更高）导致键盘焦点不可见；高亮只在 items 变化时重置 | 底栏桥接 `menuRef.onKeyDown`（↑↓/Enter/Esc 可用，Enter 统一 `preventDefault` 不会双触发）；删掉 `outline:none` 让实色焦点环生效；同时 watch `activeId`（编辑器改 @ 目标后重开列表，高亮跟得上） |
| m-22 | 快捷键弹窗 Esc 关不掉 | 非录制态 Esc 关闭弹窗；录制态 Esc 只取消录制（先取状态再 emit）；`e.repeat` 的长按重复事件被吞掉，避免按住 Esc 顺手把弹窗也关了 |
| m-23 | `App.vue` 给 `TitleBar` 传了未声明的 prop/事件，变成 fallthrough 落到 `<header>` | 删掉多余绑定与 TitleBar 里永不触发的 `open-config` |
| m-24 | `build` 未串联两个门禁脚本 | `npm run check`（tokens + contrast）已串进 `build` |
| m-25 | `--focus-ring` 是 0.35 透明度主色，实测键盘焦点仅 1.42:1 | 拆成 `--focus-ring`（实色，≥3:1）+ `--focus-ring-soft`（氛围层）；焦点环进对比度门禁 |
| m-26 | 门禁有盲区：`check-tokens` 不扫 `.ts`、`check-contrast` 主题清单手写、`readVars` 重复键静默覆盖 | 扫 `.ts` 颜色字面量（仅 token 源头豁免）；主题清单从目录派生并与 `useTheme.ts` 交叉校验、校验 `windowBg == --bg`、重复键报错 |
| m-27 | 编辑实例时 `showToken`/`testErrorMsg` 不重置，`testResult` 直接置 `ok`（假装测过）；`loadData` 串行且失败只 `console.error` | 三项统一重置（`testResult` 改 `idle`）；两个 IPC 并行 + 失败 toast |
| n-01 | 保存用固定 `.note.json.tmp` 且命令层无锁，并发写会互相截断 | 加 `SAVE_LOCK`（tokio Mutex）+ tmp 名带 pid/序号；**写失败时清掉半成品**（tmp 名不再复用，残留会永久躺在数据目录里且含不完整正文） |
| n-05 | 修剪工作集前不校验进程，快照到 OpenProcess 间 pid 可能被复用 | 加映像名白名单（自身 exe + `msedgewebview2.exe`），不匹配则跳过；日志从 `debug` 提到 `info`（有跳过时 `warn`），否则 release 下「修剪静默失效」一条痕迹都没有 |
| n-07 | `charCount` 死代码；清空按钮确认态 `aria-label` 不变 | 删除 `charCount` 全链路；`aria-label` 随确认态切换 |

> m-18（两个菜单同名 `.font-picker`）与 m-19（FontManager 无 aria-modal）已随「砍掉自定义字体」那一轮消失，无需单独处理。
> 独立子 agent 审核两轮：第一轮指出 3 处必须返工（`InstanceMenu` 的 `outline:none` 压掉焦点环、`activeId` 变化不同步高亮、文档 `charCount` 残留）+ 6 处低危，已全部修掉；第二轮复核「通过」，并再发现 3 处（老配置符号快捷键失效、首屏高亮在可视区外、文档计数不符），也已修掉。
> 验证：`cargo test` 107 → **114 全绿**（新增 7 条覆盖保存锁/唯一 tmp 名/`open_path` 校验/映像名 FFI）；`vue-tsc` EXIT 0；两个门禁脚本做了**反向校准**（焦点环回退、`windowBg` 改错、变量重复、代码写死颜色 → 四种情况都能报红）。

### 平台凭证 DPAPI 加密（2026-11-16 修复，出处：wandao 借鉴项 #1）

- **现象**：`config.json` 中 `platform_instances` 的 token/token2 与 `token_memory` 全部**明文存储**（Notion token、飞书 App Secret 可直接读）
- **修复**：新增 `src-tauri/src/security.rs`——Windows DPAPI 用户级加密 + 应用熵，密文 `enc1:` 前缀 + base64 标记
- **兼容**：读取兼容旧明文（无前缀原样返回）；启动时 `migrate_credentials_at_startup` 自动迁移残留明文，迁移前备份 `config.json` → `config.json.bak`，校验通过即删除（含历史遗留的 `config.json.pre-encrypt-backup`，见 `PLAINTEXT_BACKUPS`）
- **边界**：加解密只发生在 platform.rs 存/读 store 处，业务层与前端零改动；仅借鉴 wandao 思路自行实现（规避 AGPL）
- **诚实边界**：DPAPI + 熵可挡「拷文件到别的机器」与「同机程序空熵盲扫」，**挡不住**以同账户运行且专门逆向本应用的恶意程序

### 格式 P1 三连修（2026-08-19 提交 d09a413）

| 项目 | 状态 |
|------|------|
| FlowUs 表格单元格多 rich_text 丢失（cells 应为二维：`row_cells.push(json!(rt))`，格内多段全保留、列数 = `table_width`） | ✅ 已修复 |
| Notion 代码块语言映射（`NOTION_LANG_MAP`，40 个别名、大小写不敏感、兜底纯文本） | ✅ 已修复 |
| Notion 嵌套列表深度截断（上限 2 层，第 3 层起子项文本并入父项） | ✅ 已修复 |

### wandao 借鉴项 #1–#4 全部完成（2026-11-16）

| # | 项目 | 实现情况 |
|---|------|----------|
| 1 | 应用内自动更新 | v0.5.1 上线：tauri-plugin-updater + minisign 签名校验 + 双端点（useUpdater + commands/updater.rs） |
| 2 | CSP 收紧 | `script-src` 去 `unsafe-eval`；`connect-src` 去全网 `https:`（前端零直连，全走 IPC）；补 `object-src 'none'` / `base-uri` / `form-action` |
| 3 | 配置原子写 | `note.rs` tmp + sync_all + rename（tokio 异步）；另加 inFlightSave 并发守卫防交错写 |
| 4 | 本地文件名边界 | `local.rs::sanitize_title`：控制字符、首尾空白、保留设备名（stem 精确比对，如 con/NUL.backup/com1.md）前缀 `_`，带单元测试 |

> 均只借鉴思路自行实现（规避 AGPL）。

### 复查报告漏网小修批次（2026-11-16 修复，共 12 项）

- WEBVIEW2 附加参数改逐项合并：环境变量部分重叠时 `--js-flags` 等不再丢失（lib.rs）
- 平台「写入方式」显隐改由后端 `PlatformTypeInfo.supported_modes` 下发，删前端 `'local'/'lark'` 写死；切换/载入时 `publish_mode` 归一到支持档位
- ~~字体管理器预览不再套自定义字体~~（后续批次已整体移除自定义字体功能，本项随之作废，留痕备查）
- `resolve_notion_id` 先剥 `?query`/`#锚点`：块分享链接不再误取锚点当页面 ID（带测试）
- IR 层修复 blockquote 内嵌列表项拼一行：各项以 hardBreak 分隔，四平台共用（带测试）
- Notion 超深有序列表溢出文本按位置编号（不再全部 "1. "）
- markdown 代码块围栏：代码含 ``` 行时加长围栏；闭合围栏前补换行（修复围栏粘在末行代码后）
- `initTheme` 两个 IPC 并行（省一次串行往返；主题仍在挂载前生效防闪色）
- 快捷键默认值收敛到后端 `get_hotkeys` 单一来源（前端不再各存一份）
- 接入 `tauri-plugin-log`：release 落 `%LOCALAPPDATA%\com.jloft.sensend\logs\sensend.log`（512KB 滚动）+ dev stdout；`publish_note` 成功/失败 info/warn 留痕（不含内容与凭证）——结束线上零日志状态
- CSP 收紧见 wandao #2；本地文件名边界见 wandao #4

---

## 文档体检（2026-11-17，逐条对照脚本与配置核验）

> 起因：简乐质问「文档里还有多少胡编乱造、把方向搞偏的」。做法：4 个脚本 + 两份 tauri 配置 + package.json + 已发布 Release 资产，逐条对账。

**查出的错（全部已修）**：

| 严重度 | 位置 | 错在哪 |
|---|---|---|
| 🔴 方向性 | `run-dev.ps1` | 跑 `tauri dev` 没带 `--config tauri.dev.conf.json` → 用**正式身份** `com.jloft.sensend` 调试，抢正式版数据目录与单实例锁 |
| 🔴 方向性 | BUILD-GUIDE §4.4 | 发布命令**只传 setup.exe**，漏 `.sig` 与 `latest.json` → updater 端点 404 → 已安装用户自动更新永久失效（v0.5.0 就是这么发的） |
| 🔴 方向性 | BUILD-GUIDE §2.2 | 把 `run-app.ps1` 标成「改完代码最快看到效果」，实际是全量打包（90s）。文档亲手把用户推上最慢的路 |
| 🟠 缺步骤 | BUILD-GUIDE §2.3 | 说脚本「做三件事」，漏掉版本三处一致性校验、更新签名私钥检查（缺了 exit 1）、`latest.json` 生成与 BOM 自检 |
| 🟠 缺步骤 | BUILD-GUIDE §2.3/§4.2 | 产物清单只有 exe，漏 `.sig` / `latest.json` |
| 🟠 说反了 | BUILD-GUIDE §〇/§2.2.1 | 「`run-app.ps1` 会强杀运行中的 sensend」——实际按 Path 精确匹配只杀 `target\release` 那一个，**不碰安装版** |
| 🟠 说反了 | `FORMAT-FIXES.md` §1.6 | 同上，写成 `Get-Process -Name "sensend"` 全杀 |
| 🟠 跑不通 | `README.md` 开发段 | 教人裸跑 `npm run tauri dev` / `npm run tauri build`，本机必报找不到 cargo |
| 🟠 跑不通 | `CODE-WIKI.md` §9.2/§9.3 | 同上；且 §9.1 说 Tauri CLI 要 `npm install -g`，与「项目已内置」矛盾 |
| 🟠 描述错 | `CODE-WIKI.md` §9.4 | `npm run build` 写的是 `vue-tsc && vite build`，实际前面还有 `npm run check`（两道设计门禁） |
| 🟡 路径错 | `EXPERIENCE.md` §1.2 | 项目根写成 `F:\fzz-Project\sensend\sensend`，少了 `2-sensend` 一段 |
| 🟡 过期 | BUILD-GUIDE 全文 | 命令示例钉死 v0.5.0（实际 0.5.1）→ 全部改 `<X.Y.Z>` 占位 |
| 🟡 易混 | BUILD-GUIDE §3.5 | 「代码签名证书」与「Tauri 更新签名」混为一谈，读者会以为没买证书所以更新坏了 |

**脚本层同类问题（已修）**：

- 写死路径 5 处（`run-app.ps1` ×2、`run-tests.ps1`、`run-tests.bat`、`build-release.ps1`）→ 全部改 `$PSScriptRoot` 自定位；签名私钥路径支持 `SENSEND_SIGN_KEY` 覆盖
- `run-tests.bat` 自己重写了一套环境加载（与 ps1 重复，且 `cd /d` 写死）→ 改为只转调 ps1，环境逻辑单一真源
- `build-release.ps1 -Upload` 用 `--notes $Notes` 直传中文会乱码 → 改 `--notes-file` + UTF-8 无 BOM

**沉淀的判据**：

1. 文档里出现「最快 / 最常用 / 自动」这类形容词，必须核对它背后**实际执行的命令**；形容词和命令对不上，就是下一次事故的引信。
2. 「能编译」≠「功能在程序里」。样式类配置改完必须验产物（`grep dist/`）；发布类改动必须验 Release 资产列表。
3. 测试环境只能用 `.dev` 身份。任何新增的启动脚本都要显式带 `--config src-tauri/tauri.dev.conf.json`。
