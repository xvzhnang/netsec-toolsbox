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
import { listen } from '@tauri-apps/api/event'
import MarkdownIt from 'markdown-it'
import type { Options } from 'markdown-it/lib/index.mjs'
import type Token from 'markdown-it/lib/token.mjs'
import type Renderer from 'markdown-it/lib/renderer.mjs'
import type { RenderRule } from 'markdown-it/lib/renderer.mjs'
import hljs from 'highlight.js'
import 'highlight.js/styles/github-dark.css'
import { 
  sendAIChat, 
  sendAIChatStream,
  getAvailableModels, 
  waitForAIService, 
  startAIService,
  checkAIServiceStatus,
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
const isServiceAvailable = ref(false)
type OpState = 'idle' | 'starting' | 'running' | 'stopping' | 'stopped' | 'error'
const opState = ref<OpState>('idle')
const statusMessage = ref('')
const currentModel = ref('gpt-3.5-turbo')
const availableModels = ref<string[]>([])
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

let markdown: MarkdownIt
markdown = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
  highlight: (code: string, language: string, attrs: string): string => {
    void attrs
    const lang = (language || '').trim().toLowerCase()
    if (lang && hljs.getLanguage(lang)) {
      const highlighted = hljs.highlight(code, { language: lang, ignoreIllegals: true }).value
      return `<pre class="hljs"><code class="hljs language-${markdown.utils.escapeHtml(lang)}">${highlighted}</code></pre>`
    }
    return `<pre class="hljs"><code class="hljs">${markdown.utils.escapeHtml(code)}</code></pre>`
  },
})

markdown.validateLink = (url: string): boolean => {
  const normalized = url.trim().toLowerCase()
  if (normalized.startsWith('#') || normalized.startsWith('/')) return true
  return (
    normalized.startsWith('http://') ||
    normalized.startsWith('https://') ||
    normalized.startsWith('mailto:')
  )
}

const defaultLinkOpen: RenderRule =
  markdown.renderer.rules.link_open ??
  ((tokens: Token[], idx: number, options: Options, env: any, self: Renderer): string => {
    void env
    return self.renderToken(tokens, idx, options)
  })

markdown.renderer.rules.link_open = (
  tokens: Token[],
  idx: number,
  options: Options,
  env: any,
  self: Renderer
): string => {
  void env
  const token = tokens[idx]
  if (token) {
    token.attrSet('rel', 'noopener noreferrer')
    token.attrSet('target', '_blank')
  }
  return defaultLinkOpen(tokens, idx, options, env, self)
}

const renderAssistantMarkdown = (text: string): string => {
  return markdown.render(text || '')
}

const renderThrottleMs = 60
const lastRenderAtByMessageId = new Map<number, number>()

const maybeUpdateAssistantHtml = (msg: Message, force: boolean = false) => {
  if (msg.role !== 'assistant') return
  const now = Date.now()
  const last = lastRenderAtByMessageId.get(msg.id) ?? 0
  if (!force && now - last < renderThrottleMs) return
  msg.html = renderAssistantMarkdown(msg.text)
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
// 注意：使用连接池后，不再需要自动重启逻辑
// 连接池会自动处理故障转移和 Worker 恢复

const checkService = async (delayMs: number = 0) => {
  // 如果正在检查，跳过本次检查
  if (checkServiceInProgress) {
    debug('[服务状态检测] 跳过本次检查（已有检查正在进行）')
    return
  }
  
  checkServiceInProgress = true
  try {
    info(`[服务状态检测] 开始检查 AI Gateway 服务状态... (延迟: ${delayMs}ms)`)
    
    // 使用统一服务管理检查服务状态
    debug('[服务状态检测] 使用统一服务管理检查服务状态...')
    
    const serviceStatus = await getServiceStatus('ai-gateway')
    const isHealthy = serviceStatus?.is_available && serviceStatus?.is_healthy || false
    const previousStatus = isServiceAvailable.value
    isServiceAvailable.value = isHealthy
    const state = serviceStatus?.state
    if (!state) {
      opState.value = isHealthy ? 'running' : 'stopped'
      statusMessage.value = isHealthy ? '' : 'AI Gateway 服务未启动...'
    } else if (state === 'starting' || state === 'warmup' || state === 'restarting') {
      opState.value = 'starting'
      statusMessage.value = serviceStatus?.message || 'AI Gateway 启动中...'
    } else if (state === 'stopping') {
      opState.value = 'stopping'
      statusMessage.value = serviceStatus?.message || 'AI Gateway 停止中...'
    } else if (state === 'stopped') {
      opState.value = 'stopped'
      statusMessage.value = serviceStatus?.message || 'AI Gateway 服务未启动...'
    } else if (state === 'unhealthy') {
      opState.value = 'error'
      statusMessage.value = serviceStatus?.message || 'AI Gateway 服务异常...'
    } else {
      opState.value = 'running'
      statusMessage.value = serviceStatus?.message || ''
    }
    
    if (isHealthy) {
      if (!previousStatus) {
        info(`[服务状态检测] ✅ AI Gateway 服务已恢复运行 (状态: ${serviceStatus?.state})`)
      } else {
        debug('[服务状态检测] ✅ AI Gateway 服务运行正常')
      }
      // 获取可用模型列表
      try {
        const models = await getAvailableModels()
        debug('获取到的模型列表:', models)
        // 始终更新模型列表，即使为空也要更新（清空旧数据）
        availableModels.value = models
        
        if (models.length > 0) {
          if (!availableModels.value.includes(currentModel.value)) {
            // 如果当前模型不可用，选择第一个可用模型
            if (models[0]) {
              currentModel.value = models[0]
              debug('切换到第一个可用模型:', models[0])
            }
          }
          debug('可用模型数量:', availableModels.value.length)
        } else {
          debug('警告: 没有获取到任何模型')
          // 如果没有可用模型，但之前有模型列表，保留当前模型选择
          // 只有在确实没有模型时才清空
          if (availableModels.value.length === 0) {
            currentModel.value = ''
          }
        }
        
        // 更新欢迎消息
        if (messages.value.length === 1 && messages.value[0]?.text === '正在连接 AI Gateway 服务...') {
          messages.value[0].text = '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'
          maybeUpdateAssistantHtml(messages.value[0]!, true)
        }
      } catch (error) {
        debug('[服务状态检测] ⚠️ 获取模型列表失败，但服务可能仍在运行')
        logError('获取模型列表失败:', error)
        // 不要清空模型列表，保留之前的列表（可能只是临时网络问题）
        // availableModels.value = []
        // currentModel.value = ''
      }
    } else {
      if (previousStatus) {
        warn('[服务状态检测] ❌ AI Gateway 服务状态变为不可用（可能只是临时断开）')
      } else {
        debug('[服务状态检测] ❌ AI Gateway 服务不可用')
      }
      // 使用连接池后，不再需要手动重启服务
      // 连接池会自动处理故障转移：
      // 1. 自动选择健康的 Worker
      // 2. 自动恢复失败的 Worker（后台健康检查线程）
      // 3. 熔断机制防止问题扩散
      debug('[服务状态检测] 连接池会自动处理故障转移，无需手动重启')
      
      // 服务不可用时，不清空模型列表（保留之前的列表，可能只是临时断开）
      // availableModels.value = []
      // currentModel.value = ''
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

const applyServiceStateQuick = (state: string | undefined, message: string | undefined) => {
  if (!state) return

  if (state === 'starting' || state === 'warmup' || state === 'restarting') {
    opState.value = 'starting'
    isServiceAvailable.value = false
    statusMessage.value = message || 'AI Gateway 启动中...'
    return
  }
  if (state === 'stopping') {
    opState.value = 'stopping'
    isServiceAvailable.value = false
    statusMessage.value = message || 'AI Gateway 停止中...'
    return
  }
  if (state === 'stopped') {
    opState.value = 'stopped'
    isServiceAvailable.value = false
    statusMessage.value = message || 'AI Gateway 服务未启动...'
    return
  }
  if (state === 'unhealthy') {
    opState.value = 'error'
    isServiceAvailable.value = false
    statusMessage.value = message || 'AI Gateway 服务异常...'
    return
  }
  opState.value = 'running'
  isServiceAvailable.value = state === 'idle' || state === 'busy' || state === 'degraded'
  if (!isServiceAvailable.value) {
    statusMessage.value = message || 'AI Gateway 服务未启动...'
  } else {
    statusMessage.value = message || ''
  }
}

const handleServiceEvent = (payload: any) => {
  if (!payload || typeof payload !== 'object') return

  const stateChanged = payload.StateChanged
  if (stateChanged?.service_id === 'ai-gateway') {
    applyServiceStateQuick(stateChanged.to, undefined)
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
    applyServiceStateQuick('stopped', 'AI Gateway 服务已停止')
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
    applyServiceStateQuick('unhealthy', errorEvt.error || 'AI Gateway 服务异常...')
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
              html: msg.role === 'assistant' ? renderAssistantMarkdown(msg.text) : undefined,
              timestamp: msg.timestamp,
              usage: msg.usage,
            }))
          : [
              {
                id: 1,
                role: 'assistant',
                text: '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。',
                html: renderAssistantMarkdown(
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
      html: renderAssistantMarkdown('🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'),
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
          html: msg.role === 'assistant' ? renderAssistantMarkdown(msg.text) : undefined,
          timestamp: msg.timestamp,
          usage: msg.usage,
        }))
      : [
          {
            id: 1,
            role: 'assistant',
            text: '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。',
            html: renderAssistantMarkdown('🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'),
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
            info('[请求完成] 将在 500ms 后检测服务状态（避免误判）...')
            setTimeout(async () => {
              info('[请求完成] 开始延迟检测服务状态...')
              // 传递 0 延迟，因为已经在 setTimeout 中延迟了 500ms
              await checkService(0)
              info('[请求完成] 延迟检测服务状态完成')
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
            info('[请求完成] 将在 500ms 后检测服务状态（避免误判）...')
            setTimeout(async () => {
              info('[请求完成] 开始延迟检测服务状态...')
              // 传递 0 延迟，因为已经在 setTimeout 中延迟了 500ms
              await checkService(0)
              info('[请求完成] 延迟检测服务状态完成')
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
      setTimeout(async () => {
        await checkService()
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
    setTimeout(async () => {
      await checkService()
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
        html: renderAssistantMarkdown('🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'),
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
  // 加载聊天历史（不阻塞）
  initSession().catch(error => {
    logError('初始化会话失败:', error)
  })
  
  // 检查服务状态（快速检查，不等待）
  checkService().catch(error => {
    debug('检查服务状态失败:', error)
  })
  
  // 尝试启动服务（后台执行，不阻塞 UI）
  setTimeout(async () => {
    try {
      // 先快速检查服务是否已运行
      const isRunning = await checkAIServiceStatus().catch(() => false)
      if (isRunning) {
        debug('AI Gateway 服务已在运行')
        await checkService()
        return
      }
      
      await startAIService()
      // 异步等待服务就绪（不阻塞）
      waitForAIService(6, 500).then(isReady => {
        if (isReady) {
          info('AI Gateway 服务已启动并就绪')
          checkService().catch(() => {})
        } else {
          debug('AI Gateway 服务启动中，将在需要时自动连接')
        }
      }).catch(error => {
        debug('等待 AI Gateway 服务就绪失败:', error)
      })
    } catch (error) {
      debug('自动启动 AI Gateway 服务失败（不影响应用使用）:', error)
    }
  }, 200) // 延迟 200ms 执行
  
  // 定期检查服务状态和模型列表（缩短间隔以更快响应配置变化）
  // 注意：避免在请求处理期间频繁检查，增加延迟以避免误判
  startPeriodicServiceCheck()
  
  // 监听配置更新事件，实时刷新模型列表
  handleConfigUpdate = () => {
    debug('收到配置更新事件，刷新模型列表')
    checkService().catch(error => {
      debug('刷新模型列表失败:', error)
    })
  }
  
  window.addEventListener('ai-config-updated', handleConfigUpdate)

  handleVisibilityChange = () => {
    if (document.hidden) {
      stopPeriodicServiceCheck()
      return
    }
    startPeriodicServiceCheck()
    checkService().catch(() => {})
  }
  document.addEventListener('visibilitychange', handleVisibilityChange)
  
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
  stopPeriodicServiceCheck()
  document.removeEventListener('click', handleClickOutside)
  
  // 移除配置更新事件监听
  if (handleConfigUpdate) {
    window.removeEventListener('ai-config-updated', handleConfigUpdate)
    handleConfigUpdate = null
  }

  if (handleVisibilityChange) {
    document.removeEventListener('visibilitychange', handleVisibilityChange)
    handleVisibilityChange = null
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
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: #1e1e1e;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  overflow: hidden;
}

.panel-header {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  background: #252526;
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
  border-radius: 999px;
  background: #ef4444;
  transition: background 0.2s;
}

.dot.active {
  background: radial-gradient(circle at 30% 0, #bbf7d0, #22c55e);
}

.text {
  color: #cccccc;
  font-weight: 500;
}

.messages {
  flex: 1;
  min-height: 0;
  padding: 16px;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: #1e1e1e;
  overscroll-behavior: contain;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
}

.messages::-webkit-scrollbar {
  width: 10px;
}

.messages::-webkit-scrollbar-track {
  background: transparent;
}

.messages::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 5px;
  border: 2px solid transparent;
  background-clip: padding-box;
}

.messages::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
  background-clip: padding-box;
}

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
  border-radius: 10px;
  padding: 12px 16px;
  font-size: 13.5px;
  line-height: 1.65;
  word-wrap: break-word;
  word-break: break-word;
}

.msg-row.user .bubble {
  background: #0e639c;
  color: #ffffff;
  box-shadow: 0 2px 4px rgba(14, 99, 156, 0.2);
}

.msg-row.assistant .bubble {
  background: linear-gradient(180deg, #252526 0%, #1f1f20 100%);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #cccccc;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.bubble .plain {
  margin: 0;
  white-space: pre-wrap;
}

.msg-row.assistant .bubble .ai-markdown {
  font-size: 13.5px;
  line-height: 1.65;
}

.msg-row.assistant .bubble .ai-markdown :deep(h1),
.msg-row.assistant .bubble .ai-markdown :deep(h2),
.msg-row.assistant .bubble .ai-markdown :deep(h3),
.msg-row.assistant .bubble .ai-markdown :deep(h4),
.msg-row.assistant .bubble .ai-markdown :deep(h5),
.msg-row.assistant .bubble .ai-markdown :deep(h6) {
  margin: 0.8em 0 0.5em;
  font-weight: 650;
  color: #e6edf3;
}

.msg-row.assistant .bubble .ai-markdown :deep(h1) { font-size: 1.25em; }
.msg-row.assistant .bubble .ai-markdown :deep(h2) { font-size: 1.18em; }
.msg-row.assistant .bubble .ai-markdown :deep(h3) { font-size: 1.1em; }
.msg-row.assistant .bubble .ai-markdown :deep(h4) { font-size: 1.04em; }
.msg-row.assistant .bubble .ai-markdown :deep(h5) { font-size: 1.0em; }
.msg-row.assistant .bubble .ai-markdown :deep(h6) { font-size: 0.96em; opacity: 0.95; }

.msg-row.assistant .bubble .ai-markdown :deep(p) {
  margin: 0.6em 0;
}

.msg-row.assistant .bubble .ai-markdown :deep(p:first-child) {
  margin-top: 0;
}

.msg-row.assistant .bubble .ai-markdown :deep(p:last-child) {
  margin-bottom: 0;
}

.msg-row.assistant .bubble .ai-markdown :deep(ul),
.msg-row.assistant .bubble .ai-markdown :deep(ol) {
  padding-left: 1.4em;
  margin: 0.6em 0;
}

.msg-row.assistant .bubble .ai-markdown :deep(li) {
  margin: 0.25em 0;
}

.msg-row.assistant .bubble .ai-markdown :deep(blockquote) {
  margin: 0.8em 0;
  padding: 0.2em 0 0.2em 0.8em;
  border-left: 3px solid rgba(0, 122, 204, 0.7);
  color: rgba(204, 204, 204, 0.9);
}

.msg-row.assistant .bubble .ai-markdown :deep(code:not(pre code)) {
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 0.05em 0.35em;
  border-radius: 6px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.95em;
}

.msg-row.assistant .bubble .ai-markdown :deep(pre) {
  margin: 0.8em 0;
  padding: 10px 12px;
  background: #0d1117;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  overflow-x: auto;
}

.msg-row.assistant .bubble .ai-markdown :deep(pre code) {
  background: transparent;
  border: none;
  padding: 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 12.5px;
  line-height: 1.6;
  display: block;
}

.msg-row.assistant .bubble .ai-markdown :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: 10px;
  margin: 0.6em 0;
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.msg-row.assistant .bubble .ai-markdown :deep(a) {
  color: #4fc1ff;
  text-decoration: underline;
  text-underline-offset: 2px;
}

.msg-row.assistant .bubble .ai-markdown :deep(hr) {
  border: none;
  border-top: 1px solid rgba(255, 255, 255, 0.12);
  margin: 0.9em 0;
}

.msg-row.assistant .bubble .ai-markdown :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 0.8em 0;
}

.msg-row.assistant .bubble .ai-markdown :deep(th),
.msg-row.assistant .bubble .ai-markdown :deep(td) {
  border: 1px solid rgba(255, 255, 255, 0.12);
  padding: 6px 8px;
}

.msg-row.assistant .bubble .ai-markdown :deep(th) {
  background: rgba(255, 255, 255, 0.05);
  font-weight: 600;
}

.input-area {
  flex: 0 0 auto;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  padding: 16px 20px;
  background: #1e1e1e;
  position: relative;
}

.input-container {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 0;
  background: #2d2d30;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  overflow: hidden;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

.input-container:hover:not(.disabled) {
  border-color: rgba(255, 255, 255, 0.18);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
}

.input-container.focused:not(.disabled) {
  border-color: rgba(0, 122, 204, 0.6);
  box-shadow: 
    0 2px 8px rgba(0, 0, 0, 0.4),
    0 0 0 2px rgba(0, 122, 204, 0.15);
}

.input-container.disabled {
  opacity: 0.5;
  cursor: not-allowed;
  background: #252526;
}

.input-row {
  display: flex;
  padding: 12px 14px;
  background: transparent;
}

.input-actions-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.2);
  gap: 8px;
}

.model-select-btn {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(60, 60, 60, 0.4);
  color: #cccccc;
  font-size: 12px;
  font-weight: 500;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  height: 32px;
  white-space: nowrap;
}

.model-select-btn:hover:not(:disabled) {
  background: rgba(60, 60, 60, 0.6);
  border-color: rgba(255, 255, 255, 0.12);
  color: #ffffff;
}

.model-select-btn:active:not(:disabled) {
  background: rgba(60, 60, 60, 0.7);
}

.model-select-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  background: rgba(40, 40, 40, 0.3);
}

.model-select-text {
  user-select: none;
  letter-spacing: 0.2px;
}

.model-select-btn svg {
  width: 14px;
  height: 14px;
  opacity: 0.8;
  transition: all 0.2s ease;
}

.model-select-btn:hover:not(:disabled) svg {
  opacity: 1;
}

.model-dropdown {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 12px;
  min-width: 180px;
  max-width: 280px;
  max-height: 300px;
  overflow-y: auto;
  background: #2d2d30;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 10px;
  box-shadow: 
    0 4px 16px rgba(0, 0, 0, 0.4),
    0 0 0 1px rgba(0, 0, 0, 0.2);
  z-index: 1000;
  backdrop-filter: blur(10px);
  animation: dropdownFadeIn 0.2s ease;
}

@keyframes dropdownFadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.model-dropdown-item {
  padding: 10px 14px;
  color: #cccccc;
  font-size: 12.5px;
  font-weight: 400;
  cursor: pointer;
  transition: all 0.15s ease;
  user-select: none;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.model-dropdown-item:last-child {
  border-bottom: none;
}

.model-dropdown-item:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #ffffff;
  padding-left: 16px;
}

.model-dropdown-item.active {
  background: rgba(0, 122, 204, 0.15);
  color: #4fc3f7;
  font-weight: 500;
}

.model-dropdown-item.active:hover {
  background: rgba(0, 122, 204, 0.2);
}

.input {
  flex: 1;
  resize: none;
  border: none;
  background: transparent;
  color: #cccccc;
  font-size: 13.5px;
  padding: 0;
  outline: none;
  min-height: 40px;
  max-height: 120px;
  line-height: 1.6;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  transition: all 0.2s ease;
  overflow-y: auto;
  letter-spacing: 0.1px;
  width: 100%;
}

.input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.input::placeholder {
  color: #6b6b6b;
  opacity: 0.8;
}

.send-btn-inline {
  flex: 0 0 auto;
  width: 32px;
  height: 32px;
  border-radius: 6px;
  border: none;
  background: #0e639c;
  color: #ffffff;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 1px 3px rgba(14, 99, 156, 0.3);
}

.send-btn-inline:hover:not(:disabled) {
  background: #1177bb;
  box-shadow: 0 2px 6px rgba(14, 99, 156, 0.4);
  transform: translateY(-0.5px);
}

.send-btn-inline:active:not(:disabled) {
  background: #0a4d75;
  transform: translateY(0);
  box-shadow: 0 1px 3px rgba(14, 99, 156, 0.3);
}

.send-btn-inline:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  background: #3c3c3c;
  box-shadow: none;
  transform: none;
}

.send-btn-inline svg {
  width: 18px;
  height: 18px;
  stroke-width: 2.5;
}

.loading-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(11, 17, 32, 0.3);
  border-top-color: #0b1120;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
