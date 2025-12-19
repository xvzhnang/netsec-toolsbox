<template>
  <section class="panel">
    <header class="panel-header">
      <div class="title">
        <span class="dot"></span>
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
            placeholder="AI 功能已禁用..."
            :disabled="true"
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
            :disabled="true"
            title="AI 功能已禁用"
          >
            <span class="model-select-text">AI 功能已禁用</span>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="6 9 12 15 18 9"></polyline>
            </svg>
          </button>

          <button 
            type="button" 
            class="send-btn-inline" 
            :disabled="true"
            title="AI 功能已禁用"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <line x1="22" y1="2" x2="11" y2="13"></line>
              <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
            </svg>
          </button>
        </div>
      </div>
    </footer>
  </section>
</template>

<script setup lang="ts">
import { ref, nextTick } from 'vue'

type Role = 'user' | 'assistant'

interface Message {
  id: number
  role: Role
  text: string
}

const input = ref('')
const inputRef = ref<HTMLTextAreaElement | null>(null)
const isServiceAvailable = ref(false)
const isInputFocused = ref(false)

const messages = ref<Message[]>([
  {
    id: 1,
    role: 'assistant',
    text: 'AI 功能已禁用',
  },
])

const containerRef = ref<HTMLElement | null>(null)

const scrollToBottom = () => {
  if (containerRef.value) {
    containerRef.value.scrollTop = containerRef.value.scrollHeight
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
</style>
