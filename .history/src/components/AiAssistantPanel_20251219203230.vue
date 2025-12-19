<script setup lang="ts">
import { ref, nextTick, onMounted } from 'vue'
import { sendAIChat, checkAIServiceHealth, getAIProviders, waitForAIService, type AIMessage } from '../utils/aiService'
import { searchWiki, getWikiForTool } from '../utils/wikiReader'
import { debug, error as logError, warn } from '../utils/logger'

type Role = 'user' | 'assistant'

interface Message {
  id: number
  role: Role
  text: string
}

const input = ref('')
const isLoading = ref(false)
const isServiceAvailable = ref(false)
const currentProvider = ref('openai')
const availableProviders = ref<string[]>([])
const useWikiContext = ref(true)  // 是否使用 Wiki 上下文
const currentToolId = ref<string | undefined>(undefined)  // 当前工具 ID（用于查找相关 Wiki）

const messages = ref<Message[]>([
  {
    id: 1,
    role: 'assistant',
    text: '正在连接 AI 服务...',
  },
])

const containerRef = ref<HTMLElement | null>(null)

const scrollToBottom = () => {
  if (containerRef.value) {
    containerRef.value.scrollTop = containerRef.value.scrollHeight
  }
}

let idCounter = 2

// 获取提供商显示名称
const getProviderDisplayName = (provider: string): string => {
  const names: Record<string, string> = {
    'openai': 'OpenAI',
    'deepseek': 'DeepSeek',
    'ollama': 'Ollama',
    'lmstudio': 'LM Studio',
    'llamacpp': 'llama.cpp'
  }
  return names[provider] || provider
}

// 检查服务状态
const checkService = async () => {
  const isHealthy = await checkAIServiceHealth()
  isServiceAvailable.value = isHealthy
  
  if (isHealthy) {
    // 获取可用提供商
    const providersInfo = await getAIProviders()
    if (providersInfo.success && providersInfo.providers) {
      availableProviders.value = providersInfo.providers
      if (availableProviders.value.length > 0 && !availableProviders.value.includes(currentProvider.value)) {
        const firstProvider = availableProviders.value[0]
        if (firstProvider) {
          currentProvider.value = firstProvider
        }
      }
    }
    
    // 更新欢迎消息
    if (messages.value.length === 1 && messages.value[0]?.text === '正在连接 AI 服务...') {
      messages.value[0].text = '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'
    }
  } else {
    if (messages.value.length === 1 && messages.value[0]?.text === '正在连接 AI 服务...') {
      messages.value[0].text = '⚠️ AI 服务未启动，请确保 Python AI 服务正在运行。'
    }
  }
}

const send = async () => {
  const content = input.value.trim()
  if (!content) return
  
  if (!isServiceAvailable.value) {
    warn('AI 服务不可用，请先启动服务')
    return
  }
  
  // 添加用户消息
  const userMsg: Message = { id: idCounter++, role: 'user', text: content }
  messages.value.push(userMsg)
  input.value = ''
  isLoading.value = true
  
  // 添加加载中的占位消息
  const loadingMsg: Message = {
    id: idCounter++,
    role: 'assistant',
    text: '正在思考...',
  }
  messages.value.push(loadingMsg)
  nextTick(() => scrollToBottom())
  
  try {
    // 转换消息格式
    const aiMessages: AIMessage[] = messages.value
      .filter(msg => msg.id !== loadingMsg.id) // 排除加载消息
      .map(msg => ({
        role: msg.role,
        text: msg.text,
      }))
    
    // 获取 Wiki 上下文（如果启用）
    let wikiContext: string | undefined = undefined
    if (useWikiContext.value) {
      try {
        // 如果有关联的工具，优先使用工具的 Wiki
        if (currentToolId.value) {
          wikiContext = await getWikiForTool(currentToolId.value)
        }
        
        // 如果没有工具 Wiki 或工具 Wiki 为空，尝试从用户消息中提取关键词搜索
        if (!wikiContext && content) {
          // 简单提取关键词（可以改进）
          const keywords = content.split(/\s+/).filter(w => w.length > 2).slice(0, 3)
          if (keywords.length > 0) {
            wikiContext = await searchWiki(keywords.join(' '))
          }
        }
      } catch (error) {
        debug('获取 Wiki 上下文失败:', error)
        // 继续执行，不使用 Wiki 上下文
      }
    }
    
    // 调用 AI 服务
    const response = await sendAIChat(currentProvider.value, aiMessages, {
      wikiContext,
      timeout: 120  // 2 分钟超时
    })
    
    if (response.success && response.response) {
      // 更新加载消息为实际回复
      const index = messages.value.findIndex(msg => msg.id === loadingMsg.id)
      if (index !== -1) {
        const msg = messages.value[index]
        if (msg) {
          msg.text = response.response
        }
      }
    } else {
      // 显示错误消息
      const index = messages.value.findIndex(msg => msg.id === loadingMsg.id)
      if (index !== -1) {
        const msg = messages.value[index]
        if (msg) {
          msg.text = `❌ 错误: ${response.error || '未知错误'}`
        }
      }
    }
  } catch (error) {
    logError('发送 AI 消息失败:', error)
    const index = messages.value.findIndex(msg => msg.id === loadingMsg.id)
    if (index !== -1) {
      const msg = messages.value[index]
      if (msg) {
        msg.text = `❌ 请求失败: ${error instanceof Error ? error.message : String(error)}`
      }
    }
  } finally {
    isLoading.value = false
    nextTick(() => scrollToBottom())
  }
}

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey && !isLoading.value) {
    e.preventDefault()
    send()
  }
}

// 左侧功能按钮处理
const handleAttachFile = () => {
  // TODO: 实现文件插入功能
  debug('插入文件')
}

const handleReferenceWiki = () => {
  // TODO: 实现 Wiki 引用功能
  debug('引用 Wiki 内容')
}

const handleSelectContext = () => {
  // TODO: 实现上下文选择功能
  debug('选择上下文')
}

// 右侧工具按钮处理
const handleToolMode = () => {
  // TODO: 实现工具/模式选择功能
  debug('工具或模式')
}

// 组件挂载时检查服务
onMounted(async () => {
  // 等待服务启动（最多等待 5 秒）
  await waitForAIService(5, 1000)
  await checkService()
  
  // 定期检查服务状态（每 30 秒）
  setInterval(checkService, 30000)
})
</script>

<template>
  <section class="panel">
    <header class="panel-header">
      <div class="title">
        <span class="dot" />
        <span class="text">AI 安全助手</span>
      </div>
    </header>

    <main ref="containerRef" class="messages">
      <div
        v-for="msg in messages"
        :key="msg.id"
        class="msg-row"
        :class="msg.role"
      >
        <div class="bubble">
          <p>{{ msg.text }}</p>
        </div>
      </div>
    </main>

    <footer class="input-area">
      <div class="input-wrapper">
        <!-- 左侧功能按钮区 -->
        <div class="input-actions-left">
          <button 
            type="button" 
            class="action-btn"
            title="插入文件"
            @click="handleAttachFile"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"></path>
            </svg>
          </button>
          <button 
            type="button" 
            class="action-btn"
            title="引用 Wiki 内容"
            @click="handleReferenceWiki"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path>
              <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path>
            </svg>
          </button>
          <button 
            type="button" 
            class="action-btn"
            title="选择上下文"
            @click="handleSelectContext"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
            </svg>
          </button>
        </div>

        <!-- 中间文本输入区 -->
        <textarea
          v-model="input"
          class="input"
          rows="2"
          :placeholder="isServiceAvailable ? '描述下一步构建的内容...' : 'AI 服务未启动，请先启动 Python AI 服务'"
          :disabled="!isServiceAvailable || isLoading"
          @keydown="onKeydown"
        />

        <!-- 右侧操作按钮区 -->
        <div class="input-actions-right">
          <select 
            v-model="currentProvider" 
            class="provider-select-inline"
            :disabled="!isServiceAvailable || isLoading || availableProviders.length === 0"
            :title="`当前使用: ${getProviderDisplayName(currentProvider)}`"
          >
            <option v-if="availableProviders.length === 0" value="" disabled>
              无可用模型
            </option>
            <option 
              v-for="provider in availableProviders" 
              :key="provider" 
              :value="provider"
            >
              {{ getProviderDisplayName(provider) }}
            </option>
          </select>
          <button 
            type="button" 
            class="action-btn"
            title="工具或模式"
            @click="handleToolMode"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="3"></circle>
              <path d="M12 1v6m0 6v6M5.64 5.64l4.24 4.24m4.24 4.24l4.24 4.24M1 12h6m6 0h6M5.64 18.36l4.24-4.24m4.24-4.24l4.24-4.24"></path>
            </svg>
          </button>
          <button 
            type="button" 
            class="send-btn-inline" 
            :disabled="!isServiceAvailable || isLoading || !input.trim()"
            @click="send"
            :title="isLoading ? '发送中...' : '发送'"
          >
            <svg v-if="!isLoading" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <line x1="22" y1="2" x2="11" y2="13"></line>
              <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
            </svg>
            <span v-else class="loading-spinner"></span>
          </button>
        </div>
      </div>
    </footer>
  </section>
</template>

<style scoped>
.panel {
  height: 100%;
  min-height: 280px;
  max-height: 100%;
  display: flex;
  flex-direction: column;
  border-radius: 18px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background:
    radial-gradient(circle at top left, rgba(148, 163, 184, 0.18), transparent 55%),
    linear-gradient(145deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.96));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 18px 40px rgba(0, 0, 0, 0.85);
  overflow: hidden;
}

.panel-header {
  flex: 0 0 auto; /* 固定头部 */
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.4);
  flex-shrink: 0; /* 防止头部被压缩 */
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
  background: radial-gradient(circle at 30% 0, #bbf7d0, #22c55e);
}

.text {
  color: #e5e7eb;
}

.messages {
  flex: 1;
  min-height: 0; /* 确保可以滚动 */
  padding: 8px 10px;
  padding-bottom: 12px; /* 底部留出空间 */
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  gap: 6px;
  /* 确保可以接收鼠标滚轮事件 */
  overscroll-behavior: contain;
  /* 优化滚动条样式 */
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.4) rgba(15, 23, 42, 0.9);
}

.messages::-webkit-scrollbar {
  width: 6px;
}

.messages::-webkit-scrollbar-track {
  background: rgba(15, 23, 42, 0.9);
}

.messages::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.4);
  border-radius: 3px;
}

.messages::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.6);
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
  max-width: 80%;
  border-radius: 14px;
  padding: 6px 8px;
  font-size: 12px;
  line-height: 1.4;
}

.msg-row.user .bubble {
  background: linear-gradient(135deg, #4da3ff, #22d3ee);
  color: #0b1120;
}

.msg-row.assistant .bubble {
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(148, 163, 184, 0.5);
  color: #e5e7eb;
}

.bubble p {
  margin: 0;
}

.input-area {
  flex: 0 0 auto; /* 固定输入区域 */
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding: 12px 16px;
  background: #1e1e1e;
}

.input-wrapper {
  position: relative;
  display: flex;
  align-items: flex-end;
  gap: 8px;
  background: #252526;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 10px;
  padding: 8px 10px;
  transition: all 0.15s ease;
}

.input-wrapper:hover {
  border-color: rgba(255, 255, 255, 0.15);
  background: #2d2d30;
}

.input-wrapper:focus-within {
  border-color: rgba(0, 122, 204, 0.4);
  background: #2d2d30;
  box-shadow: 0 0 0 1px rgba(0, 122, 204, 0.2);
}

/* 左侧功能按钮区 */
.input-actions-left {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 0 0 auto;
}

.action-btn {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  color: #cccccc;
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  padding: 0;
}

.action-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.1);
  color: #ffffff;
}

.action-btn:active:not(:disabled) {
  background: rgba(255, 255, 255, 0.15);
}

.action-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.action-btn svg {
  width: 18px;
  height: 18px;
}

/* 右侧操作按钮区 */
.input-actions-right {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 0 0 auto;
}

.provider-select-inline {
  flex: 0 0 auto;
  padding: 5px 10px;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(60, 60, 60, 0.6);
  color: #cccccc;
  font-size: 12px;
  font-weight: 400;
  outline: none;
  cursor: pointer;
  transition: all 0.15s ease;
  min-width: 85px;
  height: 32px;
}

.provider-select-inline:hover:not(:disabled) {
  border-color: rgba(255, 255, 255, 0.15);
  background: rgba(60, 60, 60, 0.8);
}

.provider-select-inline:focus:not(:disabled) {
  border-color: rgba(0, 122, 204, 0.4);
  background: rgba(60, 60, 60, 0.8);
  box-shadow: 0 0 0 1px rgba(0, 122, 204, 0.2);
}

.provider-select-inline:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.wiki-context-toggle {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: #9ca3af;
  cursor: pointer;
}

.wiki-context-toggle input[type="checkbox"] {
  cursor: pointer;
}

.input {
  flex: 1;
  resize: none;
  border: none;
  background: transparent;
  color: #cccccc;
  font-size: 13px;
  padding: 8px 12px;
  outline: none;
  min-height: 36px;
  max-height: 120px;
  line-height: 1.5;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  transition: all 0.15s ease;
}

.input::placeholder {
  color: #858585;
}

.input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.input::placeholder {
  color: #6b7280;
}

.send-btn-inline {
  flex: 0 0 auto;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  border: none;
  background: linear-gradient(135deg, #4da3ff, #22d3ee);
  color: #0b1120;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 6px rgba(77, 163, 255, 0.3);
}

.send-btn-inline:hover:not(:disabled) {
  transform: translateY(-1px) scale(1.05);
  box-shadow: 0 4px 12px rgba(77, 163, 255, 0.5);
  background: linear-gradient(135deg, #5db3ff, #32e3fe);
}

.send-btn-inline:active:not(:disabled) {
  transform: translateY(0) scale(1);
  box-shadow: 0 2px 6px rgba(77, 163, 255, 0.3);
}

.send-btn-inline:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
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

