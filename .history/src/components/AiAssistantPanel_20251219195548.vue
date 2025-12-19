<script setup lang="ts">
import { ref, nextTick, onMounted } from 'vue'
import { sendAIChat, checkAIServiceHealth, getAIProviders, waitForAIService, type AIMessage } from '../utils/aiService'
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

// 检查服务状态
const checkService = async () => {
  const isHealthy = await checkAIServiceHealth()
  isServiceAvailable.value = isHealthy
  
  if (isHealthy) {
    // 获取可用提供商
    const providersInfo = await getAIProviders()
    if (providersInfo.success && providersInfo.providers) {
      availableProviders.value = providersInfo.providers
      if (availableProviders.value.length > 0) {
        currentProvider.value = availableProviders.value[0]
      }
    }
    
    // 更新欢迎消息
    if (messages.value.length === 1 && messages.value[0].text === '正在连接 AI 服务...') {
      messages.value[0].text = '🤖 AI 安全助手已就绪！我可以帮助你分析安全工具、提供攻防思路、命令示例等。'
    }
  } else {
    if (messages.value.length === 1 && messages.value[0].text === '正在连接 AI 服务...') {
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
    
    // 调用 AI 服务
    const response = await sendAIChat(currentProvider.value, aiMessages)
    
    if (response.success && response.response) {
      // 更新加载消息为实际回复
      const index = messages.value.findIndex(msg => msg.id === loadingMsg.id)
      if (index !== -1) {
        messages.value[index].text = response.response
      }
    } else {
      // 显示错误消息
      const index = messages.value.findIndex(msg => msg.id === loadingMsg.id)
      if (index !== -1) {
        messages.value[index].text = `❌ 错误: ${response.error || '未知错误'}`
      }
    }
  } catch (error) {
    logError('发送 AI 消息失败:', error)
    const index = messages.value.findIndex(msg => msg.id === loadingMsg.id)
    if (index !== -1) {
      messages.value[index].text = `❌ 请求失败: ${error instanceof Error ? error.message : String(error)}`
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
        <span class="text">AI 安全助手（预览）</span>
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
      <div v-if="availableProviders.length > 1" class="provider-selector">
        <select v-model="currentProvider" class="provider-select">
          <option v-for="provider in availableProviders" :key="provider" :value="provider">
            {{ provider }}
          </option>
        </select>
      </div>
      <textarea
        v-model="input"
        class="input"
        rows="2"
        :placeholder="isServiceAvailable ? '向 AI 询问攻防思路、命令示例或工具使用建议...' : 'AI 服务未启动，请先启动 Python AI 服务'"
        :disabled="!isServiceAvailable || isLoading"
        @keydown="onKeydown"
      />
      <button 
        type="button" 
        class="send-btn" 
        :disabled="!isServiceAvailable || isLoading || !input.trim()"
        @click="send"
      >
        {{ isLoading ? '发送中...' : '发送' }}
      </button>
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
  border-top: 1px solid rgba(148, 163, 184, 0.4);
  padding: 6px 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.provider-selector {
  display: flex;
  align-items: center;
  gap: 4px;
}

.provider-select {
  flex: 0 0 auto;
  padding: 2px 6px;
  border-radius: 6px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background: rgba(15, 23, 42, 0.98);
  color: #e5e7eb;
  font-size: 11px;
  outline: none;
  cursor: pointer;
}

.input {
  resize: none;
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background: rgba(15, 23, 42, 0.98);
  color: #e5e7eb;
  font-size: 12px;
  padding: 4px 6px;
  outline: none;
}

.input::placeholder {
  color: #6b7280;
}

.send-btn {
  align-self: flex-end;
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid #4da3ff;
  background: linear-gradient(135deg, #4da3ff, #22d3ee);
  color: #0b1120;
  font-size: 12px;
  cursor: pointer;
  transition: box-shadow 0.16s ease-out, transform 0.16s ease-out;
}

.send-btn:hover {
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 10px 22px rgba(37, 99, 235, 0.9);
  transform: translateY(-1px);
}
</style>

