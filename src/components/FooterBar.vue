<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { getInstanceDisplayName, toInstanceItem } from '../types/platform'
import type { InstanceItem, PlatformInstance, PlatformTypeInfo } from '../types/platform'
import InstanceMenu from './InstanceMenu.vue'

const props = defineProps<{
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
  'check-update': []
}>()

const showMenu = ref(false)
const showPicker = ref(false)

// 统一条目格式：映射规则在 types/platform.ts；列表格式真源是共享组件 InstanceMenu
//（与编辑器 @ 列表共用一份行格式与键盘行为）
const instanceItems = computed(() => props.instances.map(i => toInstanceItem(i, props.platformTypes)))

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

function pickTarget(item: InstanceItem) {
  emit('select-target', item.id)
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

function handleCheckUpdate() {
  closeMenu()
  emit('check-update')
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
          <span v-if="currentTarget" class="target-mention platform-mention">
            <span class="tm-text">@{{ getInstanceDisplayName(platformTypes, currentTarget) }}</span>
          </span>
          <span v-else class="target-mention platform-mention placeholder">
            <span class="tm-text">@选择页面</span>
          </span>
          <svg class="picker-chevron" :class="{ open: showPicker }" width="12" height="12"
            viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="6 9 12 15 18 9"/>
          </svg>
        </button>

        <!-- 平台选择列表（格式真源：InstanceMenu，与编辑器 @ 列表共用） -->
        <div v-if="showPicker" class="picker-anchor">
          <InstanceMenu
            :items="instanceItems"
            :active-id="currentTarget?.id ?? null"
            @select="pickTarget"
          />
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
            <button class="settings-item" @click.stop="handleCheckUpdate" aria-label="检查更新">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="23 4 23 10 17 10"/>
                <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
              </svg>
              <span>检查更新</span>
            </button>
          </div>
        </Transition>
      </div>
      <span class="word-count"><span class="wc-num">{{ wordCount }}</span><span class="wc-unit"> 字</span></span>
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
  /* 界面文字统一 --font-sans；仅目标名称沿用内容字体（方案 F-1） */
  font-family: var(--font-sans);
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
  border-radius: var(--radius-xs);
  transition: all 0.15s;
  white-space: nowrap;
  line-height: 1;
}
.target-btn:hover { background: var(--bg-hover); }
.target-btn.no-target {
  color: var(--muted);
  font-size: var(--text-xs);
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
  border-radius: var(--radius-sm);
  background: transparent;
  cursor: pointer;
  transition: all 0.15s;
  font-family: var(--font-editor);
}
.target-picker-btn:hover {
  background: var(--bg-hover);
  border-color: var(--border-color);
}
/* 药丸基础样式复用全局 .platform-mention（App.vue 定义，与编辑器内 mention 同一格式真源）；
   此处只保留底栏特有差异 */
.target-mention {
  display: inline-flex;
  align-items: center;
  font-family: var(--font-editor);
  font-size: var(--text-xs);
  cursor: pointer;
  max-width: 160px;
  overflow: hidden;
}
.tm-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.target-mention.placeholder {
  color: var(--muted);
  background: var(--bg-hover);
  border-color: var(--border-color);
}
.picker-chevron {
  color: var(--muted);
  transition: transform 0.15s;
  flex-shrink: 0;
}
.picker-chevron.open {
  transform: rotate(180deg);
}

/* 目标弹窗锚点：定位归底栏，列表格式来自 InstanceMenu */
.picker-anchor {
  position: absolute;
  bottom: calc(100% + 4px);
  left: 0;
  z-index: var(--z-dropdown);
}

/* 字数统计 */
.word-count {
  font-size: var(--text-2xs);
  color: var(--muted);
}

/* 表单控件默认不继承父级字体（UA 样式表行为），统一拉回组件字体栈（--font-editor） */
.footer-bar button {
  font-family: inherit;
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
  width: 88px;
  padding: 0;
  color: var(--danger);
  background: var(--danger-bg);
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

/* 设置下拉菜单（尺寸与 InstanceMenu 同一档方言：radius/padding/字号统一） */
.settings-menu {
  position: absolute;
  bottom: calc(100% + 4px);
  right: 0;
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: var(--space-4);
  min-width: 160px;
  box-shadow: var(--shadow-e2);
  z-index: var(--z-dropdown);
  font-family: var(--font-sans);
}
.settings-item {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  width: 100%;
  height: 28px;
  padding: 0 var(--space-8);
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--fg);
  cursor: pointer;
  white-space: nowrap;
  text-align: left;
  transition: background var(--dur-fast);
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

/* 窄窗压缩（方案 4.11）：字数只留数字，药丸进一步收窄，底栏内边距收紧 */
@media (max-width: 359px) {
  .footer-bar { padding: 8px 10px 10px; }
  .wc-unit { display: none; }
  .target-mention { max-width: 130px; }
}

</style>