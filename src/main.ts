import { createApp } from 'vue'
import App from './App.vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import './styles/vars.css'
import './styles/editor.css'

/// 主题初始化：启动时读后端存储，设到 <html data-theme>
async function initTheme() {
  try {
    const theme = await invoke<string>('get_theme')
    document.documentElement.dataset.theme = theme
  } catch {
    document.documentElement.dataset.theme = 'light'
  }
  // 监听跨窗口同步
  listen<string>('theme-updated', (e) => {
    document.documentElement.dataset.theme = e.payload
  })
}

const params = new URLSearchParams(window.location.search)
const page = params.get('page')

if (page === 'config') {
  initTheme().finally(() => {
    import('./ConfigWindow.vue').then(({ default: ConfigWindow }) => {
      createApp(ConfigWindow).mount('#app')
    })
  })
} else {
  initTheme().finally(() => {
    createApp(App).mount('#app')
  })
}