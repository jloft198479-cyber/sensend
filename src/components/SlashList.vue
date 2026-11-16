<script setup lang="ts">
import { ref, watch } from 'vue'

// ═══ 斜杠命令弹层 ═══
// 与 useSlashCommand.ts 的契约：items 渲染 + command 回传 + onKeyDown 键盘桥接。
// 视觉配方对齐 InstanceMenu（同一套 token），但不复用组件——
// InstanceMenu 是平台实例列表的格式真源，不掺命令列表。
const props = defineProps<{
  items: Array<{ id: string; label: string; hint: string }>
  command: (item: any) => void
}>()

const selectedIndex = ref(0)

// 继续打字过滤时高亮收回第一项
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
  <div class="slash-menu" role="listbox" aria-label="斜杠命令">
    <button
      v-for="(item, i) in items"
      :key="item.id"
      class="slash-item"
      :class="{ selected: i === selectedIndex }"
      role="option"
      :aria-selected="i === selectedIndex"
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
.slash-item:hover,
.slash-item.selected { background: var(--bg-hover); }
.slash-hint { font-size: var(--text-xs); color: var(--muted); }
.slash-empty {
  padding: var(--space-8) var(--space-10);
  font-size: var(--text-xs);
  color: var(--muted);
  white-space: nowrap;
}
</style>
