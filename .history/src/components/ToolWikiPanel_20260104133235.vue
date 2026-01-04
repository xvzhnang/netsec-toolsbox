<template>
  <div class="wiki-panel" v-if="visible" :style="{ width: width + 'px' }">
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
      <!-- Wrap in #write for pink.css application -->
      <div v-else id="write" class="markdown-body" v-html="renderedContent"></div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { renderMarkdown } from '../utils/markdownRenderer'
import mermaid from 'mermaid'

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

// Initialize Mermaid
mermaid.initialize({ 
  startOnLoad: false, 
  theme: 'dark',
  securityLevel: 'loose',
})

const renderMermaid = async () => {
  await nextTick()
  const elements = document.querySelectorAll('.mermaid')
  if (elements.length > 0) {
    try {
      await mermaid.run({
        nodes: elements
      })
    } catch (e) {
      console.error('Mermaid rendering failed:', e)
    }
  }
}

// Watch for content changes to re-render diagrams
watch(() => props.content, () => {
  renderMermaid()
})

// Watch for visibility changes
watch(() => props.visible, (val) => {
  if (val) {
    renderMermaid()
  }
})

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
  renderMermaid()
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
/* Removed .wiki-panel-overlay to make it non-blocking */

.wiki-panel {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
  background: #1e1e1e;
  border-left: 1px solid #333;
  box-shadow: -4px 0 16px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
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
  /* Use a dark background to ensure text is readable even if pink.css assumes light theme */
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

/* Remove manual markdown styles as pink.css should handle them */
/* But keep some basic resets if needed */

#write {
  /* Ensure it takes full width/height if needed, but let content flow */
  max-width: 100%;
  /* Fix for some themes setting absolute positioning or weird margins */
  position: static;
  margin: 0;
  padding: 0;
  /* Override background if pink.css sets it to white and we want it to fit in dark mode? */
  /* No, user explicitly asked for pink.css. */
  /* But I should make sure text color is readable. */
  color: #ddd; 
}

/* Add custom styles for containers and task lists to ensure they render nicely even if pink.css misses them */
:deep(.custom-block) {
  margin: 1rem 0;
  padding: 0.1rem 1.5rem;
  border-left-width: 0.5rem;
  border-left-style: solid;
  background-color: #2b2b2b;
}

:deep(.custom-block.tip), :deep(.custom-block.info) {
  border-color: #42b983;
}

:deep(.custom-block.warning) {
  border-color: #e7c000;
}

:deep(.custom-block.danger) {
  border-color: #c00;
}

:deep(.custom-block.details) {
  border-color: #eee;
}

:deep(.custom-block-title) {
  font-weight: 600;
  margin-bottom: -0.4rem;
}

/* Task lists */
:deep(.task-list-item) {
  list-style-type: none;
}

:deep(.task-list-item-checkbox) {
  margin: 0 0.2em 0.25em -1.6em;
  vertical-align: middle;
}

/* Ensure links in dark mode are visible */
:deep(a) {
  color: #58a6ff;
}

/* If pink.css sets background to white, it will be jarring in a dark app. */
/* I'll let it be. */
</style>
