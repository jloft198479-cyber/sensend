<script setup lang="ts">
import { platGlyph } from '../types/platform'

// ═══ [ZC-11/ZC-13] 平台图标唯一渲染点 ═══
// 四平台统一内联 SVG 线稿（同一视觉重量、stroke 2px），禁字母/汉字混排；
// 底色来自 themes/*.css 的 --plat-*（品牌身份色，与主题主色无关）。
// 未知平台兜底显示 key 首字母（字形规则在 types/platform.ts）。
withDefaults(defineProps<{
  platformType: string
  size?: 'md' | 'sm'
}>(), {
  size: 'md',
})
</script>

<template>
  <span class="target-icon" :class="[`p-${platformType}`, `s-${size}`]" aria-hidden="true">
    <!-- 本地文件夹 -->
    <svg v-if="platGlyph(platformType) === 'folder'" width="12" height="12" viewBox="0 0 24 24"
      fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
    </svg>
    <!-- Notion：文档 -->
    <svg v-else-if="platformType === 'notion'" width="12" height="12" viewBox="0 0 24 24"
      fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <rect x="4" y="3" width="16" height="18" rx="2"/>
      <path d="M8 8h8M8 12h8M8 16h5"/>
    </svg>
    <!-- FlowUs：知识块嵌套 -->
    <svg v-else-if="platformType === 'flowus'" width="12" height="12" viewBox="0 0 24 24"
      fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <rect x="4" y="4" width="16" height="16" rx="3"/>
      <rect x="9" y="9" width="6" height="6" rx="1"/>
    </svg>
    <!-- 飞书：纸飞机（发往远方） -->
    <svg v-else-if="platformType === 'lark'" width="12" height="12" viewBox="0 0 24 24"
      fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M22 2 11 13"/>
      <path d="M22 2 15 22l-4-9-9-4z"/>
    </svg>
    <!-- 未知平台：key 首字母兜底 -->
    <template v-else>{{ platGlyph(platformType) }}</template>
  </span>
</template>

<style scoped>
.target-icon {
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
.target-icon.s-sm {
  width: 16px;
  height: 16px;
  border-radius: var(--radius-xs);
}
.target-icon.p-flowus { background: var(--plat-flowus); }
.target-icon.p-notion { background: var(--plat-notion); }
.target-icon.p-lark   { background: var(--plat-lark); }
.target-icon.p-local  { background: var(--plat-local); }
</style>
