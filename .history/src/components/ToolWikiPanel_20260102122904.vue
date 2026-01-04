<template>
  <div class="tool-wiki-panel">
    <div v-if="loading" class="wiki-loading">
      <div class="spinner"></div>
      <p>加载中...</p>
    </div>
    <div v-else-if="error" class="wiki-error">
      <p>{{ error }}</p>
      <button type="button" class="btn primary" @click="retry">重试</button>
    </div>
    <div v-else-if="!contentHtml" class="wiki-empty">
      <p>暂无文档内容</p>
    </div>
    <div v-else class="wiki-content">
      <article class="markdown-body" v-html="contentHtml"></article>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, nextTick } from 'vue'
import { renderMarkdown, renderMermaidCharts } from '../utils/markdown'
import { getWikiForTool, readWikiFile } from '../utils/wikiReader'
import { debug, error as logError } from '../utils/logger'

interface Props {
  toolId?: string
  toolName?: string
  filePath?: string
}

const props = defineProps<Props>()

const loading = ref(false)
const error = ref<string | null>(null)
const contentHtml = ref('')

const loadContent = async () => {
  if (!props.toolId && !props.filePath) {
    contentHtml.value = ''
    return
  }

  loading.value = true
  error.value = null
  
  try {
    let markdown = ''
    
    // 优先使用文件路径
    if (props.filePath) {
      markdown = await readWikiFile(props.filePath)
    } 
    // 其次使用 toolId 查找
    else if (props.toolId) {
      markdown = await getWikiForTool(props.toolId)
    }

    if (!markdown) {
      contentHtml.value = '<div class="no-wiki">未找到相关 Wiki 文档</div>'
    } else {
      contentHtml.value = await renderMarkdown(markdown)
      // 渲染 Mermaid 图表
      nextTick(() => {
        renderMermaidCharts()
      })
    }
  } catch (err) {
    logError('加载 Wiki 内容失败:', err)
    error.value = '加载文档失败，请稍后重试'
  } finally {
    loading.value = false
  }
}

const retry = () => {
  loadContent()
}

watch(() => [props.toolId, props.filePath], () => {
  loadContent()
}, { immediate: true })

onMounted(() => {
  // 确保初始加载
  if ((props.toolId || props.filePath) && !contentHtml.value) {
    loadContent()
  }
})
</script>

<style scoped>
.tool-wiki-panel {
  height: 100%;
  overflow-y: auto;
  padding: 20px;
  background-color: var(--bg-secondary);
  border-left: 1px solid var(--border-color);
  box-sizing: border-box;
}

.wiki-loading, .wiki-error, .wiki-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-secondary);
}

.spinner {
  width: 40px;
  height: 40px;
  border: 4px solid var(--border-color);
  border-top-color: var(--primary-color);
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 16px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.markdown-body {
  color: var(--text-primary);
  line-height: 1.6;
}

/* 简单的 Markdown 样式适配，确保在暗色模式下可见 */
:deep(.markdown-body h1),
:deep(.markdown-body h2),
:deep(.markdown-body h3) {
  border-bottom: 1px solid var(--border-color);
  padding-bottom: 0.3em;
  color: var(--text-primary);
}

:deep(.markdown-body code) {
  background-color: rgba(127, 127, 127, 0.1);
  padding: 0.2em 0.4em;
  border-radius: 3px;
  font-family: monospace;
}

:deep(.markdown-body pre) {
  background-color: #1e1e1e;
  padding: 16px;
  border-radius: 6px;
  overflow-x: auto;
}

:deep(.markdown-body a) {
  color: var(--primary-color);
  text-decoration: none;
}

:deep(.markdown-body a:hover) {
  text-decoration: underline;
}

:deep(.markdown-body img) {
  max-width: 100%;
  border-radius: 4px;
}
</style>
