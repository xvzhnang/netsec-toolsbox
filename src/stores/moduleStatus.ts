import { ref } from 'vue'
import { defineStore } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'

export const useModuleStatusStore = defineStore('moduleStatus', () => {
  const aiReady = ref(false)
  const wikiReady = ref(false)
  const isInitializing = ref(true)

  // 获取初始状态
  const fetchStatus = async () => {
    try {
      const status = await invoke<{ ai: boolean; wiki: boolean }>('get_module_status')
      aiReady.value = status.ai
      wikiReady.value = status.wiki
    } catch (error) {
      console.warn('Failed to fetch module status:', error)
    } finally {
      isInitializing.value = false
    }
  }

  // 监听状态变化事件
  const startListening = async () => {
    await fetchStatus()
    
    // 监听后端发出的状态变更事件
    await listen('module_status_changed', async () => {
      await fetchStatus()
    })
    
    // 轮询作为兜底（每2秒检查一次，直到都就绪）
    const interval = setInterval(async () => {
      if (aiReady.value && wikiReady.value) {
        clearInterval(interval)
        return
      }
      await fetchStatus()
    }, 2000)
  }

  return {
    aiReady,
    wikiReady,
    isInitializing,
    startListening,
    fetchStatus
  }
})
