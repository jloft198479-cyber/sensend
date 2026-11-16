import { createApp } from 'vue'
import { applyCachedThemeSync, initTheme } from './composables/useTheme'
import './styles/vars.css'

const params = new URLSearchParams(window.location.search)
const page = params.get('page')

// [ZC-04] 先用本地缓存同步上色（无 IPC、挂载前生效，深色主题不再先闪浅色），
// 后端真值由 initTheme() 异步校正——initTheme 不再阻塞挂载（主题是 CSS 变量，
// 事后校正不会引起重排，此前"等 IPC 回来才 mount"白付几十毫秒）
applyCachedThemeSync()

if (page === 'config') {
  // [ZC-08] 配置页分支动态加载自身：App.vue / editor.css / TipTap 整个编辑器主包
  // 不进配置页的加载链（689 KB → 只载配置页自己的小 chunk）
  import('./ConfigWindow.vue').then(({ default: ConfigWindow }) => {
    createApp(ConfigWindow).mount('#app')
    void initTheme()
  })
} else {
  // [ZC-08] 主窗分支同样动态 import：编辑器 UI 与 editor.css 独立成 chunk，
  // 入口包只剩 main.ts + useTheme。editor.css 先注入再挂载，防首帧裸样式
  import('./App.vue').then(async ({ default: App }) => {
    await import('./styles/editor.css')
    createApp(App).mount('#app')
    void initTheme()
  })
}
