<template>
  <div class="wiki-panel-overlay" v-if="visible" @click="close">
    <div class="wiki-panel" :style="{ width: width + 'px' }" @click.stop>
      <div class="resize-handle" @mousedown="startResize"></div>
      
      <header class="panel-header">
        <h3>{{ title }}</h3>
        <button class="close-btn" @click="close" title="关闭 (Esc)">×</button>
      </header>
      
      <main class="panel-content">
        <div v-if="loading" class="loading">
          <div class="spinner"></div>
          <p>正在加载文档...</p>
        </div>
        <div v-else-if="error" class="error">
          <p>⚠️ {{ error }}</p>
          <p class="error-hint">请检查 Docsify 服务是否正常运行，或文档路径是否正确。</p>
        </div>
        <div v-else class="markdown-body" v-html="renderedContent"></div>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { renderMarkdown } from '../utils/markdownRenderer'

const props = defineProps<{
  visible: boolean
  title: string
  content: string
  loading?: boolean
  error?: string
}>()

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void
  (e: 'close'): void
}>()

const width = ref(600)
const renderedContent = computed(() => renderMarkdown(props.content))

const close = () => {
  emit('update:visible', false)
  emit('close')
}

// ESC 关闭
const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && props.visible) {
    close()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
  stopResize()
})

// Resizing logic
const isResizing = ref(false)
const startResize = () => {
  isResizing.value = true
  document.addEventListener('mousemove', handleResize)
  document.addEventListener('mouseup', stopResize)
  document.body.style.cursor = 'ew-resize'
  document.body.style.userSelect = 'none'
}

const handleResize = (e: MouseEvent) => {
  if (!isResizing.value) return
  const newWidth = window.innerWidth - e.clientX
  width.value = Math.max(300, Math.min(newWidth, window.innerWidth - 50))
}

const stopResize = () => {
  isResizing.value = false
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}
</script>

<style scoped>
.wiki-panel-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 1000;
  display: flex;
  justify-content: flex-end;
  backdrop-filter: blur(2px);
}

.wiki-panel {
  height: 100%;
  background: #1e1e1e;
  border-left: 1px solid #333;
  box-shadow: -4px 0 16px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  position: relative;
  transition: width 0.1s ease-out;
}

.resize-handle {
  position: absolute;
  left: -4px;
  top: 0;
  bottom: 0;
  width: 8px;
  cursor: ew-resize;
  z-index: 10;
  /* 透明但在 hover 时可见提示 */
}

.resize-handle:hover {
  background: rgba(255, 255, 255, 0.1);
}

.panel-header {
  padding: 16px 20px;
  border-bottom: 1px solid #333;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: #252526;
}

.panel-header h3 {
  margin: 0;
  font-size: 18px;
  color: #fff;
  font-weight: 600;
}

.close-btn {
  background: none;
  border: none;
  color: #999;
  font-size: 24px;
  cursor: pointer;
  line-height: 1;
  padding: 0 4px;
  transition: color 0.2s;
}

.close-btn:hover {
  color: #fff;
}

.panel-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
  color: #ddd;
}

.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #888;
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid rgba(255, 255, 255, 0.1);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 12px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.error {
  color: #ef4444;
  text-align: center;
  margin-top: 40px;
}

.error-hint {
  font-size: 13px;
  color: #888;
  margin-top: 8px;
}

/* Markdown Styles (简化版，复用 markdown-body 类) */
.markdown-body {
  line-height: 1.6;
  font-size: 15px;
}

:deep(.markdown-body h1),
:deep(.markdown-body h2),
:deep(.markdown-body h3) {
  color: #fff;
  margin-top: 24px;
  margin-bottom: 16px;
  font-weight: 600;
  line-height: 1.25;
}

:deep(.markdown-body h1) { font-size: 2em; border-bottom: 1px solid #333; padding-bottom: 0.3em; }
:deep(.markdown-body h2) { font-size: 1.5em; border-bottom: 1px solid #333; padding-bottom: 0.3em; }
:deep(.markdown-body h3) { font-size: 1.25em; }

:deep(.markdown-body p) { margin-bottom: 16px; }
:deep(.markdown-body ul), :deep(.markdown-body ol) { padding-left: 2em; margin-bottom: 16px; }
:deep(.markdown-body li) { margin-bottom: 4px; }

:deep(.markdown-body code) {
  background-color: rgba(255, 255, 255, 0.1);
  padding: 0.2em 0.4em;
  border-radius: 4px;
  font-family: monospace;
  font-size: 85%;
}

:deep(.markdown-body pre) {
  background-color: #161b22;
  border-radius: 6px;
  padding: 16px;
  overflow: auto;
  margin-bottom: 16px;
}

:deep(.markdown-body pre code) {
  background-color: transparent;
  padding: 0;
  font-size: 100%;
}

:deep(.markdown-body blockquote) {
  padding: 0 1em;
  color: #8b949e;
  border-left: 0.25em solid #30363d;
  margin: 0 0 16px 0;
}

:deep(.markdown-body a) {
  color: #58a6ff;
  text-decoration: none;
}
:deep(.markdown-body a:hover) {
  text-decoration: underline;
}
:deep(.markdown-body img) {
  max-width: 100%;
  box-sizing: border-box;
}
</style>
