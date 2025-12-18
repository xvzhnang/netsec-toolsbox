<template>
  <div class="wiki-view">
    <div v-if="loading" class="wiki-loading">
      <div class="spinner"></div>
      <p>加载中...</p>
    </div>
    <div v-else-if="error" class="wiki-error">
      <p>{{ error }}</p>
      <button type="button" class="btn primary" @click="retry">重试</button>
    </div>
    <div v-else class="wiki-container">
      <aside class="wiki-sidebar">
        <div class="wiki-sidebar-header">
          <h2>Wiki</h2>
          <div v-if="themes.length > 1" class="wiki-theme-selector">
            <label for="theme-select">主题：</label>
            <select id="theme-select" v-model="currentTheme" @change="changeTheme">
              <option v-for="theme in themes" :key="theme" :value="theme">
                {{ theme === 'default' ? '默认主题' : theme }}
              </option>
            </select>
          </div>
          <button type="button" class="wiki-search-btn" @click="toggleSearch">🔍 搜索</button>
        </div>
        <div v-if="showSearch" id="wiki-search" class="wiki-search">
          <input
            type="text"
            id="search-input"
            v-model="searchQuery"
            placeholder="搜索 Wiki..."
            @keyup.enter="performSearch"
          />
          <div id="search-results">
            <ul v-if="searchResults.length > 0" class="search-results-list">
              <li v-for="result in searchResults" :key="result.file_path">
                <a href="#" @click.prevent="loadFile(result.file_path)">{{ result.title }}</a>
              </li>
            </ul>
          </div>
        </div>
        <div class="wiki-file-tree">
          <h3>文件导航</h3>
          <nav class="wiki-tree-list">
            <WikiFileTree :files="fileTree" @load-file="loadFile" />
          </nav>
        </div>
        <div class="wiki-toc-section">
          <h3>页面目录</h3>
          <nav class="wiki-toc" v-html="tocHtml"></nav>
        </div>
      </aside>
      <main class="wiki-content">
        <article class="markdown-body" v-html="contentHtml"></article>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { getTauriInvoke } from '../utils/tauri'
import { error as logError } from '../utils/logger'
import type { WikiFileInfo } from '../types/wiki'
import WikiFileTree from '../components/WikiFileTree.vue'

interface Props {
  filePath?: string
  toolId?: string
  toolName?: string
}

const props = withDefaults(defineProps<Props>(), {
  filePath: '',
  toolId: '',
  toolName: '',
})

const route = useRoute()

// 从路由查询参数或 props 获取值
const currentFilePath = ref<string | undefined>(props.filePath || (route.query.filePath as string | undefined))
const currentToolId = ref<string | undefined>(props.toolId || (route.query.toolId as string | undefined))
const currentToolName = ref<string | undefined>(props.toolName || (route.query.toolName as string | undefined))

const loading = ref(true)
const error = ref<string | null>(null)
const contentHtml = ref('')
const title = ref('Wiki')
const tocHtml = ref('')
const fileTree = ref<WikiFileInfo[]>([])
const themes = ref<string[]>(['default'])
const currentTheme = ref('default')
const showSearch = ref(false)
const searchQuery = ref('')
const searchResults = ref<Array<{ file_path: string; title: string }>>([])

// 监听路由变化
watch(() => route.query, (newQuery) => {
  currentFilePath.value = (newQuery.filePath as string | undefined) || props.filePath
  currentToolId.value = (newQuery.toolId as string | undefined) || props.toolId
  currentToolName.value = (newQuery.toolName as string | undefined) || props.toolName
  // 重新加载 Wiki 内容
  if (currentFilePath.value || currentToolId.value) {
    loadWikiContent()
  }
}, { deep: true })


// 加载 Wiki 文件
const loadWikiFile = async (filePath: string) => {
  loading.value = true
  error.value = null
  
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      throw new Error('Tauri API 不可用')
    }
    
    const result = await invoker('render_wiki_file', {
      file_path: filePath
    }) as { html: string; title: string; toc: unknown }
    
    contentHtml.value = result.html
    title.value = result.title
    generateTOC()
    loading.value = false
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    loading.value = false
    logError('加载 Wiki 文件失败:', err)
  }
}

// 生成页面目录
const generateTOC = () => {
  nextTick(() => {
    const headings = document.querySelectorAll('article.markdown-body h1, article.markdown-body h2, article.markdown-body h3')
    if (headings.length === 0) {
      tocHtml.value = '<p>暂无目录</p>'
      return
    }
    
    let toc = '<ul>'
    let currentLevel = 0
    
    headings.forEach((heading, index) => {
      const level = parseInt(heading.tagName.charAt(1))
      const id = heading.id || `heading-${index}`
      const text = heading.textContent || ''
      
      if (level > currentLevel) {
        toc += '<ul>'.repeat(level - currentLevel)
      } else if (level < currentLevel) {
        toc += '</ul>'.repeat(currentLevel - level)
      }
      
      toc += `<li><a href="#${id}">${text}</a></li>`
      currentLevel = level
    })
    
    toc += '</ul>'.repeat(currentLevel) + '</ul>'
    tocHtml.value = toc
  })
}

// 加载文件树
const loadFileTree = async () => {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      return
    }
    
    const files = await invoker('get_wiki_files') as WikiFileInfo[]
    fileTree.value = files
  } catch (err) {
    logError('加载文件树失败:', err)
  }
}

// 加载主题列表
const loadThemes = async () => {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      return
    }
    
    const themeList = await invoker('get_wiki_themes') as string[]
    themes.value = themeList
    
    // 从 localStorage 读取保存的主题
    const savedTheme = localStorage.getItem('wiki-theme')
    if (savedTheme && themeList.includes(savedTheme)) {
      currentTheme.value = savedTheme
    }
  } catch (err) {
    logError('加载主题列表失败:', err)
  }
}

// 切换主题
const changeTheme = async () => {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      return
    }
    
    await invoker('set_wiki_theme', { params: { themeName: currentTheme.value } })
    localStorage.setItem('wiki-theme', currentTheme.value)
    
    // 重新加载当前文件以应用新主题
    if (currentFilePath.value) {
      await loadWikiFile(currentFilePath.value)
    } else {
      await loadWikiContent()
    }
  } catch (err) {
    logError('切换主题失败:', err)
  }
}

// 搜索
const performSearch = async () => {
  if (!searchQuery.value.trim()) {
    searchResults.value = []
    return
  }
  
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      return
    }
    
    const results = await invoker('search_wiki', {
      params: { query: searchQuery.value }
    }) as Array<{ file_path: string; title: string }>
    searchResults.value = results
  } catch (err) {
    logError('搜索失败:', err)
  }
}

// 切换搜索
const toggleSearch = () => {
  showSearch.value = !showSearch.value
  if (showSearch.value) {
    nextTick(() => {
      const input = document.getElementById('search-input') as HTMLInputElement
      input?.focus()
    })
  }
}

// 加载文件
const loadFile = async (filePath: string) => {
  await loadWikiFile(filePath)
}

// 加载 Wiki 内容
const loadWikiContent = async () => {
  loading.value = true
  error.value = null
  
  try {
    // 确定要加载的文件
    if (currentFilePath.value) {
      await loadWikiFile(currentFilePath.value)
    } else if (currentToolId.value) {
      try {
        const invoker = getTauriInvoke()
        if (invoker) {
          const found = await invoker('find_wiki_for_tool', {
            tool_id: currentToolId.value,
            tool_name: currentToolName.value,
          }) as { path: string } | null
          if (found && found.path) {
            await loadWikiFile(found.path)
          } else {
            error.value = '未找到该工具的 Wiki 文档'
            loading.value = false
          }
        }
      } catch (err) {
        error.value = err instanceof Error ? err.message : String(err)
        loading.value = false
      }
    } else {
      // 加载首页
      await loadWikiFile('index.md')
    }
    
    // 为标题添加锚点
    nextTick(() => {
      document.querySelectorAll('article.markdown-body h1, article.markdown-body h2, article.markdown-body h3, article.markdown-body h4, article.markdown-body h5, article.markdown-body h6').forEach((heading, index) => {
        const id = heading.textContent?.toLowerCase().replace(/[^a-z0-9]+/g, '-') || `heading-${index}`
        heading.id = id
      })
      generateTOC()
    })
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    loading.value = false
  }
}

// 重试
const retry = () => {
  loadWikiContent()
}

// 初始化
onMounted(async () => {
  await loadFileTree()
  await loadThemes()
  await loadWikiContent()
})
</script>

<style scoped>
.wiki-view {
  width: 100%;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #ffffff;
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

.wiki-container {
  display: flex;
  min-height: 100vh;
}

.wiki-sidebar {
  width: 280px;
  background-color: #f6f8fa;
  border-right: 1px solid #e1e4e8;
  padding: 20px;
  overflow-y: auto;
  position: sticky;
  top: 0;
  height: 100vh;
}

.wiki-sidebar-header {
  margin-bottom: 20px;
  padding-bottom: 16px;
  border-bottom: 1px solid #e1e4e8;
}

.wiki-sidebar-header h2 {
  font-size: 18px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: #24292e;
}

.wiki-theme-selector {
  margin-bottom: 12px;
}

.wiki-theme-selector label {
  display: block;
  font-size: 12px;
  color: #586069;
  margin-bottom: 4px;
}

.wiki-theme-selector select {
  width: 100%;
  padding: 6px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 13px;
  background: white;
}

.wiki-search-btn {
  width: 100%;
  padding: 8px;
  background: #0366d6;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}

.wiki-search {
  margin-bottom: 20px;
}

.wiki-search input {
  width: 100%;
  padding: 8px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 13px;
}

.wiki-file-tree {
  margin-bottom: 20px;
}

.wiki-file-tree h3 {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: #24292e;
}

.wiki-tree-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.wiki-tree-dir,
.wiki-tree-file {
  margin: 4px 0;
}

.wiki-tree-toggle {
  cursor: pointer;
  user-select: none;
}

.wiki-tree-children {
  margin-left: 16px;
  margin-top: 4px;
}

.wiki-tree-file a {
  color: #0366d6;
  text-decoration: none;
}

.wiki-tree-file a:hover {
  text-decoration: underline;
}

.wiki-toc-section {
  margin-top: 20px;
}

.wiki-toc-section h3 {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: #24292e;
}

.wiki-toc ul {
  list-style: none;
  padding-left: 16px;
  margin: 0;
}

.wiki-toc li {
  margin: 4px 0;
}

.wiki-toc a {
  color: #586069;
  text-decoration: none;
  font-size: 13px;
}

.wiki-toc a:hover {
  color: #0366d6;
}

.wiki-content {
  flex: 1;
  overflow-y: auto;
  padding: 40px;
  background: #ffffff;
}

.markdown-body {
  max-width: 980px;
  margin: 0 auto;
}

/* 深色主题适配 */
@media (prefers-color-scheme: dark) {
  .wiki-view {
    background: #0d1117;
  }
  
  .wiki-sidebar {
    background-color: #161b22;
    border-right-color: #30363d;
  }
  
  .wiki-content {
    background: #0d1117;
    color: #c9d1d9;
  }
}
</style>

