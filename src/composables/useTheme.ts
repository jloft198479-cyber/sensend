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

// 本文件是「主题元数据」的唯一真源（token 源头之一，故 check-tokens 对颜色字面量豁免）。
// 字段约束（check-contrast 门禁会校验，不是君子协定）：
//   1. id 必须与 styles/themes/<id>.css 一一对应；
//   2. windowBg 必须等于该主题 CSS 的 --bg —— 否则切主题时窗口会先闪一下别的底色；
//   3. preview 只是菜单里的身份色点，**不要求**等于 --accent：深色主题的 accent
//      画成小圆点会糊成一团，取更亮/更中性的近似色反而更好认（ink 就是这个原因取 #5A6570，
//      见 ink.css 顶部说明）。这条差异是有意的，不是色值漂移。
export const THEMES: ThemeMeta[] = [
  { id: 'light', name: '草绿', preview: '#2CAF68', windowBg: '#ffffff' },
  { id: 'dark', name: '暗夜', preview: '#1a1a1a', windowBg: '#1a1a1a' },
  { id: 'wenyi', name: '秋珀', preview: '#a17a45', windowBg: '#faf8f3' },
  { id: 'tech', name: '魅蓝', preview: '#3b82f6', windowBg: '#fbfcfe' },
  { id: 'ink', name: '墨白', preview: '#5A6570', windowBg: '#ffffff' },
]

/** 当前主题（跨组件共享的单一状态） */
export const currentTheme = ref('light')

function applyDom(theme: string) {
  document.documentElement.dataset.theme = theme
}

/** main.ts 启动时调用：读后端存储 + 监听跨窗口同步。
 *  两个 IPC 并行发起（此前串行多付一次往返）；主题仍在挂载前生效，防启动闪色 */
export async function initTheme() {
  const [theme] = await Promise.all([
    invoke<string>('get_theme').catch(() => 'light'),
    // 监听注册失败不能拖垮主题应用（否则启动后主题丢失 + unhandled rejection）
    listen<string>('theme-updated', (e) => {
      currentTheme.value = e.payload
      applyDom(e.payload)
    }).catch((err) => {
      console.error('主题同步监听注册失败:', err)
      return null
    }),
  ])
  currentTheme.value = theme
  applyDom(theme)
}

/** 切换主题：立即改 DOM（跟手），再把 id + 窗口底色交给后端存下并广播 */
export async function setTheme(id: string) {
  currentTheme.value = id
  applyDom(id)
  const meta = THEMES.find((t) => t.id === id) ?? THEMES[0]
  await invoke('set_theme', { theme: id, windowBg: meta.windowBg })
}
