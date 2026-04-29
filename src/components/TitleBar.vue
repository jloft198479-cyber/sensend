<script setup lang="ts">
import { ref, onMounted } from 'vue'

interface FontOption {
  value: string
  label: string
}

defineProps<{
  isPinned: boolean
  isSending: boolean
  showFontMenu: boolean
  fontOptions: FontOption[]
  currentFont: string
  saveStatus: 'idle' | 'saved' | 'saving' | 'unsaved'
}>()

defineEmits<{
  'toggle-pin': []
  'publish': []
  'hide': []
  'toggle-font-menu': []
  'select-font': [opt: FontOption]
  'add-font': []
  'open-config': []
}>()

// ── 拖拽区域：递归标记所有非交互子元素 ──
const titlebarRef = ref<HTMLElement | null>(null)

onMounted(() => {
  const container = titlebarRef.value
  if (!container) return

  const applyDrag = (el: HTMLElement) => {
    if (el.closest('.title-actions')) return
    if (el.hasAttribute('data-no-drag')) return
    el.setAttribute('data-tauri-drag-region', '')
    Array.from(el.children).forEach((child) => {
      if (child instanceof HTMLElement) applyDrag(child)
    })
  }
  applyDrag(container)
})
</script>

<template>
  <header ref="titlebarRef" class="titlebar">
    <div class="title-left">
      <span class="title">Sensend</span>
      <Transition name="fade">
        <span v-if="saveStatus === 'saving'" class="save-status saving" key="saving">
          <span class="saving-dots">保存中</span>
        </span>
        <span v-else-if="saveStatus === 'saved'" class="save-status saved" key="saved">已保存</span>
      </Transition>
    </div>
    <div class="title-actions">
      <!-- 字体切换 -->
      <div class="font-picker">
        <button class="action-btn" @click.stop="$emit('toggle-font-menu')" title="切换字体" aria-label="切换字体">
          <span class="font-icon">T</span>
        </button>
        <div v-if="showFontMenu" class="font-menu" @click.stop>
          <div class="font-menu-title">编辑器字体</div>
          <button
            v-for="opt in fontOptions"
            :key="opt.value"
            class="font-menu-item"
            :class="{ active: currentFont === opt.value }"
            @click="$emit('select-font', opt)"
          >
            <span :style="{ fontFamily: opt.value }">{{ opt.label }}</span>
            <svg v-if="currentFont === opt.value" width="14" height="14" viewBox="0 0 24 24"
              fill="none" stroke="var(--accent)" stroke-width="2.5" stroke-linecap="round">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
          </button>
          <div class="font-menu-divider"></div>
          <button class="font-menu-item font-menu-action" @click="$emit('add-font')" aria-label="管理字体">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
              stroke-width="2" stroke-linecap="round">
              <path d="M12 5v14M5 12h14"/>
            </svg>
            <span>管理字体…</span>
          </button>
        </div>
      </div>

      <!-- 置顶 -->
      <button class="action-btn" :class="{ 'pin-active': isPinned }"
        @click.stop="$emit('toggle-pin')" :title="isPinned ? '取消置顶' : '始终置顶'" :aria-label="isPinned ? '取消置顶' : '始终置顶'">
        <svg width="14" height="14" viewBox="-1 -1 26 26" fill="none" stroke="currentColor"
          stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
          style="transform: rotate(45deg);">
          <line x1="12" y1="17" x2="12" y2="22"/>
          <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"/>
        </svg>
      </button>

      <!-- 发送 -->
      <button class="send-btn" :disabled="isSending"
        @click.stop="$emit('publish')"
        title="发送到目标平台" aria-label="发送到目标平台">
        <svg v-if="!isSending" width="14" height="14" viewBox="0 0 24 24" fill="none"
          stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <line x1="22" y1="2" x2="11" y2="13"/>
          <polygon points="22 2 15 22 11 13 2 9 22 2"/>
        </svg>
        <svg v-else class="spin-icon" width="14" height="14" viewBox="0 0 24 24" fill="none"
          stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 11-6.219-8.56"/>
        </svg>
      </button>

      <!-- 分隔：功能组 / 窗口组 -->
      <span class="action-sep"></span>

      <!-- 最小化到托盘 -->
      <button class="action-btn close-btn" @click.stop="$emit('hide')" title="最小化到托盘" aria-label="最小化到托盘">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2.5" stroke-linecap="round">
          <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>
  </header>
</template>

<style scoped>
/* ═══ 顶栏 ═══ */
.titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 10px 10px 14px;
  height: 44px;
  cursor: default;
  user-select: none;
  flex-shrink: 0;
  border-bottom: none;
  box-shadow: 0 1px 0 rgba(0, 0, 0, 0.04);
  background: var(--accent-light);
}
.title-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.title {
  font-family: var(--font-mono);
  font-size: 17px;
  font-weight: 700;
  font-style: italic;
  color: var(--accent);
}
.save-status {
  font-size: 11px;
  opacity: 1;
  transition: opacity 0.4s ease;
}
.save-status.saved { color: var(--accent); }
.save-status.saving { color: var(--muted); }

/* fade 过渡（Vue Transition） */
.fade-enter-active { transition: all 0.25s ease-out; }
.fade-leave-active { transition: all 0.4s ease-in; }
.fade-enter-from { opacity: 0; }
.fade-leave-to { opacity: 0; }

.saving-dots {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}
.saving-dots::after {
  content: '';
  display: inline-block;
  width: 12px;
  height: 3px;
  background: currentColor;
  border-radius: 1.5px;
  animation: saving-bar 0.9s ease-in-out infinite;
}

.title-actions {
  display: flex;
  align-items: center;
  gap: var(--btn-gap);
}
.action-sep {
  width: 1px;
  height: 16px;
  background: var(--border-color);
  margin: 0 calc(var(--btn-gap) + 2px);
}

/* 功能按钮 */
.action-btn {
  width: var(--btn-size);
  height: var(--btn-size);
  border: none;
  background: transparent;
  color: var(--fg-secondary);
  cursor: pointer;
  border-radius: var(--btn-radius);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--transition-fast);
}
.action-btn:hover {
  background: var(--bg-hover);
  color: var(--fg);
}
.action-btn.pin-active {
  color: var(--accent);
}
.action-btn.pin-active svg path {
  fill: currentColor;
  fill-opacity: 0.15;
}
.action-btn.pin-active:hover {
  background: var(--accent-light);
}

/* 关闭按钮特殊 hover */
.close-btn:hover {
  background: var(--danger-bg) !important;
  color: var(--danger) !important;
}

/* 发送按钮 */
.send-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  width: var(--btn-size);
  height: var(--btn-size);
  border: none;
  background: var(--accent);
  color: white;
  cursor: pointer;
  border-radius: var(--btn-radius);
  margin-left: 4px;
  transition: all var(--transition-fast);
}
.send-btn:hover:not(:disabled) {
  background: var(--accent-hover);
  box-shadow: 0 2px 6px rgba(44,175,104,0.3);
}
.send-btn:active:not(:disabled) {
  transform: scale(0.96);
}
.send-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.font-icon {
  font-size: 14px;
  font-family: Georgia, 'Times New Roman', serif;
  font-weight: 700;
  line-height: 1;
}
.spin-icon {
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}



/* 字体选择器 */
.font-picker {
  position: relative;
}
.font-menu {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: 6px;
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  box-shadow: 0 4px 16px rgba(0,0,0,0.1);
  padding: 6px;
  min-width: 150px;
  z-index: 100;
  font-family: var(--font-ui, 'DM Sans', system-ui, sans-serif);
}
.font-menu-title {
  font-size: 11px;
  color: var(--muted);
  padding: 4px 10px 6px;
  font-weight: 500;
}
.font-menu-item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 7px 10px;
  border: none;
  border-radius: 6px;
  background: none;
  font-size: 13px;
  font-weight: 500;
  color: var(--fg);
  cursor: pointer;
  transition: background 0.15s;
}
.font-menu-item:hover {
  background: var(--bg-hover);
}
.font-menu-item.active {
  color: var(--accent);
  font-weight: 500;
}
.font-menu-divider {
  height: 1px;
  background: var(--border-color);
  margin: 4px 0;
}
.font-menu-action {
  color: var(--muted);
  gap: 4px;
}
.font-menu-action:hover {
  color: var(--fg);
  background: var(--bg-hover);
}
</style>