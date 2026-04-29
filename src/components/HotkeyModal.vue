<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'

const props = defineProps<{
  visible: boolean
  hotkeyForm: { show: string; send: string }
  hotkeyEditState: { field: 'show' | 'send' | null; display: string }
  hotkeySaveStatus: 'idle' | 'ok' | 'fail'
}>()

const emit = defineEmits<{
  'close': []
  'start-recording': [field: 'show' | 'send']
  'save': []
  'keydown': [e: KeyboardEvent]
}>()

const panelRef = ref<HTMLElement | null>(null)
let previousFocusEl: HTMLElement | null = null

// 打开时记住触发按钮、聚焦弹窗；关闭时焦点回触发按钮
watch(() => props.visible, async (v) => {
  if (v) {
    previousFocusEl = document.activeElement as HTMLElement
    await nextTick()
    const firstBtn = panelRef.value?.querySelector<HTMLElement>('button')
    firstBtn?.focus()
  } else {
    previousFocusEl?.focus()
    previousFocusEl = null
  }
})

// focus trap：Tab / Shift+Tab 不出弹窗
function onPanelKeydown(e: KeyboardEvent) {
  emit('keydown', e)
  if (e.key !== 'Tab') return
  const panel = panelRef.value
  if (!panel) return
  const focusable = panel.querySelectorAll<HTMLElement>(
    'button:not([disabled]), [tabindex]:not([tabindex="-1"])'
  )
  if (focusable.length === 0) return
  const first = focusable[0]
  const last = focusable[focusable.length - 1]
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault()
    last.focus()
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault()
    first.focus()
  }
}
</script>

<template>
  <Transition name="fade">
    <div v-if="visible" class="modal-overlay" @click.self="$emit('close')" @keydown="onPanelKeydown"
      role="dialog" aria-modal="true" aria-labelledby="hotkey-modal-title">
      <div class="modal-panel hotkey-panel" ref="panelRef" @click.stop>
        <div class="modal-header">
          <span class="modal-title" id="hotkey-modal-title">快捷键设置</span>
          <button class="modal-close" @click="$emit('close')" aria-label="关闭">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
              stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
        <div class="modal-body hotkey-body">
          <!-- 唤醒窗口 -->
          <div class="hotkey-item">
            <div class="hotkey-label">唤醒窗口</div>
            <div class="hotkey-desc">全局快捷键，随时呼出/隐藏 Sensend</div>
            <button class="hotkey-record-btn"
              :class="{ recording: hotkeyEditState.field === 'show' }"
              @click="emit('start-recording', 'show')"
              aria-label="录制唤醒窗口快捷键">
              <span v-if="hotkeyEditState.field === 'show'" class="recording-text">{{ hotkeyEditState.display }}</span>
              <kbd v-else class="hotkey-kbd">{{ hotkeyForm.show }}</kbd>
            </button>
          </div>
          <!-- 发送 -->
          <div class="hotkey-item">
            <div class="hotkey-label">发送笔记</div>
            <div class="hotkey-desc">编辑器内快捷键，快速发送当前笔记</div>
            <button class="hotkey-record-btn"
              :class="{ recording: hotkeyEditState.field === 'send' }"
              @click="emit('start-recording', 'send')"
              aria-label="录制发送笔记快捷键">
              <span v-if="hotkeyEditState.field === 'send'" class="recording-text">{{ hotkeyEditState.display }}</span>
              <kbd v-else class="hotkey-kbd">{{ hotkeyForm.send }}</kbd>
            </button>
          </div>
          <!-- 保存 -->
          <div class="hotkey-actions">
            <button class="config-btn save-btn" @click="$emit('save')">保存</button>
          </div>
          <Transition name="fade">
            <div v-if="hotkeySaveStatus === 'ok'" class="test-msg success">✓ 快捷键已更新</div>
            <div v-else-if="hotkeySaveStatus === 'fail'" class="test-msg fail">✕ 保存失败，请检查快捷键是否冲突</div>
          </Transition>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
/* ═══ 弹窗遮罩 ═══ */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.25);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}
.modal-panel {
  background: white;
  border-radius: 14px;
  box-shadow: 0 20px 60px rgba(0,0,0,0.15), 0 8px 20px rgba(0,0,0,0.08);
  width: 280px;
  overflow: hidden;
}
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px 10px;
  border-bottom: 1px solid var(--border-color);
}
.modal-title {
  font-size: 13.5px;
  font-weight: 650;
  color: var(--fg);
}
.modal-close {
  width: 24px; height: 24px;
  border: none; background: transparent;
  color: var(--muted); cursor: pointer;
  border-radius: 6px;
  display: flex; align-items: center; justify-content: center;
  transition: all 0.15s ease; padding: 0;
}
.modal-close:hover { background: #f4f4f5; color: var(--fg); }

.modal-body { padding: 6px; }

/* ═══ 快捷键设置 ═══ */
.hotkey-panel { width: 300px; }
.hotkey-body { padding: 16px; display: flex; flex-direction: column; gap: 14px; }
.hotkey-item { display: flex; flex-direction: column; gap: 4px; }
.hotkey-label { font-size: 12px; font-weight: 600; color: var(--fg); }
.hotkey-desc { font-size: 10.5px; color: var(--muted); }
.hotkey-record-btn {
  display: flex; align-items: center; justify-content: center;
  padding: 8px 12px; border: 1px solid var(--gray-border, #e4e4e7); border-radius: 8px;
  background: var(--gray-input-bg, #fafafa); cursor: pointer; transition: all 0.15s;
  min-height: 36px;
}
.hotkey-record-btn:hover { border-color: var(--accent); background: white; }
.hotkey-record-btn.recording {
  border-color: var(--accent); background: var(--accent-light);
  animation: pulse-border 1.5s infinite;
}
@keyframes pulse-border {
  0%, 100% { box-shadow: 0 0 0 0 rgba(44,175,104,0.2); }
  50% { box-shadow: 0 0 0 3px rgba(44,175,104,0.15); }
}
.hotkey-kbd {
  font-family: var(--font-sans); font-size: 12px; font-weight: 600;
  color: var(--fg); padding: 0; border: none; background: none;
  pointer-events: none;
}
.recording-text {
  font-size: 11px; color: var(--accent); font-weight: 500;
  animation: blink 1s ease-in-out infinite;
}
@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
.hotkey-actions { margin-top: 4px; display: flex; }

.config-btn {
  flex: 1; padding: 11px 0; border: none; border-radius: 8px;
  font-size: 12.5px; font-weight: 600; cursor: pointer;
  transition: all 0.15s ease; display: flex; align-items: center;
  justify-content: center; gap: 6px; width: 100%;
}
.config-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.save-btn { background: var(--accent); color: white; }
.save-btn:hover:not(:disabled) { background: var(--accent-hover); }
</style>