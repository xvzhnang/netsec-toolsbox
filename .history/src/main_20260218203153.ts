import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router'
import './style.css'
import './assets/pink.css' // Imported moved CSS

// 尽早初始化应用，不要等待 DOMContentLoaded
// Tauri 的 WebView 在加载此脚本时 DOM 通常已经准备好了（或至少 body 存在）
function initApp() {
  const app = createApp(App)
  app.use(createPinia())
  app.use(router)
  app.mount('#app')

  const hideBootLoader = () => {
    // 立即隐藏，不要等待 timeout
    const bootLoader = document.getElementById('boot-loader')
    if (!bootLoader) return
    bootLoader.classList.add('boot-hide')
    // 延时移除 DOM 节点
    setTimeout(() => bootLoader.remove(), 500)
  }

  // 监听自定义的 app-ready 事件
  window.addEventListener('app-ready', hideBootLoader, { once: true })
}

initApp()
