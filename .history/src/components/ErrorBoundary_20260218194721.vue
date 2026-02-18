<script setup lang="ts">
import { ref, onErrorCaptured, provide, type ComponentPublicInstance } from 'vue'
import { error as logError } from '../utils/logger'

interface Props {
  fallback?: string
}

const props = withDefaults(defineProps<Props>(), {
  fallback: '发生了一些错误，请刷新页面重试。',
})

const hasError = ref(false)
const error = ref<Error | null>(null)

// 捕获子组件的错误
onErrorCaptured((err: Error, _instance: ComponentPublicInstance | null, info: string) => {
  hasError.value = true
  error.value = err
  logError('ErrorBoundary caught an error:', err, info)
  
  // 可以在这里发送错误报告到后端
  // reportError(err, info)
  
  // 返回 false 阻止错误继续传播
  return false
})

// 提供错误处理函数给子组件
provide('handleError', (err: Error) => {
  hasError.value = true
  error.value = err
})

const reset = () => {
  hasError.value = false
  error.value = null
  // 刷新页面
  window.location.reload()
}
</script>

<template>
  <div v-if="hasError" class="error-boundary">
    <div class="error-container">
      <div class="error-icon">⚠️</div>
      <h2 class="error-title">出现错误</h2>
      <p class="error-message">{{ props.fallback }}</p>
      <div v-if="error" class="error-details">
        <details>
          <summary>错误详情（开发模式）</summary>
          <pre class="error-stack">{{ error.message }}\n{{ error.stack }}</pre>
        </details>
      </div>
      <div class="error-actions">
        <button type="button" class="btn primary" @click="reset">刷新页面</button>
      </div>
    </div>
  </div>
  <slot v-else />
</template>

<style scoped>
.error-boundary {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: var(--bg-primary);
}

.error-container {
  max-width: 500px;
  width: 100%;
  padding: 32px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: var(--bg-secondary);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  text-align: center;
}

.error-icon {
  font-size: 64px;
  margin-bottom: 16px;
}

.error-title {
  margin: 0 0 12px 0;
  font-size: 20px;
  font-weight: 600;
  color: #ef4444;
}

.error-message {
  margin: 0 0 20px 0;
  font-size: 14px;
  color: var(--text-secondary);
  line-height: 1.6;
}

.error-details {
  margin: 20px 0;
  text-align: left;
}

.error-details summary {
  cursor: pointer;
  padding: 8px;
  border-radius: 6px;
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  font-size: 12px;
  user-select: none;
}

.error-details summary:hover {
  background: var(--bg-hover);
}

.error-stack {
  margin: 8px 0 0 0;
  padding: 12px;
  border-radius: 6px;
  background: var(--bg-primary);
  color: #ef4444;
  font-size: 11px;
  font-family: 'Courier New', monospace;
  white-space: pre-wrap;
  word-break: break-all;
  overflow-x: auto;
  max-height: 200px;
  overflow-y: auto;
  border: 1px solid var(--border-color);
}

.error-actions {
  margin-top: 24px;
}

.btn {
  padding: 10px 20px;
  border-radius: 6px;
  border: 1px solid var(--accent-color);
  background: var(--accent-color);
  color: white;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.14s ease-out;
}

.btn:hover {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
}

.btn.primary {
  border-color: var(--accent-color);
}
</style>

