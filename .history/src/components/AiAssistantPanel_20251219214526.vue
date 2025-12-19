<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { sendAIChat, checkAIServiceHealth, getAIProviders, waitForAIService, type AIMessage } from '../utils/aiService'
import { searchWiki, getWikiForTool } from '../utils/wikiReader'
import { debug, error as logError, warn, info } from '../utils/logger'
import { loadChatHistory, saveMessagesToHistory, addMessageToHistory, updateMessageInHistory, clearChatHistory, type ChatMessage } from '../utils/aiHistory'

type Role = 'user' | 'assistant'

interface Message {
  id: number
  role: Role
  text: string
}

const input = ref('')
const inputRef = ref<HTMLTextAreaElement | null>(null)
const isLoading = ref(false)
const isServiceAvailable = ref(false)
const currentProvider = ref('openai')
const availableProviders = ref<string[]>([])
const useWikiContext = ref(true)  // 是否使用 Wiki 上下文
const currentToolId = ref<string | undefined>(undefined)  // 当前工具 ID（用于查找相关 Wiki）
const isInputFocused = ref(false)
const showModelDropdown = ref(false)

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
    'claude': 'Claude (Anthropic)',
    'gemini': 'Gemini (Google)',
    'zhipu': '智谱AI (GLM)',
    'qwen': '通义千问 (Qwen)',
    'mistral': 'Mistral AI',
    'groq': 'Groq (快速推理)',
    'together': 'Together AI',
    'ollama': 'Ollama',
    'lmstudio': 'LM Studio',
    'llamacpp': 'llama.cpp',
    'vllm': 'vLLM (高性能)',
    'localai': 'LocalAI',
    'tgi': 'TGI (Hugging Face)'
  }
  // 如果是自定义提供商，直接返回名称（首字母大写）
  if (!names[provider]) {
    return provider.charAt(0).toUpperCase() + provider.slice(1).replace(/_/g, ' ')
  }
  return names[provider] || provider
}

// 检查服务状态
const checkService = async () => {
  // 静默检查，不输出错误
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
    // 重置高度以获取正确的 scrollHeight
    inputRef.value.style.height = 'auto'
    // 设置新高度，但不超过 max-height
    const maxHeight = 120
    const newHeight = Math.min(inputRef.value.scrollHeight, maxHeight)
    inputRef.value.style.height = `${newHeight}px`
  }
}

// 获取 placeholder 文本
const getPlaceholderText = (): string => {
  if (!isServiceAvailable.value) {
    return 'AI 服务未启动，请先启动服务...'
  }
  if (availableProviders.value.length === 0) {
    return '未配置 AI 模型，请在设置中配置...'
  }
  return '描述下一步构建的内容...'
}

// 切换模型下拉菜单
const toggleModelDropdown = () => {
  if (!isServiceAvailable.value || isLoading.value || availableProviders.value.length === 0) {
    return
  }
  showModelDropdown.value = !showModelDropdown.value
}

// 选择模型
const selectModel = (provider: string) => {
  currentProvider.value = provider
  showModelDropdown.value = false
}

// 点击外部关闭下拉菜单
const handleClickOutside = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  if (!target.closest('.input-wrapper') && !target.closest('.model-dropdown')) {
    showModelDropdown.value = false
  }
}

// 加载聊天历史记录
const loadHistory = async () => {
  try {
    const history = await loadChatHistory()
    
    if (history.messages && history.messages.length > 0) {
      // 恢复历史消息（排除系统消息）
      const historyMessages = history.messages.filter(
        msg => msg.text !== '正在连接 AI 服务...' && 
               msg.text !== '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。' &&
               msg.text !== '⚠️ AI 服务未启动，请确保 Python AI 服务正在运行。'
      )
      
      if (historyMessages.length > 0) {
        // 恢复消息 ID 计数器
        const maxId = Math.max(...historyMessages.map(msg => msg.id), 0)
        idCounter = maxId + 1
        
        // 如果有历史记录，替换初始消息
        messages.value = historyMessages as Message[]
        
        // 恢复提供商（如果有）
        if (history.provider && availableProviders.value.includes(history.provider)) {
          currentProvider.value = history.provider
        }
        
        debug('聊天历史记录已加载:', historyMessages.length, '条消息')
      }
    }
  } catch (error) {
    debug('加载聊天历史记录失败:', error)
    // 继续执行，不影响正常使用
  }
}

// 清除聊天历史记录
const clearHistory = async () => {
  if (confirm('确定要清除所有聊天历史记录吗？此操作不可恢复。')) {
    try {
      await clearChatHistory()
      // 重置消息列表
      messages.value = [{
        id: 1,
        role: 'assistant',
        text: '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。',
      }]
      idCounter = 2
      info('聊天历史记录已清除')
    } catch (error) {
      logError('清除聊天历史记录失败:', error)
    }
  }
}

// 监听消息变化，自动保存（防抖）
let saveTimer: ReturnType<typeof setTimeout> | null = null
watch(messages, () => {
  // 防抖：延迟 2 秒后保存，避免频繁保存
  if (saveTimer) {
    clearTimeout(saveTimer)
  }
  
  saveTimer = setTimeout(async () => {
    try {
      // 排除系统消息
      const messagesToSave = messages.value.filter(
        msg => msg.text !== '正在连接 AI 服务...' && 
               msg.text !== '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。' &&
               msg.text !== '⚠️ AI 服务未启动，请确保 Python AI 服务正在运行。'
      )
      
      if (messagesToSave.length > 0) {
        await saveMessagesToHistory(messagesToSave as ChatMessage[], currentProvider.value)
      }
    } catch (error) {
      debug('自动保存聊天历史记录失败:', error)
      // 不抛出错误，避免影响用户体验
    }
  }, 2000)
}, { deep: true })

// 组件挂载时检查服务
onMounted(async () => {
  // 先加载历史记录
  await loadHistory()
  
  // 检查服务状态
  await checkService()
  
  // 如果服务不可用，尝试等待启动（最多等待 10 秒）
  if (!isServiceAvailable.value) {
    await waitForAIService(10, 1000)
    await checkService()
  }
  
  // 如果加载历史记录后没有消息，显示欢迎消息
  if (messages.value.length === 0 || 
      (messages.value.length === 1 && messages.value[0]?.text === '正在连接 AI 服务...')) {
    messages.value = [{
      id: 1,
      role: 'assistant',
      text: isServiceAvailable.value 
        ? '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'
        : '⚠️ AI 服务未启动，请确保 Python AI 服务正在运行。',
    }]
    idCounter = 2
  }
  
  // 定期检查服务状态（每 30 秒）
  const checkInterval = setInterval(checkService, 30000)
  
  // 监听点击外部关闭下拉菜单
  document.addEventListener('click', handleClickOutside)
  
  // 初始化输入框高度
  nextTick(() => {
    if (inputRef.value) {
      handleInput()
    }
  })
  
  // 组件卸载时清理
  onUnmounted(() => {
    clearInterval(checkInterval)
    document.removeEventListener('click', handleClickOutside)
    if (saveTimer) {
      clearTimeout(saveTimer)
    }
  })
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
            :disabled="!isServiceAvailable || isLoading || availableProviders.length === 0"
            :title="`当前模型: ${getProviderDisplayName(currentProvider)}`"
            @click="toggleModelDropdown"
          >
            <span class="model-select-text">{{ getProviderDisplayName(currentProvider) }}</span>
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
      </div>

      <!-- 模型选择下拉菜单 -->
      <div v-if="showModelDropdown" class="model-dropdown" @click.stop>
        <div 
          v-for="provider in availableProviders" 
          :key="provider"
          class="model-dropdown-item"
          :class="{ 'active': provider === currentProvider }"
          @click="selectModel(provider)"
        >
          {{ getProviderDisplayName(provider) }}
        </div>
      </div>
    </footer>
  </section>
</template>

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
  flex: 0 0 auto; /* 固定头部 */
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  background: #252526;
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
  color: #cccccc;
  font-weight: 500;
}

.messages {
  flex: 1;
  min-height: 0; /* 确保可以滚动 */
  padding: 16px;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: #1e1e1e;
  /* 确保可以接收鼠标滚轮事件 */
  overscroll-behavior: contain;
  /* 优化滚动条样式 */
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
  background: #252526;
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #cccccc;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.bubble p {
  margin: 0;
}

.input-area {
  flex: 0 0 auto; /* 固定输入区域 */
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

/* 上方：文本输入行 */
.input-row {
  display: flex;
  padding: 12px 14px;
  background: transparent;
}

/* 下方：操作按钮行 */
.input-actions-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.2);
  gap: 8px;
}

/* AI 模型选择按钮（Cursor 风格） */
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

/* 模型下拉菜单 */
.model-dropdown {
  position: absolute;
  bottom: calc(100% + 10px);
  left: 20px;
  min-width: 160px;
  background: #2d2d30;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 10px;
  box-shadow: 
    0 4px 16px rgba(0, 0, 0, 0.4),
    0 0 0 1px rgba(0, 0, 0, 0.2);
  z-index: 1000;
  overflow: hidden;
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

.input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.input::placeholder {
  color: #6b7280;
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

