<script setup lang="ts">
import { ref, watch, nextTick, onMounted } from 'vue'
import type { InstanceItem } from '../types/platform'

// ═══ 平台实例选择列表 —— 底栏目标弹窗 / 编辑器 @ 列表的唯一格式真源 ═══
// 行格式（图标锚点 + 名字 + 来源右对齐 + 当前目标 ✓）、键盘上下/回车导航、
// 悬停态、滚动跟随、空态、图标映射，均只在此处定义；消费方只负责定位与选后行为。

const props = withDefaults(defineProps<{
  items: InstanceItem[]
  activeId?: string | null   // 当前正在使用的目标（✓ + 淡绿底）
  maxHeight?: string         // 弹窗最大高度（定位策略由消费方决定）
}>(), {
  activeId: null,
  maxHeight: 'var(--menu-max-h)',
})

const emit = defineEmits<{
  select: [item: InstanceItem]
}>()

const selectedIndex = ref(0)
const hoverIndex = ref<number | null>(null)
const listRef = ref<HTMLElement | null>(null)

/** [m-21] 打开时把高亮定位到「当前目标」而不是永远第一项：
 *  用户打开列表就是为了确认/改当前目标，光标落在第一项会让人多按一次方向键。 */
function syncSelectedToActive() {
  const idx = props.activeId ? props.items.findIndex(i => i.id === props.activeId) : -1
  selectedIndex.value = idx >= 0 ? idx : 0
}

watch(() => props.items, () => {
  syncSelectedToActive()
  hoverIndex.value = null
}, { immediate: true })

// [m-21] activeId 也会自己变：编辑器里改 @mention 目标会同步底栏当前目标（App.vue 的
// onMentionChange）。只盯 items 的话，重开列表时高亮仍停在上次鼠标点的那项，
// 一按回车就选错目标。两项都要跟。
watch(() => props.activeId, () => {
  syncSelectedToActive()
}, { immediate: true })

// ── 平台图标字形（唯一规则源：key → 字形；底色在 themes/*.css 的 --plat-* token）──
// 字形缺省取类型 key 首字母大写；'folder' 渲染文件夹 SVG
const PLAT_GLYPHS: Record<string, string> = {
  local: 'folder',
  notion: 'N',
  flowus: 'F',
  lark: '飞',
}
function platGlyph(type: string): string {
  return PLAT_GLYPHS[type] ?? type.charAt(0).toUpperCase()
}

// 键盘选中项自动滚动到可见区域
async function scrollSelectedIntoView() {
  await nextTick()
  if (!listRef.value || hoverIndex.value !== null) return
  const selected = listRef.value.querySelector('.inst-item.is-selected') as HTMLElement
  if (selected) selected.scrollIntoView({ block: 'nearest' })
}
watch(selectedIndex, () => { void scrollSelectedIntoView() })

// [m-21] 首次挂载补一次滚动：高亮在 setup 阶段就由 immediate 的 watch 定好了，
// selectedIndex 之后不再变化 → 上面的 watch 不会触发。实例超过一屏（默认 8 行）时，
// 打开弹窗看不到高亮，按一下 ↓ 才突然跳过去。
onMounted(() => { void scrollSelectedIntoView() })

function onKeyDown(event: KeyboardEvent): boolean {
  if (props.items.length === 0) return false
  hoverIndex.value = null // 键盘操作时清除悬停态
  if (event.key === 'ArrowUp') {
    selectedIndex.value = (selectedIndex.value + props.items.length - 1) % props.items.length
    return true
  }
  if (event.key === 'ArrowDown') {
    selectedIndex.value = (selectedIndex.value + 1) % props.items.length
    return true
  }
  if (event.key === 'Enter') {
    selectItem(hoverIndex.value ?? selectedIndex.value)
    return true
  }
  return false
}

function selectItem(index: number) {
  const item = props.items[index]
  if (item) emit('select', item)
}

/** [m-21] Tab 焦点进入某行时，把「键盘选中」同步到该行：
 *  列表里只有一个**选中态**（.is-selected 的 hover 底 + 内描边），
 *  而「焦点在哪」由 base.css 的 `button:focus-visible` 实色环给出（≥3:1，看得见）。
 *  注意：这里**不能**写 outline:none —— scoped 选择器特异性高于全局 :focus-visible，
 *  会把键盘焦点提示整个抹掉（本行原来的 bug）。 */
function onItemFocus(index: number) {
  hoverIndex.value = null
  selectedIndex.value = index
}

defineExpose({ onKeyDown })
</script>

<template>
  <div class="inst-menu" ref="listRef" v-if="items.length > 0" :style="{ maxHeight }">
    <button
      v-for="(item, index) in items"
      :key="item.id"
      class="inst-item"
      :class="{
        'is-selected': index === selectedIndex && hoverIndex === null,
        active: item.id === activeId,
      }"
      @click="selectItem(index)"
      @focus="onItemFocus(index)"
      @mouseenter="hoverIndex = index"
      @mouseleave="hoverIndex = null"
      :aria-label="item.label"
    >
      <span class="inst-icon" :class="`p-${item.platform_type}`">
        <svg v-if="platGlyph(item.platform_type) === 'folder'" width="12" height="12"
          viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
        </svg>
        <template v-else>{{ platGlyph(item.platform_type) }}</template>
      </span>
      <span class="inst-name">{{ item.name }}</span>
      <span class="inst-type">
        {{ item.typeName }}
        <svg v-if="item.id === activeId" class="inst-check" width="12" height="12"
          viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="20 6 9 17 4 12"/>
        </svg>
      </span>
    </button>
  </div>
  <div class="inst-menu inst-empty" v-else :style="{ maxHeight }">
    无匹配平台
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
/* 键盘选中：hover 底 + 内描边焦点环；与"当前目标"（文字色 + ✓）彻底分开 */
.inst-item.is-selected {
  box-shadow: var(--shadow-ring-inset);
}
/* 当前目标：只改文字色 + 对勾，不铺整行底色（方案 P2-4） */
.inst-item.active .inst-name {
  color: var(--accent-strong);
}

/* ── 平台图标：底色来自主题 --plat-* token，未知类型自动落 --plat-default ── */
.inst-icon {
  width: 20px;
  height: 20px;
  border-radius: var(--radius-icon);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--text-2xs);
  font-weight: 700;
  line-height: 1;
  color: var(--plat-glyph);
  background: var(--plat-default);
}
.inst-icon.p-flowus { background: var(--plat-flowus); }
.inst-icon.p-notion { background: var(--plat-notion); }
.inst-icon.p-lark   { background: var(--plat-lark); }
.inst-icon.p-local  { background: var(--plat-local); }

.inst-name {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
}
.inst-type {
  margin-left: auto;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: var(--space-4);
  font-size: var(--text-2xs);
  font-weight: 500;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
}
.inst-item.active .inst-type {
  color: var(--accent-strong);
  font-weight: 600;
}
.inst-check {
  color: var(--accent-strong);
  flex-shrink: 0;
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
