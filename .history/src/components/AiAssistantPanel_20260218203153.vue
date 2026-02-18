<template>
  <section class="panel">
    <header class="panel-header">
      <div class="title">
        <span class="dot" :class="{ 'active': isServiceAvailable }"></span>
        <span class="text">AI 助手</span>
      </div>
      <div class="header-actions">
        <select
          v-model="selectedSessionId"
          class="session-select"
          :disabled="isLoadingSessions || sessions.length === 0"
          @change="handleSessionChange"
          title="选择历史会话"
        >
          <option v-for="s in sessions" :key="s.id" :value="s.id">
            {{ s.title }}
          </option>
        </select>
        <button
          type="button"
          class="new-session-btn"
          title="新会话"
          @click="startNewSession"
        >
          ＋
        </button>
        <button
          type="button"
          class="clear-history-btn"
          title="清空历史"
          @click="clearHistory"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
          </svg>
        </button>
      </div>
    </header>

    <main class="messages" ref="containerRef">
      <div 
        v-for="msg in messages" 
        :key="msg.id" 
        class="msg-row" 
        :class="msg.role"
      >
        <div class="bubble">
          <div
            v-if="msg.role === 'assistant'"
            class="ai-markdown"
            v-html="msg.html || ''"
          ></div>
          <p v-else class="plain">{{ msg.text }}</p>
        </div>
      </div>
    </main>

    <footer class="input-area">
      <div class="input-container" :class="{ 'focused': isInputFocused, 'disabled': !isServiceAvailable }">
        <!-- 上方：文本输入区 -->
        <div class="input-row">
          <textarea
            v-model="input"
            ref="inputRef"
            class="input"
            :placeholder="getPlaceholderText()"
            :disabled="!isServiceAvailable || isLoading"
            @keydown="onKeydown"
            @focus="isInputFocused = true"
            @blur="isInputFocused = false"
            @input="handleInput"
          />
        </div>

        <!-- 下方：模型选择和发送按钮 -->
        <div class="input-actions-row">
          <button 
            type="button"
            class="model-select-btn"
            :disabled="!isServiceAvailable || isLoading || availableModels.length === 0"
            :title="availableModels.length === 0 ? '没有可用模型，请检查配置' : `当前模型: ${getModelDisplayName(currentModel)} (${availableModels.length} 个可用)`"
            @click.stop="toggleModelDropdown"
          >
            <span class="model-select-text">{{ currentModel ? getModelDisplayName(currentModel) : '选择模型' }}</span>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="6 9 12 15 18 9"></polyline>
            </svg>
          </button>

          <button 
            type="button" 
            class="send-btn-inline" 
            :disabled="!isServiceAvailable || isLoading || !input.trim()"
            @click="send"
            :title="isLoading ? '发送中...' : '发送 (Enter)'"
          >
            <svg v-if="!isLoading" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <line x1="22" y1="2" x2="11" y2="13"></line>
              <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
            </svg>
            <span v-else class="loading-spinner"></span>
          </button>
        </div>

        <!-- 模型选择下拉菜单（放在 input-container 内部以便正确定位） -->
        <div v-if="showModelDropdown" class="model-dropdown" @click.stop>
          <div 
            v-for="model in availableModels" 
            :key="model"
            class="model-dropdown-item"
            :class="{ 'active': model === currentModel }"
            @click="selectModel(model)"
          >
            {{ getModelDisplayName(model) }}
          </div>
        </div>
      </div>
    </footer>
  </section>
</template>

<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { useAiServiceStore } from '../stores/aiServiceStore'
import { listen } from '@tauri-apps/api/event'
import { storeToRefs } from 'pinia'
import { renderMarkdown } from '../utils/markdownRenderer'

const store = useAiServiceStore()
import {
  sendAIChat,
  sendAIChatStream,
  type OpenAIStreamChunk
} from '../utils/aiService'
import { getServiceStatus } from '../utils/serviceManager'
import {
  loadChatHistory,
  loadSession,
  saveSession,
  createSession,
  addMessageToSession,
  updateSessionMessage,
  clearChatHistory,
  type ChatSession,
  type ChatSessionSummary
} from '../utils/aiHistory'
import { debug, error as logError, info, warn } from '../utils/logger'
import { recordRequest, type RequestMetrics } from '../utils/aiPerformance'
import { isTauriEnvironment } from '../utils/tauri'

// 配置更新事件处理函数（需要在 onUnmounted 中移除）
let handleConfigUpdate: (() => void) | null = null
let unlistenServiceEvent: (() => void) | null = null

type Role = 'user' | 'assistant' | 'system'

interface Message {
  id: number
  role: Role
  text: string
  html?: string
  timestamp?: number
  usage?: {
    prompt_tokens?: number
    completion_tokens?: number
    total_tokens?: number
  }
}

const input = ref('')
const inputRef = ref<HTMLTextAreaElement | null>(null)
const isLoading = ref(false)
const {
  availableModels,
  currentModel,
  isAvailable: isServiceAvailable,
  statusMessage
} = storeToRefs(store)
const isInputFocused = ref(false)
const showModelDropdown = ref(false)
const useStreaming = ref(true) // 默认使用流式响应
const currentSession = ref<ChatSession | null>(null)
const sessions = ref<ChatSessionSummary[]>([])
const selectedSessionId = ref('')
const isLoadingSessions = ref(false)

// 性能统计
const performanceStats = ref<{
  requestCount: number
  totalTokens: number
  averageResponseTime: number
  totalResponseTime: number
  modelStats: Record<string, {
    count: number
    totalTokens: number
    totalTime: number
    averageTime: number
  }>
}>({
  requestCount: 0,
  totalTokens: 0,
  averageResponseTime: 0,
  totalResponseTime: 0,
  modelStats: {},
})

const messages = ref<Message[]>([
  {
    id: 1,
    role: 'assistant',
    text: '正在连接 AI Gateway 服务...',
    html: '',
  },
])

const containerRef = ref<HTMLElement | null>(null)

let idCounter = 2

const renderThrottleMs = 60
const lastRenderAtByMessageId = new Map<number, number>()

const maybeUpdateAssistantHtml = (msg: Message, force: boolean = false) => {
  if (msg.role !== 'assistant') return
  const now = Date.now()
  const last = lastRenderAtByMessageId.get(msg.id) ?? 0
  if (!force && now - last < renderThrottleMs) return
  msg.html = renderMarkdown(msg.text)
  lastRenderAtByMessageId.set(msg.id, now)
}

// 获取模型显示名称
const getModelDisplayName = (modelId: string): string => {
  if (!modelId) {
    return '选择模型'
  }
  
  const modelNames: Record<string, string> = {
    'gpt-3.5-turbo': 'GPT-3.5 Turbo',
    'gpt-4': 'GPT-4',
    'gpt-4-turbo': 'GPT-4 Turbo',
    'deepseek-chat': 'DeepSeek Chat',
    'deepseek-coder': 'DeepSeek Coder',
    'ollama-llama3': 'Ollama Llama3',
  }
  
  if (modelNames[modelId]) {
    return modelNames[modelId]
  }
  
  return modelId
    .replace(/[-_]/g, ' ')
    .split(' ')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ')
}

// 获取占位符文本
const getPlaceholderText = (): string => {
  if (!isServiceAvailable.value) {
    return statusMessage.value || 'AI Gateway 服务未启动...'
  }
  if (availableModels.value.length === 0) {
    return '未配置 AI 模型，请在配置文件中添加模型...'
  }
  return '输入消息...'
}

// 检查服务状态（添加防抖，避免并发检查）
let checkServiceInProgress = false
let lastModelsRefreshAt = 0
const modelsRefreshMinIntervalMs = 2000

const checkService = async (delayMs: number = 0) => {
  // 如果正在检查，跳过本次检查
  if (checkServiceInProgress) {
    debug('[服务状态检测] 跳过本次检查（已有检查正在进行）')
    return
  }
  
  checkServiceInProgress = true
  try {
    if (delayMs > 0) {
      await new Promise<void>(resolve => setTimeout(resolve, delayMs))
    }
    
    // 使用 Store 检查状态
    await store.checkService()
    
    // 关键优化：只有在服务状态不是 stopped 时才进行健康检查
    if (isServiceAvailable.value) {
      if (messages.value.length === 1 && messages.value[0]?.text === '正在连接 AI Gateway 服务...') {
        messages.value[0].text = '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'
        maybeUpdateAssistantHtml(messages.value[0]!, true)
      }
      
      // 获取可用模型列表
      try {
        const now = Date.now()
        if (now - lastModelsRefreshAt >= modelsRefreshMinIntervalMs) {
          lastModelsRefreshAt = now
          await store.fetchModels()
        }
      } catch (error) {
        logError('获取模型列表失败:', error)
      }
    } else {
      // 服务不可用时的处理
      if (messages.value.length === 1 && messages.value[0]?.text === '正在连接 AI Gateway 服务...') {
        messages.value[0].text = '⚠️ AI Gateway 服务未启动，请确保服务正在运行。'
        maybeUpdateAssistantHtml(messages.value[0]!, true)
      }
    }
  } finally {
    checkServiceInProgress = false
  }
}

let serviceEventRefreshTimer: ReturnType<typeof setTimeout> | null = null

const scheduleServiceRefresh = (delay: number = 200) => {
  if (serviceEventRefreshTimer) {
    clearTimeout(serviceEventRefreshTimer)
  }
  serviceEventRefreshTimer = setTimeout(() => {
    checkService().catch(error => {
      debug('事件驱动刷新服务状态失败:', error)
    })
  }, delay)
}

const handleServiceEvent = (payload: any) => {
  if (!payload || typeof payload !== 'object') return

  const stateChanged = payload.StateChanged
  if (stateChanged?.service_id === 'ai-gateway') {
    store.applyServiceState(stateChanged.to, undefined)
    scheduleServiceRefresh(150)
    return
  }

  const started = payload.Started
  if (started?.service_id === 'ai-gateway') {
    scheduleServiceRefresh(0)
    return
  }

  const stopped = payload.Stopped
  if (stopped?.service_id === 'ai-gateway') {
    store.applyServiceState('stopped', 'AI Gateway 服务已停止')
    scheduleServiceRefresh(0)
    return
  }

  const restarted = payload.Restarted
  if (restarted?.service_id === 'ai-gateway') {
    scheduleServiceRefresh(0)
    return
  }

  const errorEvt = payload.Error
  if (errorEvt?.service_id === 'ai-gateway') {
    store.applyServiceState('unhealthy', errorEvt.error || 'AI Gateway 服务异常...')
    scheduleServiceRefresh(200)
  }
}

// 初始化或加载会话
const initSession = async () => {
  if (currentSession.value) return

  isLoadingSessions.value = true
  try {
    const history = await loadChatHistory()
    sessions.value = history.sessions
    selectedSessionId.value = history.currentSessionId || history.sessions[0]?.id || ''

    if (selectedSessionId.value) {
      const session = await loadSession(selectedSessionId.value)
      if (session) {
        currentSession.value = session
        lastRenderAtByMessageId.clear()
        messages.value = session.messages.length
          ? session.messages.map(msg => ({
              id: msg.id,
              role: msg.role,
              text: msg.text,
              html: msg.role === 'assistant' ? renderMarkdown(msg.text) : undefined,
              timestamp: msg.timestamp,
              usage: msg.usage,
            }))
          : [
              {
                id: 1,
                role: 'assistant',
                text: '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。',
                html: renderMarkdown(
                  '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'
                ),
              },
            ]

        idCounter = Math.max(...messages.value.map(m => m.id), 0) + 1
        nextTick(() => scrollToBottom())
        return
      }
    }

    await startNewSession()
  } finally {
    isLoadingSessions.value = false
  }
}

const refreshSessionList = async () => {
  const history = await loadChatHistory()
  sessions.value = history.sessions
  if (history.currentSessionId) {
    selectedSessionId.value = history.currentSessionId
  } else if (!selectedSessionId.value) {
    selectedSessionId.value = history.sessions[0]?.id || ''
  }
}

const startNewSession = async () => {
  const session = createSession(currentModel.value)
  currentSession.value = session
  lastRenderAtByMessageId.clear()
  messages.value = [
    {
      id: 1,
      role: 'assistant',
      text: '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。',
      html: renderMarkdown('🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'),
    },
  ]
  idCounter = 2
  await saveSession(session)
  await refreshSessionList()
  nextTick(() => scrollToBottom())
}

const handleSessionChange = async () => {
  const sid = selectedSessionId.value
  if (!sid) return
  if (currentSession.value?.id === sid) return

  isLoadingSessions.value = true
  try {
    const session = await loadSession(sid)
    if (!session) {
      await startNewSession()
      return
    }

    currentSession.value = session
    lastRenderAtByMessageId.clear()
    messages.value = session.messages.length
      ? session.messages.map(msg => ({
          id: msg.id,
          role: msg.role,
          text: msg.text,
          html: msg.role === 'assistant' ? renderMarkdown(msg.text) : undefined,
          timestamp: msg.timestamp,
          usage: msg.usage,
        }))
      : [
          {
            id: 1,
            role: 'assistant',
            text: '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。',
            html: renderMarkdown('🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'),
          },
        ]
    idCounter = Math.max(...messages.value.map(m => m.id), 0) + 1
    await saveSession({
      ...session,
      messages: session.messages,
    })
    await refreshSessionList()
    nextTick(() => scrollToBottom())
  } finally {
    isLoadingSessions.value = false
  }
}

// 保存消息到历史
const saveMessagesToHistory = async () => {
  if (!currentSession.value) {
    await initSession()
  }
  
  if (currentSession.value) {
    // 更新会话消息
    currentSession.value.messages = messages.value.map(msg => ({
      id: msg.id,
      role: msg.role,
      text: msg.text,
      timestamp: msg.timestamp || Date.now(),
      model: currentModel.value,
      usage: msg.usage,
    }))
    currentSession.value.model = currentModel.value
    if (messages.value.length > 0 && messages.value[0]?.text) {
      const firstUserMsg = messages.value.find(m => m.role === 'user')
      if (firstUserMsg) {
        currentSession.value.title = firstUserMsg.text.substring(0, 30)
      }
    }
    
    await saveSession(currentSession.value)
  }
}

// 性能监控：记录请求指标
const recordPerformanceMetrics = async (
  model: string, 
  tokens: number, 
  responseTime: number,
  success: boolean,
  promptTokens: number = 0,
  completionTokens: number = 0,
  error?: string
) => {
  // 更新本地统计（用于实时显示）
  performanceStats.value.requestCount++
  if (success) {
    performanceStats.value.totalTokens += tokens
    performanceStats.value.totalResponseTime += responseTime
    performanceStats.value.averageResponseTime = 
      performanceStats.value.totalResponseTime / performanceStats.value.requestCount
    
    if (!performanceStats.value.modelStats[model]) {
      performanceStats.value.modelStats[model] = {
        count: 0,
        totalTokens: 0,
        totalTime: 0,
        averageTime: 0,
      }
    }
    
    const modelStat = performanceStats.value.modelStats[model]
    modelStat.count++
    modelStat.totalTokens += tokens
    modelStat.totalTime += responseTime
    modelStat.averageTime = modelStat.totalTime / modelStat.count
  }
  
  // 持久化到文件
  try {
    const metrics: RequestMetrics = {
      model,
      timestamp: Date.now(),
      responseTime,
      promptTokens: promptTokens || Math.floor(tokens * 0.6), // 估算
      completionTokens: completionTokens || Math.floor(tokens * 0.4), // 估算
      totalTokens: tokens,
      success,
      error,
    }
    await recordRequest(metrics)
  } catch (error) {
    logError('记录性能指标失败:', error)
  }
}

// 发送消息
const send = async () => {
  if (!input.value.trim() || isLoading.value || !isServiceAvailable.value) {
    return
  }
  
  // 确保会话已初始化
  if (!currentSession.value) {
    await initSession()
  }
  
  const userMessage = input.value.trim()
  input.value = ''
  handleInput()
  
  const requestStartTime = Date.now()
  
  // 添加用户消息
  const userMsg: Message = {
    id: idCounter++,
    role: 'user',
    text: userMessage,
    timestamp: Date.now(),
  }
  messages.value.push(userMsg)
  
  // 保存用户消息
  if (currentSession.value) {
    await addMessageToSession(currentSession.value.id, {
      id: userMsg.id,
      role: userMsg.role,
      text: userMsg.text,
      timestamp: userMsg.timestamp,
      model: currentModel.value,
    })
  }
  
  // 添加加载中的助手消息
  const assistantMsgId = idCounter++
  const loadingMsg: Message = {
    id: assistantMsgId,
    role: 'assistant',
    text: '',
    html: '',
    timestamp: Date.now(),
  }
  messages.value.push(loadingMsg)
  
  // 立即保存加载中的消息到会话，确保后续可以更新
  if (currentSession.value) {
    await addMessageToSession(currentSession.value.id, {
      id: assistantMsgId,
      role: 'assistant',
      text: '',
      timestamp: loadingMsg.timestamp,
      model: currentModel.value,
    })
  }
  
  isLoading.value = true
  nextTick(() => scrollToBottom())
  
  try {
    const requestMessages = messages.value
      .filter(msg => msg.id !== assistantMsgId)
      .map(msg => ({
        role: msg.role,
        content: msg.text,
      }))
    
    if (useStreaming.value) {
      // 流式响应
      let fullContent = ''
      
      await sendAIChatStream(
        currentModel.value,
        requestMessages,
        {
          temperature: 0.7,
          max_tokens: 2000,
          onChunk: (chunk: OpenAIStreamChunk) => {
            // 提取增量内容
            const delta = chunk.choices[0]?.delta
            if (delta?.content) {
              fullContent += delta.content
              
              // 更新消息
              const index = messages.value.findIndex(msg => msg.id === assistantMsgId)
              if (index !== -1) {
                const msg = messages.value[index]
                if (msg) {
                  msg.text = fullContent
                  maybeUpdateAssistantHtml(msg)
                  nextTick(() => scrollToBottom())
                }
              }
            }
          },
          onComplete: async (usage) => {
            const responseTime = Date.now() - requestStartTime
            
            // 更新消息
            const index = messages.value.findIndex(msg => msg.id === assistantMsgId)
            if (index !== -1) {
              const msg = messages.value[index]
              if (msg) {
                msg.text = fullContent || '无响应'
                maybeUpdateAssistantHtml(msg, true)
                msg.usage = usage
                
                // 更新性能统计
                if (usage) {
                  const totalTokens = usage.total_tokens || 0
                  const promptTokens = usage.prompt_tokens || 0
                  const completionTokens = usage.completion_tokens || 0
                  await recordPerformanceMetrics(
                    currentModel.value,
                    totalTokens,
                    responseTime,
                    true,
                    promptTokens,
                    completionTokens
                  )
                }
                
                // 保存消息
                if (currentSession.value) {
                  await updateSessionMessage(currentSession.value.id, assistantMsgId, {
                    text: msg.text,
                    usage: msg.usage,
                  })
                }
              }
            }
            
            isLoading.value = false
            nextTick(() => scrollToBottom())
            
            // 延迟检测服务状态，避免在事件循环关闭瞬间误判
            // 关键优化：只有在服务运行中时才检测，避免用户停止后继续检查
            setTimeout(async () => {
              const status = await getServiceStatus('ai-gateway')
              // 只有在服务不是 stopped 状态时才检测
              if (status && status.state !== 'stopped') {
                debug('[请求完成] 开始延迟检测服务状态...')
                await checkService(0)
                debug('[请求完成] 延迟检测服务状态完成')
              } else {
                debug('[请求完成] 服务已停止，跳过状态检测')
              }
            }, 500) // 延迟 500ms 检测
          },
          onError: async (error) => {
            logError('流式响应错误:', error)
            const responseTime = Date.now() - requestStartTime
            await recordPerformanceMetrics(
              currentModel.value,
              0,
              responseTime,
              false,
              0,
              0,
              error.message
            )
            const index = messages.value.findIndex(msg => msg.id === assistantMsgId)
            if (index !== -1) {
              const msg = messages.value[index]
              if (msg) {
                // 检查是否是连接错误
                let errorText = `❌ 错误: ${error.message}`
                if (error.message.includes('无法连接') || error.message.includes('CONNECTION_REFUSED')) {
                  errorText = '❌ 无法连接到 AI Gateway 服务，请检查服务是否正在运行'
                }
                msg.text = errorText
                maybeUpdateAssistantHtml(msg, true)
              }
            }
            isLoading.value = false
            nextTick(() => scrollToBottom())
            
            // 延迟检测服务状态，避免在事件循环关闭瞬间误判
            // 关键优化：只有在服务运行中时才检测，避免用户停止后继续检查
            setTimeout(async () => {
              const status = await getServiceStatus('ai-gateway')
              // 只有在服务不是 stopped 状态时才检测
              if (status && status.state !== 'stopped') {
                debug('[请求完成] 开始延迟检测服务状态...')
                await checkService(0)
                debug('[请求完成] 延迟检测服务状态完成')
              } else {
                debug('[请求完成] 服务已停止，跳过状态检测')
              }
            }, 500) // 延迟 500ms 检测
          },
        }
      )
    } else {
      // 非流式响应
      const response = await sendAIChat(
        currentModel.value,
        requestMessages,
        {
          temperature: 0.7,
          max_tokens: 2000,
          stream: false,
        }
      )
      
      const responseTime = Date.now() - requestStartTime
      
      // 更新助手消息
      const index = messages.value.findIndex(msg => msg.id === assistantMsgId)
      if (index !== -1) {
        const msg = messages.value[index]
        if (msg) {
          const content = response.choices[0]?.message?.content || '无响应'
          msg.text = content
          maybeUpdateAssistantHtml(msg, true)
          msg.usage = response.usage
          
          // 更新性能统计
          if (response.usage) {
            const totalTokens = response.usage.total_tokens || 0
            const promptTokens = response.usage.prompt_tokens || 0
            const completionTokens = response.usage.completion_tokens || 0
            await recordPerformanceMetrics(
              currentModel.value,
              totalTokens,
              responseTime,
              true,
              promptTokens,
              completionTokens
            )
          }
          
          // 保存消息
          if (currentSession.value) {
            await updateSessionMessage(currentSession.value.id, assistantMsgId, {
              text: msg.text,
              usage: msg.usage,
            })
          }
        }
      }
      
      isLoading.value = false
      nextTick(() => scrollToBottom())
      
      // 延迟检测服务状态，避免在事件循环关闭瞬间误判
      // 关键优化：只有在服务运行中时才检测，避免用户停止后继续检查
      setTimeout(async () => {
        const status = await getServiceStatus('ai-gateway')
        // 只有在服务不是 stopped 状态时才检测
        if (status && status.state !== 'stopped') {
          await checkService()
        } else {
          debug('[请求完成] 服务已停止，跳过状态检测')
        }
      }, 500) // 延迟 500ms 检测
    }
  } catch (error) {
    logError('发送 AI 消息失败:', error)
    const responseTime = Date.now() - requestStartTime
    await recordPerformanceMetrics(
      currentModel.value,
      0,
      responseTime,
      false,
      0,
      0,
      error instanceof Error ? error.message : String(error)
    )
    const index = messages.value.findIndex(msg => msg.id === assistantMsgId)
    if (index !== -1) {
      const msg = messages.value[index]
      if (msg) {
        const errorMessage = error instanceof Error ? error.message : String(error)
        // 检查是否是连接错误
        let errorText = `❌ 错误: ${errorMessage}`
        if (errorMessage.includes('无法连接') || errorMessage.includes('CONNECTION_REFUSED')) {
          errorText = '❌ 无法连接到 AI Gateway 服务，请检查服务是否正在运行'
        }
        msg.text = errorText
        maybeUpdateAssistantHtml(msg, true)
      }
    }
    isLoading.value = false
    nextTick(() => scrollToBottom())
    
    // 延迟检测服务状态，避免在事件循环关闭瞬间误判
    // 关键优化：只有在服务运行中时才检测，避免用户停止后继续检查
    setTimeout(async () => {
      const status = await getServiceStatus('ai-gateway')
      // 只有在服务不是 stopped 状态时才检测
      if (status && status.state !== 'stopped') {
        await checkService()
      } else {
        debug('[请求完成] 服务已停止，跳过状态检测')
      }
    }, 500) // 延迟 500ms 检测
  }
}

const onKeydown = (e: KeyboardEvent) => {
  // Enter 发送，Shift + Enter 换行
  if (e.key === 'Enter' && !e.shiftKey && !isLoading.value && !e.isComposing) {
    e.preventDefault()
    send()
  }
  // ESC 关闭下拉菜单
  if (e.key === 'Escape' && showModelDropdown.value) {
    showModelDropdown.value = false
  }
}

// 输入框内容变化时调整高度
const handleInput = () => {
  if (inputRef.value) {
    inputRef.value.style.height = 'auto'
    const newHeight = Math.min(inputRef.value.scrollHeight, 120)
    inputRef.value.style.height = `${newHeight}px`
  }
}

// 切换模型下拉菜单
const toggleModelDropdown = (e?: Event) => {
  if (e) {
    e.stopPropagation()
  }
  
  if (!isServiceAvailable.value || isLoading.value || availableModels.value.length === 0) {
    return
  }
  
  showModelDropdown.value = !showModelDropdown.value
  debug('切换模型下拉菜单:', showModelDropdown.value, '可用模型:', availableModels.value)
}

// 选择模型
const selectModel = (modelId: string) => {
  currentModel.value = modelId
  showModelDropdown.value = false
  debug('选择模型:', modelId)
}

// 点击外部关闭下拉菜单
const handleClickOutside = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  if (!target.closest('.input-container') && !target.closest('.model-dropdown')) {
    showModelDropdown.value = false
  }
}

const scrollToBottom = () => {
  if (containerRef.value) {
    containerRef.value.scrollTop = containerRef.value.scrollHeight
  }
}

// 清空历史
const clearHistory = async () => {
  if (confirm('确定要清空所有聊天历史吗？')) {
    await clearChatHistory()
    currentSession.value = null
    lastRenderAtByMessageId.clear()
    messages.value = [
      {
        id: 1,
        role: 'assistant',
        text: '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。',
        html: renderMarkdown('🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'),
      },
    ]
    idCounter = 2
    info('聊天历史已清空')
    
    // 清空历史后，确保模型列表仍然可用（如果服务可用）
    // 不要清空 availableModels，因为它与聊天历史无关
    if (isServiceAvailable.value && availableModels.value.length > 0) {
      // 如果当前模型不在可用列表中，选择第一个可用模型
      if (!availableModels.value.includes(currentModel.value)) {
        const firstModel = availableModels.value[0]
        if (firstModel) {
          currentModel.value = firstModel
        }
      }
    } else {
      // 如果服务不可用，尝试重新检查服务状态
      checkService().catch(error => {
        debug('清空历史后检查服务状态失败:', error)
      })
    }
  }
}

// 监听模型变化，更新会话
watch(currentModel, async () => {
  if (currentSession.value) {
    currentSession.value.model = currentModel.value
    await saveSession(currentSession.value)
  }
})

onMounted(async () => {
  if (messages.value[0]) {
    maybeUpdateAssistantHtml(messages.value[0], true)
  }

  if (isTauriEnvironment() && !unlistenServiceEvent) {
    try {
      unlistenServiceEvent = await listen('service_event', (event: any) => {
        handleServiceEvent(event?.payload)
      })
    } catch (error) {
      warn('[AiAssistantPanel] 订阅 service_event 失败:', error)
    }
  }

  // 加载聊天历史（不阻塞）
  initSession().catch(error => {
    logError('初始化会话失败:', error)
  })
  
  // 检查服务状态（快速检查，不等待）
  checkService().catch(error => {
    debug('检查服务状态失败:', error)
  })
  
  // 监听配置更新事件，实时刷新模型列表
  handleConfigUpdate = () => {
    debug('收到配置更新事件，刷新模型列表')
    checkService().catch(error => {
      debug('刷新模型列表失败:', error)
    })
  }
  
  window.addEventListener('ai-config-updated', handleConfigUpdate)
  
  // 定期保存消息（防抖）
  let saveTimer: ReturnType<typeof setTimeout> | null = null
  watch(messages, () => {
    if (saveTimer) {
      clearTimeout(saveTimer)
    }
    saveTimer = setTimeout(() => {
      saveMessagesToHistory()
    }, 2000) // 2秒后保存
  }, { deep: true })
  
  // 添加点击外部关闭下拉菜单的监听
  document.addEventListener('click', handleClickOutside)
  
  // 初始化输入框高度
  nextTick(() => {
    if (inputRef.value) {
      handleInput()
    }
    // 每次进入时滚动到最后一条消息处
    scrollToBottom()
  })
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
  
  // 移除配置更新事件监听
  if (handleConfigUpdate) {
    window.removeEventListener('ai-config-updated', handleConfigUpdate)
    handleConfigUpdate = null
  }
  
  if (unlistenServiceEvent) {
    unlistenServiceEvent()
    unlistenServiceEvent = null
  }
  
  // 保存最终状态
  saveMessagesToHistory()
})
</script>

<style scoped>
.panel {
  height: 100%;
  min-height: 400px;
  max-height: 100%;
  display: flex;
  flex-direction: column;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background: var(--bg-secondary);
  box-shadow: var(--shadow-md);
  overflow: hidden;
}

.panel-header {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-tertiary);
  flex-shrink: 0;
}

.title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #ef4444;
  transition: background 0.2s;
}

.dot.active {
  background: #22c55e;
}

.text {
  color: var(--text-primary);
  font-weight: 500;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.session-select {
  height: 28px;
  max-width: 200px;
  padding: 4px 28px 4px 8px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 500;
  outline: none;
  cursor: pointer;
  appearance: none;
  -webkit-appearance: none;
  transition: border-color 0.15s ease;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%238b949e' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  background-size: 12px 12px;
}

.session-select:hover:not(:disabled) {
  background-color: var(--bg-tertiary);
  border-color: var(--text-secondary);
}

.session-select:focus-visible {
  border-color: var(--accent-primary);
}

.session-select:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.session-select option {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.new-session-btn,
.clear-history-btn {
  width: 28px;
  height: 28px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-secondary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.1s ease;
  user-select: none;
}

.new-session-btn {
  font-size: 16px;
  line-height: 1;
}

.new-session-btn:hover:not(:disabled) {
  background: var(--bg-tertiary);
  color: var(--text-primary);
  border-color: var(--text-secondary);
}

.clear-history-btn:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
  border-color: rgba(239, 68, 68, 0.5);
}

.new-session-btn:disabled,
.clear-history-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.messages {
  flex: 1;
  min-height: 0;
  padding: 16px;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  gap: 16px;
  background: var(--bg-primary);
  overscroll-behavior: contain;
}

/* Scrollbar styles inherited from global */

.msg-row {
  display: flex;
}

.msg-row.user {
  justify-content: flex-end;
}

.msg-row.assistant {
  justify-content: flex-start;
}

.bubble {
  max-width: 85%;
  border-radius: 6px;
  padding: 10px 14px;
  font-size: 13.5px;
  line-height: 1.6;
  word-wrap: break-word;
  word-break: break-word;
}

.msg-row.user .bubble {
  background: var(--accent-primary);
  color: #ffffff;
  border: 1px solid transparent;
}

.msg-row.assistant .bubble {
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
}

.bubble .plain {
  margin: 0;
  white-space: pre-wrap;
}

.msg-row.assistant .bubble .ai-markdown {
  font-size: 13.5px;
  line-height: 1.6;
}

.msg-row.assistant .bubble .ai-markdown :deep(h1),
.msg-row.assistant .bubble .ai-markdown :deep(h2),
.msg-row.assistant .bubble .ai-markdown :deep(h3),
.msg-row.assistant .bubble .ai-markdown :deep(h4),
.msg-row.assistant .bubble .ai-markdown :deep(h5),
.msg-row.assistant .bubble .ai-markdown :deep(h6) {
  margin: 0.8em 0 0.5em;
  font-weight: 600;
  color: var(--text-primary);
}

.msg-row.assistant .bubble .ai-markdown :deep(p) {
  margin: 0.5em 0;
}

.msg-row.assistant .bubble .ai-markdown :deep(blockquote) {
  margin: 0.8em 0;
  padding: 0.2em 0 0.2em 0.8em;
  border-left: 3px solid var(--border-color);
  color: var(--text-muted);
}

.msg-row.assistant .bubble .ai-markdown :deep(code:not(pre code)) {
  background: rgba(110, 118, 129, 0.4);
  border-radius: 4px;
  padding: 0.2em 0.4em;
  font-family: ui-monospace, SFMono-Regular, monospace;
  font-size: 0.9em;
}

.msg-row.assistant .bubble .ai-markdown :deep(pre) {
  margin: 0.8em 0;
  padding: 12px;
  background: #161b22; /* Specific dark for code blocks */
  border: 1px solid var(--border-color);
  border-radius: 6px;
  overflow-x: auto;
}

.msg-row.assistant .bubble .ai-markdown :deep(pre code) {
  background: transparent;
  padding: 0;
  border: none;
  font-family: ui-monospace, SFMono-Regular, monospace;
  font-size: 12.5px;
  color: var(--text-primary);
}

.msg-row.assistant .bubble .ai-markdown :deep(a) {
  color: var(--accent-primary);
  text-decoration: none;
}

.msg-row.assistant .bubble .ai-markdown :deep(a):hover {
  text-decoration: underline;
}

.msg-row.assistant .bubble .ai-markdown :deep(hr) {
  border: none;
  border-top: 1px solid var(--border-color);
  margin: 1em 0;
}

.msg-row.assistant .bubble .ai-markdown :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 0.8em 0;
}

.msg-row.assistant .bubble .ai-markdown :deep(th),
.msg-row.assistant .bubble .ai-markdown :deep(td) {
  border: 1px solid var(--border-color);
  padding: 6px 10px;
}

.msg-row.assistant .bubble .ai-markdown :deep(th) {
  background: var(--bg-tertiary);
  font-weight: 600;
}

.input-area {
  flex: 0 0 auto;
  border-top: 1px solid var(--border-color);
  padding: 12px;
  background: var(--bg-secondary);
  position: relative;
}

.input-container {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 0;
  background: var(--bg-input);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  overflow: hidden;
  transition: border-color 0.15s ease;
}

.input-container:hover:not(.disabled) {
  border-color: var(--text-secondary);
}

.input-container.focused:not(.disabled) {
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 1px var(--accent-primary);
}

.input-container.disabled {
  opacity: 0.6;
  cursor: not-allowed;
  background: var(--bg-secondary);
}

.input-row {
  display: flex;
  padding: 8px 10px;
  background: transparent;
}

.input-actions-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-tertiary);
  gap: 8px;
}

.model-select-btn {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.1s ease;
  height: 28px;
  white-space: nowrap;
}

.model-select-btn:hover:not(:disabled) {
  background: rgba(110, 118, 129, 0.1);
  color: var(--text-primary);
}

.model-select-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.model-select-text {
  user-select: none;
}

.model-select-btn svg {
  width: 14px;
  height: 14px;
  opacity: 0.8;
}

.model-dropdown {
  position: absolute;
  bottom: calc(100% + 4px);
  left: 0;
  min-width: 180px;
  max-width: 280px;
  max-height: 300px;
  overflow-y: auto;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  box-shadow: var(--shadow-lg);
  z-index: 1000;
}

.model-dropdown-item {
  padding: 8px 12px;
  color: var(--text-secondary);
  font-size: 12.5px;
  cursor: pointer;
  transition: background 0.1s ease;
  user-select: none;
  border-bottom: 1px solid var(--border-color);
}

.model-dropdown-item:last-child {
  border-bottom: none;
}

.model-dropdown-item:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.model-dropdown-item.active {
  background: rgba(31, 111, 235, 0.1);
  color: var(--accent-primary);
  font-weight: 500;
}

.input {
  flex: 1;
  resize: none;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 13.5px;
  padding: 0;
  outline: none;
  min-height: 40px;
  max-height: 120px;
  line-height: 1.5;
  font-family: inherit;
  width: 100%;
}

.input:disabled {
  cursor: not-allowed;
}

.input::placeholder {
  color: var(--text-muted);
}

.send-btn-inline {
  flex: 0 0 auto;
  width: 28px;
  height: 28px;
  border-radius: 4px;
  border: none;
  background: var(--accent-primary);
  color: #ffffff;
  cursor: pointer;
  transition: all 0.1s ease;
  display: flex;
  align-items: center;
  justify-content: center;
}

.send-btn-inline:hover:not(:disabled) {
  background: var(--accent-hover);
}

.send-btn-inline:active:not(:disabled) {
  transform: translateY(1px);
}

.send-btn-inline:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  background: var(--bg-tertiary);
}

</style>
