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
        nodes: Array.from(elements) as HTMLElement[]
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
  background: rgba(22, 27, 34, 0.95); /* --bg-secondary with opacity */
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-left: 1px solid var(--border-color);
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
  transition: width 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.resize-handle {
  position: absolute;
  left: -4px;
  top: 0;
  bottom: 0;
  width: 8px;
  cursor: ew-resize;
  z-index: 10;
  transition: background 0.2s;
}

.resize-handle:hover, .resize-handle:active {
  background: var(--accent-primary);
  opacity: 0.5;
}

.panel-header {
  padding: 16px 24px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: rgba(13, 17, 23, 0.8); /* --bg-primary with opacity */
  backdrop-filter: blur(8px);
}

.panel-header h3 {
  margin: 0;
  font-size: 16px;
  color: var(--text-primary);
  font-weight: 600;
  letter-spacing: 0.5px;
}

.close-btn {
  background: transparent;
  border: none;
  color: var(--text-secondary);
  width: 28px;
  height: 28px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  cursor: pointer;
  transition: all 0.2s;
}

.close-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-primary);
}

.panel-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 32px;
  scrollbar-width: thin;
  scrollbar-color: var(--border-color) transparent;
}

.panel-content::-webkit-scrollbar {
  width: 6px;
}

.panel-content::-webkit-scrollbar-thumb {
  background-color: var(--border-color);
  border-radius: 3px;
}

.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-secondary);
  gap: 16px;
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid rgba(255, 255, 255, 0.1);
  border-top-color: var(--accent-primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.error {
  color: #ef4444;
  text-align: center;
  margin-top: 40px;
  padding: 20px;
  background: rgba(239, 68, 68, 0.1);
  border-radius: 8px;
  border: 1px solid rgba(239, 68, 68, 0.2);
}

.error-hint {
  font-size: 13px;
  color: var(--text-secondary);
  margin-top: 8px;
}

/* Markdown Content Styling */
#write {
  max-width: 100%;
  position: static;
  margin: 0;
  padding: 0;
  color: var(--text-primary);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
  line-height: 1.6;
}

/* Custom block styles matching theme */
:deep(.custom-block) {
  margin: 1.5rem 0;
  padding: 1rem 1.5rem;
  border-left-width: 4px;
  border-left-style: solid;
  background-color: rgba(22, 27, 34, 0.5);
  border-radius: 0 4px 4px 0;
}

:deep(.custom-block.tip), :deep(.custom-block.info) {
  border-color: var(--accent-primary);
  background-color: rgba(31, 111, 235, 0.1);
}

:deep(.custom-block.warning) {
  border-color: #e3b341;
  background-color: rgba(227, 179, 65, 0.1);
}

:deep(.custom-block.danger) {
  border-color: #f85149;
  background-color: rgba(248, 81, 73, 0.1);
}

:deep(.custom-block-title) {
  font-weight: 600;
  margin-bottom: 0.5rem;
  color: var(--text-primary);
}

/* Links */
:deep(a) {
  color: var(--accent-primary);
  text-decoration: none;
}
:deep(a:hover) {
  text-decoration: underline;
}

/* Code blocks */
:deep(pre), :deep(code) {
  font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace;
  font-size: 0.9em;
}

:deep(pre) {
  background-color: #0d1117 !important; /* Darker than panel */
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 16px;
  overflow: auto;
}

:deep(p code), :deep(li code) {
  background-color: rgba(110, 118, 129, 0.4) !important;
  padding: 0.2em 0.4em;
  border-radius: 6px;
  color: var(--text-primary);
}

:deep(blockquote) {
  color: var(--text-secondary);
  border-left: 4px solid var(--border-color);
  padding: 0 1em;
  margin: 1em 0;
}

/* Headings */
:deep(h1), :deep(h2), :deep(h3), :deep(h4), :deep(h5), :deep(h6) {
  color: var(--text-primary);
  margin-top: 24px;
  margin-bottom: 16px;
  font-weight: 600;
  line-height: 1.25;
}

:deep(h1) { font-size: 2em; border-bottom: 1px solid var(--border-color); padding-bottom: 0.3em; }
:deep(h2) { font-size: 1.5em; border-bottom: 1px solid var(--border-color); padding-bottom: 0.3em; }
:deep(h3) { font-size: 1.25em; }

/* Tables */
:deep(table) {
  border-spacing: 0;
  border-collapse: collapse;
  margin-bottom: 16px;
  width: 100%;
}

:deep(table th), :deep(table td) {
  padding: 6px 13px;
  border: 1px solid var(--border-color);
}

:deep(table tr) {
  background-color: transparent;
  border-top: 1px solid var(--border-color);
}

:deep(table tr:nth-child(2n)) {
  background-color: rgba(22, 27, 34, 0.5);
}

:deep(img) {
  max-width: 100%;
  box-sizing: content-box;
  background-color: transparent;
  border-radius: 6px;
}
</style>
