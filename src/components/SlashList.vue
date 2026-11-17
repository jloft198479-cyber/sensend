<script setup lang="ts">
import { ref, watch } from 'vue'
import type { SlashCommandItem } from '../composables/useSlashCommand'

// ═══ 斜杠命令弹层 ═══
// 与 useSlashCommand.ts 的契约：items 渲染 + command 回传 + onKeyDown 键盘桥接。
// 视觉配方对齐 InstanceMenu（同一套 token），但不复用组件——
// InstanceMenu 是平台实例列表的格式真源，不掺命令列表。
const props = defineProps<{
  items: SlashCommandItem[]
  command: (item: SlashCommandItem) => void
}>()

const selectedIndex = ref(0)

// [ZC-07] 键鼠单一高亮（与 InstanceMenu 同配方）：mousemove 即意图选中，
// 回车执行的永远是「看得见的那个高亮」；继续打字过滤时收回第一项
watch(() => props.items, () => { selectedIndex.value = 0 })

defineExpose({
  onKeyDown: (e: KeyboardEvent): boolean => {
    const count = props.items.length
    if (count === 0) return false
    if (e.key === 'ArrowUp') {
      selectedIndex.value = (selectedIndex.value + count - 1) % count
      return true
    }
    if (e.key === 'ArrowDown') {
      selectedIndex.value = (selectedIndex.value + 1) % count
      return true
    }
    if (e.key === 'Enter') {
      const item = props.items[selectedIndex.value]
      if (item) props.command(item)
      return true
    }
    return false
  },
})
</script>

<template>
  <div class="slash-menu">
    <button
      v-for="(item, i) in items"
      :key="item.id"
      class="slash-item"
      :class="{ selected: i === selectedIndex }"
      @mousemove="i !== selectedIndex && (selectedIndex = i)"
      @focus="selectedIndex = i"
      @click="props.command(item)"
    >
      <span class="slash-label">{{ item.label }}</span>
      <span class="slash-hint">{{ item.hint }}</span>
    </button>
    <div v-if="items.length === 0" class="slash-empty">没有匹配的命令</div>
  </div>
</template>

<style scoped>
.slash-menu {
  width: max-content;
  min-width: 220px;
  max-width: 280px;
  /* 限高与 @ 列表同一几何来源（--menu-row-h × --mention-max-rows + 上下留白）：
     超出滚动，小窗里末项也够得着；翻转交给 tippy 内置 flip */
  max-height: calc(var(--menu-row-h) * var(--mention-max-rows, 8) + var(--menu-pad-y) * 2);
  overflow-y: auto;
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: var(--menu-pad-y);
  box-shadow: var(--shadow-e2);
  font-family: var(--font-sans);
  z-index: var(--z-dropdown);
}
.slash-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
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
  transition: all var(--dur-fast) ease;
}
.slash-item:hover { background: var(--bg-hover); }
/* 键盘选中行与悬停行同底色，但加内描边拉开——「回车会执行哪条」永远只有一个可见答案 */
.slash-item.selected { background: var(--bg-hover); box-shadow: var(--shadow-ring-inset); }
.slash-hint { font-size: var(--text-xs); color: var(--muted); }
.slash-empty {
  padding: var(--space-8) var(--space-10);
  font-size: var(--text-xs);
  color: var(--muted);
  white-space: nowrap;
}
</style>
