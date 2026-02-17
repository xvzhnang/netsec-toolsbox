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
    
    // 轮询作为兜底（避免永远轮询：Wiki 改为按需启动，不作为停止条件）
    let tries = 0
    const maxTries = 15
    const interval = setInterval(async () => {
      tries += 1
      if (aiReady.value || tries >= maxTries) {
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
