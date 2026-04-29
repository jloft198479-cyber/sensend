import { ref, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'

/**
 * 编辑器字体管理 composable
 * 负责字体切换、用户字体加载、字体菜单/管理器状态
 */
export function useEditorFont() {
  const fontOptions = ref([
    { value: "'Microsoft YaHei UI', 'PingFang SC', 'Hiragino Sans GB', Georgia, serif", label: '系统默认' },
  ])
  const currentFont = ref("'Microsoft YaHei UI', 'PingFang SC', 'Hiragino Sans GB', Georgia, serif")
  const currentFontLabel = ref('系统默认')
  const showFontMenu = ref(false)
  const showFontManager = ref(false)

  function selectFont(opt: typeof fontOptions.value[number]) {
    currentFont.value = opt.value
    currentFontLabel.value = opt.label
    document.documentElement.style.setProperty('--font-editor', opt.value)
    showFontMenu.value = false
  }

  function addFont() {
    showFontMenu.value = false
    showFontManager.value = true
  }

  function toggleFontMenu() {
    showFontMenu.value = !showFontMenu.value
  }

  function applyUserFonts(userFonts: Array<{ name: string; path: string }>) {
    const oldStyle = document.getElementById('dynamic-font-faces')
    if (oldStyle) oldStyle.remove()

    if (userFonts.length > 0) {
      const style = document.createElement('style')
      style.id = 'dynamic-font-faces'
      style.textContent = userFonts.map(f =>
        `@font-face { font-family: '${f.name}'; src: url('${f.path}'); }`
      ).join('\n')
      document.head.appendChild(style)
    }

    fontOptions.value = [
      { value: "'Microsoft YaHei UI', 'PingFang SC', 'Hiragino Sans GB', Georgia, serif", label: '系统默认' },
      ...userFonts.map(f => ({
        value: `'${f.name}', 'Microsoft YaHei UI', Georgia, serif`,
        label: f.name,
      })),
    ]

    const currentStillExists = fontOptions.value.some(f => f.value === currentFont.value)
    if (!currentStillExists) {
      const defaultFont = fontOptions.value[0]
      currentFont.value = defaultFont.value
      currentFontLabel.value = defaultFont.label
      document.documentElement.style.setProperty('--font-editor', defaultFont.value)
    }
  }

  function onFontManagerRefreshed(userFonts: Array<{ name: string; path: string }>) {
    applyUserFonts(userFonts)
  }

  // 点击外部关闭字体菜单
  function onDocClickForFontMenu(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest('.font-picker')) showFontMenu.value = false
  }

  // 生命周期：加载用户字体、注册/清理点击监听
  onMounted(async () => {
    try {
      const userFonts = await invoke<Array<{ name: string; path: string }>>('scan_user_fonts')
      applyUserFonts(userFonts)
    } catch (e) {
      console.error('加载用户字体列表失败:', e)
    }
    document.addEventListener('click', onDocClickForFontMenu)
  })

  onBeforeUnmount(() => {
    document.removeEventListener('click', onDocClickForFontMenu)
  })

  return {
    fontOptions,
    currentFont,
    currentFontLabel,
    showFontMenu,
    showFontManager,
    selectFont,
    addFont,
    toggleFontMenu,
    onFontManagerRefreshed,
  }
}