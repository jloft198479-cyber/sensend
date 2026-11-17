<script setup lang="ts">
import { ref, watch, nextTick, onMounted } from 'vue'
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
const listRef = ref<HTMLElement | null>(null)

// [ZC-07] 键鼠单一高亮（与 InstanceMenu 同配方）：mousemove 即意图选中，
// 回车执行的永远是「看得见的那个高亮」；继续打字过滤时收回第一项
watch(() => props.items, () => { selectedIndex.value = 0 })

/** 键盘选中项自动滚到可见区。
 *  命令数超过限高（--mention-max-rows，8 行）时必须跟随：否则按 ↓ 走到第 9、10 条，
 *  高亮已滚出视野——看不见自己选中哪条，回车却会执行它。
 *  与 InstanceMenu.scrollSelectedIntoView 同一条规则，同一写法。 */
async function scrollSelectedIntoView() {
  await nextTick()
  const el = listRef.value?.querySelector('.slash-item.selected') as HTMLElement | null
  if (el) el.scrollIntoView({ block: 'nearest' })
}
watch(selectedIndex, () => { void scrollSelectedIntoView() })
// 首帧补一次：高亮在 setup 阶段就定好了，之后 selectedIndex 不变 → 上面的 watch 不触发
onMounted(() => { void scrollSelectedIntoView() })

/** [ZC-07] 悬停 = 意图选中：只在鼠标真正移动（mousemove）时同步。
 *  高亮永远只有一个，且与回车将要执行的那条一致（"最后一次有效输入"为准）。 */
function onItemHover(index: number) {
  if (selectedIndex.value !== index) selectedIndex.value = index
}

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
  <div class="slash-menu" ref="listRef" :class="{ 'is-empty': items.length === 0 }">
    <button
      v-for="(item, i) in items"
      :key="item.id"
      class="slash-item"
      :class="{ selected: i === selectedIndex }"
      @mousemove="onItemHover(i)"
      @focus="onItemHover(i)"
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
  max-height: calc(var(--menu-row-h) * var(--mention-max-rows) + var(--menu-pad-y) * 2);
  overflow-y: auto;
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  /* 上下留白不用 padding（滚动时会随内容滑走、首末行贴边框），改用 sticky 占位条——
     与 InstanceMenu 同配方，e1eac6b「@列表滚动留白」已踩过这个坑 */
  padding: 0 var(--menu-pad);
  box-shadow: var(--shadow-e2);
  font-family: var(--font-sans);
  z-index: var(--z-dropdown);
}
.slash-menu::before,
.slash-menu::after {
  content: '';
  display: block;
  position: sticky;
  height: var(--menu-pad-y);
  background: var(--bg);
  z-index: var(--z-sticky);
}
.slash-menu::before {
  top: 0;
  border-radius: var(--radius-md) var(--radius-md) 0 0;
}
.slash-menu::after {
  bottom: 0;
  border-radius: 0 0 var(--radius-md) var(--radius-md);
}
/* 空态自带内边距，不需要占位条 */
.slash-menu.is-empty::before,
.slash-menu.is-empty::after { display: none; }

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
  white-space: nowrap;
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

/* 滚动条与 @ 列表同一档（默认粗条会跟弹层气质打架） */
.slash-menu::-webkit-scrollbar {
  width: 4px;
}
.slash-menu::-webkit-scrollbar-track {
  background: transparent;
}
.slash-menu::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: var(--radius-full);
}
.slash-menu::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}
</style>
