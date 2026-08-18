import { createApp } from 'vue'
import App from './App.vue'
import './styles/vars.css'
import './styles/editor.css'

const params = new URLSearchParams(window.location.search)
const page = params.get('page')

if (page === 'config') {
  // 配置窗口：动态加载独立配置组件，Rollup 自动分包
  import('./ConfigWindow.vue').then(({ default: ConfigWindow }) => {
    createApp(ConfigWindow).mount('#app')
  })
} else {
  // 主窗口：加载编辑器
  createApp(App).mount('#app')
}