<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'

const props = defineProps<{
  items: any[]
  command: (item: any) => void
}>()

const selectedIndex = ref(0)
const hoverIndex = ref<number | null>(null)
const listRef = ref<HTMLElement | null>(null)

watch(() => props.items, () => {
  selectedIndex.value = 0
  hoverIndex.value = null
})

// 键盘选中项自动滚动到可见区域
watch(selectedIndex, async () => {
  await nextTick()
  if (!listRef.value || hoverIndex.value !== null) return
  const selected = listRef.value.querySelector('.sensend-mention-item.is-selected') as HTMLElement
  if (selected) selected.scrollIntoView({ block: 'nearest' })
})

function onKeyDown(event: KeyboardEvent) {
  // 键盘操作时清除悬停态
  hoverIndex.value = null
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
  if (item) props.command(item)
}

defineExpose({ onKeyDown })
</script>

<template>
  <div class="sensend-mention-list" ref="listRef" v-if="items.length > 0">
    <button
      v-for="(item, index) in items"
      :key="item.id"
      class="sensend-mention-item"
      :class="{ 'is-selected': index === selectedIndex && hoverIndex === null }"
      @click="selectItem(index)"
      @mouseenter="hoverIndex = index"
      @mouseleave="hoverIndex = null"
      :aria-label="item.label"
    >
      <span class="sensend-mention-name">{{ item.name }}</span>
      <span class="sensend-mention-sep">—</span>
      <span class="sensend-mention-type">{{ item.typeName }}</span>
    </button>
  </div>
  <div class="sensend-mention-list sensend-mention-empty" v-else>
    无匹配平台
  </div>
</template>

<style>
/* ═══ @建议列表 — 与底栏下拉统一风格 ═══ */
.sensend-mention-list {
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 4px;
  width: max-content;
  min-width: 100px;
  max-width: 280px;
  overflow-y: auto;
  max-height: var(--mention-list-max-h, none);
  box-shadow: 0 -4px 16px rgba(0,0,0,0.08), 0 -1px 4px rgba(0,0,0,0.04);
  font-family: var(--font-editor);
  z-index: 99999;
}

.sensend-mention-empty {
  padding: 8px 10px;
  font-size: 11px;
  color: var(--muted);
  white-space: nowrap;
}

.sensend-mention-item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 5px 10px;
  margin: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  font-size: 12px;
  color: var(--fg);
  cursor: pointer;
  white-space: nowrap;
  text-align: left;
  outline: none;
  transition: background 0.12s ease;
}

.sensend-mention-item:hover,
.sensend-mention-item.is-selected {
  background: var(--accent-light);
}

.sensend-mention-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
}

.sensend-mention-sep {
  font-size: 12px;
  color: var(--muted);
  flex-shrink: 0;
}

.sensend-mention-type {
  font-size: 12px;
  font-weight: 500;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 滚动条 */
.sensend-mention-list::-webkit-scrollbar {
  width: 4px;
}
.sensend-mention-list::-webkit-scrollbar-track {
  background: transparent;
}
.sensend-mention-list::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: 2px;
}
.sensend-mention-list::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}
</style>