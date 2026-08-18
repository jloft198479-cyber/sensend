// ═══ 主题注册表 & 切换逻辑 ═══
// SSOT：主题元数据（含窗口底色）只有本文件一份，后端不存颜色映射，只按本表存值/涂色。
// 新增主题两步：styles/themes/ 加 CSS → 本表 THEMES 注册（菜单自动出现，窗口底色自动跟随）。
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface ThemeMeta {
  id: string
  /** 菜单显示名 */
  name: string
  /** 菜单预览色点（代表该主题的主色） */
  preview: string
  /** 原生窗口底色（切主题时交给后端存下并涂色，防加载闪色） */
  windowBg: string
}

export const THEMES: ThemeMeta[] = [
  { id: 'light', name: '草绿', preview: '#2CAF68', windowBg: '#ffffff' },
  { id: 'dark', name: '暗夜', preview: '#1a1a1a', windowBg: '#1a1a1a' },
  { id: 'wenyi', name: '秋珀', preview: '#a17a45', windowBg: '#faf8f3' },
  { id: 'tech', name: '魅蓝', preview: '#3b82f6', windowBg: '#fbfcfe' },
]

/** 当前主题（跨组件共享的单一状态） */
export const currentTheme = ref('light')

function applyDom(theme: string) {
  document.documentElement.dataset.theme = theme
}

/** main.ts 启动时调用：读后端存储 + 监听跨窗口同步 */
export async function initTheme() {
  try {
    currentTheme.value = await invoke<string>('get_theme')
  } catch {
    currentTheme.value = 'light'
  }
  applyDom(currentTheme.value)
  await listen<string>('theme-updated', (e) => {
    currentTheme.value = e.payload
    applyDom(e.payload)
  })
}

/** 切换主题：立即改 DOM（跟手），再把 id + 窗口底色交给后端存下并广播 */
export async function setTheme(id: string) {
  currentTheme.value = id
  applyDom(id)
  const meta = THEMES.find((t) => t.id === id) ?? THEMES[0]
  await invoke('set_theme', { theme: id, windowBg: meta.windowBg })
}
