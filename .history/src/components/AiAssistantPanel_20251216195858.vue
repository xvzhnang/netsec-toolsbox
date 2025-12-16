<script setup lang="ts">
import { ref, nextTick } from 'vue'

type Role = 'user' | 'assistant'

interface Message {
  id: number
  role: Role
  text: string
}

const input = ref('')
const messages = ref<Message[]>([
  {
    id: 1,
    role: 'assistant',
    text: 'AI 问答功能开发中，目前仅提供界面预览。后续将支持多家模型与类型配置。',
  },
])

const containerRef = ref<HTMLElement | null>(null)

const scrollToBottom = () => {
  if (containerRef.value) {
    containerRef.value.scrollTop = containerRef.value.scrollHeight
  }
}

let idCounter = 2

const send = () => {
  const content = input.value.trim()
  if (!content) return
  const userMsg: Message = { id: idCounter++, role: 'user', text: content }
  messages.value.push(userMsg)
  input.value = ''
  // 模拟占位回复
  const reply: Message = {
    id: idCounter++,
    role: 'assistant',
    text: '（AI 后端未接入）收到你的问题，后续接入模型后会在这里给出安全分析与建议。',
  }
  messages.value.push(reply)
  nextTick(() => scrollToBottom())
}

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    send()
  }
}
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
      <textarea
        v-model="input"
        class="input"
        rows="2"
        placeholder="向 AI 询问攻防思路、命令示例或工具使用建议（暂为占位 UI）..."
        @keydown="onKeydown"
      />
      <button type="button" class="send-btn" @click="send">
        发送
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
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.4);
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
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  gap: 6px;
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
  border-top: 1px solid rgba(148, 163, 184, 0.4);
  padding: 6px 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
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

