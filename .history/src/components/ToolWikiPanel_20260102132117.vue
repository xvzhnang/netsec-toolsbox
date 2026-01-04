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
import { ref, watch, onMounted, nextTick, onUnmounted } from 'vue'
import { renderMarkdown, initMermaid } from '../utils/markdown'
import { getWikiForTool, readWikiFile } from '../utils/wikiReader'
import { debug, error as logError } from '../utils/logger'
import { 
  addCopyButtonsToCodeBlocks, 
  processLinks, 
  initCollapsibleBlocks
} from '../utils/wikiRenderer'

interface Props {
  toolId?: string
  toolName?: string
  filePath?: string
}

const props = defineProps<Props>()

const loading = ref(false)
const error = ref<string | null>(null)
const contentHtml = ref('')

// 加载 PinkFairy 主题
const loadTheme = () => {
  let link = document.getElementById('pinkfairy-theme') as HTMLLinkElement
  if (!link) {
    link = document.createElement('link')
    link.rel = 'stylesheet'
    link.href = '/PinkFairy/pinkfairy.css'
    link.id = 'pinkfairy-theme'
    link.type = 'text/css'
    document.head.appendChild(link)
  }
}

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
      
      // 应用各种渲染效果
      nextTick(async () => {
        const container = document.querySelector('.tool-wiki-panel .markdown-body') as HTMLElement
        if (container) {
          // 1. 代码高亮
          await applyCodeHighlighting(container)
          // 2. Mermaid 图表
          await renderMermaidCharts(container)
          // 3. 复制按钮
          addCopyButtonsToCodeBlocks(container)
          // 4. 链接处理
          processLinks(container) // 不传 callback，因为不需要内部文件跳转
          // 5. 折叠块
          initCollapsibleBlocks(container)
          // 6. KaTeX
          renderKaTeX(container)
        }
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
  loadTheme()
  // 确保初始加载
  if ((props.toolId || props.filePath) && !contentHtml.value) {
    loadContent()
  }
})
</script>

<style>
/* 这里使用全局样式，因为 markdown-body 内容是动态插入的，且我们需要覆盖 PinkFairy 主题 */
/* 也可以使用 deep selector，但 vue3 中 :deep() 更好 */

/* 淡绿色主题适配样式 - 从 WikiView.vue 移植 */
.tool-wiki-panel .markdown-body {
  position: static;
  max-width: 100%;
  margin: 0;
  padding: 20px;
  transform: none;
  background: transparent !important;
  font-family: "仿宋", "FangSong", serif;
  font-weight: bold;
  line-height: 1.6;
  color: #f1f3f6;
}

/* 文本选中样式 */
.tool-wiki-panel .markdown-body ::selection,
.tool-wiki-panel .markdown-body pre ::selection {
  color: #fff !important;
  background-color: rgba(255, 119, 204, 0.6) !important;
}

/* 代码块样式 */
.tool-wiki-panel .markdown-body pre {
  position: relative;
  background: rgba(9, 12, 16, 0.85) !important;
  border: 1px solid rgba(255, 158, 200, 0.3);
  border-radius: 12px;
  padding: 1rem 0 1rem 1rem;
  margin: 1rem 0;
  overflow-x: auto;
  backdrop-filter: blur(10px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
}

.tool-wiki-panel .markdown-body pre code {
  background: transparent !important;
  border: none !important;
  padding: 0;
  margin: 0;
  font-family: "Consolas", "Courier New", monospace;
  font-size: 0.95rem; /* 稍微调小一点，适应侧边栏 */
  line-height: 1.5;
  display: block;
  color: #F39ACD;
}

/* 复制按钮样式 */
.tool-wiki-panel .code-copy-button {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  background: linear-gradient(135deg, rgba(255, 158, 200, 0.25) 0%, rgba(255, 119, 204, 0.2) 100%);
  border: 1px solid rgba(255, 158, 200, 0.3);
  border-radius: 6px;
  color: #FF9EC8;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.3s;
  z-index: 10;
  opacity: 0;
  pointer-events: none;
}

.tool-wiki-panel .markdown-body pre:hover .code-copy-button {
  opacity: 1;
  pointer-events: all;
}

.tool-wiki-panel .code-copy-button:hover {
  background: linear-gradient(135deg, rgba(255, 119, 204, 0.4) 0%, rgba(255, 158, 200, 0.35) 100%);
  transform: translateY(-2px);
}

.tool-wiki-panel .code-copy-button.copied {
  background: rgba(16, 185, 129, 0.3);
  border-color: rgba(16, 185, 129, 0.5);
  color: #10b981;
}

/* 引用块 */
.tool-wiki-panel .markdown-body blockquote {
  margin: 1.5em 0;
  padding: 12px 20px;
  background: rgba(255, 158, 200, 0.05);
  border-left: 3px solid #FF77CC;
  border-radius: 4px;
  color: #FFD1EB;
}

/* 标题样式 */
/* Removed old h1-h3 styles as they are now handled in the main block above */

/* 表格样式 */
.tool-wiki-panel .markdown-body table {
  background: rgba(9, 12, 16, 0.6);
  border-radius: 8px;
  overflow: hidden;
}

.tool-wiki-panel .markdown-body table th {
  background: rgba(255, 158, 200, 0.2);
  color: #FF9EC8;
}

.tool-wiki-panel .markdown-body table td {
  border-color: rgba(255, 158, 200, 0.1);
}

/* 链接样式 */
.tool-wiki-panel .markdown-body a {
  color: #FF77CC;
  text-decoration: none;
}

.tool-wiki-panel .markdown-body a:hover {
  text-decoration: underline;
  color: #FF9EC8;
}

.tool-wiki-panel .external-link-icon {
  font-size: 0.8em;
  margin-left: 4px;
}
</style>

<style scoped>
.tool-wiki-panel {
  height: 100%;
  overflow-y: auto;
  /* padding: 20px; Moved to .markdown-body padding */
  background-color: rgba(15, 23, 42, 0.95); /* Match CategoryView panel bg */
  box-sizing: border-box;
}

.wiki-loading, .wiki-error, .wiki-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #94a3b8;
  gap: 16px;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(255, 255, 255, 0.1);
  border-radius: 50%;
  border-top-color: #FF77CC;
  animation: spin 1s ease-in-out infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.btn.primary {
  background-color: #FF77CC;
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
}

.btn.primary:hover {
  background-color: #FF9EC8;
}
</style>
