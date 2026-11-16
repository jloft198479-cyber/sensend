<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { currentTheme, setTheme, THEMES } from '../composables/useTheme'

defineProps<{
  isPinned: boolean
  isSending: boolean
  saveStatus: 'idle' | 'saved' | 'saving' | 'unsaved'
}>()

defineEmits<{
  'toggle-pin': []
  'publish': []
  'hide': []
}>()

// ── 主题菜单（主窗口唯一入口，主题列表来自 useTheme 注册表）──
const showThemeMenu = ref(false)
function pickTheme(id: string) {
  showThemeMenu.value = false
  setTheme(id)
}
function closeThemeMenu() { showThemeMenu.value = false }
onMounted(() => document.addEventListener('click', closeThemeMenu))
onUnmounted(() => document.removeEventListener('click', closeThemeMenu))

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
      <!-- 主题切换 -->
      <div class="menu-picker">
        <button class="action-btn" @click.stop="showThemeMenu = !showThemeMenu" title="切换主题" aria-label="切换主题">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 22a10 10 0 1 1 10-10c0 1.5-1.2 2.7-2.7 2.7H16a2.5 2.5 0 0 0-2.4 3.4c.4 1 0 1.8-1.6 1.9"/>
            <circle cx="7.5" cy="11.5" r="1" fill="currentColor"/>
            <circle cx="10.5" cy="7.5" r="1" fill="currentColor"/>
            <circle cx="15.5" cy="7.5" r="1" fill="currentColor"/>
            <circle cx="18.5" cy="11.5" r="1" fill="currentColor"/>
          </svg>
        </button>
        <div v-if="showThemeMenu" class="dropdown-menu" @click.stop>
          <button v-for="t in THEMES" :key="t.id" class="dropdown-item"
            :class="{ active: currentTheme === t.id }" @click="pickTheme(t.id)">
            <span class="theme-dot" :style="{ background: t.preview }"></span>
            <span>{{ t.name }}</span>
            <svg v-if="currentTheme === t.id" width="14" height="14" viewBox="0 0 24 24"
              fill="none" stroke="var(--accent-strong)" stroke-width="2" stroke-linecap="round">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
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
          stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="22" y1="2" x2="11" y2="13"/>
          <polygon points="22 2 15 22 11 13 2 9 22 2"/>
        </svg>
        <svg v-else class="spin-icon" width="14" height="14" viewBox="0 0 24 24" fill="none"
          stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 11-6.219-8.56"/>
        </svg>
      </button>

      <!-- 分隔：功能组 / 窗口组 -->
      <span class="action-sep"></span>

      <!-- 最小化到托盘 -->
      <button class="action-btn close-btn" @click.stop="$emit('hide')" title="最小化到托盘" aria-label="最小化到托盘">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
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
  padding: var(--space-10) var(--space-10) var(--space-10) var(--space-14);
  height: var(--titlebar-h);
  cursor: default;
  user-select: none;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-color);
  background: var(--accent-light);
}
.title-left {
  display: flex;
  align-items: center;
  gap: var(--space-8);
}
.title {
  font-family: var(--font-brand);
  font-size: var(--text-lg);
  font-weight: 700;
  font-style: italic;
  letter-spacing: -0.01em;
  color: var(--accent-strong);
}
.save-status {
  font-size: var(--text-2xs);
  opacity: 1;
  transition: opacity var(--dur-slow) ease;
}
.save-status.saved { color: var(--accent-strong); }
.save-status.saving { color: var(--muted); }

.saving-dots {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}
.saving-dots::after {
  content: '';
  display: inline-block;
  width: 12px;
  height: 3px;
  background: currentColor;
  border-radius: var(--radius-full);
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
  margin: 0 calc(var(--btn-gap) + var(--space-2));
}

/* 功能按钮 */
.action-btn {
  width: var(--btn-size);
  height: var(--btn-size);
  border: none;
  background: transparent;
  color: var(--fg-secondary);
  cursor: pointer;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--dur-fast);
}
.action-btn:hover {
  background: var(--bg-hover);
  color: var(--fg);
}
.action-btn.pin-active {
  color: var(--accent-strong);
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
  background: var(--accent-fill);
  color: var(--fg-on-accent);
  cursor: pointer;
  border-radius: var(--radius-sm);
  margin-left: var(--space-4);
  transition: all var(--dur-fast);
}
.send-btn:hover:not(:disabled) {
  background: var(--accent-fill-hover);
}
.send-btn:active:not(:disabled) {
  transform: scale(0.96);
}
.send-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}



/* 下拉菜单（主题切换） */
.menu-picker {
  position: relative;
}
.dropdown-menu {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: var(--space-6);
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-e2);
  padding: var(--space-4);
  min-width: 160px;
  z-index: var(--z-dropdown);
  font-family: var(--font-sans);
}
.dropdown-item {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  width: 100%;
  height: var(--menu-row-h);
  padding: 0 var(--space-8);
  border: none;
  border-radius: var(--radius-sm);
  background: none;
  /* button 不继承父级字体（UA 行为），显式拉回菜单声明的 --font-sans */
  font-family: inherit;
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--fg);
  cursor: pointer;
  transition: background var(--dur-fast);
}
.dropdown-item:hover {
  background: var(--bg-hover);
}
.dropdown-item.active {
  color: var(--accent-strong);
  font-weight: 500;
}

/* 主题菜单预览色点 */
.theme-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 1px solid var(--border-color);
  flex-shrink: 0;
}

/* 窄窗压缩（方案 4.11）：品牌字降一档，保存状态收成圆点（读屏仍读原文案） */
@media (max-width: 359px) {
  .title { font-size: var(--text-md); }
  .save-status { font-size: 0; }
  .save-status::after {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
}
</style>