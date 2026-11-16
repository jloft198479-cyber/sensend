import { ref, reactive, onMounted, onBeforeUnmount, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

/**
 * 快捷键设置 composable
 * 包含：快捷键配置加载/保存、录制、发送快捷键拦截
 */
export function useHotkey(publishNote: () => void, editorRef: () => any, isSending: Ref<boolean>) {
  const showHotkeyModal = ref(false)
  // 默认值唯一来源 = 后端 get_hotkeys（首跑无配置时由后端回退并落盘）。
  // 前端不再各存一份默认串防漂移；加载返回前的空串由 parseHotkeyString 的空 key 守卫兜住
  const hotkeyForm = reactive({
    show: '',
    send: '',
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

  /** [m-09] 按键命名：一律按**物理键位**（e.code）取名，不用 e.key。
   *
   *  两个理由：
   *  1. 匹配侧：录制把空格写成 `Space`，而键盘事件的 `e.key` 是 `' '`、`e.code` 是 `'Space'`，
   *     旧代码比的是 `'space'` / `'keyspace'`，两边都不等 → 含空格的组合键永远触发不了。
   *  2. 录制侧：`e.key` 受 Shift 影响，`Shift+1` 会记成 `Shift+!`、`Shift+/` 记成 `Shift+?`，
   *     这两个写法后端（global-hotkey 的 parse_key）不认，保存会直接报「格式无效」。
   *     用 e.code 取名则稳定得到 `1` / `Slash`，后端全部认识。
   *
   *  返回形如 `A` / `1` / `F5` / `Space` / `Enter` / `ArrowUp` / `Slash`（大小写仅用于展示，
   *  比较时一律转小写）。 */
  function codeNameFromEvent(e: KeyboardEvent): string {
    const code = e.code || ''
    if (/^Key[A-Z]$/.test(code)) return code.slice(3)
    if (/^Digit\d$/.test(code)) return code.slice(5)
    if (code) return code
    // 极端兜底：拿不到 e.code 时才退回 e.key
    const k = e.key || ''
    if (k === ' ' || k === 'Spacebar') return 'Space'
    return k.length === 1 ? k.toUpperCase() : k
  }

  /** 匹配用：统一小写成配置里 parse 后的样子 */
  function normalizeKey(e: KeyboardEvent): string {
    return codeNameFromEvent(e).toLowerCase()
  }

  /** 旧配置兜底：2026-11-16 之前的录制是按 `e.key` 存名字的（`Shift+/` 存成 `Shift+?`、
   *  `Shift+1` 存成 `Shift+!`）。改成按物理键位命名后，这些老值永远命不中新算法，
   *  用户会以为「快捷键突然失灵」。所以匹配时**两套命名都认**：
   *  新录制值靠 normalizeKey 命中，老配置值靠这里命中，互不干扰（两套命名没有交集冲突）。 */
  function legacyKey(e: KeyboardEvent): string {
    const k = e.key || ''
    if (k === ' ' || k === 'Spacebar') return 'space'
    return k.toLowerCase()
  }

  function handleSendHotkey(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement || e.target instanceof HTMLTextAreaElement) return
    const editor = editorRef()
    if (!editor || !editor.isFocused) return
    if (isSending.value) return  // 发送中，忽略快捷键

    const parsed = parseHotkeyString(hotkeyForm.send)
    if (!parsed.key) return

    const keyMatch = normalizeKey(e) === parsed.key || legacyKey(e) === parsed.key
    // Mac 上 Cmd 键对应 metaKey，Windows/Linux 上 Ctrl 键对应 ctrlKey
    // 配置里存的是 Control，但 Mac 用户录制时按 Cmd 实际存成 Control
    // 所以这里 ctrl 或 meta 任一按下都算匹配
    const ctrlMatch = parsed.ctrl ? (e.ctrlKey || e.metaKey) : (!e.ctrlKey && !e.metaKey)
    if (keyMatch && ctrlMatch && e.altKey === parsed.alt && e.shiftKey === parsed.shift) {
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

    // 修饰键单独按下：只更新提示，不落配置
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) {
      hotkeyEditState.display = parts.join('+') + '+…'
      return
    }
    if (e.key === 'Escape') { hotkeyEditState.field = null; hotkeyEditState.display = ''; return }

    // 键名与匹配侧同源（物理键位），见 codeNameFromEvent 注释
    parts.push(codeNameFromEvent(e))
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