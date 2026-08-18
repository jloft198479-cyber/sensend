import { createApp } from 'vue'
import App from './App.vue'
import { initTheme } from './composables/useTheme'
import './styles/vars.css'
import './styles/editor.css'

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