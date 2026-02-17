<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { 
  checkAIServiceHealth, 
  checkAIServiceStatus, 
  startAIService, 
  stopAIService,
  getAvailableModels,
  reloadAIConfig
} from '../utils/aiService'
import MonitoringPanel from '../components/MonitoringPanel.vue'
import {
  readModelsConfig,
  writeModelsConfig,
  getValidModels,
  groupModelsByAdapter,
  type ModelConfig,
  type ModelsConfig
} from '../utils/aiConfig'
import { debug, info, warn, error as logError } from '../utils/logger'

import AppLayout from '../layouts/AppLayout.vue'

const router = useRouter()

// AI Gateway 服务状态
const aiServiceStatus = ref<'unknown' | 'running' | 'stopped'>('unknown')
const isCheckingStatus = ref(false)
const availableModels = ref<string[]>([])
const isLoadingModels = ref(false)

// 模型配置
const modelsConfig = ref<ModelsConfig>({ models: [] })
const isLoadingConfig = ref(false)
const isSavingConfig = ref(false)
const activeTab = ref<'service' | 'models' | 'monitoring'>('service')
const expandedAdapters = ref<Record<string, boolean>>({})
const editingApiKeys = ref<Record<string, string>>({})
const showingApiKeys = ref<Record<string, boolean>>({})

// 检查 AI Gateway 服务状态
const checkStatus = async () => {
  isCheckingStatus.value = true
  try {
    // 检查进程状态
    const processRunning = await checkAIServiceStatus()
    // 检查健康状态
    const isHealthy = await checkAIServiceHealth()
    
    if (processRunning && isHealthy) {
      aiServiceStatus.value = 'running'
      // 加载模型列表
      await loadModels()
    } else if (processRunning) {
      aiServiceStatus.value = 'running'
      warn('AI Gateway 进程在运行，但健康检查失败，可能是服务还在启动中')
      // 即使健康检查失败，也尝试加载模型列表（可能服务刚启动，还没完全就绪）
      await loadModels().catch(error => {
        debug('健康检查失败时加载模型列表失败:', error)
      })
    } else {
      aiServiceStatus.value = 'stopped'
      availableModels.value = []
    }
  } catch (error) {
    logError('检查 AI Gateway 状态失败:', error)
    aiServiceStatus.value = 'stopped'
  } finally {
    isCheckingStatus.value = false
  }
}

// 启动 AI Gateway 服务
const startService = async () => {
  try {
    info('正在启动 AI Gateway 服务...')
    await startAIService()
    // 等待服务启动（给服务更多时间初始化）
    await new Promise(resolve => setTimeout(resolve, 3000))
    await checkStatus()
    info('AI Gateway 服务已启动')
    
    // 服务启动后，重新加载模型列表
    if (aiServiceStatus.value === 'running') {
      await loadModels()
    }
  } catch (error) {
    logError('启动 AI Gateway 服务失败:', error)
    alert(`启动服务失败: ${error instanceof Error ? error.message : String(error)}`)
  }
}

// 停止 AI Gateway 服务
const stopService = async () => {
  try {
    info('正在停止 AI Gateway 服务...')
    await stopAIService()
    await checkStatus()
    info('AI Gateway 服务已停止')
  } catch (error) {
    logError('停止 AI Gateway 服务失败:', error)
    alert(`停止服务失败: ${error instanceof Error ? error.message : String(error)}`)
  }
}

// 重启 AI Gateway 服务
const restartService = async () => {
  try {
    const wasRunning = aiServiceStatus.value === 'running'
    
    if (wasRunning) {
      info('正在停止 AI Gateway 服务...')
      await stopAIService()
      // 等待服务完全停止
      await new Promise(resolve => setTimeout(resolve, 1000))
      await checkStatus()
    }
    
    info('正在启动 AI Gateway 服务...')
    await startAIService()
    // 等待服务启动（给服务更多时间初始化）
    await new Promise(resolve => setTimeout(resolve, 3000))
    await checkStatus()
    
    if (aiServiceStatus.value === 'running') {
      info('AI Gateway 服务已重启')
      // 重新加载模型列表
      await loadModels()
      // 触发配置更新事件，通知聊天页面刷新模型列表
      window.dispatchEvent(new CustomEvent('ai-config-updated'))
      return true
    } else {
      warn('服务重启后状态异常')
      return false
    }
  } catch (error) {
    logError('重启 AI Gateway 服务失败:', error)
    warn(`重启服务失败: ${error instanceof Error ? error.message : String(error)}`)
    return false
  }
}

// 加载可用模型列表
const loadModels = async () => {
  if (aiServiceStatus.value !== 'running') {
    return
  }
  
  isLoadingModels.value = true
  try {
    const models = await getAvailableModels()
    availableModels.value = models
    debug('已加载模型列表:', models)
  } catch (error) {
    logError('加载模型列表失败:', error)
    availableModels.value = []
  } finally {
    isLoadingModels.value = false
  }
}

// 状态显示文本
const statusText = computed(() => {
  switch (aiServiceStatus.value) {
    case 'running':
      return '运行中'
    case 'stopped':
      return '已停止'
    default:
      return '未知'
  }
})

// 状态颜色
const statusColor = computed(() => {
  switch (aiServiceStatus.value) {
    case 'running':
      return '#22c55e'
    case 'stopped':
      return '#ef4444'
    default:
      return '#94a3b8'
  }
})

// 加载模型配置
const loadModelsConfig = async () => {
  isLoadingConfig.value = true
  try {
    const config = await readModelsConfig()
    modelsConfig.value = config
    debug('已加载模型配置')
    
    // 初始化展开状态
    const grouped = groupModelsByAdapter(getValidModels(config))
    for (const adapter in grouped) {
      if (!(adapter in expandedAdapters.value)) {
        expandedAdapters.value[adapter] = true
      }
    }
  } catch (error) {
    logError('加载模型配置失败:', error)
  } finally {
    isLoadingConfig.value = false
  }
}

// 保存模型配置
const saveModelsConfig = async () => {
  isSavingConfig.value = true
  try {
    await writeModelsConfig(modelsConfig.value)
    info('模型配置已保存')
    
    // 不重新加载前端配置，避免覆盖当前状态
    // 只在需要时重新加载（比如从外部修改了配置文件）
    
    // 如果服务正在运行，尝试自动重新加载配置，失败则自动重启
    if (aiServiceStatus.value === 'running') {
      try {
        // 等待一小段时间确保文件已写入
        await new Promise(resolve => setTimeout(resolve, 100))
        await reloadAIConfig()
        info('配置已重新加载到服务')
        // 重新加载可用模型列表
        await loadModels()
        
        // 触发配置更新事件，通知聊天页面刷新模型列表
        window.dispatchEvent(new CustomEvent('ai-config-updated'))
      } catch (error) {
        // 处理 /reload 端点不可用的情况（服务需要重启）
        const errorMsg = error instanceof Error ? error.message : String(error)
        if (errorMsg.includes('404') || errorMsg.includes('Not Found')) {
          // 404 错误说明服务需要重启才能加载新配置，自动重启
          info('服务需要重启以加载新配置，正在自动重启...')
          const restartSuccess = await restartService()
          if (!restartSuccess) {
            warn('自动重启服务失败，请手动重启服务')
          }
        } else {
          // 其他错误，尝试自动重启
          warn('自动重新加载配置失败，尝试重启服务...')
          logError('重新加载配置失败:', error)
          const restartSuccess = await restartService()
          if (!restartSuccess) {
            warn('自动重启服务失败，请手动重启服务')
          }
        }
      }
    } else {
      // 即使服务未运行，也触发事件，让聊天页面知道配置已更新
      window.dispatchEvent(new CustomEvent('ai-config-updated'))
    }
  } catch (error) {
    logError('保存模型配置失败:', error)
    alert(`保存配置失败: ${error instanceof Error ? error.message : String(error)}`)
  } finally {
    isSavingConfig.value = false
  }
}

// 切换模型启用状态
const toggleModel = async (model: ModelConfig) => {
  try {
    const newEnabled = !model.enabled
    
    // 同时更新本地 model 和 modelsConfig 中的状态
    model.enabled = newEnabled
    
    // 确保同步到 modelsConfig.value
    const modelIndex = modelsConfig.value.models.findIndex(
      (m) => !('_comment' in m) && !('_note' in m) && (m as ModelConfig).id === model.id
    )
    if (modelIndex !== -1) {
      (modelsConfig.value.models[modelIndex] as ModelConfig).enabled = newEnabled
    }
    
    // 保存配置（不重新加载，避免覆盖状态）
    await writeModelsConfig(modelsConfig.value)
    info('模型状态已更新')
    
    // 如果服务正在运行，尝试重新加载配置，失败则自动重启
    if (aiServiceStatus.value === 'running') {
      try {
        await new Promise(resolve => setTimeout(resolve, 100))
        await reloadAIConfig()
        // 重新加载可用模型列表
        await loadModels()
        
        // 触发配置更新事件，通知聊天页面刷新模型列表
        window.dispatchEvent(new CustomEvent('ai-config-updated'))
      } catch (error) {
        // 处理 /reload 端点不可用的情况（服务需要重启）
        const errorMsg = error instanceof Error ? error.message : String(error)
        if (errorMsg.includes('404') || errorMsg.includes('Not Found')) {
          // 404 错误说明服务需要重启才能加载新配置，自动重启
          info('服务需要重启以加载新配置，正在自动重启...')
          const restartSuccess = await restartService()
          if (!restartSuccess) {
            warn('自动重启服务失败，请手动重启服务')
          }
        } else {
          // 其他错误，尝试自动重启
          debug('重新加载配置失败，尝试重启服务...')
          const restartSuccess = await restartService()
          if (!restartSuccess) {
            debug('自动重启服务失败')
          }
        }
      }
    } else {
      // 即使服务未运行，也触发事件，让聊天页面知道配置已更新
      window.dispatchEvent(new CustomEvent('ai-config-updated'))
    }
  } catch (error) {
    logError('切换模型状态失败:', error)
    // 恢复状态
    const oldEnabled = !model.enabled
    model.enabled = oldEnabled
    const modelIndex = modelsConfig.value.models.findIndex(
      (m) => !('_comment' in m) && !('_note' in m) && (m as ModelConfig).id === model.id
    )
    if (modelIndex !== -1) {
      (modelsConfig.value.models[modelIndex] as ModelConfig).enabled = oldEnabled
    }
    alert(`切换模型状态失败: ${error instanceof Error ? error.message : String(error)}`)
  }
}

// 更新模型 API Key
const saveModelApiKey = async (model: ModelConfig) => {
  const newApiKey = editingApiKeys.value[model.id]
  if (newApiKey === undefined) {
    return
  }
  
  try {
    // 更新本地配置对象
    model.api_key = newApiKey
    
    // 确保同步到 modelsConfig
    const modelIndex = modelsConfig.value.models.findIndex(
      (m) => !('_comment' in m) && !('_note' in m) && (m as ModelConfig).id === model.id
    )
    if (modelIndex !== -1) {
      (modelsConfig.value.models[modelIndex] as ModelConfig).api_key = newApiKey
    }
    
    delete editingApiKeys.value[model.id]
    
    // 直接保存到配置文件，不重新加载
    await writeModelsConfig(modelsConfig.value)
    info('API Key 已保存到配置文件')
    
    // 如果服务正在运行，尝试重新加载配置，失败则自动重启
    if (aiServiceStatus.value === 'running') {
      try {
        await new Promise(resolve => setTimeout(resolve, 100))
        await reloadAIConfig()
        await loadModels()
        
        // 触发配置更新事件，通知聊天页面刷新模型列表
        window.dispatchEvent(new CustomEvent('ai-config-updated'))
      } catch (error) {
        // 处理 /reload 端点不可用的情况（服务需要重启）
        const errorMsg = error instanceof Error ? error.message : String(error)
        if (errorMsg.includes('404') || errorMsg.includes('Not Found')) {
          // 404 错误说明服务需要重启才能加载新配置，自动重启
          info('服务需要重启以加载新配置，正在自动重启...')
          const restartSuccess = await restartService()
          if (!restartSuccess) {
            warn('自动重启服务失败，请手动重启服务')
          }
        } else {
          // 其他错误，尝试自动重启
          debug('重新加载配置失败，尝试重启服务...')
          const restartSuccess = await restartService()
          if (!restartSuccess) {
            debug('自动重启服务失败')
          }
        }
      }
    } else {
      // 即使服务未运行，也触发事件，让聊天页面知道配置已更新
      window.dispatchEvent(new CustomEvent('ai-config-updated'))
    }
  } catch (error) {
    logError('更新 API Key 失败:', error)
    alert(`更新 API Key 失败: ${error instanceof Error ? error.message : String(error)}`)
  }
}

// 开始编辑 API Key
const startEditApiKey = (model: ModelConfig) => {
  editingApiKeys.value[model.id] = model.api_key
}

// 取消编辑 API Key
const cancelEditApiKey = (modelId: string) => {
  delete editingApiKeys.value[modelId]
}

// 切换 API Key 显示/隐藏
const toggleShowApiKey = (modelId: string) => {
  showingApiKeys.value[modelId] = !showingApiKeys.value[modelId]
}

// 获取有效的模型列表
const validModels = computed(() => getValidModels(modelsConfig.value))

// 按适配器分组的模型
const groupedModels = computed(() => groupModelsByAdapter(validModels.value))

// 获取适配器显示名称
const getAdapterDisplayName = (adapter: string): string => {
  const names: Record<string, string> = {
    'openai_compat': 'OpenAI 兼容',
    'custom_http': '自定义 HTTP',
    'websocket': 'WebSocket',
    'process': '本地进程',
    'unknown': '未知类型'
  }
  return names[adapter] || adapter.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase())
}

// 切换适配器展开状态
const toggleAdapter = (adapter: string) => {
  expandedAdapters.value[adapter] = !expandedAdapters.value[adapter]
}

// 滚动到指定区域
const scrollToSection = (sectionId: string) => {
  const element = document.getElementById(sectionId)
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

const goBack = () => {
  router.back()
}

onMounted(async () => {
  // 并行加载，不阻塞 UI
  Promise.all([
    checkStatus(),
    loadModelsConfig()
  ]).catch(error => {
    logError('加载设置页面数据失败:', error)
  })
  
  // 定期检查状态（每 5 秒）
  const statusInterval = setInterval(checkStatus, 5000)
  
  onUnmounted(() => {
    clearInterval(statusInterval)
  })
})
</script>

<template>
  <div class="settings-root">
    <header class="settings-header">
      <button
        type="button"
        class="back-button"
        title="返回上层"
        @click="goBack"
      >
        ←
      </button>
      <div class="header-text">
        <h1>设置中心</h1>
        <p>管理全局配置和 AI Gateway 服务。</p>
      </div>
    </header>

    <div class="settings-main">
      <aside class="settings-nav">
        <div class="nav-section">
          <h3 class="nav-section-title">设置</h3>
          <nav class="nav-list">
            <button 
              type="button"
              class="nav-item"
              :class="{ active: activeTab === 'service' }"
              @click="activeTab = 'service'; scrollToSection('ai-gateway')"
            >
              <span class="nav-item-icon">🤖</span>
              <span class="nav-item-text">AI Gateway</span>
            </button>
            <button 
              type="button"
              class="nav-item"
              :class="{ active: activeTab === 'models' }"
              @click="activeTab = 'models'; scrollToSection('models-config')"
            >
              <span class="nav-item-icon">⚙️</span>
              <span class="nav-item-text">模型配置</span>
            </button>
            <button 
              type="button"
              class="nav-item"
              :class="{ active: activeTab === 'monitoring' }"
              @click="activeTab = 'monitoring'; scrollToSection('monitoring-panel')"
            >
              <span class="nav-item-icon">📊</span>
              <span class="nav-item-text">线程监控</span>
            </button>
          </nav>
        </div>
      </aside>

      <section class="settings-content">
        <!-- 服务管理 -->
        <div id="ai-gateway" class="config-panel" :class="{ hidden: activeTab !== 'service' }">
          <div class="panel-header">
            <h2>AI Gateway 服务</h2>
            <p>管理 AI Gateway 服务的启动和停止</p>
          </div>

          <div class="config-section">
            <div class="section-item">
              <div class="section-label">
                <span class="label-text">服务状态</span>
                <span class="label-desc">当前 AI Gateway 服务的运行状态</span>
              </div>
              <div class="section-value">
                <div class="status-indicator">
                  <span 
                    class="status-dot" 
                    :style="{ backgroundColor: statusColor }"
                  ></span>
                  <span class="status-text">{{ statusText }}</span>
                  <button
                    type="button"
                    class="refresh-btn"
                    :disabled="isCheckingStatus"
                    @click="checkStatus"
                    title="刷新状态"
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <polyline points="23 4 23 10 17 10"></polyline>
                      <polyline points="1 20 1 14 7 14"></polyline>
                      <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
                    </svg>
                  </button>
                </div>
              </div>
            </div>

            <div class="section-item">
              <div class="section-label">
                <span class="label-text">服务控制</span>
                <span class="label-desc">启动或停止 AI Gateway 服务</span>
              </div>
              <div class="section-value">
                <div class="action-buttons">
                  <button
                    type="button"
                    class="action-btn start-btn"
                    :disabled="aiServiceStatus === 'running' || isCheckingStatus"
                    @click="startService"
                  >
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <polygon points="5 3 19 12 5 21 5 3"></polygon>
                    </svg>
                    启动服务
                  </button>
                  <button
                    type="button"
                    class="action-btn stop-btn"
                    :disabled="aiServiceStatus !== 'running' || isCheckingStatus"
                    @click="stopService"
                  >
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <rect x="6" y="6" width="12" height="12"></rect>
                    </svg>
                    停止服务
                  </button>
                </div>
              </div>
            </div>

            <div v-if="aiServiceStatus === 'running'" class="section-item">
              <div class="section-label">
                <span class="label-text">可用模型</span>
                <span class="label-desc">当前已启用并可用的 AI 模型列表</span>
              </div>
              <div class="section-value">
                <div v-if="isLoadingModels" class="models-loading">
                  <span class="loading-spinner"></span>
                  <span>加载中...</span>
                </div>
                <div v-else-if="availableModels.length > 0" class="models-list">
                  <div
                    v-for="model in availableModels"
                    :key="model"
                    class="model-item"
                  >
                    <span class="model-icon">✨</span>
                    <span class="model-name">{{ model }}</span>
                  </div>
                </div>
                <div v-else class="models-empty">
                  <span>暂无可用模型，请检查配置文件</span>
                </div>
              </div>
            </div>

            <div class="section-item">
              <div class="section-label">
                <span class="label-text">服务信息</span>
                <span class="label-desc">AI Gateway 服务的基本信息</span>
              </div>
              <div class="section-value">
                <div class="info-list">
                  <div class="info-item">
                    <span class="info-label">监听地址:</span>
                    <span class="info-value">http://127.0.0.1:8765</span>
                  </div>
                  <div class="info-item">
                    <span class="info-label">配置文件:</span>
                    <span class="info-value">ai_service/config/models.json</span>
                  </div>
                  <div class="info-item">
                    <span class="info-label">Python 路径:</span>
                    <span class="info-value">python313/python.exe</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 模型配置 -->
        <div id="models-config" class="config-panel" :class="{ hidden: activeTab !== 'models' }">
          <div class="panel-header">
            <h2>AI 模型配置</h2>
            <p>配置 API Key 和选择要使用的模型</p>
          </div>

          <div v-if="isLoadingConfig" class="config-loading">
            <span class="loading-spinner"></span>
            <span>加载配置中...</span>
          </div>

          <div v-else class="config-section">
            <!-- 保存按钮 -->
            <div class="section-item save-header">
              <div class="section-label">
                <span class="label-text">配置操作</span>
                <span class="label-desc">保存配置后需要重启服务才能生效</span>
              </div>
              <div class="section-value">
                <button
                  type="button"
                  class="action-btn save-btn"
                  :disabled="isSavingConfig"
                  @click="saveModelsConfig"
                >
                  <svg v-if="!isSavingConfig" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
                    <polyline points="17 21 17 13 7 13 7 21"></polyline>
                    <polyline points="7 3 7 8 15 8"></polyline>
                  </svg>
                  <span v-else class="loading-spinner-small"></span>
                  {{ isSavingConfig ? '保存中...' : '保存配置' }}
                </button>
              </div>
            </div>

            <!-- 按适配器分组显示模型 -->
            <div
              v-for="(models, adapter) in groupedModels"
              :key="adapter"
              class="adapter-group"
            >
              <div class="adapter-header" @click="toggleAdapter(adapter)">
                <div class="adapter-info">
                  <svg
                    class="expand-icon"
                    :class="{ expanded: expandedAdapters[adapter] }"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <polyline points="6 9 12 15 18 9"></polyline>
                  </svg>
                  <span class="adapter-name">{{ getAdapterDisplayName(adapter) }}</span>
                  <span class="adapter-count">({{ models.length }})</span>
                </div>
              </div>

              <div v-if="expandedAdapters[adapter]" class="models-list-group">
                <div
                  v-for="model in models"
                  :key="model.id"
                  class="model-config-item"
                >
                  <div class="model-header">
                    <div class="model-info">
                      <label class="model-toggle">
                        <input type="checkbox" :checked="model.enabled" @change.prevent="toggleModel(model)" />
                        <span class="toggle-slider"></span>
                      </label>
                      <div class="model-details">
                        <div class="model-id">{{ model.id }}</div>
                        <div class="model-meta">
                          <span class="model-endpoint">{{ model.base_url }}</span>
                        </div>
                      </div>
                    </div>
                  </div>

                  <div class="model-config">
                    <div class="config-row">
                      <label class="config-label">API Key:</label>
                      <div class="config-input-group">
                        <template v-if="editingApiKeys[model.id] !== undefined">
                          <input
                            v-model="editingApiKeys[model.id]"
                            type="text"
                            class="config-input"
                            placeholder="输入 API Key（直接保存到配置文件）"
                            @keydown.enter.prevent="saveModelApiKey(model)"
                            @keydown.escape.prevent="cancelEditApiKey(model.id)"
                          />
                          <button
                            type="button"
                            class="config-btn save-btn-small"
                            @click="saveModelApiKey(model)"
                          >
                            保存
                          </button>
                          <button
                            type="button"
                            class="config-btn cancel-btn-small"
                            @click="cancelEditApiKey(model.id)"
                          >
                            取消
                          </button>
                        </template>
                        <template v-else>
                          <div class="api-key-display">
                            <span v-if="model.api_key === 'not-needed' || model.api_key === ''" class="no-key">
                              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <circle cx="12" cy="12" r="10"></circle>
                                <line x1="12" y1="8" x2="12" y2="12"></line>
                                <line x1="12" y1="16" x2="12.01" y2="16"></line>
                              </svg>
                              无需 API Key
                            </span>
                            <span v-else class="direct-key">
                              {{ showingApiKeys[model.id] ? model.api_key : '••••••••••••' }}
                            </span>
                          </div>
                          <button
                            type="button"
                            class="config-btn edit-btn"
                            @click="startEditApiKey(model)"
                          >
                            编辑
                          </button>
                          <button
                            v-if="model.api_key !== 'not-needed' && model.api_key !== ''"
                            type="button"
                            class="config-btn show-btn"
                            @click="toggleShowApiKey(model.id)"
                          >
                            {{ showingApiKeys[model.id] ? '隐藏' : '显示' }}
                          </button>
                        </template>
                      </div>
                    </div>

                    <!-- 模型信息显示 -->
                    <div class="config-row model-info-row">
                      <label class="config-label">模型信息:</label>
                      <div class="model-info-details">
                        <div class="info-item">
                          <span class="info-label">适配器:</span>
                          <span class="info-value">{{ getAdapterDisplayName(model.adapter) }}</span>
                        </div>
                        <div v-if="model.base_url" class="info-item">
                          <span class="info-label">Base URL:</span>
                          <span class="info-value">{{ model.base_url }}</span>
                        </div>
                        <div v-if="model.model && model.model !== model.id" class="info-item">
                          <span class="info-label">模型名称:</span>
                          <span class="info-value">{{ model.model }}</span>
                        </div>
                        <div v-if="model.request_format || model.response_format" class="info-item">
                          <span class="info-label">协议:</span>
                          <span class="info-value">
                            {{ model.request_format || 'default' }} → {{ model.response_format || 'openai' }}
                          </span>
                        </div>
                        <div v-if="model.endpoint" class="info-item">
                          <span class="info-label">Endpoint:</span>
                          <span class="info-value">{{ model.endpoint }}</span>
                        </div>
                        <div v-if="model.command" class="info-item">
                          <span class="info-label">命令:</span>
                          <span class="info-value">{{ model.command }} {{ (model.args || []).join(' ') }}</span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div v-if="validModels.length === 0" class="empty-state">
              <p>暂无模型配置，请检查 models.json 文件</p>
            </div>
          </div>
        </div>

        <!-- 线程监控 -->
        <div id="monitoring-panel" class="config-panel" :class="{ hidden: activeTab !== 'monitoring' }">
          <MonitoringPanel />
        </div>
      </section>
    </div>

    <footer class="page-footer">
      <div class="footer-content">
        <span class="copyright">© 2025 By 序章</span>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.settings-root {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  color: var(--text-primary);
  overflow: hidden;
}

.settings-header {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 22px;
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-secondary);
  z-index: 10;
}

.back-button {
  flex: 0 0 auto;
  width: 28px;
  height: 28px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-primary);
  cursor: pointer;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.16s ease-out;
}

.back-button:hover {
  background: var(--bg-hover);
  border-color: var(--text-primary);
}

.header-text h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.3px;
}

.header-text p {
  margin: 2px 0 0;
  font-size: 12px;
  color: var(--text-secondary);
}

.settings-main {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

.settings-nav {
  flex: 0 0 240px;
  padding: 20px 0;
  border-right: 1px solid var(--border-color);
  background: var(--bg-secondary);
  overflow-y: auto;
}

.nav-section {
  padding: 0 16px;
}

.nav-section-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
  margin: 0 0 12px;
  padding: 0 12px;
}

.nav-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  border-radius: 6px;
  transition: all 0.2s ease;
}

.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.nav-item.active {
  background: var(--accent-color);
  color: white;
}

.nav-item.active .nav-item-icon {
    color: white;
}

.nav-item-icon {
  font-size: 16px;
  width: 20px;
  text-align: center;
  color: var(--text-secondary);
}

.settings-content {
  flex: 1;
  min-width: 0;
  padding: 24px 32px;
  overflow-y: auto;
  background: var(--bg-primary);
}

.config-panel {
  max-width: 900px;
  margin: 0 auto;
}

.panel-header {
  margin-bottom: 24px;
}

.panel-header h2 {
  margin: 0 0 8px;
  font-size: 24px;
  font-weight: 600;
  color: var(--text-primary);
}

.panel-header p {
  margin: 0;
  font-size: 14px;
  color: var(--text-secondary);
}

.config-section {
  background: var(--bg-secondary);
  border-radius: 8px;
  border: 1px solid var(--border-color);
  overflow: hidden;
}

.section-item {
  display: flex;
  padding: 20px 24px;
  border-bottom: 1px solid var(--border-color);
}

.section-item:last-child {
  border-bottom: none;
}

.section-label {
  flex: 0 0 200px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.label-text {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.label-desc {
  font-size: 12px;
  color: var(--text-secondary);
}

.section-value {
  flex: 1;
  display: flex;
  align-items: center;
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 10px;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.status-text {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.refresh-btn {
  width: 28px;
  height: 28px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-primary);
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.refresh-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  border-color: var(--text-primary);
}

.refresh-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.action-buttons {
  display: flex;
  gap: 12px;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
  font-weight: 500;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.action-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  border-color: var(--text-primary);
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.start-btn:hover:not(:disabled) {
  background: rgba(34, 197, 94, 0.1);
  border-color: #22c55e;
  color: #22c55e;
}

.stop-btn:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.1);
  border-color: #ef4444;
  color: #ef4444;
}

.action-btn svg {
  width: 16px;
  height: 16px;
}

.models-loading {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--text-secondary);
  font-size: 13px;
}

.models-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 300px;
  overflow-y: auto;
}

.model-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
}

.model-item:hover {
  border-color: var(--accent-color);
}

.model-icon {
  font-size: 16px;
}

.model-name {
  font-size: 13px;
  color: var(--text-primary);
  font-family: 'Consolas', 'Monaco', monospace;
}

.models-empty {
  color: var(--text-secondary);
  font-size: 13px;
  font-style: italic;
}

.info-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.info-item {
  display: flex;
  gap: 12px;
  font-size: 13px;
}

.info-label {
  color: var(--text-secondary);
  min-width: 100px;
}

.info-value {
  color: var(--text-primary);
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 12px;
}

.loading-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid var(--border-color);
  border-top-color: var(--text-primary);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.page-footer {
  flex: 0 0 auto;
  padding: 12px 24px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-secondary);
}

.footer-content {
  max-width: 1400px;
  margin: 0 auto;
  text-align: center;
}

.copyright {
  font-size: 12px;
  color: var(--text-secondary);
}

/* 模型配置样式 */
.config-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 40px;
  color: var(--text-secondary);
  font-size: 14px;
}

.save-header {
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-color);
}

.save-btn {
  background: var(--accent-color);
  border-color: var(--accent-color);
  color: white;
}

.save-btn:hover:not(:disabled) {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
  color: white;
}

.loading-spinner-small {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

.adapter-group {
  margin-bottom: 16px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  overflow: hidden;
}

.adapter-header {
  padding: 12px 16px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-color);
  cursor: pointer;
  transition: background 0.2s ease;
}

.adapter-header:hover {
  background: var(--bg-hover);
}

.adapter-info {
  display: flex;
  align-items: center;
  gap: 10px;
}

.expand-icon {
  transition: transform 0.2s ease;
  color: var(--text-secondary);
}

.expand-icon.expanded {
  transform: rotate(180deg);
}

.adapter-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.adapter-count {
  font-size: 12px;
  color: var(--text-secondary);
}

.models-list-group {
  background: var(--bg-primary);
}

.model-config-item {
  padding: 16px;
  border-bottom: 1px solid var(--border-color);
}

.model-config-item:last-child {
  border-bottom: none;
}

.model-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.model-info {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
}

.model-toggle {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
  cursor: pointer;
}

.model-toggle input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--border-color);
  transition: 0.3s;
  border-radius: 24px;
}

.toggle-slider:before {
  position: absolute;
  content: "";
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background-color: white;
  transition: 0.3s;
  border-radius: 50%;
}

.model-toggle input:checked + .toggle-slider {
  background-color: var(--accent-color);
}

.model-toggle input:checked + .toggle-slider:before {
  transform: translateX(20px);
}

.model-details {
  flex: 1;
}

.model-id {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  font-family: 'Consolas', 'Monaco', monospace;
  margin-bottom: 4px;
}

.model-meta {
  font-size: 12px;
  color: var(--text-secondary);
}

.model-endpoint {
  font-family: 'Consolas', 'Monaco', monospace;
}

.model-config {
  margin-left: 56px;
}

.config-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.config-label {
  flex: 0 0 80px;
  font-size: 13px;
  color: var(--text-secondary);
}

.config-input-group {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
}

.config-input {
  flex: 1;
  padding: 8px 12px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: 13px;
  font-family: 'Consolas', 'Monaco', monospace;
  transition: border-color 0.2s ease;
}

.config-input:focus {
  outline: none;
  border-color: var(--accent-color);
}

.api-key-display {
  flex: 1;
  padding: 8px 12px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  font-size: 13px;
  font-family: 'Consolas', 'Monaco', monospace;
  display: flex;
  align-items: center;
  gap: 8px;
}

.env-key {
  color: var(--accent-color);
  display: flex;
  align-items: center;
  gap: 6px;
}

.direct-key {
  color: var(--text-primary);
}

.config-btn {
  padding: 6px 12px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.config-btn:hover {
  background: var(--bg-hover);
  border-color: var(--text-primary);
}

.save-btn-small {
  background: rgba(34, 197, 94, 0.1);
  border-color: #22c55e;
  color: #22c55e;
}

.save-btn-small:hover {
  background: rgba(34, 197, 94, 0.2);
}

.cancel-btn-small {
  background: rgba(239, 68, 68, 0.1);
  border-color: #ef4444;
  color: #ef4444;
}

.cancel-btn-small:hover {
  background: rgba(239, 68, 68, 0.2);
}

.edit-btn {
  color: var(--accent-color);
}

.show-btn {
  color: var(--text-secondary);
}

.model-info-row {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--border-color);
}

.model-info-details {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.model-info-details .info-item {
  display: flex;
  gap: 8px;
  font-size: 12px;
}

.model-info-details .info-label {
  color: var(--text-secondary);
  min-width: 70px;
}

.model-info-details .info-value {
  color: var(--text-primary);
  font-family: 'Consolas', 'Monaco', monospace;
  word-break: break-all;
}

.empty-state {
  padding: 40px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 14px;
}

.hidden {
  display: none;
}
</style>
