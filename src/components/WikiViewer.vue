<template>
  <div class="wiki-viewer" :class="{ embedded: embedded }">
    <div v-if="loading" class="wiki-loading">
      <div class="spinner"></div>
      <p>加载中...</p>
    </div>
    <div v-else-if="error" class="wiki-error">
      <p>{{ error }}</p>
      <button type="button" class="btn primary" @click="retry">重试</button>
    </div>
    <div v-else class="wiki-content-wrapper">
      <iframe
        v-if="embedded"
        ref="wikiFrame"
        :src="wikiUrl"
        class="wiki-iframe"
        frameborder="0"
        @load="onFrameLoad"
      ></iframe>
      <div v-else class="wiki-standalone">
        <div class="wiki-header">
          <h1>{{ title || 'Wiki' }}</h1>
          <div class="wiki-actions">
            <button type="button" class="btn ghost" @click="openInBrowser">在浏览器中打开</button>
            <button type="button" class="btn ghost" @click="refresh">刷新</button>
          </div>
        </div>
        <div class="wiki-body" v-html="htmlContent"></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { getTauriInvoke } from '../utils/tauri'
import { openUrlInBrowser } from '../utils/tauri'
import { error as logError } from '../utils/logger'

interface Props {
  wikiUrl?: string
  toolId?: string
  embedded?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  wikiUrl: '',
  toolId: '',
  embedded: false,
})

const loading = ref(true)
const error = ref<string | null>(null)
const htmlContent = ref('')
const title = ref('')
const wikiFrame = ref<HTMLIFrameElement | null>(null)

// 解析 Wiki URL
const resolveWikiUrl = async (url?: string): Promise<string> => {
  const invoker = getTauriInvoke()
  if (!invoker) {
    return url || 'http://127.0.0.1:8777'
  }
  
  // 确保 Wiki 服务器已启动
  try {
    await invoker('start_wiki_server')
  } catch (err) {
    // 服务器可能已经在运行，忽略错误
  }
  
  if (!url) {
    // 如果没有提供 URL，尝试根据 toolId 自动查找
    if (props.toolId) {
      try {
        // 使用 find_wiki_for_tool 命令查找
        const found = await invoker<{ path: string } | null>('find_wiki_for_tool', {
          params: {
            toolId: props.toolId,
            toolName: undefined,
          }
        })
        if (found && found.path) {
          return `http://127.0.0.1:8777/file/${found.path}`
        }
      } catch (err) {
        logError('查找 Wiki 文件失败:', err)
      }
    }
    return 'http://127.0.0.1:8777'
  }
  
  // 如果是相对路径，转换为完整 URL
  if (url.startsWith('/') || !url.startsWith('http')) {
    if (url.startsWith('/')) {
      return `http://127.0.0.1:8777/file${url}`
    } else {
      return `http://127.0.0.1:8777/file/${url}`
    }
  }
  
  return url
}

// 加载 Wiki 内容
const loadWiki = async () => {
  loading.value = true
  error.value = null
  
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      throw new Error('Tauri API 不可用')
    }
    
    // 确保 Wiki 服务器已启动
    try {
      await invoker('start_wiki_server')
    } catch (err) {
      // 服务器可能已经在运行，忽略错误
    }
    
    const url = await resolveWikiUrl(props.wikiUrl)
    
    if (props.embedded) {
      // 内嵌模式：使用 iframe
      // URL 已经准备好，iframe 会自动加载
      // 确保 URL 包含主题参数（如果有保存的主题）
      const savedTheme = localStorage.getItem('wiki-theme')
      if (savedTheme && !url.includes('theme=')) {
        const urlObj = new URL(url)
        urlObj.searchParams.set('theme', savedTheme)
        // 更新 iframe src（通过 ref）
        if (wikiFrame.value) {
          wikiFrame.value.src = urlObj.toString()
        }
      }
    } else {
      // 独立模式：获取 HTML 内容
      if (url.includes('/file/')) {
        const filePath = url.split('/file/')[1].split('?')[0] // 移除查询参数
        const result = await invoker<{ html: string; title: string; toc: unknown }>('render_wiki_file', {
          params: { filePath }
        })
        htmlContent.value = result.html
        title.value = result.title
      } else {
        // 首页或其他页面，直接使用 URL
        const response = await fetch(url)
        htmlContent.value = await response.text()
      }
    }
    
    loading.value = false
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    loading.value = false
    logError('加载 Wiki 失败:', err)
  }
}

const onFrameLoad = () => {
  loading.value = false
}

const retry = () => {
  loadWiki()
}

const refresh = () => {
  loadWiki()
}

const openInBrowser = async () => {
  try {
    const invoker = getTauriInvoke()
    
    // 确保 Wiki 服务器已启动
    if (invoker) {
      try {
        await invoker('start_wiki_server')
      } catch {
        // 服务器可能已经在运行
      }
    }
    
    // 构建完整 URL
    let url = props.wikiUrl || 'http://127.0.0.1:8777'
    if (invoker && props.toolId && !url.includes('/file/')) {
      // 如果没有提供 URL，尝试根据 toolId 查找
      try {
        const found = await invoker<{ path: string } | null>('find_wiki_for_tool', {
          params: {
            toolId: props.toolId,
            toolName: undefined,
          }
        })
        if (found && found.path) {
          url = `http://127.0.0.1:8777/file/${found.path}`
        }
      } catch {
        // 查找失败，使用默认 URL
      }
    }
    
    // 添加保存的主题参数（如果有）
    const savedTheme = localStorage.getItem('wiki-theme')
    if (savedTheme && savedTheme !== 'default') {
      const urlObj = new URL(url)
      urlObj.searchParams.set('theme', savedTheme)
      url = urlObj.toString()
    }
    
    // 使用 openUrlInBrowser 函数打开浏览器
    const { openUrlInBrowser } = await import('../utils/tauri')
    await openUrlInBrowser(url)
  } catch (err) {
    logError('在浏览器中打开 Wiki 失败:', err)
    // 降级到 window.open
    const url = props.wikiUrl || 'http://127.0.0.1:8777'
    const opened = window.open(url, '_blank', 'noopener,noreferrer')
    if (!opened) {
      logError('浏览器阻止了弹窗')
    }
  }
}

watch(() => props.wikiUrl, () => {
  loadWiki()
}, { immediate: true })

onMounted(() => {
  loadWiki()
})
</script>

<style scoped>
.wiki-viewer {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #ffffff;
}

.wiki-viewer.embedded {
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 8px;
  overflow: hidden;
}

.wiki-loading,
.wiki-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px;
  min-height: 200px;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 4px solid rgba(77, 163, 255, 0.2);
  border-top-color: #4da3ff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 16px;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.wiki-error p {
  color: #ef4444;
  margin-bottom: 16px;
}

.wiki-content-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.wiki-iframe {
  width: 100%;
  height: 100%;
  border: none;
  flex: 1;
}

.wiki-standalone {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.wiki-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.2);
  background: rgba(15, 23, 42, 0.96);
}

.wiki-header h1 {
  font-size: 20px;
  font-weight: 600;
  color: #e5e7eb;
  margin: 0;
}

.wiki-actions {
  display: flex;
  gap: 8px;
}

.wiki-body {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
  background: #ffffff;
}

/* 深色主题适配 */
@media (prefers-color-scheme: dark) {
  .wiki-viewer {
    background: #0d1117;
  }
  
  .wiki-body {
    background: #0d1117;
    color: #c9d1d9;
  }
}
</style>

