import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getServiceStatus } from '../utils/serviceManager'
import { getAvailableModels } from '../utils/aiService'
import { error as logError } from '../utils/logger'

export type ServiceOpState = 'idle' | 'starting' | 'running' | 'stopping' | 'stopped' | 'error'

export const useAiServiceStore = defineStore('aiService', () => {
  // 状态
  const opState = ref<ServiceOpState>('idle')
  const statusMessage = ref('')
  const isAvailable = ref(false)
  const availableModels = ref<string[]>([])
  const currentModel = ref('gpt-3.5-turbo')
  
  // 内部状态
  const lastCheckTime = ref(0)

  // Getters
  const isRunning = computed(() => opState.value === 'running')
  const isWorking = computed(() => ['starting', 'stopping', 'restarting'].includes(opState.value))

  // Actions
  
  /**
   * 应用服务状态（通常来自后端事件或轮询结果）
   */
  function applyServiceState(state: string | undefined, message?: string) {
    if (!state) return

    // 映射后端状态到前端 OpState
    if (['starting', 'warmup', 'restarting'].includes(state)) {
      opState.value = 'starting'
      isAvailable.value = false
      statusMessage.value = message || 'AI Gateway 启动中...'
    } else if (state === 'stopping') {
      opState.value = 'stopping'
      isAvailable.value = false
      statusMessage.value = message || 'AI Gateway 停止中...'
    } else if (state === 'stopped') {
      opState.value = 'stopped'
      isAvailable.value = false
      statusMessage.value = message || 'AI Gateway 服务未启动'
    } else if (state === 'unhealthy' || state === 'failed') {
      // 这里的 Unhealthy 可能只是暂时的（例如刚启动时），但在明确收到 error 事件前，我们暂时认为是 error
      // 但为了配合非阻塞启动优化，如果当前已经是 starting，且收到 unhealthy，可能仍在启动中
      // 这里保持简单，由调用者决定是否忽略
      opState.value = 'error'
      isAvailable.value = false
      statusMessage.value = message || 'AI Gateway 服务异常'
    } else {
      // Running / Idle / Busy / Degraded
      opState.value = 'running'
      // 只有 Idle, Busy, Degraded 算是可用
      isAvailable.value = ['idle', 'busy', 'degraded'].includes(state)
      
      if (!isAvailable.value) {
         // 虽然 running 但不可用（例如初始化完成但无 worker）
         statusMessage.value = message || 'AI Gateway 服务就绪但不可用'
      } else {
         statusMessage.value = message || ''
      }
    }
  }

  /**
   * 检查服务状态
   */
  async function checkService() {
    try {
      const status = await getServiceStatus('ai-gateway')
      if (status) {
        applyServiceState(status.state, status.message)
        // 如果服务可用且模型列表为空，顺便获取模型列表
        if (isAvailable.value && availableModels.value.length === 0) {
          fetchModels()
        }
      } else {
        // 无法获取状态，假设未运行
        opState.value = 'stopped'
        isAvailable.value = false
        statusMessage.value = '无法连接到服务管理器'
      }
      lastCheckTime.value = Date.now()
    } catch (e) {
      logError('检查 AI 服务状态失败:', e)
      opState.value = 'error'
      isAvailable.value = false
      statusMessage.value = '检查服务状态失败'
    }
  }

  /**
   * 获取可用模型列表
   */
  async function fetchModels() {
    try {
      const models = await getAvailableModels()
      availableModels.value = models
      // 如果当前模型不在列表中，重置为第一个
      if (models.length > 0 && !models.includes(currentModel.value)) {
        currentModel.value = models[0]
      }
    } catch (e) {
      logError('获取模型列表失败:', e)
    }
  }

  /**
   * 启动服务
   */
  async function start() {
    if (opState.value === 'starting' || opState.value === 'running') return
    
    opState.value = 'starting'
    statusMessage.value = '正在启动 AI Gateway...'
    try {
      await startService('ai-gateway')
      // 启动命令返回后，通常意味着初始化已触发（非阻塞优化后）
      // 我们立即检查一次状态，然后依赖事件或轮询更新
      setTimeout(checkService, 500)
    } catch (e: any) {
      opState.value = 'error'
      statusMessage.value = `启动失败: ${e.toString()}`
      logError('启动 AI 服务失败:', e)
    }
  }

  /**
   * 停止服务
   */
  async function stop() {
    if (opState.value === 'stopping' || opState.value === 'stopped') return

    opState.value = 'stopping'
    statusMessage.value = '正在停止 AI Gateway...'
    try {
      await stopService('ai-gateway')
      opState.value = 'stopped'
      isAvailable.value = false
      statusMessage.value = '服务已停止'
    } catch (e: any) {
      opState.value = 'error'
      statusMessage.value = `停止失败: ${e.toString()}`
      logError('停止 AI 服务失败:', e)
    }
  }

  /**
   * 重启服务
   */
  async function restart() {
    opState.value = 'starting' // 重启也视为启动中
    statusMessage.value = '正在重启 AI Gateway...'
    try {
      await apiRestartService('ai-gateway')
      setTimeout(checkService, 1000)
    } catch (e: any) {
      opState.value = 'error'
      statusMessage.value = `重启失败: ${e.toString()}`
      logError('重启 AI 服务失败:', e)
    }
  }

  return {
    // State
    opState,
    statusMessage,
    isAvailable,
    availableModels,
    currentModel,
    
    // Getters
    isRunning,
    isWorking,
    
    // Actions
    applyServiceState,
    checkService,
    fetchModels,
    start,
    stop,
    restart
  }
})
