<script setup lang="ts">
import TargetIcon from './TargetIcon.vue'

// ═══ [ZC-13] 目标条目唯一渲染规则：图标 + 名字 + 类型徽标 ═══
// 消费方：底栏目标 chip（可关类型）、管理页当前目标卡。@ 列表行因行内有
// 高亮/选中态等自有布局，复用 TargetIcon + 全局 .type-badge 自行组合。
withDefaults(defineProps<{
  name: string
  platformType: string
  typeName: string
  showType?: boolean
  size?: 'md' | 'sm'
}>(), {
  showType: true,
  size: 'md',
})
</script>

<template>
  <span class="target-badge">
    <TargetIcon :platform-type="platformType" :size="size" />
    <span class="target-badge-name">{{ name }}</span>
    <span v-if="showType" class="type-badge">{{ typeName }}</span>
  </span>
</template>

<style scoped>
.target-badge {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  min-width: 0;
}
.target-badge-name {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
