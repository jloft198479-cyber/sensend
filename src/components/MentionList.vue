<script setup lang="ts">
import { ref } from 'vue'
import InstanceMenu from './InstanceMenu.vue'

// ═══ TipTap 建议插件适配层 ═══
// 只保留与 useEditor.ts 的契约（onKeyDown 键盘桥接 + command 回传）；
// 列表格式、键盘导航、滚动全部来自共享组件 InstanceMenu，此处零样式。
const props = defineProps<{
  items: any[]
  command: (item: any) => void
}>()

const menuRef = ref<{ onKeyDown: (e: KeyboardEvent) => boolean } | null>(null)

function onSelect(item: any) {
  props.command(item)
}

defineExpose({
  onKeyDown: (e: KeyboardEvent) => menuRef.value?.onKeyDown(e) ?? false,
})
</script>

<template>
  <InstanceMenu
    ref="menuRef"
    :items="props.items"
    max-height="var(--mention-list-max-h, none)"
    @select="onSelect"
  />
</template>
