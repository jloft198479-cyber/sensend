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

// [ZC-04] 主题本地缓存键：setTheme 成功时写入，启动时同步读出先上色
const THEME_CACHE_KEY = 'sensend:theme'

function applyDom(theme: string) {
  document.documentElement.dataset.theme = theme
}

/** [ZC-04] 用 localStorage 里的上次主题**同步**上色（无 IPC、无异步）。
 *  在 mount 之前调用：深色用户不再经历"先浅后深"的闪变。
 *  没有缓存 / 缓存值非法则什么都不做，等 initTheme() 的后端真值校正。
 *  注意：localStorage 在两窗口间不共享，管理窗首开可能闪一次浅色——
 *  但管理窗已由后端在 builder 阶段涂了正确的窗口底色，肉眼只见底色不见内容。 */
export function applyCachedThemeSync() {
  try {
    const cached = localStorage.getItem(THEME_CACHE_KEY)
    if (cached && THEMES.some(t => t.id === cached)) {
      currentTheme.value = cached
      applyDom(cached)
    }
  } catch {
    // localStorage 不可用（隐私模式等）就跳过，不阻塞启动
  }
}

/** main.ts 启动时调用：读后端存储 + 监听跨窗口同步。
 *  两个 IPC 并行发起（此前串行多付一次往返）；挂载不被它阻塞（main.ts 先同步上色再 mount）。
 *  [ZC-04] 校正语义：**读到了后端真值才覆盖**。此前失败回落 'light' 会把缓存上好的
 *  深色强行拉回浅色（store 读异常时深色用户每次启动都闪浅色）；失败时保留缓存值。 */
export async function initTheme() {
  const [theme] = await Promise.all([
    invoke<string>('get_theme').catch(() => null),
    // 监听注册失败不能拖垮主题应用（否则启动后主题丢失 + unhandled rejection）
    listen<string>('theme-updated', (e) => {
      currentTheme.value = e.payload
      applyDom(e.payload)
    }).catch((err) => {
      console.error('主题同步监听注册失败:', err)
      return null
    }),
  ])
  if (theme) {
    currentTheme.value = theme
    applyDom(theme)
  }
}

/** 切换主题：立即改 DOM（跟手），再把 id + 窗口底色交给后端存下并广播 */
export async function setTheme(id: string) {
  currentTheme.value = id
  applyDom(id)
  // [ZC-04] 同步写本地缓存，下次启动 applyCachedThemeSync 先用它上色
  try {
    localStorage.setItem(THEME_CACHE_KEY, id)
  } catch {
    // 缓存写失败不影响切换本身
  }
  const meta = THEMES.find((t) => t.id === id) ?? THEMES[0]
  await invoke('set_theme', { theme: id, windowBg: meta.windowBg })
}
