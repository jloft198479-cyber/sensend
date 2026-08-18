# Sensend 用户体验与性能评估报告

> 版本：v0.4.0 ｜ 评估日期：2026-08-18
> 评估范围：UI 设计 / 交互设计 / 内存控制 / 响应速度与性能
> 配套报告：SEND-EVALUATION.md（发送格式保真度 + 发送链路质量）

---

## 一、评估方法

本报告从四个维度审视 Sensend 的用户体验与性能表现：

**UI 设计**——视觉体系、组件工艺、主题实现、窗口形态的完成度。

**交互设计**——操作链路、反馈机制、可达性、边界态处理、防重入守卫。

**内存控制**——资源生命周期、事件监听/定时器清理、进程模型开销。

**响应速度与性能**——启动链路、输入延迟、I/O 模式、构建产物体积、已做优化与剩余空间。

每个维度独立评分（满分 100），最后汇总综合建议。

---

## 二、UI 设计评估（88 分）

### 2.1 做得好的地方

**Design token 体系完整**：vars.css 把配色/字体/圆角/阴影/滚动条/动画时长全部收敛为 CSS 变量，语义分层清晰（bg/fg/accent/danger/gray 五组）。暗夜主题用 `[data-theme="dark"]` 整体覆盖，不是简单反色——accent 从 `#2CAF68` 提亮到 `#3dbd7e`（暗底需要更高亮度），边框改白色低透明度，背景用 `#1a1a1a` 而非纯黑，符合暗色设计规范。

**像素级克制**：主窗口 420×210、按钮统一 28px（`--btn-size`）、圆角分层（按钮 6px / 菜单 10px），与"极简轻盈"产品理念一致，没有过度设计。

**"目标"视觉语义统一**：编辑区 @mention 徽章、底栏 target-picker、MentionList 下拉三处用同一套绿色渐变+细边框样式，代码注释明确写"与底栏下拉统一风格"——用户扫一眼绿色就知道这是发送目标。

**字体系统三层分明**：UI 字体 DM Sans（本地 npm 包离线可用）、编辑器字体默认系统栈+可切换、品牌名 Segoe UI Semibold 斜体。字体菜单每一项用该字体自身渲染预览，细节到位。

**微反馈齐全**：保存状态条（saving 呼吸动画→saved 淡出）、发送按钮 spinner、Toast 进出场+move 动画、"✓ 已发送" pop 动画、菜单 fade，交互有呼吸感。

**无边框窗口处理规范**：拖拽区域递归标记并避开交互元素、关闭按钮 hover 变红、置顶按钮激活态有填充反馈。

### 2.2 发现的问题

| 编号 | 问题 | 严重度 | 位置 |
|------|------|:---:|------|
| UI-1 | 窗口底色与暗夜主题冲突：`backgroundColor: "#f5f5f5"` 硬编码浅色，暗夜模式唤起窗口时先闪一帧亮屏 | 中 | tauri.conf.json L28 |
| UI-2 | 底部弹出菜单无空间感知：FooterBar target-picker 无 max-height 限制，窗口仅 210px 高，实例多时菜单盖住编辑区 | 中 | FooterBar.vue L266-281 |
| UI-3 | font-menu 同样无 max-height，用户字体多时溢出窗口 | 低 | TitleBar.vue L311-324 |
| UI-4 | z-index 无统一刻度：99999/9999/100 混用，目前不重叠但属隐患 | 低 | 多处 |
| UI-5 | 键盘可访问性：多处 `outline: none` 后未补 `:focus-visible` 样式 | 低 | picker-item/settings-item/BubbleMenu |
| UI-6 | 未定义变量引用：`.font-menu`/`.settings-menu` 引用 `--font-ui`（vars.css 只定义了 `--font-sans`） | 信息 | TitleBar.vue / FooterBar.vue |

---

## 三、交互设计评估（90 分）

### 3.1 做得好的地方

**主链路极短且符合产品定位**：全局快捷键唤醒 → 记录 → Ctrl+Enter 发送 → Toast"发送成功"带"查看 ↗"按钮 + 编辑区"✓ 已发送"标记。三步完成"速记→扔出去"，与中转站定位完全吻合。

**mention ↔ 底栏双向同步**：单一 `resolvedTarget` 计算属性做解析入口（mention 优先于底栏默认），两个入口互相同步不分裂，选择结果实时记忆到后端 config.json。

**弹窗无障碍做到位**：HotkeyModal 有完整 focus trap（Tab/Shift+Tab 循环）、打开记住焦点/关闭恢复焦点、`role="dialog"` + `aria-modal` + `aria-labelledby`。录制快捷键时有 pulse 边框动画 + 闪烁文字提示，Esc 取消录制。

**MentionList 键鼠状态机干净**：方向键循环选择、Enter 确认、键盘选中项自动 scrollIntoView、键盘操作时主动清除 hover 态——鼠标和键盘不会互相打架。弹出方向根据剩余空间自动翻转。

**防重入守卫全覆盖**：发送（按钮 disabled + 快捷键拦截双保险）、保存实例（isSaving）、测试连接（isTesting），用户狂点不会出问题。

**配置窗口细节体贴**：授权码按平台记忆预填、测试成功后保存按钮 pulse 引导下一步、删除用内联二次确认而非原生弹窗、表单 canSave 校验精确到字段级。

**空态引导完整**：无实例时底栏显示"配置页面"引导按钮，配置窗口有空状态插图+提示文案。

### 3.2 发现的问题

| 编号 | 问题 | 严重度 | 位置 |
|------|------|:---:|------|
| IX-1 | 暗夜模式弹窗硬编码白色：HotkeyModal `.modal-panel { background: white }`、录制按钮 hover `background: white`、ConfigWindow `.form-input:focus { background: white }` | 高 | HotkeyModal.vue L123/L165, ConfigWindow.vue L348 |
| IX-2 | 带操作按钮的 Toast 消失太快：success 统一 3000ms，用户读到 Toast + 决定点击 + 移动鼠标，3 秒紧张 | 中 | useToast.ts L28 |
| IX-3 | 浮层关闭方式不统一：模态弹窗支持 Esc，但 font-menu/target-picker/settings-menu 只支持点击外部关闭 | 低 | 多处 |
| IX-4 | 快捷键保存失败提示是猜测式："请检查快捷键是否冲突"，后端注册失败具体原因未透传 | 低 | HotkeyModal.vue L102 |
| IX-5 | 发送中无进度/无取消：长文多批次时只有 spinner，无"第 N/M 批"反馈 | 低 | TitleBar.vue L134-146 |
| IX-6 | 底栏字数统计始终显示"N 字"，对不在意字数的用户是噪音 | 信息 | FooterBar.vue L159 |

---

## 四、内存控制评估（93 分）

### 4.1 做得好的地方

**事件监听全部成对清理**：全项目仅 3 处 document 级监听（FooterBar 外部点击、字体菜单外部点击、发送快捷键），全部在 `onBeforeUnmount` 配对 removeEventListener；Tauri 事件监听（instances-updated、app-exit-request、theme-updated）同样有 unlisten 清理。无 setInterval、无 Worker、无 Observer——没有长生命周期隐式持有。

**TipTap 编辑器正确销毁**：`onBeforeUnmount` 中 `editor.value?.destroy()`，ProseMirror 视图、插件、decoration 全部释放。

**tippy 实例生命周期完整**：Mention suggestion 的 `onExit` 中 `popup?.destroy()` + `component?.destroy()`（VueRenderer），并清理 CSS 变量残留。每次 @ 弹出/关闭不累积。

**定时器清理到位**：saveTimer、wordCountTimer 在卸载时清除，退出前也清一次。markSent 的 3 秒 setTimeout 未追踪，但回调内检查 `editor.isDestroyed`，无害。

**动态字体样式不累积**：`applyUserFonts` 先删旧 `#dynamic-font-faces` style 再注入新的，刷新字体列表不会叠加。

**配置窗口用完即毁**：config 窗口有关闭即销毁 WebView（Rust 侧不拦截其 CloseRequested），不常驻内存；主窗口关闭转隐藏是刻意的秒开设计。

**ConfigWindow 懒加载分包**：`import()` 动态导入 + Rollup 自动分包，主窗口不加载配置页代码。

**Rust 侧资源克制**：HTTP Client 全局单例（连接池复用）、飞书 token 缓存按 app_id 收敛、tokio 异步运行时无线程膨胀。单实例插件防止重复启动翻倍内存。

### 4.2 发现的问题与固有开销

| 编号 | 问题 | 严重度 | 说明 |
|------|------|:---:|------|
| MEM-1 | 隐藏主窗口常驻内存（WebView2 渲染进程常驻，估 60-100MB） | 设计取舍 | 悬浮记事本"秒开"的必要代价，Tauri 已比 Electron 轻一个量级 |
| MEM-2 | TOKEN_CACHE 无上限无淘汰 | 极低 | HashMap 只增不减，但实际 app_id 数量为个位数，不会膨胀 |
| MEM-3 | ProseMirror undo 历史无上限 | 极低 | 速记场景内容短、会话短，几乎无感 |
| MEM-4 | lastSavedContent 持有一份全文副本 | 合理 | 差异检测需要，速记场景 KB 级 |
| MEM-5 | DM Sans 加载 4 个字重 | 极致优化 | UI 主要用 500/600/700，400 可减 |

---

## 五、响应速度与性能评估（92 分）

### 5.1 实测数据

| 指标 | 数值 | 说明 |
|------|------|------|
| 前端 dist 总体积 | 969KB | 主 bundle 701KB + CSS 30KB + ConfigWindow 分包 11.5KB + 字体 190KB |
| Rust release 二进制 | 14.9MB | Tauri 应用正常水平（对比 Electron 80-150MB） |
| 后端测试执行 | 0.06s | 72 个测试，IR 转换为纯内存操作 |

### 5.2 已实施的系统性优化（v0.4.0）

代码中有完整 [OPT] 标记的三个方案：

**方案 1b 启动 IPC 并行化**：`get_platform_types` 和 `list_platform_instances` 从串行 await 改为并行发起，省一次 IPC 往返（约 20-50ms）。

**方案 2 Rust 命令异步化**：`read_note`/`save_note` 改用 tokio::fs，文件 I/O 不阻塞线程池，启动阶段多命令真正并行。

**方案 3 onUpdate 回调瘦身**（输入延迟的关键）：mention 检测走快路径（lastMentionId 缓存，普通打字不做 doc.descendants 全量遍历）；字数统计独立防抖 300ms（不再每次按键跑正则）；doSave 内容差异检测（撤销回原样不做无意义磁盘写入）。

### 5.3 响应速度链路

| 链路 | 延迟 | 说明 |
|------|------|------|
| 唤醒 | 毫秒级 | 全局快捷键 → 常驻窗口 show()，隐藏而非销毁的设计兑现价值 |
| 输入 | 无感 | TipTap/ProseMirror 原生渲染 + onUpdate 瘦身 |
| 保存 | 零阻塞 | 防抖 800ms + 差异检测 + tokio 异步原子写入 |
| 启动 | ~1s | visible:false 防白屏 → initTheme → mount → 并行 IPC |

### 5.4 剩余空间（锦上添花）

| 编号 | 项目 | 预估收益 |
|------|------|------|
| PERF-1 | `get_theme` 重复调用（main.ts + TitleBar 各一次） | ~10ms |
| PERF-2 | 字体双格式打包（woff + woff2 冗余，WebView2 只需 woff2） | ~75KB |
| PERF-3 | latin-ext 子集无用（中文用户永远用不到） | ~60KB |
| PERF-4 | 保存时全量序列化（速记体量无感，长文编辑器才需增量） | 微秒级 |

---

## 六、综合评分与优先建议

### 6.1 评分汇总

| 维度 | 评分 | 一句话 |
|------|:---:|------|
| UI 设计 | 88 | 设计语言统一、token 化彻底，扣分在暗夜弹窗硬编码与弹出层空间感知 |
| 交互设计 | 90 | 主链路精准、无障碍意识超前，扣分在暗夜弹窗白色与 Toast 生命周期 |
| 内存控制 | 93 | 清理纪律近乎完美，主要开销是 WebView2 常驻的合理取舍 |
| 响应速度与性能 | 92 | 三个优化方案方向精准，产物体积优秀，剩余空间都是 KB 级洁癖 |
| **综合** | **91** | |

### 6.2 优先修复建议（按影响排序）

| 优先级 | 项目 | 维度 | 预估工作量 |
|:---:|------|------|:---:|
| P1 | 暗夜模式弹窗白色硬编码（3 处 `background: white` → `var(--bg)`） | UI + 交互 | 小 |
| P1 | 暗夜模式窗口底色（tauri.conf.json backgroundColor 按主题动态设置） | UI | 小 |
| P2 | 底部弹出菜单加 max-height 限制（target-picker / font-menu） | UI | 小 |
| P2 | 带 action 的 Toast 延长到 5-6 秒或 hover 暂停倒计时 | 交互 | 小 |
| P2 | 浮层统一支持 Esc 关闭 | 交互 | 小 |
| P3 | z-index 统一刻度（建议 100/1000/10000 三级） | UI | 小 |
| P3 | 键盘焦点补 `:focus-visible` 样式 | UI | 中 |
| P3 | `get_theme` 去重（main.ts 与 TitleBar 共享） | 性能 | 小 |
| 极致 | 字体打包减 woff + 去 latin-ext（省 ~135KB） | 性能 | 小 |

### 6.3 总体评价

Sensend v0.4.0 在用户体验与性能方面展现了远超个人项目平均水准的工程成熟度。Design token 体系、focus trap、防重入守卫、事件清理纪律——这些在团队项目中都未必齐全的基础设施，在这里一一到位。v0.4.0 的三个 [OPT] 优化方案（启动并行、I/O 异步、输入路径瘦身）方向精准，实测产物体积不到 1MB。

主要短板集中在暗夜模式的工艺一致性上：弹窗硬编码白色、窗口底色不跟随主题——这是 v0.4.0 暗夜功能上线时遗漏的收尾工作，修复成本极低（3 行 CSS + 1 处配置）。其余问题（弹出层空间感知、Toast 生命周期、z-index 统一）都是"好→更好"的打磨项，不影响核心使用。

对"超轻量桌面悬浮记事本"这个产品形态，当前体验已触及合理上限。
