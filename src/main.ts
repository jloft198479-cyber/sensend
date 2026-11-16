import { createApp } from 'vue'
import App from './App.vue'
import { applyCachedThemeSync, initTheme } from './composables/useTheme'
import './styles/vars.css'
import './styles/editor.css'

const params = new URLSearchParams(window.location.search)
const page = params.get('page')

// [ZC-04] 先用本地缓存同步上色（无 IPC、挂载前生效，深色主题不再先闪浅色），
// 后端真值由 initTheme() 异步校正——initTheme 不再阻塞挂载（主题是 CSS 变量，
// 事后校正不会引起重排，此前"等 IPC 回来才 mount"白付几十毫秒）
applyCachedThemeSync()

if (page === 'config') {
  import('./ConfigWindow.vue').then(({ default: ConfigWindow }) => {
    createApp(ConfigWindow).mount('#app')
    void initTheme()
  })
} else {
  createApp(App).mount('#app')
  void initTheme()
}
