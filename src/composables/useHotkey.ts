import { ref, reactive, onMounted, onBeforeUnmount, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

/**
 * 快捷键设置 composable
 * 包含：快捷键配置加载/保存、录制、发送快捷键拦截
 */
export function useHotkey(publishNote: () => void, editorRef: () => any, isSending: Ref<boolean>) {
  const showHotkeyModal = ref(false)
  const hotkeyForm = reactive({
    show: 'Alt+Shift+F',
    send: 'Control+Enter',
  })
  const hotkeyEditState = reactive({
    field: null as 'show' | 'send' | null,
    display: '',
  })
  const hotkeySaveStatus = ref<'idle' | 'ok' | 'fail'>('idle')

  // ── 发送快捷键拦截 ──
  function parseHotkeyString(hotkey: string): { ctrl: boolean; alt: boolean; shift: boolean; key: string } {
    const parts = hotkey.toLowerCase().split('+').map(s => s.trim())
    return {
      ctrl: parts.includes('control') || parts.includes('ctrl'),
      alt: parts.includes('alt'),
      shift: parts.includes('shift'),
      key: parts.filter(p => !['control', 'ctrl', 'alt', 'shift', 'meta', 'command', 'cmd'].includes(p))[0] || '',
    }
  }

  function handleSendHotkey(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement || e.target instanceof HTMLTextAreaElement) return
    const editor = editorRef()
    if (!editor || !editor.isFocused) return
    if (isSending.value) return  // 发送中，忽略快捷键

    const parsed = parseHotkeyString(hotkeyForm.send)
    if (!parsed.key) return

    const keyMatch = e.key.toLowerCase() === parsed.key || e.code.toLowerCase() === `key${parsed.key}`
    if (keyMatch && e.ctrlKey === parsed.ctrl && e.altKey === parsed.alt && e.shiftKey === parsed.shift) {
      e.preventDefault()
      publishNote()
    }
  }

  // ── 快捷键设置弹窗 ──
  function openHotkeyModal() {
    hotkeyEditState.field = null
    hotkeyEditState.display = ''
    hotkeySaveStatus.value = 'idle'
    showHotkeyModal.value = true
  }

  function startRecordingHotkey(field: 'show' | 'send') {
    hotkeyEditState.field = field
    hotkeyEditState.display = '请按下快捷键…'
  }

  function onKeyDownForHotkey(e: KeyboardEvent) {
    if (!hotkeyEditState.field) return
    e.preventDefault()
    e.stopPropagation()

    const parts: string[] = []
    if (e.ctrlKey || e.metaKey) parts.push('Control')
    if (e.altKey) parts.push('Alt')
    if (e.shiftKey) parts.push('Shift')

    let key = e.key
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(key)) {
      hotkeyEditState.display = parts.join('+') + '+…'
      return
    }
    if (key === ' ') key = 'Space'
    else if (key === 'Escape') { hotkeyEditState.field = null; hotkeyEditState.display = ''; return }
    else if (key === 'Enter') key = 'Enter'
    else key = key.length === 1 ? key.toUpperCase() : key

    parts.push(key)
    const combo = parts.join('+')
    hotkeyEditState.display = combo
    hotkeyForm[hotkeyEditState.field] = combo
    hotkeyEditState.field = null
  }

  async function saveHotkeys() {
    try {
      await invoke('save_hotkeys', { show: hotkeyForm.show, send: hotkeyForm.send })
      hotkeySaveStatus.value = 'ok'
      setTimeout(() => { showHotkeyModal.value = false }, 800)
    } catch (e: any) {
      hotkeySaveStatus.value = 'fail'
      throw e
    }
  }

  // ── 生命周期 ──
  onMounted(async () => {
    try {
      const hotkeys = await invoke<{ show: string; send: string }>('get_hotkeys')
      hotkeyForm.show = hotkeys.show
      hotkeyForm.send = hotkeys.send
    } catch (e) {
      console.error('加载快捷键配置失败:', e)
    }
    document.addEventListener('keydown', handleSendHotkey)
  })

  onBeforeUnmount(() => {
    document.removeEventListener('keydown', handleSendHotkey)
  })

  return {
    showHotkeyModal,
    hotkeyForm,
    hotkeyEditState,
    hotkeySaveStatus,
    openHotkeyModal,
    startRecordingHotkey,
    onKeyDownForHotkey,
    saveHotkeys,
  }
}