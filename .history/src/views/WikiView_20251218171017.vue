<template>
  <div class="wiki-view" :class="{ 'wiki-view-modal': isModal }">
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
                {{ getThemeDisplayName(theme) }}
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
import { renderMarkdown, extractTitle, renderMermaidCharts } from '../utils/markdown'

interface Props {
  filePath?: string
  toolId?: string
  toolName?: string
  isModal?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  filePath: '',
  toolId: '',
  toolName: '',
  isModal: false,
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


// 加载 Wiki 文件（纯前端渲染）
const loadWikiFile = async (filePath: string) => {
  loading.value = true
  error.value = null
  
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      throw new Error('Tauri API 不可用')
    }
    
    // 读取 Markdown 文件内容（不渲染）
    // Tauri 会自动将 camelCase 转换为 snake_case，所以前端使用 filePath
    const markdownText = await invoker('read_wiki_file', { filePath }) as string
    
    // 在前端渲染 Markdown（传入文件路径用于处理相对路径）
    const html = renderMarkdown(markdownText, filePath)
    contentHtml.value = html
    
    // 提取标题
    const extractedTitle = extractTitle(markdownText)
    title.value = extractedTitle || filePath.split('/').pop()?.replace('.md', '') || 'Wiki'
    
    // 生成目录
    generateTOC()
    
    // 等待 DOM 更新后渲染 Mermaid 图表和处理内部链接
    await nextTick()
    const contentElement = document.querySelector('.wiki-content article')
    if (contentElement) {
      await renderMermaidCharts(contentElement as HTMLElement)
      
      // 处理内部链接点击事件
      const internalLinks = contentElement.querySelectorAll('.wiki-internal-link')
      internalLinks.forEach((link) => {
        link.addEventListener('click', (e) => {
          e.preventDefault()
          const targetPath = (link as HTMLElement).dataset.wikiLink
          if (targetPath) {
            loadFile(targetPath)
          }
        })
      })
    }
    
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
    const headings = document.querySelectorAll('article.markdown-body h1, article.markdown-body h2, article.markdown-body h3, article.markdown-body h4, article.markdown-body h5, article.markdown-body h6')
    if (headings.length === 0) {
      tocHtml.value = '<p>暂无目录</p>'
      return
    }
    
    // 从已渲染的 HTML 中提取标题信息
    const tocItems: Array<{ level: number; id: string; text: string }> = []
    headings.forEach((heading) => {
      const level = parseInt(heading.tagName.charAt(1))
      const id = heading.id || heading.textContent?.toLowerCase().replace(/[^\w\s-]/g, '').replace(/\s+/g, '-') || ''
      const text = heading.textContent || ''
      if (id && text) {
        tocItems.push({ level, id, text })
      }
    })
    
    if (tocItems.length === 0) {
      tocHtml.value = '<p>暂无目录</p>'
      return
    }
    
    // 生成嵌套的目录 HTML
    let toc = '<ul class="wiki-toc-list">'
    let currentLevel = 0
    
    tocItems.forEach((item) => {
      if (item.level > currentLevel) {
        toc += '<ul>'.repeat(item.level - currentLevel)
      } else if (item.level < currentLevel) {
        toc += '</ul>'.repeat(currentLevel - item.level)
      }
      
      toc += `<li><a href="#${item.id}" class="wiki-toc-link" data-id="${item.id}">${escapeHtml(item.text)}</a></li>`
      currentLevel = item.level
    })
    
    toc += '</ul>'.repeat(currentLevel) + '</ul>'
    tocHtml.value = toc
    
    // 添加点击事件处理（平滑滚动）
    nextTick(() => {
      const tocLinks = document.querySelectorAll('.wiki-toc-link')
      tocLinks.forEach((link) => {
        link.addEventListener('click', (e) => {
          e.preventDefault()
          const targetId = (link as HTMLElement).dataset.id
          if (targetId) {
            const target = document.getElementById(targetId)
            if (target) {
              target.scrollIntoView({ behavior: 'smooth', block: 'start' })
              // 更新 URL hash（不触发页面跳转）
              window.history.replaceState(null, '', `#${targetId}`)
            }
          }
        })
      })
    })
  })
}

// HTML 转义函数
function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
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

// 加载主题 CSS
const loadThemeCSS = async (themeName: string) => {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      return
    }
    
    // 读取主题 CSS 文件（从 wiki/themes/ 目录）
    const themePath = `themes/${themeName}.css`
    let cssContent = ''
    
    try {
      cssContent = await invoker('read_wiki_file', { filePath: themePath }) as string
    } catch (err) {
      // 如果主题文件不存在，使用默认样式
      logError('加载主题文件失败，使用默认样式:', err)
      return
    }
    
    // 应用主题 CSS
    let styleElement = document.getElementById('wiki-theme-style')
    if (!styleElement) {
      styleElement = document.createElement('style')
      styleElement.id = 'wiki-theme-style'
      document.head.appendChild(styleElement)
    }
    styleElement.textContent = cssContent
  } catch (err) {
    logError('加载主题失败:', err)
  }
}

// 切换主题
const changeTheme = async () => {
  try {
    const invoker = getTauriInvoke()
    if (invoker) {
      await invoker('set_wiki_theme', { theme_name: currentTheme.value })
    }
    localStorage.setItem('wiki-theme', currentTheme.value)
    
    // 加载新主题 CSS
    await loadThemeCSS(currentTheme.value)
  } catch (err) {
    logError('切换主题失败:', err)
  }
}

// 搜索
const performSearch = async () => {
  if (!searchQuery.value.trim()) {
    searchResults.value = []
    // 清除高亮
    clearSearchHighlight()
    return
  }
  
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      return
    }
    
    const results = await invoker('search_wiki', {
      query: searchQuery.value
    }) as Array<{ file_path: string; title: string }>
    searchResults.value = results
    
    // 如果当前页面在搜索结果中，高亮搜索关键词
    if (currentFilePath.value) {
      const isInResults = results.some(r => r.file_path === currentFilePath.value)
      if (isInResults) {
        highlightSearchTerms(searchQuery.value)
      }
    }
  } catch (err) {
    logError('搜索失败:', err)
  }
}

// 高亮搜索关键词
const highlightSearchTerms = (query: string) => {
  const contentElement = document.querySelector('.wiki-content article')
  if (!contentElement) return
  
  // 清除之前的高亮
  clearSearchHighlight()
  
  // 分割查询词（支持多关键词）
  const terms = query.trim().split(/\s+/).filter(t => t.length > 0)
  if (terms.length === 0) return
  
  // 创建高亮样式（如果不存在）
  let styleElement = document.getElementById('wiki-search-highlight-style')
  if (!styleElement) {
    styleElement = document.createElement('style')
    styleElement.id = 'wiki-search-highlight-style'
    styleElement.textContent = `
      .wiki-search-highlight {
        background-color: #ffeb3b;
        padding: 2px 4px;
        border-radius: 2px;
        font-weight: 500;
      }
    `
    document.head.appendChild(styleElement)
  }
  
  // 高亮所有文本节点中的关键词
  const walker = document.createTreeWalker(
    contentElement,
    NodeFilter.SHOW_TEXT,
    null
  )
  
  const textNodes: Text[] = []
  let node: Node | null
  while (node = walker.nextNode()) {
    if (node.textContent && node.textContent.trim()) {
      textNodes.push(node as Text)
    }
  }
  
  textNodes.forEach(textNode => {
    let text = textNode.textContent || ''
    let hasMatch = false
    
    terms.forEach(term => {
      const regex = new RegExp(`(${escapeRegex(term)})`, 'gi')
      if (regex.test(text)) {
        hasMatch = true
      }
    })
    
    if (hasMatch) {
      let highlightedText = text
      terms.forEach(term => {
        const regex = new RegExp(`(${escapeRegex(term)})`, 'gi')
        highlightedText = highlightedText.replace(regex, '<mark class="wiki-search-highlight">$1</mark>')
      })
      
      const wrapper = document.createElement('span')
      wrapper.innerHTML = highlightedText
      textNode.parentNode?.replaceChild(wrapper, textNode)
    }
  })
}

// 清除搜索高亮
const clearSearchHighlight = () => {
  const contentElement = document.querySelector('.wiki-content article')
  if (!contentElement) return
  
  const highlights = contentElement.querySelectorAll('.wiki-search-highlight')
  highlights.forEach(highlight => {
    const parent = highlight.parentNode
    if (parent) {
      parent.replaceChild(document.createTextNode(highlight.textContent || ''), highlight)
      parent.normalize()
    }
  })
}

// 转义正则表达式特殊字符
function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
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

// 获取主题显示名称
const getThemeDisplayName = (themeName: string): string => {
  const themeNames: Record<string, string> = {
    'default': '默认主题',
    'github': 'GitHub',
    'dark': '深色主题',
  }
  return themeNames[themeName] || themeName
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
      await loadWikiFile('README.md')
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
  
  // 加载保存的主题
  const savedTheme = localStorage.getItem('wiki-theme')
  if (savedTheme && themes.value.includes(savedTheme)) {
    currentTheme.value = savedTheme
    await loadThemeCSS(savedTheme)
  } else if (themes.value.length > 0 && themes.value[0] !== 'default') {
    await loadThemeCSS(currentTheme.value)
  }
  
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
  height: 100%;
  min-height: 600px;
  flex: 1;
}

.wiki-view-modal .wiki-container {
  min-height: 100%;
  height: 100%;
}

.wiki-sidebar {
  width: 300px;
  background: linear-gradient(180deg, #2c3e50 0%, #34495e 100%);
  border-right: none;
  padding: 0;
  overflow-y: auto;
  position: sticky;
  top: 0;
  height: 100%;
  flex-shrink: 0;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.1);
}

.wiki-view-modal .wiki-sidebar {
  height: 100%;
}

.wiki-sidebar-header {
  padding: 24px 20px;
  background: rgba(0, 0, 0, 0.2);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  margin-bottom: 0;
}

.wiki-sidebar-header h2 {
  font-size: 20px;
  font-weight: 700;
  margin: 0 0 16px 0;
  color: #ffffff;
  letter-spacing: 0.5px;
}

.wiki-theme-selector {
  margin-bottom: 16px;
}

.wiki-theme-selector label {
  display: block;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.7);
  margin-bottom: 6px;
  font-weight: 500;
}

.wiki-theme-selector select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 6px;
  font-size: 13px;
  background: rgba(255, 255, 255, 0.1);
  color: #ffffff;
  cursor: pointer;
  transition: all 0.2s;
}

.wiki-theme-selector select:hover {
  background: rgba(255, 255, 255, 0.15);
  border-color: rgba(255, 255, 255, 0.3);
}

.wiki-theme-selector select:focus {
  outline: none;
  border-color: rgba(255, 255, 255, 0.4);
  background: rgba(255, 255, 255, 0.15);
}

.wiki-search-btn {
  width: 100%;
  padding: 10px;
  background: #3498db;
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.2s;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

.wiki-search-btn:hover {
  background: #2980b9;
  transform: translateY(-1px);
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
}

.wiki-search {
  margin-bottom: 20px;
  padding: 0 20px;
}

.wiki-search input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 6px;
  font-size: 13px;
  background: rgba(255, 255, 255, 0.1);
  color: #ffffff;
  transition: all 0.2s;
}

.wiki-search input::placeholder {
  color: rgba(255, 255, 255, 0.5);
}

.wiki-search input:focus {
  outline: none;
  border-color: rgba(255, 255, 255, 0.4);
  background: rgba(255, 255, 255, 0.15);
  box-shadow: 0 0 0 3px rgba(255, 255, 255, 0.1);
}

.wiki-file-tree {
  margin-bottom: 24px;
  padding: 0 20px;
}

.wiki-file-tree h3 {
  font-size: 13px;
  font-weight: 700;
  margin: 0 0 12px 0;
  color: rgba(255, 255, 255, 0.9);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
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
  color: rgba(255, 255, 255, 0.8);
  text-decoration: none;
  display: block;
  padding: 6px 8px;
  border-radius: 4px;
  transition: all 0.2s;
  font-size: 14px;
}

.wiki-tree-file a:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #ffffff;
  text-decoration: none;
}

.wiki-toc-section {
  margin-top: 24px;
  padding: 0 20px 20px;
}

.wiki-toc-section h3 {
  font-size: 13px;
  font-weight: 700;
  margin: 0 0 12px 0;
  color: rgba(255, 255, 255, 0.9);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.wiki-toc ul {
  list-style: none;
  padding-left: 16px;
  margin: 0;
}

.wiki-toc li {
  margin: 4px 0;
}

.wiki-toc-list {
  list-style: none;
  padding-left: 0;
  margin: 0;
}

.wiki-toc-list ul {
  list-style: none;
  padding-left: 16px;
  margin: 0;
}

.wiki-toc-list li {
  margin: 4px 0;
}

.wiki-toc-link {
  color: #586069;
  text-decoration: none;
  font-size: 13px;
  display: block;
  padding: 2px 4px;
  border-radius: 3px;
  transition: all 0.2s;
}

.wiki-toc-link:hover {
  color: #0366d6;
  background-color: #f6f8fa;
}

/* Mermaid 图表样式 */
.mermaid {
  text-align: center;
  margin: 20px 0;
  background: #fff;
  padding: 20px;
  border-radius: 4px;
  overflow-x: auto;
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

