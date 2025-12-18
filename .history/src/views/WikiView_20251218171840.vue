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
      <!-- 左侧文件目录 -->
      <aside class="wiki-sidebar wiki-sidebar-left">
        <div class="wiki-sidebar-header">
          <h2>Wiki</h2>
        </div>
        <!-- 搜索栏（文件导航上侧） -->
        <div class="wiki-search-section">
          <div class="wiki-search-wrapper">
            <input
              type="text"
              id="search-input"
              v-model="searchQuery"
              placeholder="搜索 Wiki..."
              @keyup.enter="performSearch"
              @focus="showSearch = true"
              @blur="handleSearchBlur"
            />
            <div v-if="showSearch && searchResults.length > 0" class="search-results-dropdown">
              <ul class="search-results-list">
                <li v-for="result in searchResults" :key="result.file_path">
                  <a href="#" @click.prevent="loadFile(result.file_path)">{{ result.title }}</a>
                </li>
              </ul>
            </div>
          </div>
        </div>
        <div class="wiki-file-tree">
          <h3>文件导航</h3>
          <nav class="wiki-tree-list">
            <WikiFileTree :files="fileTree" @load-file="loadFile" />
          </nav>
        </div>
      </aside>
      
      <!-- 中间内容区域 -->
      <main class="wiki-content">
        <article class="markdown-body" v-html="contentHtml"></article>
      </main>
      
      <!-- 右侧大纲 -->
      <aside class="wiki-sidebar wiki-sidebar-right">
        <div class="wiki-toc-section">
          <h3>页面目录</h3>
          <nav class="wiki-toc" v-html="tocHtml"></nav>
        </div>
      </aside>
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
  // 内置主题 CSS（GitHub 风格 - 完整版）
  const builtinThemeCSS = `
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

/* 适配我们的 HTML 结构：将 #write 改为 .markdown-body */
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

h1:hover a.anchor,
h2:hover a.anchor,
h3:hover a.anchor,
h4:hover a.anchor,
h5:hover a.anchor,
h6:hover a.anchor {
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

/* 引用块样式优化 */
blockquote {
    border-left: 4px solid #dfe2e5;
    color: #6a737d;
    padding: 0 1em;
    margin: 0;
}

blockquote > :first-child {
    margin-top: 0;
}

blockquote > :last-child {
    margin-bottom: 0;
}

blockquote blockquote {
    padding-right: 0;
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

.CodeMirror-lines {
    padding-left: 4px;
}

.code-tooltip {
    box-shadow: 0 1px 1px 0 rgba(0,28,36,.3);
    border-top: 1px solid #eef2f2;
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

/* 任务列表样式优化 */
.md-task-list-item {
    list-style-type: none;
}

.md-task-list-item > input {
    margin: 0 0.2em 0.25em -1.6em;
    vertical-align: middle;
}

.md-task-list-item input[type="checkbox"] {
    cursor: pointer;
}

@media print {
    html {
        font-size: 13px;
    }
    pre {
        page-break-inside: avoid;
        word-wrap: break-word;
    }
}

.md-fences {
    background-color: #f8f8f8;
}

/* 代码块样式优化 */
.markdown-body pre {
    background-color: #f6f8fa;
    border-radius: 6px;
    font-size: 85%;
    line-height: 1.45;
    overflow: auto;
    padding: 16px;
    word-wrap: normal;
}

.markdown-body pre code {
    background-color: transparent;
    border: 0;
    display: inline;
    line-height: inherit;
    margin: 0;
    overflow: visible;
    padding: 0;
    word-wrap: normal;
    font-size: 100%;
}

.markdown-body code {
    background-color: rgba(175, 184, 193, 0.2);
    border-radius: 3px;
    font-size: 85%;
    margin: 0;
    padding: 0.2em 0.4em;
}

.markdown-body pre code {
    background-color: transparent;
    padding: 0;
}

/* highlight.js 代码高亮样式增强 */
.markdown-body .hljs {
    background: #f6f8fa;
    color: #24292f;
    display: block;
    overflow-x: auto;
    padding: 16px;
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

.mathjax-block>.code-tooltip {
    bottom: .375rem;
}

.md-mathjax-midline {
    background: #fafafa;
}

.markdown-body>h3.md-focus:before {
    left: -1.5625rem;
    top: .375rem;
}

.markdown-body>h4.md-focus:before {
    left: -1.5625rem;
    top: .285714286rem;
}

.markdown-body>h5.md-focus:before {
    left: -1.5625rem;
    top: .285714286rem;
}

.markdown-body>h6.md-focus:before {
    left: -1.5625rem;
    top: .285714286rem;
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

.sidebar-tabs {
    border-bottom: none;
}

#typora-quick-open {
    border: 1px solid #ddd;
    background-color: #f8f8f8;
}

#typora-quick-open-item {
    background-color: #FAFAFA;
    border-color: #FEFEFE #e5e5e5 #e5e5e5 #eee;
    border-style: solid;
    border-width: 1px;
}

.on-focus-mode blockquote {
    border-left-color: rgba(85, 85, 85, 0.12);
}

header, .context-menu, .megamenu-content, footer {
    font-family: "Segoe UI", "Arial", sans-serif;
}

.file-node-content:hover .file-node-icon,
.file-node-content:hover .file-node-open-state {
    visibility: visible;
}

.mac-seamless-mode #typora-sidebar {
    background-color: #fafafa;
    background-color: var(--side-bar-bg-color);
}

.mac-os .markdown-body {
    caret-color: AccentColor;
}

.md-lang {
    color: #b4654d;
}

#md-notification .btn {
    border: 0;
}

.dropdown-menu .divider {
    border-color: #e5e5e5;
    opacity: 0.4;
}

.ty-preferences .window-content {
    background-color: #fafafa;
}

.ty-preferences .nav-group-item.active {
    color: white;
    background: #999;
}

.menu-item-container a.menu-style-btn {
    background-color: #f5f8fa;
    background-image: linear-gradient(180deg, hsla(0, 0%, 100%, 0.8), hsla(0, 0%, 100%, 0));
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

// 处理搜索框失焦
const handleSearchBlur = () => {
  // 延迟隐藏，以便点击搜索结果时不会立即关闭
  window.setTimeout(() => {
    showSearch.value = false
  }, 200)
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
  loadBuiltinTheme()
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
  display: grid;
  grid-template-columns: 280px 1fr 240px;
  grid-template-rows: 1fr;
  grid-template-areas: "sidebar-left content sidebar-right";
  height: 100%;
  min-height: 600px;
  flex: 1;
  gap: 0;
}

.wiki-view-modal .wiki-container {
  min-height: 100%;
  height: 100%;
}

.wiki-sidebar {
  display: flex;
  flex-direction: column;
  padding: 0;
  overflow: hidden;
  height: 100%;
  flex-shrink: 0;
}

.wiki-sidebar-left {
  grid-area: sidebar-left;
  width: 280px;
  background: linear-gradient(180deg, #2c3e50 0%, #34495e 100%);
  border-right: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.1);
}

.wiki-view-modal .wiki-sidebar-left {
  height: 100%;
}

.wiki-sidebar-header {
  padding: 20px;
  background: rgba(0, 0, 0, 0.2);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  margin-bottom: 0;
  flex-shrink: 0;
}

.wiki-sidebar-header h2 {
  font-size: 18px;
  font-weight: 700;
  margin: 0;
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
  flex: 1;
  padding: 20px;
  overflow-y: auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
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
  flex: 1;
  padding: 20px;
  overflow-y: auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
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
  flex-shrink: 0;
}

.wiki-toc {
  flex: 1;
  overflow-y: auto;
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
  grid-area: content;
  overflow-y: auto;
  padding: 0;
  background: #ffffff;
  position: relative;
}

.markdown-body {
  max-width: 100%;
  margin: 0 auto;
  padding: 40px 60px 100px;
  background: #ffffff;
  min-height: 100%;
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

