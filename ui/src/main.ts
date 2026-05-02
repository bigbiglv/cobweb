import { createApp } from 'vue'
import './styles/globals.css'
import App from './App.vue'
import router from './router'
import { initializeTheme } from './lib/theme'

initializeTheme()

document.addEventListener('contextmenu', (event) => {
  event.preventDefault()
})

const app = createApp(App)
app.use(router)
app.mount('#app')
