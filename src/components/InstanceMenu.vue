<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted } from 'vue'
import type { InstanceItem } from '../types/platform'
import TargetIcon from './TargetIcon.vue'

// ═══ 平台实例选择列表 —— 底栏目标弹窗 / 编辑器 @ 列表的唯一格式真源 ═══
// 行格式（图标锚点 + 名字 + 来源右对齐 + 当前目标 ✓）、键盘上下/回车导航、
// 悬停态、滚动跟随、空态、图标映射，均只在此处定义；消费方只负责定位与选后行为。

const props = withDefaults(defineProps<{
  items: InstanceItem[]
  activeId?: string | null   // 当前正在使用的目标（✓ + 淡绿底）
  maxHeight?: string         // 弹窗最大高度（定位策略由消费方决定）
  query?: string             // [ZC-09] 当前搜索词：命中字符高亮；空态据此区分"搜不到"与"没配置"
}>(), {
  activeId: null,
  maxHeight: 'var(--menu-max-h)',
  query: '',
})

const emit = defineEmits<{
  select: [item: InstanceItem]
}>()

const selectedIndex = ref(0)
const listRef = ref<HTMLElement | null>(null)

// [ZC-11] 当前目标置顶（用户打开列表首先要确认"现在发给谁"），其余维持录入顺序。
// "最近使用"排序需要使用记录数据源，暂缺（见任务书），先按录入顺序。
const orderedItems = computed(() => {
  if (!props.activeId) return props.items
  const cur = props.items.find(i => i.id === props.activeId)
  if (!cur) return props.items
  return [cur, ...props.items.filter(i => i.id !== props.activeId)]
})

/** [m-21] 打开时把高亮定位到「当前目标」而不是永远第一项：
 *  用户打开列表就是为了确认/改当前目标，光标落在第一项会让人多按一次方向键。
 *  [ZC-11] 当前目标已置顶，定位到第 0 项即它；无 activeId 时也定位 0。 */
function syncSelectedToActive() {
  const idx = props.activeId
    ? orderedItems.value.findIndex(i => i.id === props.activeId)
    : -1
  selectedIndex.value = idx >= 0 ? idx : 0
}

watch(() => props.items, () => {
  syncSelectedToActive()
}, { immediate: true })

// [m-21] activeId 也会自己变：编辑器里改 @mention 目标会同步底栏当前目标（App.vue 的
// onMentionChange）。只盯 items 的话，重开列表时高亮仍停在上次鼠标点的那项，
// 一按回车就选错目标。两项都要跟。
watch(() => props.activeId, () => {
  syncSelectedToActive()
}, { immediate: true })

// ── 平台图标字形 [ZC-10] 已上收 types/platform.ts 单一源，此处仅消费 ──

// 键盘选中项自动滚动到可见区域
async function scrollSelectedIntoView() {
  await nextTick()
  if (!listRef.value) return
  const selected = listRef.value.querySelector('.inst-item.is-selected') as HTMLElement
  if (selected) selected.scrollIntoView({ block: 'nearest' })
}
watch(selectedIndex, () => { void scrollSelectedIntoView() })

// [m-21] 首次挂载补一次滚动：高亮在 setup 阶段就由 immediate 的 watch 定好了，
// selectedIndex 之后不再变化 → 上面的 watch 不会触发。实例超过一屏（默认 8 行）时，
// 打开弹窗看不到高亮，按一下 ↓ 才突然跳过去。
onMounted(() => { void scrollSelectedIntoView() })

function onKeyDown(event: KeyboardEvent): boolean {
  if (orderedItems.value.length === 0) return false
  if (event.key === 'ArrowUp') {
    selectedIndex.value = (selectedIndex.value + orderedItems.value.length - 1) % orderedItems.value.length
    return true
  }
  if (event.key === 'ArrowDown') {
    selectedIndex.value = (selectedIndex.value + 1) % orderedItems.value.length
    return true
  }
  if (event.key === 'Enter' || event.key === ' ') {
    // [ZC-07] 统一 preventDefault：否则 button 原生 Enter/Space click 与外层 keydown
    // 各触发一次选中（双次发送的来源）；高亮只有 selectedIndex 一套，回车选的就是它
    event.preventDefault()
    selectItem(selectedIndex.value)
    return true
  }
  return false
}

/** [ZC-07] 悬停 = 意图选中：只在鼠标真正移动（mousemove）时把 selectedIndex 同步过来。
 *  此前是 mouseenter/mouseleave 维护一套独立的 hoverIndex，与键盘 selectedIndex 两套高亮
 *  打架——键盘停在第 3 行、鼠标恰好滑过第 1 行时按回车，选中的是 hover 行且界面看不出来。
 *  现在高亮永远只有一个，且与回车将要选中的那一项一致（"最后一次有效输入"为准）。
 *  mouseleave 不再清高亮：离开列表后高亮留在最后悬停项，仍是"最后一次输入"。 */
function onItemHover(index: number) {
  if (selectedIndex.value !== index) selectedIndex.value = index
}

function selectItem(index: number) {
  const item = orderedItems.value[index]
  if (item) emit('select', item)
}

/** [ZC-09] 把 text 按 query 命中位置切成分段数组：命中段标 hit=true，模板里包 <mark>。
 *  大小写不敏感（与 items 的 toLowerCase 筛选口径一致）；query 为空或不在 text 里
 *  返回单段原文。搜索串是多字段拼接（名字/类型/key），这里对每个字段单独找命中。 */
function splitHit(text: string, q: string): Array<{ t: string; hit: boolean }> {
  if (!q) return [{ t: text, hit: false }]
  const lower = text.toLowerCase()
  const ql = q.toLowerCase()
  const out: Array<{ t: string; hit: boolean }> = []
  let i = 0
  while (i <= text.length) {
    const idx = lower.indexOf(ql, i)
    if (idx < 0) {
      out.push({ t: text.slice(i), hit: false })
      break
    }
    if (idx > i) out.push({ t: text.slice(i, idx), hit: false })
    out.push({ t: text.slice(idx, idx + q.length), hit: true })
    i = idx + q.length
  }
  return out
}

/** [m-21] Tab 焦点进入某行时，把「键盘选中」同步到该行：
 *  列表里只有一个**选中态**（.is-selected 的 hover 底 + 内描边），
 *  而「焦点在哪」由 base.css 的 `button:focus-visible` 实色环给出（≥3:1，看得见）。
 *  注意：这里**不能**写 outline:none —— scoped 选择器特异性高于全局 :focus-visible，
 *  会把键盘焦点提示整个抹掉（本行原来的 bug）。 */
function onItemFocus(index: number) {
  selectedIndex.value = index
}

defineExpose({ onKeyDown })
</script>

<template>
  <div class="inst-menu" ref="listRef" v-if="orderedItems.length > 0" :style="{ maxHeight }">
    <button
      v-for="(item, index) in orderedItems"
      :key="item.id"
      class="inst-item"
      :class="{
        'is-selected': index === selectedIndex,
        'is-current': item.id === activeId,
      }"
      @click="selectItem(index)"
      @focus="onItemFocus(index)"
      @mousemove="onItemHover(index)"
      :aria-label="item.label"
    >
      <!-- [ZC-11] 当前目标：行首 2px 竖条（::before）+「当前」小标签，删除原行尾 ✓（与类型文字挤在一起读不出来） -->
      <TargetIcon :platform-type="item.platform_type" />
      <span class="inst-name">
        <template v-for="(seg, si) in splitHit(item.name, props.query)" :key="si">
          <mark v-if="seg.hit" class="inst-hit">{{ seg.t }}</mark>
          <template v-else>{{ seg.t }}</template>
        </template>
      </span>
      <span v-if="item.id === activeId" class="current-tag">当前</span>
      <span class="type-badge">
        <template v-for="(seg, si) in splitHit(item.typeName, props.query)" :key="si">
          <mark v-if="seg.hit" class="inst-hit">{{ seg.t }}</mark>
          <template v-else>{{ seg.t }}</template>
        </template>
      </span>
    </button>
  </div>
  <div class="inst-menu inst-empty" v-else :style="{ maxHeight }">
    <!-- [ZC-09] @ 场景（query 非空）零实例已被 allow 拦截不弹层，走到这里必然是"有实例但搜不到"；
         底栏目标弹窗（query 空）维持原文案 -->
    <template v-if="props.query">没有匹配「{{ props.query }}」的目标</template>
    <template v-else>无匹配平台</template>
  </div>
</template>

<style scoped>
/* ═══ 列表容器 ═══ */
.inst-menu {
  width: max-content;
  min-width: 220px;
  max-width: 280px;
  overflow-y: auto;
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  /* 上下留白不用 padding（会随内容滚走），改用下方 sticky 占位条 */
  padding: 0 var(--menu-pad);
  box-shadow: var(--shadow-e2);
  font-family: var(--font-sans);
  z-index: var(--z-dropdown);
}
/* 滚动时上下留白钉在边缘：内容从占位条下方滑过，永不贴边 */
.inst-menu::before,
.inst-menu::after {
  content: '';
  display: block;
  position: sticky;
  height: var(--menu-pad-y);
  background: var(--bg);
  z-index: var(--z-sticky);
}
.inst-menu::before {
  top: 0;
  border-radius: var(--radius-md) var(--radius-md) 0 0;
}
.inst-menu::after {
  bottom: 0;
  border-radius: 0 0 var(--radius-md) var(--radius-md);
}
/* 空态自带内边距，不需要占位条 */
.inst-empty::before,
.inst-empty::after { display: none; }
.inst-empty {
  padding: var(--space-8) var(--space-10);
  font-size: var(--text-xs);
  color: var(--muted);
  white-space: nowrap;
}

/* ═══ 行 ═══ */
.inst-item {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-8);
  width: 100%;
  height: var(--menu-row-h);
  padding: 0 var(--space-8);
  margin: 0;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  /* button 不继承父级字体（UA 行为），显式拉回容器声明的字体栈 */
  font-family: inherit;
  font-size: var(--text-sm);
  color: var(--fg);
  cursor: pointer;
  white-space: nowrap;
  text-align: left;
  /* 不再 outline:none —— 那会把全局 :focus-visible 的实色焦点环压掉（scoped 特异性更高），
     键盘用户 Tab 进来后完全看不出焦点在哪。焦点环走 base.css 统一规则。 */
  transition: background var(--dur-fast);
}
.inst-item:hover,
.inst-item.is-selected {
  background: var(--bg-hover);
}
/* 键盘选中：hover 底 + 内描边焦点环；与"当前目标"（竖条 + 标签）彻底分开 */
.inst-item.is-selected {
  box-shadow: var(--shadow-ring-inset);
}
/* [ZC-11] 当前目标：行首 2px 实色竖条（--focus-ring）+ 名字强调，替代原行尾 ✓ */
.inst-item.is-current .inst-name {
  color: var(--accent-strong);
}
.inst-item.is-current::before {
  content: '';
  position: absolute;
  left: var(--space-2);
  top: 50%;
  transform: translateY(-50%);
  width: 2px;
  height: var(--menu-row-h);
  max-height: 14px;
  border-radius: var(--radius-xs);
  background: var(--focus-ring);
}
/* 「当前」小标签：比 ✓ 语义明确（"这是现在正用的"），不与类型文字挤在一起 */
.current-tag {
  flex-shrink: 0;
  font-size: var(--text-2xs);
  font-weight: 600;
  line-height: 1;
  color: var(--accent-strong);
  background: var(--accent-light);
  border-radius: var(--radius-xs);
  padding: var(--space-2) var(--space-4);
}
.inst-name {
  flex: 1 1 auto;
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
}

/* [ZC-09] 搜索命中高亮：底/字走主题 token（覆盖 mark 的 UA 默认黄底） */
.inst-hit {
  background: var(--accent-light);
  color: var(--accent-strong);
  border-radius: var(--radius-xs);
  font-weight: 600;
}

/* 滚动条 */
.inst-menu::-webkit-scrollbar {
  width: 4px;
}
.inst-menu::-webkit-scrollbar-track {
  background: transparent;
}
.inst-menu::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: var(--radius-full);
}
.inst-menu::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}
</style>
