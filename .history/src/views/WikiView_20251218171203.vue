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

// 加载内置主题 CSS
const loadBuiltinTheme = () => {
  // 内置主题 CSS（GitHub 风格）
  const builtinThemeCSS = `
/* GitHub 风格主题 - 内置 */
:root {
  --side-bar-bg-color: #fafafa;
  --control-text-color: #777;
}

html {
  font-size: 16px;
  -webkit-font-smoothing: antialiased;
}

body {
  font-family: "Open Sans","Clear Sans", "Helvetica Neue", Helvetica, Arial, 'Segoe UI Emoji', 'SF Pro', sans-serif;
  color: rgb(51, 51, 51);
  line-height: 1.6;
}

.markdown-body {
  max-width: 860px;
  margin: 0 auto;
  padding: 30px;
  padding-bottom: 100px;
}

@media only screen and (min-width: 1400px) {
  .markdown-body {
    max-width: 1024px;
  }
}

@media only screen and (min-width: 1800px) {
  .markdown-body {
    max-width: 1200px;
  }
}

.markdown-body > ul:first-child,
.markdown-body > ol:first-child {
  margin-top: 30px;
}

a {
  color: #4183C4;
}

h1, h2, h3, h4, h5, h6 {
  position: relative;
  margin-top: 1rem;
  margin-bottom: 1rem;
  font-weight: bold;
  line-height: 1.4;
  cursor: text;
}

h1:hover a.anchor, h2:hover a.anchor, h3:hover a.anchor, h4:hover a.anchor, h5:hover a.anchor, h6:hover a.anchor {
  text-decoration: none;
}

h1 tt, h1 code { font-size: inherit; }
h2 tt, h2 code { font-size: inherit; }
h3 tt, h3 code { font-size: inherit; }
h4 tt, h4 code { font-size: inherit; }
h5 tt, h5 code { font-size: inherit; }
h6 tt, h6 code { font-size: inherit; }

h1 {
  font-size: 2.25em;
  line-height: 1.2;
  border-bottom: 1px solid #eee;
}

h2 {
  font-size: 1.75em;
  line-height: 1.225;
  border-bottom: 1px solid #eee;
}

h3 {
  font-size: 1.5em;
  line-height: 1.43;
}

h4 {
  font-size: 1.25em;
}

h5 {
  font-size: 1em;
}

h6 {
  font-size: 1em;
  color: #777;
}

p, blockquote, ul, ol, dl, table {
  margin: 0.8em 0;
}

li>ol, li>ul {
  margin: 0 0;
}

hr {
  height: 2px;
  padding: 0;
  margin: 16px 0;
  background-color: #e7e7e7;
  border: 0 none;
  overflow: hidden;
  box-sizing: content-box;
}

li p.first {
  display: inline-block;
}

ul, ol {
  padding-left: 30px;
}

ul:first-child, ol:first-child {
  margin-top: 0;
}

ul:last-child, ol:last-child {
  margin-bottom: 0;
}

blockquote {
  border-left: 4px solid #dfe2e5;
  padding: 0 15px;
  color: #777777;
}

blockquote blockquote {
  padding: 0;
}

table {
  padding: 0;
  word-break: initial;
}

table tr {
  border: 1px solid #dfe2e5;
  margin: 0;
  padding: 0;
}

table tr:nth-child(2n), thead {
  background-color: #f8f8f8;
}

table th {
  font-weight: bold;
  border: 1px solid #dfe2e5;
  border-bottom: 0;
  margin: 0;
  padding: 6px 13px;
}

table td {
  border: 1px solid #dfe2e5;
  margin: 0;
  padding: 6px 13px;
}

table th:first-child, table td:first-child {
  margin-top: 0;
}

table th:last-child, table td:last-child {
  margin-bottom: 0;
}

.md-fences, code, tt {
  border: 1px solid #e7eaed;
  background-color: #f8f8f8;
  border-radius: 3px;
  padding: 0;
  padding: 2px 4px 0px 4px;
  font-size: 0.9em;
}

code {
  background-color: #f3f4f4;
  padding: 0 2px 0 2px;
}

.md-fences {
  margin-bottom: 15px;
  margin-top: 15px;
  padding-top: 8px;
  padding-bottom: 6px;
}

.md-task-list-item > input {
  margin-left: -1.3em;
}

.md-fences {
  background-color: #f8f8f8;
}

.markdown-body pre.md-meta-block {
  padding: 1rem;
  font-size: 85%;
  line-height: 1.45;
  background-color: #f7f7f7;
  border: 0;
  border-radius: 3px;
  color: #777777;
  margin-top: 0 !important;
}

.md-image>.md-meta {
  border-radius: 3px;
  padding: 2px 0px 0px 4px;
  font-size: 0.9em;
  color: inherit;
}

.md-tag {
  color: #a7a7a7;
  opacity: 1;
}

.md-toc {
  margin-top: 20px;
  padding-bottom: 20px;
}
`
  
  // 应用内置主题 CSS
  let styleElement = document.getElementById('wiki-theme-style')
  if (!styleElement) {
    styleElement = document.createElement('style')
    styleElement.id = 'wiki-theme-style'
    document.head.appendChild(styleElement)
  }
  styleElement.textContent = builtinThemeCSS
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
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #f7f9fc;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
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
  color: rgba(255, 255, 255, 0.7);
  text-decoration: none;
  font-size: 13px;
  display: block;
  padding: 6px 8px;
  border-radius: 4px;
  transition: all 0.2s;
  line-height: 1.5;
}

.wiki-toc-link:hover {
  color: #ffffff;
  background-color: rgba(255, 255, 255, 0.1);
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
  padding: 0;
  background: #ffffff;
  position: relative;
}

.markdown-body {
  max-width: 900px;
  margin: 0 auto;
  padding: 60px 40px 100px;
  background: #ffffff;
  min-height: 100%;
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.05);
}

/* 优化滚动条样式 */
.wiki-sidebar::-webkit-scrollbar {
  width: 8px;
}

.wiki-sidebar::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.2);
}

.wiki-sidebar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 4px;
}

.wiki-sidebar::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}

.wiki-content::-webkit-scrollbar {
  width: 10px;
}

.wiki-content::-webkit-scrollbar-track {
  background: #f7f9fc;
}

.wiki-content::-webkit-scrollbar-thumb {
  background: #cbd5e0;
  border-radius: 5px;
}

.wiki-content::-webkit-scrollbar-thumb:hover {
  background: #a0aec0;
}

/* 搜索结果样式优化 */
.search-results-list {
  list-style: none;
  padding: 0;
  margin: 12px 0 0;
}

.search-results-list li {
  margin: 4px 0;
}

.search-results-list a {
  color: rgba(255, 255, 255, 0.8);
  text-decoration: none;
  display: block;
  padding: 8px 12px;
  border-radius: 4px;
  font-size: 13px;
  transition: all 0.2s;
}

.search-results-list a:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #ffffff;
}

/* 深色主题适配 */
@media (prefers-color-scheme: dark) {
  .wiki-view {
    background: #0d1117;
  }
  
  .wiki-content {
    background: #0d1117;
  }
  
  .markdown-body {
    background: #0d1117;
    color: #c9d1d9;
  }
  
  .wiki-sidebar {
    background: linear-gradient(180deg, #1a1a1a 0%, #2d2d2d 100%);
  }
  
  .wiki-content {
    background: #0d1117;
    color: #c9d1d9;
  }
}
</style>

