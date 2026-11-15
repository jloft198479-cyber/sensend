<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
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
  maxHeight: '240px',
})

const emit = defineEmits<{
  select: [item: InstanceItem]
}>()

const selectedIndex = ref(0)
const hoverIndex = ref<number | null>(null)
const listRef = ref<HTMLElement | null>(null)

watch(() => props.items, () => {
  selectedIndex.value = 0
  hoverIndex.value = null
})

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
watch(selectedIndex, async () => {
  await nextTick()
  if (!listRef.value || hoverIndex.value !== null) return
  const selected = listRef.value.querySelector('.inst-item.is-selected') as HTMLElement
  if (selected) selected.scrollIntoView({ block: 'nearest' })
})

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
      @mouseenter="hoverIndex = index"
      @mouseleave="hoverIndex = null"
      :aria-label="item.label"
    >
      <span class="inst-icon" :class="`p-${item.platform_type}`">
        <svg v-if="platGlyph(item.platform_type) === 'folder'" width="12" height="12"
          viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
  border-radius: var(--btn-radius);
  padding: 5px;
  box-shadow: 0 -4px 16px rgba(0,0,0,0.08), 0 -1px 4px rgba(0,0,0,0.04);
  font-family: var(--font-editor);
  z-index: 99999;
}
.inst-empty {
  padding: 8px 10px;
  font-size: 11px;
  color: var(--muted);
  white-space: nowrap;
}

/* ═══ 行 ═══ */
.inst-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 8px;
  margin: 0;
  border: none;
  border-radius: var(--btn-radius);
  background: transparent;
  /* button 不继承父级字体（UA 行为），显式拉回容器声明的 --font-editor */
  font-family: inherit;
  font-size: 12.5px;
  color: var(--fg);
  cursor: pointer;
  white-space: nowrap;
  text-align: left;
  outline: none;
  transition: background 0.12s;
}
.inst-item:hover {
  background: var(--bg-hover);
}
/* 键盘选中（is-selected）与当前目标（active）同用主色淡底 */
.inst-item.is-selected,
.inst-item.active {
  background: var(--accent-light);
}

/* ── 平台图标：底色来自主题 --plat-* token，未知类型自动落 --plat-default ── */
.inst-icon {
  width: 20px;
  height: 20px;
  border-radius: 5px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
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
  font-size: 12.5px;
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
  gap: 3px;
  font-size: 11px;
  font-weight: 500;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
}
.inst-item.active .inst-type {
  color: var(--accent);
  font-weight: 600;
}
.inst-check {
  color: var(--accent);
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
  border-radius: 2px;
}
.inst-menu::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}
</style>
