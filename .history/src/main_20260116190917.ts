import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router'
import './style.css'
import './assets/pink.css' // Imported moved CSS

// 等待 DOM 加载完成后再挂载应用
// 这确保 Tauri API 有足够时间加载
function initApp() {
  const app = createApp(App)
  app.use(createPinia())
  app.use(router)
  app.mount('#app')
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initApp)
} else {
  // DOM 已经加载完成
  initApp()
}
