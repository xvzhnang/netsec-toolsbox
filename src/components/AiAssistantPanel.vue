<template>
  <section class="panel">
    <header class="panel-header">
      <div class="title">
        <span class="dot" :class="{ 'active': isServiceAvailable }"></span>
        <span class="text">AI 助手</span>
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
            :disabled="!isServiceAvailable || isLoading || availableModels.length === 0"
            :title="`当前模型: ${getModelDisplayName(currentModel)} (${availableModels.length} 个可用)`"
            @click.stop="toggleModelDropdown"
          >
            <span class="model-select-text">{{ getModelDisplayName(currentModel) }}</span>
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
import { ref, nextTick, onMounted, onUnmounted } from 'vue'
import { sendAIChat, checkAIServiceHealth, getAvailableModels, waitForAIService, startAIService } from '../utils/aiService'
import { debug, error as logError, info } from '../utils/logger'

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
const currentModel = ref('gpt-3.5-turbo')
const availableModels = ref<string[]>([])
const isInputFocused = ref(false)
const showModelDropdown = ref(false)

const messages = ref<Message[]>([
  {
    id: 1,
    role: 'assistant',
    text: '正在连接 AI Gateway 服务...',
  },
])

const containerRef = ref<HTMLElement | null>(null)

let idCounter = 2

// 获取模型显示名称
const getModelDisplayName = (modelId: string): string => {
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
    return 'AI Gateway 服务未启动...'
  }
  if (availableModels.value.length === 0) {
    return '未配置 AI 模型，请在配置文件中添加模型...'
  }
  return '输入消息...'
}

// 检查服务状态
const checkService = async () => {
  const isHealthy = await checkAIServiceHealth()
  isServiceAvailable.value = isHealthy
  
  if (isHealthy) {
    // 获取可用模型列表
    const models = await getAvailableModels()
    debug('获取到的模型列表:', models)
    if (models.length > 0) {
      availableModels.value = models
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
    }
    
    // 更新欢迎消息
    if (messages.value.length === 1 && messages.value[0]?.text === '正在连接 AI Gateway 服务...') {
      messages.value[0].text = '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'
    }
  } else {
    if (messages.value.length === 1 && messages.value[0]?.text === '正在连接 AI Gateway 服务...') {
      messages.value[0].text = '⚠️ AI Gateway 服务未启动，请确保服务正在运行。'
    }
  }
}

// 发送消息
const send = async () => {
  if (!input.value.trim() || isLoading.value || !isServiceAvailable.value) {
    return
  }
  
  const userMessage = input.value.trim()
  input.value = ''
  handleInput()
  
  // 添加用户消息
  const userMsg: Message = {
    id: idCounter++,
    role: 'user',
    text: userMessage,
  }
  messages.value.push(userMsg)
  
  // 添加加载中的助手消息
  const loadingMsg: Message = {
    id: idCounter++,
    role: 'assistant',
    text: '思考中...',
  }
  messages.value.push(loadingMsg)
  
  isLoading.value = true
  nextTick(() => scrollToBottom())
  
  try {
    // 调用 AI Gateway API
    const response = await sendAIChat(
      currentModel.value,
      messages.value
        .filter(msg => msg.role !== 'assistant' || msg.text !== '思考中...')
        .map(msg => ({
          role: msg.role,
          content: msg.text,
        })),
      {
        temperature: 0.7,
        max_tokens: 2000,
      }
    )
    
    // 更新助手消息
    const index = messages.value.findIndex(msg => msg.id === loadingMsg.id)
    if (index !== -1) {
      const msg = messages.value[index]
      if (msg) {
        const content = response.choices[0]?.message?.content || '无响应'
        msg.text = content
      }
    }
  } catch (error) {
    logError('发送 AI 消息失败:', error)
    const index = messages.value.findIndex(msg => msg.id === loadingMsg.id)
    if (index !== -1) {
      const msg = messages.value[index]
      if (msg) {
        const errorText = `❌ 错误: ${error instanceof Error ? error.message : String(error)}`
        msg.text = errorText
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

onMounted(async () => {
  // 尝试启动服务
  try {
    await startAIService()
    // 等待服务就绪
    const isReady = await waitForAIService(10, 1000)
    if (isReady) {
      info('AI Gateway 服务已启动并就绪')
    } else {
      debug('AI Gateway 服务启动超时，但将继续尝试连接')
    }
  } catch (error) {
    debug('自动启动 AI Gateway 服务失败（不影响应用使用）:', error)
  }
  
  // 检查服务状态
  await checkService()
  
  // 定期检查服务状态
  const checkInterval = setInterval(checkService, 5000)
  
  // 添加点击外部关闭下拉菜单的监听
  document.addEventListener('click', handleClickOutside)
  
  // 初始化输入框高度
  nextTick(() => {
    if (inputRef.value) {
      handleInput()
    }
  })
  
  onUnmounted(() => {
    clearInterval(checkInterval)
    document.removeEventListener('click', handleClickOutside)
  })
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
  background: #252526;
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #cccccc;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.bubble p {
  margin: 0;
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
