<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { getInstanceDisplayName } from '../types/platform'
import type { PlatformInstance, PlatformTypeInfo } from '../types/platform'

defineProps<{
  instances: PlatformInstance[]
  platformTypes: PlatformTypeInfo[]
  currentTarget: PlatformInstance | null
  wordCount: number
  charCount: number
}>()

const emit = defineEmits<{
  'open-config': []
  'open-hotkey': []
  'open-data-dir': []
  'select-target': [instanceId: string]
  'clear': []
}>()

const showMenu = ref(false)
const showPicker = ref(false)

// ── 清空笔记（两步确认防误触，3 秒无操作还原）──
const confirmingClear = ref(false)
let clearTimer: ReturnType<typeof setTimeout> | null = null
function onClearClick() {
  if (confirmingClear.value) {
    emit('clear')
    confirmingClear.value = false
    if (clearTimer) { clearTimeout(clearTimer); clearTimer = null }
  } else {
    confirmingClear.value = true
    if (clearTimer) clearTimeout(clearTimer)
    clearTimer = setTimeout(() => { confirmingClear.value = false }, 3000)
  }
}

function selectTarget(id: string) {
  emit('select-target', id)
  showPicker.value = false
}

function toggleMenu() {
  showMenu.value = !showMenu.value
  showPicker.value = false
}

function closeMenu() {
  showMenu.value = false
}

function togglePicker(e: MouseEvent) {
  e.stopPropagation()
  showPicker.value = !showPicker.value
  showMenu.value = false
}

function handleAddPlatform() {
  closeMenu()
  emit('open-config')
}

function handleHotkey() {
  closeMenu()
  emit('open-hotkey')
}

function handleOpenDataDir() {
  closeMenu()
  emit('open-data-dir')
}

// 点击外部关闭菜单
onMounted(() => {
  document.addEventListener('click', onDocClick)
})
onBeforeUnmount(() => {
  document.removeEventListener('click', onDocClick)
  if (clearTimer) { clearTimeout(clearTimer); clearTimer = null }
})
function onDocClick(e: MouseEvent) {
  if (showMenu.value && !(e.target as HTMLElement).closest('.settings-group')) {
    closeMenu()
  }
  if (showPicker.value && !(e.target as HTMLElement).closest('.target-picker-wrap')) {
    showPicker.value = false
  }
}
</script>

<template>
  <footer class="footer-bar">
    <div class="footer-left">
      <!-- 无平台：提示配置 -->
      <button v-if="instances.length === 0" class="target-btn no-target" @click.stop="$emit('open-config')"
        title="配置发送页面" aria-label="配置发送页面">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/>
          <line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        <span>配置页面</span>
      </button>

      <!-- 有平台：可点击选择目标 -->
      <div v-else class="target-picker-wrap">
        <button class="target-picker-btn" @click="togglePicker" title="选择发送目标" aria-label="选择发送目标">
          <span v-if="currentTarget" class="target-mention">
            @{{ getInstanceDisplayName(platformTypes, currentTarget) }}
          </span>
          <span v-else class="target-mention placeholder">
            @选择页面
          </span>
          <svg class="picker-chevron" :class="{ open: showPicker }" width="12" height="12"
            viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="6 9 12 15 18 9"/>
          </svg>
        </button>

        <!-- 平台选择列表 -->
        <div v-if="showPicker" class="target-picker-menu">
          <div v-if="instances.length === 0" class="picker-empty">无匹配平台</div>
          <button
            v-for="inst in instances" :key="inst.id"
            class="picker-item"
            @click="selectTarget(inst.id)"
          >
            <span class="picker-item-name">{{ inst.name }}</span>
            <span class="picker-item-sep">—</span>
            <span class="picker-item-type">{{ platformTypes.find(t => t.key === inst.platform_type)?.name }}</span>
          </button>
        </div>
      </div>
    </div>
    <div class="footer-right">
      <!-- 清空笔记（两步确认防误触） -->
      <button class="footer-btn clear-btn" :class="{ confirming: confirmingClear }"
        @click.stop="onClearClick"
        :title="confirmingClear ? '再次点击确认清空' : '清空笔记'" aria-label="清空笔记">
        <svg v-if="!confirmingClear" width="14" height="14" viewBox="0 0 24 24" fill="none"
          stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="3 6 5 6 21 6"/>
          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
          <line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/>
        </svg>
        <span v-else class="clear-confirm-text">确认清空？</span>
      </button>
      <!-- 设置按钮 + 下拉菜单 -->
      <div class="settings-group">
        <button class="footer-btn" @click.stop="toggleMenu" title="设置" aria-label="设置">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3.5"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
        </button>
        <Transition name="menu-fade">
          <div v-if="showMenu" class="settings-menu">
            <button class="settings-item" @click.stop="handleAddPlatform" aria-label="添加和修改页面">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="16"/>
                <line x1="8" y1="12" x2="16" y2="12"/>
              </svg>
              <span>添加和修改页面</span>
            </button>
            <button class="settings-item" @click.stop="handleHotkey" aria-label="快捷键设置">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <rect x="2" y="4" width="20" height="16" rx="2"/>
                <path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M8 12h.01M12 12h.01M16 12h.01M8 16h8"/>
              </svg>
              <span>快捷键设置</span>
            </button>
            <button class="settings-item" @click.stop="handleOpenDataDir" aria-label="打开数据文件夹">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
              </svg>
              <span>打开数据文件夹</span>
            </button>
          </div>
        </Transition>
      </div>
      <span class="word-count">{{ wordCount }} 字</span>
    </div>
  </footer>
</template>

<style scoped>
/* ═══ 底栏 ═══ */
.footer-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px 14px;
  min-height: 40px;
  background: var(--bg);
  flex-shrink: 0;
  user-select: none;
  cursor: default;
  font-family: var(--font-editor);
}
.footer-left {
  display: flex;
  align-items: center;
  gap: 0;
  min-width: 0;
  flex: 1;
  overflow: visible;
}
.footer-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

/* 无平台按钮 */
.target-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  border: none;
  background: transparent;
  cursor: pointer;
  border-radius: 4px;
  transition: all 0.15s;
  white-space: nowrap;
  line-height: 1;
}
.target-btn:hover { background: rgba(0, 0, 0, 0.05); }
.target-btn.no-target {
  color: var(--muted);
  font-size: 12px;
}

/* ═══ 平台选择器 ═══ */
.target-picker-wrap {
  position: relative;
}
.target-picker-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  cursor: pointer;
  transition: all 0.15s;
  font-family: var(--font-editor);
}
.target-picker-btn:hover {
  background: var(--bg-hover, rgba(0,0,0,0.04));
  border-color: var(--border-color, rgba(0,0,0,0.06));
}
.target-mention {
  display: inline-flex;
  align-items: center;
  color: var(--accent);
  background: linear-gradient(135deg, var(--accent-light) 0%, color-mix(in srgb, var(--accent) 3%, transparent) 100%);
  border: 1px solid color-mix(in srgb, var(--accent) 15%, transparent);
  border-radius: 5px;
  padding: 1px 6px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  user-select: none;
}
.target-mention:hover {
  background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 12%, transparent) 0%, color-mix(in srgb, var(--accent) 6%, transparent) 100%);
  border-color: color-mix(in srgb, var(--accent) 25%, transparent);
}
.target-mention.placeholder {
  color: var(--muted);
  background: rgba(0,0,0,0.03);
  border-color: rgba(0,0,0,0.08);
}
.picker-chevron {
  color: var(--muted);
  transition: transform 0.15s;
  flex-shrink: 0;
}
.picker-chevron.open {
  transform: rotate(180deg);
}

/* 下拉菜单定位 */
.target-picker-menu {
  position: absolute;
  bottom: calc(100% + 4px);
  left: 0;
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 4px;
  min-width: 100px;
  max-width: 280px;
  overflow-y: auto;
  max-height: 200px;
  box-shadow: 0 -4px 16px rgba(0,0,0,0.08), 0 -1px 4px rgba(0,0,0,0.04);
  font-family: var(--font-editor);
  z-index: 99999;
}
.picker-empty {
  padding: 8px 10px;
  font-size: 11px;
  color: var(--muted);
  white-space: nowrap;
}
.picker-item {
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
}
.picker-item:hover {
  background: var(--accent-light);
}
.picker-item-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
}
.picker-item-sep {
  font-size: 12px;
  color: var(--muted);
  flex-shrink: 0;
}
.picker-item-type {
  font-size: 12px;
  font-weight: 500;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 滚动条 */
.target-picker-menu::-webkit-scrollbar {
  width: 4px;
}
.target-picker-menu::-webkit-scrollbar-track {
  background: transparent;
}
.target-picker-menu::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: 2px;
}
.target-picker-menu::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}

/* 字数统计 */
.word-count {
  font-size: 12px;
  color: var(--fg-secondary);
}

/* 底栏设置按钮 */
.footer-btn {
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
.footer-btn:hover {
  background: var(--bg-hover);
  color: var(--fg);
}

/* 清空按钮（两步确认态） */
.clear-btn { color: var(--fg-secondary); }
.clear-btn.confirming {
  width: auto;
  padding: 0 10px;
  color: var(--danger);
  background: var(--danger-bg, rgba(239, 68, 68, 0.1));
}
.clear-confirm-text {
  font-size: 12px;
  font-weight: 600;
  white-space: nowrap;
}

/* 设置按钮组 */
.settings-group {
  position: relative;
}

/* 设置下拉菜单 */
.settings-menu {
  position: absolute;
  bottom: calc(100% + 4px);
  right: 0;
  background: var(--bg, #fff);
  border: 1px solid var(--border-color, rgba(0,0,0,0.06));
  border-radius: 6px;
  padding: 4px;
  min-width: 130px;
  box-shadow: 0 -4px 16px rgba(0,0,0,0.08);
  z-index: 9999;
  font-family: var(--font-ui, 'DM Sans', system-ui, sans-serif);
}
.settings-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 7px 10px;
  border: none;
  border-radius: 4px;
  background: transparent;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--fg);
  cursor: pointer;
  white-space: nowrap;
  text-align: left;
  transition: background 0.12s;
}
.settings-item:hover {
  background: var(--bg-hover);
}
.settings-item svg {
  color: var(--muted);
  flex-shrink: 0;
}

/* 菜单过渡 */
.menu-fade-enter-active,
.menu-fade-leave-active {
  transition: opacity 0.15s ease;
}
.menu-fade-enter-from,
.menu-fade-leave-to {
  opacity: 0;
}

</style>