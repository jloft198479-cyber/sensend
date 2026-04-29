import { createApp } from 'vue'
import App from './App.vue'
import ConfigWindow from './ConfigWindow.vue'
import './styles/vars.css'
import './styles/editor.css'

const params = new URLSearchParams(window.location.search)
const page = params.get('page')

if (page === 'config') {
  // 配置窗口：加载独立的配置组件
  createApp(ConfigWindow).mount('#app')
} else {
  // 主窗口：加载编辑器
  createApp(App).mount('#app')
}