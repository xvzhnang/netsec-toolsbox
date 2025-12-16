<script setup lang="ts">
import { ref, onErrorCaptured, provide, type ComponentPublicInstance } from 'vue'

interface Props {
  fallback?: string
}

const props = withDefaults(defineProps<Props>(), {
  fallback: '发生了一些错误，请刷新页面重试。',
})

const hasError = ref(false)
const error = ref<Error | null>(null)

// 捕获子组件的错误
onErrorCaptured((err: Error, instance: ComponentPublicInstance | null, info: string) => {
  hasError.value = true
  error.value = err
  // eslint-disable-next-line no-console
  console.error('ErrorBoundary caught an error:', err, info)
  
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
  background: radial-gradient(circle at top, #020617 0, #000000 100%);
}

.error-container {
  max-width: 500px;
  width: 100%;
  padding: 32px;
  border-radius: 16px;
  border: 1px solid rgba(248, 113, 113, 0.4);
  background: linear-gradient(145deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.96));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 24px 60px rgba(0, 0, 0, 0.9);
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
  color: #f87171;
}

.error-message {
  margin: 0 0 20px 0;
  font-size: 14px;
  color: #9ca3af;
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
  background: rgba(15, 23, 42, 0.5);
  color: #9ca3af;
  font-size: 12px;
  user-select: none;
}

.error-details summary:hover {
  background: rgba(15, 23, 42, 0.7);
}

.error-stack {
  margin: 8px 0 0 0;
  padding: 12px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.3);
  color: #f87171;
  font-size: 11px;
  font-family: 'Courier New', monospace;
  white-space: pre-wrap;
  word-break: break-all;
  overflow-x: auto;
  max-height: 200px;
  overflow-y: auto;
}

.error-actions {
  margin-top: 24px;
}

.btn {
  padding: 10px 20px;
  border-radius: 8px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background: linear-gradient(135deg, #4da3ff, #22d3ee);
  color: #0b1120;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.14s ease-out;
}

.btn:hover {
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 10px 24px rgba(37, 99, 235, 0.9);
  transform: translateY(-1px);
}

.btn.primary {
  border-color: #4da3ff;
}
</style>

