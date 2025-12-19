<template>
  <div class="wiki-view" :class="{ 'wiki-view-modal': isModal }">
    <div v-if="loading && isInitialLoad && !error" class="wiki-loading">
      <div class="spinner"></div>
      <p>加载中...</p>
    </div>
    <div v-if="error" class="wiki-error">
      <p>{{ error }}</p>
      <button type="button" class="btn primary" @click="retry">重试</button>
    </div>
    <div v-if="!loading || !isInitialLoad" v-show="!error" class="wiki-container">
      <!-- 左侧文件目录 -->
      <aside class="wiki-sidebar wiki-sidebar-left">
        <div class="wiki-sidebar-header">
          <h2>Wiki</h2>
        </div>
        <!-- 搜索栏（文件导航上侧） -->
        <div class="wiki-search-section">
          <div class="wiki-search-wrapper">
            <div class="search-input-container">
              <span class="search-icon">🔍</span>
              <input
                type="text"
                id="search-input"
                v-model="searchQuery"
                placeholder="搜索 Wiki..."
                @input="handleSearchInput"
                @keyup.enter="performSearch"
                @focus="showSearch = true"
                @blur="handleSearchBlur"
              />
              <button
                v-if="searchQuery"
                type="button"
                class="search-clear-btn"
                @click="clearSearch"
                title="清除"
              >
                ✕
              </button>
            </div>
            <div v-if="showSearch && (searchResults.length > 0 || (searchQuery && searchResults.length === 0))" class="search-results-dropdown">
              <div v-if="searchResults.length > 0" class="search-results-header">
                <span>找到 {{ searchResults.length }} 个结果</span>
              </div>
              <ul v-if="searchResults.length > 0" class="search-results-list">
                <li v-for="result in searchResults" :key="result.file_path" class="search-result-item">
                  <a href="#" @click.prevent="loadFile(result.file_path)" @mousedown.prevent>
                    <span class="result-icon">📄</span>
                    <span class="result-content">
                      <span class="result-title">{{ highlightMatch(result.title, searchQuery) }}</span>
                      <span class="result-path">{{ result.file_path }}</span>
                    </span>
                  </a>
                </li>
              </ul>
              <div v-else-if="searchQuery && !isSearching" class="search-no-results">
                <span>未找到匹配的结果</span>
              </div>
              <div v-if="isSearching" class="search-loading">
                <span>搜索中...</span>
              </div>
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
        <!-- 面包屑导航 -->
        <nav class="wiki-breadcrumb" v-if="breadcrumbs.length > 0">
          <button class="breadcrumb-btn" @click="goHome" title="返回首页">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M8 1L2 6V14H6V10H10V14H14V6L8 1Z" fill="#FF6B35" stroke="#FF6B35" stroke-width="0.5"/>
            </svg>
          </button>
          <span v-for="(crumb, index) in breadcrumbs" :key="index" class="breadcrumb-item">
            <span class="breadcrumb-separator">/</span>
            <button 
              v-if="index < breadcrumbs.length - 1"
              class="breadcrumb-link"
              @click="navigateToPath(crumb.path)"
            >
              {{ crumb.name }}
            </button>
            <span v-else class="breadcrumb-current">{{ crumb.name }}</span>
          </span>
        </nav>
        
        <article class="markdown-body" v-html="contentHtml"></article>
      </main>
      
      <!-- 移动端菜单按钮 -->
      <button class="mobile-menu-toggle" @click="mobileMenuOpen = !mobileMenuOpen" v-if="isMobile">
        ☰
      </button>
      
      <!-- 移动端浮动菜单 -->
      <div class="mobile-menu-overlay" v-if="isMobile && mobileMenuOpen" @click="mobileMenuOpen = false">
        <div class="mobile-menu" @click.stop>
          <div class="mobile-menu-header">
            <h3>导航</h3>
            <button @click="mobileMenuOpen = false">✕</button>
          </div>
          <div class="mobile-menu-content">
            <div class="mobile-file-tree">
              <h4>文件导航</h4>
              <WikiFileTree :files="fileTree" @load-file="loadFile" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { getTauriInvoke, openUrlInBrowser } from '../utils/tauri'
import { error as logError, debug } from '../utils/logger'
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
  filePath: undefined,
  toolId: undefined,
  toolName: undefined,
  isModal: false,
})

const route = useRoute()

// 从路由查询参数或 props 获取值
// 注意：空字符串应该被视为有效值（表示要加载首页），只有 undefined 才表示未指定
const currentFilePath = ref<string | undefined>(
  props.filePath !== undefined ? props.filePath : (route.query.filePath as string | undefined)
)
const currentToolId = ref<string | undefined>(
  props.toolId !== undefined && props.toolId !== '' ? props.toolId : (route.query.toolId as string | undefined)
)
const currentToolName = ref<string | undefined>(
  props.toolName !== undefined && props.toolName !== '' ? props.toolName : (route.query.toolName as string | undefined)
)

const loading = ref(true)
const isInitialLoad = ref(true) // 区分初始加载和切换加载
const error = ref<string | null>(null)
const contentHtml = ref('')
const title = ref('Wiki')
const fileTree = ref<WikiFileInfo[]>([])
const showSearch = ref(false)
const searchQuery = ref('')
const searchResults = ref<Array<{ file_path: string; title: string }>>([])
const isSearching = ref(false)
const searchDebounceTimer = ref<number | null>(null)
// 已移除代码主题和行号功能，使用内置 GitHub 暗色主题
const breadcrumbs = ref<Array<{ name: string; path: string }>>([])
const isMobile = ref(false)
const mobileMenuOpen = ref(false)

// 用于跟踪组件是否已卸载，避免在卸载后执行异步操作
const isMounted = ref(true)
// 用于取消未完成的异步操作
let currentAbortController: AbortController | null = null

// 监听 props 变化
watch(() => props.filePath, (newFilePath) => {
  debug('WikiView props.filePath 变化')
  const normalizedNew = newFilePath !== undefined ? newFilePath : undefined
  if (normalizedNew !== currentFilePath.value) {
    currentFilePath.value = normalizedNew
    if (contentHtml.value === '' || normalizedNew !== undefined) {
      isInitialLoad.value = true
      loading.value = true
      error.value = null
    }
    debug('触发 loadWikiContent，因为 filePath 变化')
    nextTick(() => {
      loadWikiContent()
    })
  }
}, { immediate: true })

// 同时监听 toolId 和 toolName 变化
watch(() => [props.toolId, props.toolName], ([newToolId, newToolName]) => {
  debug('WikiView props.toolId/toolName 变化:', { 
    newToolId, 
    newToolName, 
    current: { toolId: currentToolId.value, toolName: currentToolName.value },
    currentFilePath: currentFilePath.value
  })
  const normalizedToolId = newToolId !== undefined && newToolId !== '' ? newToolId : undefined
  const normalizedToolName = newToolName !== undefined && newToolName !== '' ? newToolName : undefined
  if (normalizedToolId !== currentToolId.value || normalizedToolName !== currentToolName.value) {
    currentToolId.value = normalizedToolId
    currentToolName.value = normalizedToolName
    // 如果 filePath 未设置，但有 toolId，重置初始加载状态并加载
    if (!currentFilePath.value && currentToolId.value) {
      isInitialLoad.value = true
      loading.value = true
      error.value = null
      debug('触发 loadWikiContent，因为 toolId 变化:', { 
        toolId: currentToolId.value,
        toolName: currentToolName.value,
        isInitialLoad: isInitialLoad.value
      })
      nextTick(() => {
        loadWikiContent()
      })
    }
  }
}, { immediate: true })

// 保存当前 Wiki 状态到 sessionStorage（用于刷新后恢复）
const saveWikiState = () => {
  try {
    const state = {
      filePath: currentFilePath.value,
      toolId: currentToolId.value,
      toolName: currentToolName.value,
      timestamp: Date.now()
    }
    sessionStorage.setItem('wiki-view-state', JSON.stringify(state))
  } catch (err) {
    debug('保存 Wiki 状态失败:', err)
  }
}

// 从 sessionStorage 恢复 Wiki 状态
const restoreWikiState = () => {
  try {
    const saved = sessionStorage.getItem('wiki-view-state')
    if (saved) {
      const state = JSON.parse(saved)
      // 检查状态是否过期（5分钟内有效）
      if (state.timestamp && Date.now() - state.timestamp < 5 * 60 * 1000) {
        if (state.filePath !== undefined) {
          currentFilePath.value = state.filePath
        }
        if (state.toolId !== undefined) {
          currentToolId.value = state.toolId
        }
        if (state.toolName !== undefined) {
          currentToolName.value = state.toolName
        }
        debug('已恢复 Wiki 状态:', state)
        return true
      }
    }
  } catch (err) {
    debug('恢复 Wiki 状态失败:', err)
  }
  return false
}

// 监听路由变化
watch(() => route.query, (newQuery) => {
  const queryFilePath = newQuery.filePath as string | undefined
  const queryToolId = newQuery.toolId as string | undefined
  const queryToolName = newQuery.toolName as string | undefined
  
  debug('WikiView 路由查询参数变化:', { queryFilePath, queryToolId, queryToolName, propsFilePath: props.filePath })
  
  // 优先使用路由参数，如果没有则使用 props，最后尝试恢复保存的状态
  if (queryFilePath !== undefined) {
    currentFilePath.value = queryFilePath
  } else if (props.filePath !== undefined) {
    currentFilePath.value = props.filePath
  } else if (!currentFilePath.value) {
    // 如果没有路由参数和 props，尝试恢复保存的状态
    restoreWikiState()
  }
  
  if (queryToolId !== undefined && queryToolId !== '') {
    currentToolId.value = queryToolId
  } else if (props.toolId !== undefined && props.toolId !== '') {
    currentToolId.value = props.toolId
  }
  
  if (queryToolName !== undefined && queryToolName !== '') {
    currentToolName.value = queryToolName
  } else if (props.toolName !== undefined && props.toolName !== '') {
    currentToolName.value = props.toolName
  }
  
  debug('更新后的值:', { currentFilePath: currentFilePath.value, currentToolId: currentToolId.value, currentToolName: currentToolName.value })
  
  // 保存状态
  saveWikiState()
  
  // 重新加载 Wiki 内容（只要有 filePath 或 toolId）
  if (currentFilePath.value !== undefined || currentToolId.value) {
    loadWikiContent()
  }
}, { deep: true })

// 监听状态变化，自动保存
watch([currentFilePath, currentToolId, currentToolName], () => {
  saveWikiState()
})


// 加载 Wiki 文件（纯前端渲染）
const loadWikiFile = async (filePath: string) => {
  // 如果正在加载相同文件，直接返回
  const isSameFile = currentFilePath.value === filePath
  const hasContent = contentHtml.value !== ''
  const shouldSkip = loading.value && isSameFile && hasContent
  if (shouldSkip) {
    debug('正在加载相同文件且有内容，跳过')
    return
  }
  
  // 取消之前的请求（如果存在）
  // console.log('检查之前的请求:', { hasAbortController: !!currentAbortController })
  if (currentAbortController) {
    // console.log('取消之前的请求')
    debug('取消之前的请求')
    currentAbortController.abort()
  }
  
  // 创建新的 AbortController
  currentAbortController = new AbortController()
  const abortSignal = currentAbortController.signal
  // console.log('创建新的 AbortController')
  debug('创建新的 AbortController')
  
  // 只在初始加载时显示加载页面，切换时只使用淡入淡出效果
  const isSwitching = !isInitialLoad.value && contentHtml.value !== ''
  // console.log('加载状态判断:', { isSwitching, isInitialLoad: isInitialLoad.value, hasContent: contentHtml.value !== '' })
  debug('加载状态判断')
  if (!isSwitching) {
    loading.value = true
    // console.log('设置 loading = true')
    debug('设置 loading = true')
  }
  error.value = null
  
  // 设置超时保护，防止一直加载
  let timeoutId: ReturnType<typeof setTimeout> | null = setTimeout(() => {
    if (loading.value && !abortSignal.aborted && isMounted.value) {
      // console.error('========== 加载超时 ==========')
      debug('加载超时，强制关闭加载状态')
      error.value = '加载超时，请检查文件路径是否正确或文件是否存在'
      loading.value = false
      isInitialLoad.value = false
      logError('Wiki 文件加载超时:', filePath)
    }
  }, 10000) // 10秒超时
  // console.log('设置超时保护: 10秒')
  debug('设置超时保护: 10秒')
  
  // 获取当前内容元素
  const contentElement = document.querySelector('.wiki-content article') as HTMLElement | null
  // console.log('获取内容元素:', { hasElement: !!contentElement, hasContent: contentHtml.value !== '' })
  
  // 如果有旧内容，先淡出（但不立即清空，保持显示）
  if (contentElement && contentHtml.value) {
    // console.log('有旧内容，先淡出...')
    contentElement.style.transition = 'opacity 0.15s ease-out'
    contentElement.style.opacity = '0'
    // 等待淡出动画完成
    await new Promise(resolve => setTimeout(resolve, 150))
    // console.log('淡出动画完成')
  }
  
  // 检查是否已卸载
  // console.log('检查组件状态:', { isMounted: isMounted.value, aborted: abortSignal.aborted })
  if (!isMounted.value || abortSignal.aborted) {
    // console.log('组件已卸载或请求已取消，提前返回')
    return
  }
  
  try {
    // console.log('开始读取文件...')
    debug('开始读取文件')
    const invoker = getTauriInvoke()
    // console.log('获取 invoker:', { hasInvoker: !!invoker })
    if (!invoker) {
      // console.error('❌ Tauri API 不可用')
      debug('Tauri API 不可用')
      throw new Error('Tauri API 不可用')
    }
    // console.log('✅ Tauri invoker 可用')
    debug('Tauri invoker 可用')
    
    // 更新面包屑
    updateBreadcrumbs(filePath)
    currentFilePath.value = filePath
    debug('更新面包屑和当前文件路径')
    
    // 滚动到顶部（平滑滚动）
    const wikiContent = document.querySelector('.wiki-content')
    if (wikiContent) {
      wikiContent.scrollTo({ top: 0, behavior: 'smooth' })
    }
    
    // 读取 Markdown 文件内容（不渲染）
    // Tauri 会自动将 camelCase 转换为 snake_case，所以前端使用 filePath
    debug('调用 Tauri read_wiki_file')
    debug('正在读取 Wiki 文件:', filePath)
    const startTime = Date.now()
    // console.log('调用前时间:', startTime)
    
    let markdownText: string
    try {
      // console.log('准备调用 Tauri read_wiki_file，参数:', { filePath })
      markdownText = await invoker('read_wiki_file', { filePath }) as string
      // console.log('Tauri read_wiki_file 调用成功，返回数据长度:', markdownText?.length || 0)
    } catch (invokeErr) {
      // console.error('========== Tauri read_wiki_file 调用失败 ==========')
      // console.error('调用失败详情:', {
      //   error: invokeErr,
      //   errorMessage: invokeErr instanceof Error ? invokeErr.message : String(invokeErr),
      //   errorStack: invokeErr instanceof Error ? invokeErr.stack : undefined,
      //   filePath: filePath
      // })
      debug('Tauri read_wiki_file 调用失败')
      logError('调用失败:', invokeErr)
      throw invokeErr
    }
    
    const endTime = Date.now()
    // console.log('========== Tauri read_wiki_file 返回 ==========')
    // console.log('调用后时间:', endTime, '耗时:', `${endTime - startTime}ms`)
    // console.log('Wiki 文件读取成功:', { 
    //   filePath,
    //   length: markdownText.length,
    //   duration: `${endTime - startTime}ms`,
    //   preview: markdownText.substring(0, 100)
    // })
    debug(`Wiki 文件读取成功，耗时: ${endTime - startTime}ms`)
    
    // 检查是否已卸载或已取消
    if (!isMounted.value || abortSignal.aborted) {
      return
    }
    
    // 在前端渲染 Markdown（传入文件路径用于处理相对路径）
    debug('开始渲染 Markdown...')
    const html = await renderMarkdown(markdownText, filePath)
    debug('Markdown 渲染完成:', { htmlLength: html.length })
    
    // 先隐藏内容，更新 HTML，然后处理
    if (contentElement) {
      contentElement.style.opacity = '0'
      contentElement.style.transition = 'none'
    }
    
    // 先设置 loading = false，确保容器可见（v-if="!loading || !isInitialLoad"）
    // 这样容器才能渲染，我们才能找到 article 元素
    loading.value = false
    isInitialLoad.value = false
    
    contentHtml.value = html
    debug('contentHtml 已更新')
    // console.log('contentHtml 已更新，长度:', html.length, 'loading:', loading.value, 'isInitialLoad:', isInitialLoad.value)
    
    // 提取标题
    const extractedTitle = extractTitle(markdownText)
    const fileName = filePath.split('/').pop() || ''
    title.value = extractedTitle || fileName.replace(/\.md$/, '') || 'Wiki'
    
    // 等待 DOM 更新后渲染 Mermaid 图表、处理内部链接和添加代码块复制按钮
    // 使用轮询方式等待元素出现，最多等待 2 秒
    let newContentElement: HTMLElement | null = null
    const maxWaitTime = 2000 // 最多等待 2 秒
    const checkInterval = 50 // 每 50ms 检查一次
    let waitedTime = 0
    
    // console.log('开始等待 DOM 更新，查找 .wiki-content article 元素...')
    
    while (!newContentElement && waitedTime < maxWaitTime) {
      await nextTick()
      await new Promise(resolve => setTimeout(resolve, checkInterval))
      
      // 尝试多种选择器
      newContentElement = document.querySelector('.wiki-content article.markdown-body') as HTMLElement | null
      if (!newContentElement) {
        newContentElement = document.querySelector('.wiki-content article') as HTMLElement | null
      }
      if (!newContentElement) {
        newContentElement = document.querySelector('article.markdown-body') as HTMLElement | null
      }
      // 调试代码已注释
      // if (!newContentElement) {
      //   // 检查是否有 .wiki-content 元素
      //   const wikiContent = document.querySelector('.wiki-content')
      //   if (wikiContent) {
      //     console.log('找到 .wiki-content，但未找到 article，等待中...', {
      //       wikiContentHTML: wikiContent.innerHTML.substring(0, 200),
      //       hasArticle: wikiContent.querySelector('article') !== null
      //     })
      //   } else {
      //     console.log('未找到 .wiki-content 元素，等待中...')
      //   }
      // }
      
      waitedTime += checkInterval
    }
    
    // 如果还是找不到，尝试直接通过 Vue 的 ref 获取
    if (!newContentElement) {
      // console.warn('通过 querySelector 找不到元素，尝试其他方式...')
      // console.warn('当前 DOM 状态:', {
      //   hasWikiContent: !!document.querySelector('.wiki-content'),
      //   hasArticle: !!document.querySelector('article'),
      //   hasMarkdownBody: !!document.querySelector('.markdown-body'),
      //   contentHtmlLength: contentHtml.value.length
      // })
      debug('通过 querySelector 找不到元素，尝试其他方式')
      // 等待更长时间
      await new Promise(resolve => setTimeout(resolve, 200))
      newContentElement = document.querySelector('.wiki-content article.markdown-body') as HTMLElement | null
      if (!newContentElement) {
        newContentElement = document.querySelector('.wiki-content article') as HTMLElement | null
      }
      if (!newContentElement) {
        newContentElement = document.querySelector('article.markdown-body') as HTMLElement | null
      }
    }
    
    if (newContentElement) {
      const element = newContentElement
      // console.log('开始处理 markdown 渲染效果:', {
      //   hasElement: !!element,
      //   elementTag: element.tagName,
      //   innerHTMLLength: element.innerHTML.length
      // })
      
      // 先应用代码高亮（highlight.js 已经在 renderMarkdown 中处理，但需要确保样式正确）
      console.log('[WikiView] 开始应用代码高亮...')
      await applyCodeHighlighting(element)
      console.log('[WikiView] 代码高亮完成')
      
      // 渲染 Mermaid 图表
      console.log('[WikiView] 开始渲染 Mermaid 图表...')
      await renderMermaidCharts(element)
      console.log('[WikiView] Mermaid 图表渲染完成')
      
      // 添加复制按钮到所有代码块
      // console.log('开始添加复制按钮...')
      addCopyButtonsToCodeBlocks(element)
      // console.log('复制按钮添加完成')
      
      // 处理链接
      // console.log('开始处理链接...')
      processLinks(element)
      // console.log('链接处理完成')
      
      // 初始化折叠块
      // console.log('开始初始化折叠块...')
      initCollapsibleBlocks(element)
      // console.log('折叠块初始化完成')
      
      // 渲染 KaTeX 数学公式
      console.log('[WikiView] 开始渲染 KaTeX 公式...')
      renderKaTeX(element)
      console.log('[WikiView] KaTeX 公式渲染完成')
      
      // 代码高亮已通过 loadHighlightJS 和 applyCodeHighlighting 完成
      
      // 处理内部链接点击事件
      const internalLinks = element.querySelectorAll('.wiki-internal-link')
      internalLinks.forEach((link) => {
        link.addEventListener('click', (e) => {
          e.preventDefault()
          const targetPath = (link as HTMLElement).dataset.wikiLink
          if (targetPath) {
            loadFile(targetPath)
          }
        })
      })
      
      // 处理锚点链接点击事件（目录跳转）
      const anchorLinks = element.querySelectorAll('a[href^="#"]')
      anchorLinks.forEach((link) => {
        link.addEventListener('click', async (e) => {
          const href = link.getAttribute('href')
          if (href && href.startsWith('#')) {
            e.preventDefault()
            const targetId = decodeURIComponent(href.substring(1)) // 移除 # 号并解码
            
            // 尝试多种方式查找目标元素
            let targetElement: HTMLElement | null = null
            
            // 1. 直接通过 ID 查找
            targetElement = document.getElementById(targetId)
            
            // 2. 在容器内查找
            if (!targetElement) {
              targetElement = element.querySelector(`[id="${targetId}"]`) as HTMLElement
            }
            
            // 3. 查找标题元素（markdown-it 可能将 ID 放在标题上）
            if (!targetElement) {
              const headings = element.querySelectorAll('h1, h2, h3, h4, h5, h6')
              headings.forEach((heading) => {
                if (heading.id === targetId || heading.getAttribute('id') === targetId) {
                  targetElement = heading as HTMLElement
                }
              })
            }
            
            // 4. 如果还是找不到，尝试查找包含该 ID 的父元素
            if (!targetElement) {
              const allElements = element.querySelectorAll('[id]')
              allElements.forEach((el) => {
                if (el.id === targetId || el.getAttribute('id') === targetId) {
                  targetElement = el as HTMLElement
                }
              })
            }
            
            if (targetElement) {
              // 等待 DOM 更新完成
              await nextTick()
              
              // 平滑滚动到目标元素
              // 添加一点偏移，避免被固定头部遮挡
              const offset = 20
              
              // 使用 requestAnimationFrame 确保 DOM 已更新
              requestAnimationFrame(() => {
                const elementPosition = targetElement!.getBoundingClientRect().top + window.pageYOffset
                const offsetPosition = elementPosition - offset
                
                window.scrollTo({
                  top: offsetPosition,
                  behavior: 'smooth'
                })
              })
              
              // 更新 URL hash（不触发页面跳转）
              if (window.history && window.history.pushState) {
                window.history.pushState(null, '', `#${targetId}`)
              }
              
              debug('目录跳转成功:', { targetId, found: !!targetElement, elementTag: targetElement.tagName })
            } else {
              // 调试：列出所有标题的 ID
              const allHeadings = element.querySelectorAll('h1, h2, h3, h4, h5, h6')
              const headingIds: string[] = []
              allHeadings.forEach((h) => {
                const id = h.id || h.getAttribute('id') || ''
                if (id) headingIds.push(id)
              })
              debug('目录跳转失败，未找到目标元素:', { targetId, href, availableIds: headingIds })
            }
          }
        })
      })
      
      // console.log('所有 markdown 渲染效果处理完成')
      debug('所有 markdown 渲染效果处理完成')
      
      // 淡入新内容
      element.style.transition = 'opacity 0.25s ease-in'
      await nextTick()
      // 使用 requestAnimationFrame 确保样式已应用
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          element.style.opacity = '1'
        })
      })
    } else {
      // console.error('未找到 .wiki-content article 元素，无法应用渲染效果')
      debug('未找到 .wiki-content article 元素，无法应用渲染效果')
      logError('未找到 .wiki-content article 元素，无法应用渲染效果')
    }
    
    // 检查是否已卸载或已取消
    if (!isMounted.value || abortSignal.aborted) {
      return
    }
    
    // 清除超时
    if (timeoutId) {
      clearTimeout(timeoutId)
      timeoutId = null
      debug('清除超时定时器')
    }
    
    // 标记初始加载完成
    if (isInitialLoad.value) {
      isInitialLoad.value = false
      debug('设置 isInitialLoad = false')
    }
    
    loading.value = false
    debug('========== loadWikiFile 成功完成 ==========')
    debug('loadWikiFile 成功完成:', { 
      filePath, 
      contentLength: contentHtml.value.length,
      loading: loading.value,
      isInitialLoad: isInitialLoad.value
    })
  } catch (err) {
    debug('========== loadWikiFile 捕获错误 ==========')
    // 清除超时
    if (timeoutId) {
      clearTimeout(timeoutId)
      timeoutId = null
      debug('清除超时定时器（错误时）')
    }
    // 如果请求被取消，不显示错误
    if (abortSignal.aborted || !isMounted.value) {
      debug('请求已取消或组件已卸载，不显示错误:', { 
        aborted: abortSignal.aborted,
        isMounted: isMounted.value
      })
      return
    }
    
    error.value = err instanceof Error ? err.message : String(err)
    loading.value = false
    isInitialLoad.value = false
    logError('========== 加载 Wiki 文件失败 ==========')
    logError('加载 Wiki 文件失败:', err)
    debug('loadWikiFile 错误详情:', { 
      filePath, 
      error: err instanceof Error ? err.message : String(err),
      errorStack: err instanceof Error ? err.stack : undefined,
      isMounted: isMounted.value,
      aborted: abortSignal.aborted,
      loading: loading.value,
      isInitialLoad: isInitialLoad.value
    })
    
    // 出错时恢复显示
    if (contentElement) {
      contentElement.style.opacity = '1'
      contentElement.style.transition = 'opacity 0.3s ease'
    }
  } finally {
    // 清除超时（防止内存泄漏）
    if (timeoutId) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
    
    // 确保在任何情况下都重置加载状态（防止卡在加载页面）
    if (isMounted.value && !abortSignal.aborted) {
      // 如果仍然在加载状态，强制关闭（防止卡在加载页面）
      if (loading.value) {
        debug('finally 块中强制关闭加载状态:', { 
          filePath, 
          hasError: !!error.value,
          isMounted: isMounted.value,
          aborted: abortSignal.aborted
        })
        loading.value = false
        isInitialLoad.value = false
        // 如果没有错误信息，设置一个默认错误
        if (!error.value) {
          error.value = `加载失败: ${filePath}，请检查文件路径是否正确或文件是否存在`
          logError('Wiki 文件加载失败（无错误信息）:', filePath)
        }
      }
    }
    // 清理 AbortController（如果这是当前活动的请求）
    if (currentAbortController && currentAbortController.signal === abortSignal) {
      currentAbortController = null
    }
  }
}

// 应用代码高亮（对动态插入的 Markdown 内容使用 hljs.highlightElement）
const applyCodeHighlighting = async (container: HTMLElement) => {
  // console.log('applyCodeHighlighting 开始:', { containerTag: container.tagName })
  // 从 public 目录加载 highlight.js
  let hljs: any = null
  try {
    // console.log('开始加载 highlight.js...')
    hljs = await loadHighlightJS()
    // console.log('highlight.js 加载成功:', { hasHljs: !!hljs })
  } catch (err) {
    // console.error('无法加载 highlight.js:', err)
    logError('无法加载 highlight.js:', err)
    return
  }
  
  if (!hljs) {
    // console.error('hljs 为空，无法应用代码高亮')
    logError('hljs 为空，无法应用代码高亮')
    return
  }
  
  // 使用 highlightElement 方法，它会自动识别 language-xxx 类
  // highlightElement 会自动处理所有未高亮的代码块
  // 排除 mermaid 图表（它们不应该被高亮）
  const codeBlocks = container.querySelectorAll('pre code:not(.hljs):not(.mermaid), pre:not(.mermaid) code:not(.hljs)')
  // console.log('找到未高亮的代码块数量:', codeBlocks.length)
  
  codeBlocks.forEach((codeElement, index) => {
    // 跳过 mermaid 相关的元素
    if (codeElement.closest('.mermaid') || codeElement.classList.contains('mermaid')) {
      return
    }
    try {
      // 处理语言别名
      // 注意：highlight.js 支持 'powershell' 但不支持 'ps1'
      // 所以我们将所有 PowerShell 相关别名都映射到 'powershell'
      const langMap: Record<string, string> = {
        'ps1': 'powershell',  // ps1 -> powershell
        'pwsh': 'powershell', // pwsh -> powershell
        'ps': 'powershell',   // ps -> powershell
        'powershell': 'powershell', // 保持 powershell
        'shell': 'bash',
        'sh': 'bash',
        'zsh': 'bash',
      }
      
      // 获取当前语言类
      const classList = codeElement.classList
      
      // 查找 language-xxx 类
      for (const className of classList) {
        if (className.startsWith('language-')) {
          const lang = className.replace('language-', '')
          const normalizedLang = langMap[lang.toLowerCase()] || lang
          
          // 如果语言需要映射，更新类名
          if (normalizedLang !== lang) {
            classList.remove(className)
            classList.add(`language-${normalizedLang}`)
          }
          break
        }
      }
      
      // 确保代码内容已转义（防止 XSS）
      if (codeElement.innerHTML && !codeElement.classList.contains('hljs')) {
        // 如果内容包含 HTML 标签，需要先转义
        const textContent = codeElement.textContent || (codeElement as HTMLElement).innerText || ''
        if (textContent && codeElement.innerHTML !== textContent) {
          // 内容包含 HTML，需要转义
          codeElement.textContent = textContent
        }
      }
      
      // highlightElement 会自动识别 language-xxx 类并应用高亮
      hljs.highlightElement(codeElement as HTMLElement)
    } catch (err) {
      debug(`代码块 ${index} 高亮失败`)
    }
  })
  
  console.log('[WikiView] 所有代码块处理完成，处理了', codeBlocks.length, '个代码块')
}


// 加载文件树
const loadFileTree = async () => {
  if (!isMounted.value) {
    return
  }
  
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      return
    }
    
    const files = await invoker('get_wiki_files') as WikiFileInfo[]
    
    // 检查是否已卸载
    if (!isMounted.value) {
      return
    }
    
    fileTree.value = files
  } catch (err) {
    // 如果组件已卸载，不处理错误
    if (!isMounted.value) {
      return
    }
    logError('加载文件树失败:', err)
  }
}

// 加载 See Yue 主题 CSS
// 加载内置主题（PinkFairy 主题）
const loadBuiltinTheme = () => {
  // 创建 link 标签加载 PinkFairy 主题
  let link = document.getElementById('pinkfairy-theme') as HTMLLinkElement
  
  // 检查是否已加载
  if (!link) {
    link = document.createElement('link')
    link.rel = 'stylesheet'
    link.href = '/PinkFairy/pinkfairy.css'
    link.id = 'pinkfairy-theme'
    link.type = 'text/css'
    document.head.appendChild(link)
    debug('PinkFairy 主题 CSS 已加载:', link.href)
  } else {
    debug('PinkFairy 主题 CSS 已存在，跳过加载')
  }
  
  // 确保主题 CSS 加载完成后再应用适配样式
  if (link) {
    link.onload = () => {
      debug('PinkFairy 主题 CSS 加载完成')
    }
    link.onerror = () => {
      logError('PinkFairy 主题 CSS 加载失败:', link.href)
    }
  }
  
  // 添加适配样式，将 Typora 的 #write 选择器适配为 .markdown-body
  // 适配淡绿色主题
  const adapterCSS = `
/* 淡绿色主题适配样式 */
/* 将 Typora 的 #write 选择器映射到 .markdown-body */

/* 主要容器适配 - 完全透明，与整体背景一体化 */
.wiki-content article.markdown-body {
  position: static;
  max-width: 1000px;
  margin: 0 auto;
  padding: 56px 72px 140px;
  transform: none;
  background: transparent !important;
  font-family: "仿宋", "FangSong", serif;
  font-weight: bold;
  line-height: 1.6;
  color: #f1f3f6;
}

/* 确保主题样式应用到我们的容器 */
.wiki-content {
  background-color: transparent;
}

/* 文本选中样式适配 - 粉色主题 */
.wiki-content article.markdown-body ::selection,
.wiki-content article.markdown-body pre ::selection {
  color: #fff !important;
  background-color: rgba(255, 119, 204, 0.6) !important;
}

/* 代码块样式适配 - 确保代码高亮正常工作，暗色背景 */
.wiki-content article.markdown-body pre {
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

/* 代码块内代码样式 */
.markdown-body pre code {
  background: transparent !important;
  border: none !important;
  padding: 0;
  margin: 0;
  font-family: "Consolas", "Courier New", monospace;
  font-size: 1.35rem;
  line-height: 2rem;
  display: block;
  color: #F39ACD;
}

/* highlight.js 代码高亮样式 - 适配暗色背景，保留 PinkFairy 主题 */
.markdown-body pre code.hljs {
  background: transparent !important;
  border: none !important;
  padding: 0;
  margin: 0;
  display: block;
  overflow-x: auto;
  font-family: "Consolas", "Courier New", monospace;
  font-size: 1.35rem;
  line-height: 2rem;
}

/* 行内代码样式 - 暗色背景，保留 PinkFairy 主题颜色 */
.markdown-body code:not(pre code) {
  background: rgba(9, 12, 16, 0.8);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 1.25rem;
  color: #F39ACD;
  font-family: "仿宋", "FangSong", serif;
  word-break: break-all;
  border: 1px solid rgba(255, 158, 200, 0.2);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

/* markdown-it-attrs 支持的属性样式 - 适配暗色背景，保留 PinkFairy 主题 */
/* 带属性的文本 */
.markdown-body p.important,
.markdown-body .important {
    background: linear-gradient(to right,
      rgba(255, 238, 248, 0.12) 0%,
      rgba(9, 12, 16, 0.7) 100%);
    padding: 10px 14px;
    border-left: 4px solid #FF77CC;
    border-radius: 6px;
    margin: 1em 0;
    color: #FF77CC;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4),
                inset 0 1px 0 rgba(255, 158, 200, 0.1);
    backdrop-filter: blur(10px);
    transition: all 0.3s ease;
}

.markdown-body p.important:hover,
.markdown-body .important:hover {
    border-left-color: #FF9EC8;
    box-shadow: 0 4px 12px rgba(255, 158, 200, 0.2),
                inset 0 1px 0 rgba(255, 158, 200, 0.15);
}

.markdown-body p[id] {
    /* 支持 id 属性的段落 */
}

/* 带属性的标题 */
.markdown-body h1.custom-title,
.markdown-body h2.custom-title,
.markdown-body h3.custom-title,
.markdown-body h4.custom-title,
.markdown-body h5.custom-title,
.markdown-body h6.custom-title,
.markdown-body .custom-title {
    color: #FF9EC8;
    border-bottom: 2px solid rgba(255, 158, 200, 0.4);
    padding-bottom: 10px;
    margin-top: 1.5em;
    margin-bottom: 1em;
    position: relative;
    transition: all 0.3s ease;
}

.markdown-body h1.custom-title::after,
.markdown-body h2.custom-title::after,
.markdown-body h3.custom-title::after,
.markdown-body h4.custom-title::after,
.markdown-body h5.custom-title::after,
.markdown-body h6.custom-title::after,
.markdown-body .custom-title::after {
    content: '';
    position: absolute;
    bottom: -2px;
    left: 0;
    width: 0;
    height: 2px;
    background: linear-gradient(to right, #FF9EC8, #FF77CC);
    transition: width 0.3s ease;
}

.markdown-body h1.custom-title:hover::after,
.markdown-body h2.custom-title:hover::after,
.markdown-body h3.custom-title:hover::after,
.markdown-body h4.custom-title:hover::after,
.markdown-body h5.custom-title:hover::after,
.markdown-body h6.custom-title:hover::after,
.markdown-body .custom-title:hover::after {
    width: 100%;
}

/* 支持其他自定义类名 */
.markdown-body [class*="important"] {
    font-weight: 600;
}

.markdown-body [id] {
    /* 支持 id 属性的元素 */
    scroll-margin-top: 80px; /* 用于锚点跳转时的偏移 */
}

/* 代码块复制按钮样式 */

/* 代码块复制按钮样式 - 适配暗色背景，保留 PinkFairy 主题 */
.code-copy-button {
    position: absolute;
    top: 8px;
    right: 8px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: linear-gradient(135deg,
      rgba(255, 158, 200, 0.25) 0%,
      rgba(255, 119, 204, 0.2) 100%);
    border: 1px solid rgba(255, 158, 200, 0.3);
    border-radius: 6px;
    color: #FF9EC8;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    z-index: 10;
    font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
    opacity: 0;
    pointer-events: none;
    backdrop-filter: blur(10px);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
}

.markdown-body pre:hover .code-copy-button {
    opacity: 1;
    pointer-events: all;
}

.code-copy-button:hover {
    background: linear-gradient(135deg,
      rgba(255, 119, 204, 0.4) 0%,
      rgba(255, 158, 200, 0.35) 100%);
    border-color: rgba(255, 119, 204, 0.6);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(255, 119, 204, 0.4),
                0 2px 4px rgba(0, 0, 0, 0.2);
}

.code-copy-button:active {
    transform: translateY(0);
}

.code-copy-button.copied {
    background: linear-gradient(135deg,
      rgba(16, 185, 129, 0.3) 0%,
      rgba(16, 185, 129, 0.25) 100%);
    border-color: rgba(16, 185, 129, 0.5);
    color: #10b981;
    box-shadow: 0 2px 8px rgba(16, 185, 129, 0.3);
}

.code-copy-button .copy-icon {
    font-size: 14px;
    line-height: 1;
}

.code-copy-button .copy-text {
    font-weight: 500;
}

/* KaTeX 公式样式 - 适配暗色背景，保留 PinkFairy 主题 */
.markdown-body .katex,
.markdown-body .katex-display {
  color: #FF77CC;
}

.markdown-body .katex-display {
  background: linear-gradient(to right,
    rgba(255, 238, 248, 0.12) 0%,
    rgba(9, 12, 16, 0.7) 100%);
  padding: 1.2rem 1.5rem;
  border-radius: 10px;
  border-left: 4px solid #FF9EC8;
  margin: 1.5rem 0;
  backdrop-filter: blur(10px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4),
              inset 0 1px 0 rgba(255, 158, 200, 0.1);
  transition: all 0.3s ease;
}

.markdown-body .katex-display:hover {
  border-left-color: #FF77CC;
  box-shadow: 0 4px 16px rgba(255, 158, 200, 0.2),
              inset 0 1px 0 rgba(255, 158, 200, 0.15);
}

.markdown-body .katex-inline {
  color: #FF77CC;
  background: rgba(9, 12, 16, 0.6);
  padding: 3px 6px;
  border-radius: 4px;
  border: 1px solid rgba(255, 158, 200, 0.2);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

/* Mermaid 图表样式 - 适配暗色背景，保留 PinkFairy 主题 */
.markdown-body .mermaid {
  background: rgba(9, 12, 16, 0.8);
  border: 1px solid rgba(255, 158, 200, 0.3);
  border-radius: 12px;
  padding: 1rem;
  margin: 1rem 0;
  backdrop-filter: blur(10px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
  transition: all 0.3s ease;
}

.markdown-body .mermaid:hover {
  border-color: rgba(255, 158, 200, 0.5);
  box-shadow: 0 4px 16px rgba(255, 158, 200, 0.2);
}

`
  
  // 应用适配样式
  let styleElement = document.getElementById('wiki-theme-adapter')
  if (!styleElement) {
    styleElement = document.createElement('style')
    styleElement.id = 'wiki-theme-adapter'
    document.head.appendChild(styleElement)
  }
  styleElement.textContent = adapterCSS
}

// 处理搜索输入（实时搜索，带防抖）
const handleSearchInput = () => {
  // 清除之前的定时器
  if (searchDebounceTimer.value) {
    clearTimeout(searchDebounceTimer.value)
  }
  
  // 如果搜索框为空，清除结果
  if (!searchQuery.value.trim()) {
    searchResults.value = []
    clearSearchHighlight()
    return
  }
  
  // 延迟执行搜索（防抖，300ms）
  searchDebounceTimer.value = window.setTimeout(() => {
    performSearch()
  }, 300)
}

// 清除搜索
const clearSearch = () => {
  searchQuery.value = ''
  searchResults.value = []
  showSearch.value = false
  clearSearchHighlight()
  if (searchDebounceTimer.value) {
    clearTimeout(searchDebounceTimer.value)
    searchDebounceTimer.value = null
  }
}

// 高亮匹配文本
const highlightMatch = (text: string, query: string): string => {
  if (!query || !text) return text
  const regex = new RegExp(`(${escapeRegex(query)})`, 'gi')
  return text.replace(regex, '<mark class="search-match">$1</mark>')
}

// 搜索
const performSearch = async () => {
  if (!searchQuery.value.trim()) {
    searchResults.value = []
    clearSearchHighlight()
    return
  }
  
  if (!isMounted.value) {
    return
  }
  
  isSearching.value = true
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      return
    }
    
    const results = await invoker('search_wiki', {
      query: searchQuery.value
    }) as Array<{ file_path: string; title: string }>
    
    // 检查是否已卸载
    if (!isMounted.value) {
      return
    }
    
    searchResults.value = results
    
    // 如果当前页面在搜索结果中，高亮搜索关键词
    if (currentFilePath.value) {
      const isInResults = results.some(r => r.file_path === currentFilePath.value)
      if (isInResults) {
        highlightSearchTerms(searchQuery.value)
      }
    }
  } catch (err) {
    // 如果组件已卸载，不处理错误
    if (!isMounted.value) {
      return
    }
    logError('搜索失败:', err)
  } finally {
    if (isMounted.value) {
      isSearching.value = false
    }
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

// 已移除代码主题切换和行号功能，使用内置 GitHub 暗色主题

// 处理链接（内部链接高亮，外部链接添加图标）
const processLinks = (container: HTMLElement) => {
  const links = container.querySelectorAll('a')
  
  links.forEach((link) => {
    const href = link.getAttribute('href')
    if (!href) return
    
    // 如果是外部链接
    if (href.startsWith('http://') || href.startsWith('https://')) {
      link.classList.add('external-link')
      if (!link.querySelector('.external-link-icon')) {
        const icon = document.createElement('span')
        icon.className = 'external-link-icon'
        icon.innerHTML = '↗'
        icon.title = '外部链接'
        link.appendChild(icon)
      }
      
      // 拦截点击事件，使用 Tauri API 在默认浏览器中打开
      link.addEventListener('click', async (e) => {
        e.preventDefault()
        e.stopPropagation()
        try {
          await openUrlInBrowser(href)
          debug('已在默认浏览器中打开链接:', href)
        } catch (err) {
          logError('打开链接失败:', err)
          // 如果 Tauri API 失败，降级到 window.open
          window.open(href, '_blank', 'noopener,noreferrer')
        }
      })
    }
    // 如果是内部链接（已由 markdown.ts 处理）
    else if (link.classList.contains('wiki-internal-link')) {
      link.classList.add('internal-link')
      // 内部链接保持原有行为（在当前页面导航）
    }
  })
}

// 初始化折叠内容块
const initCollapsibleBlocks = (container: HTMLElement) => {
  const collapsibles = container.querySelectorAll('.collapsible-block')
  collapsibles.forEach((block) => {
    const header = block.querySelector('.collapsible-header') as HTMLElement
    const content = block.querySelector('.collapsible-content') as HTMLElement
    const icon = block.querySelector('.collapsible-icon') as HTMLElement
    
    if (header && content && icon) {
      content.style.display = 'none'
      block.classList.add('collapsed')
      
      header.addEventListener('click', () => {
        const isCollapsed = block.classList.contains('collapsed')
        block.classList.toggle('collapsed')
        content.style.display = isCollapsed ? 'block' : 'none'
        icon.textContent = isCollapsed ? '▼' : '▶'
      })
    }
  })
}

// 加载 highlight.js（仅从 public 目录，不使用 CDN）
const loadHighlightJS = (): Promise<any> => {
  return new Promise((resolve, reject) => {
    // 检查是否已经加载
    if (typeof (window as any).hljs !== 'undefined') {
      // console.log('highlight.js 已加载，使用已存在的实例')
      resolve((window as any).hljs)
      return
    }
    
    // 仅从 public 目录加载构建好的文件
    // 构建文件在 /highlight.js-11.11.1/build/highlight.min.js
    const localPaths = [
      '/highlight.js-11.11.1/build/highlight.min.js',
      '/highlight.js-11.11.1/build/highlight.js',
      '/highlight.js-11.11.1/dist/highlight.min.js',
      '/highlight.min.js'
    ]
    
    let currentPathIndex = 0
    
    const tryLoadLocal = () => {
      if (currentPathIndex >= localPaths.length) {
        // 所有本地路径都失败，拒绝 Promise
        // console.error('所有本地 highlight.js 路径都失败，无法加载 highlight.js')
        reject(new Error('无法从本地加载 highlight.js，请确保文件存在于 public 目录'))
        return
      }
      
      const script = document.createElement('script')
      const path = localPaths[currentPathIndex]
      if (!path) {
        currentPathIndex++
        tryLoadLocal()
        return
      }
      // console.log(`尝试从本地加载 highlight.js: ${path}`)
      script.src = path
      script.onerror = () => {
        // console.warn(`本地路径加载失败: ${path}，尝试下一个路径`)
        currentPathIndex++
        tryLoadLocal()
      }
      script.onload = () => {
        // console.log(`✅ 成功从本地加载 highlight.js: ${path}`)
        // 加载 CSS（从 public 目录）
        // 使用 github-dark.css 主题，与 GitHub Dark High Contrast 主题兼容
        const cssPaths = [
          '/highlight.js-11.11.1/build/demo/styles/github-dark.css',
          '/highlight.js-11.11.1/src/styles/github-dark.css',
          '/highlight.js-11.11.1/src/styles/github-dark.min.css',
          '/highlight.js-11.11.1/build/demo/styles/github-dark-dimmed.css',
          '/github-dark.css'
        ]
        
        let cssIndex = 0
        const tryLoadCSS = () => {
          if (cssIndex >= cssPaths.length) {
            // 如果本地 CSS 不存在，拒绝 Promise
            // console.error('所有本地 CSS 路径都失败，无法加载 highlight.js CSS')
            reject(new Error('无法从本地加载 highlight.js CSS，请确保文件存在于 public 目录'))
            return
          }
          
          const link = document.createElement('link')
          link.rel = 'stylesheet'
          const cssPath = cssPaths[cssIndex]
          if (!cssPath) {
            cssIndex++
            tryLoadCSS()
            return
          }
          // console.log(`尝试从本地加载 highlight.js CSS: ${cssPath}`)
          link.href = cssPath
          link.onerror = () => {
            // console.warn(`本地 CSS 路径加载失败: ${cssPath}，尝试下一个路径`)
            cssIndex++
            tryLoadCSS()
          }
          link.onload = () => {
            // console.log(`✅ 成功从本地加载 highlight.js CSS: ${cssPath}`)
            // CSS 加载成功后，resolve hljs
            resolve((window as any).hljs)
          }
          document.head.appendChild(link)
        }
        tryLoadCSS()
      }
      document.head.appendChild(script)
    }
    
    // 不再使用 CDN，只从本地加载
    tryLoadLocal()
  })
}

// 加载 KaTeX（仅从本地 public 目录，不使用 CDN）
const loadKaTeX = (): Promise<void> => {
  return new Promise((resolve, reject) => {
    // 检查是否已经加载
    if (typeof (window as any).katex !== 'undefined') {
      resolve()
      return
    }
    
    // 仅从 public 目录加载（按优先级顺序尝试）
    const localPaths = [
      '/katex/katex.min.js',       // 优先：根目录（GitHub 发布版本）
      '/katex/dist/katex.min.js',  // 备选：dist 目录结构（npm 包）
    ]
    
    let currentPathIndex = 0
    
    const tryLoadScript = () => {
      if (currentPathIndex >= localPaths.length) {
        logError('无法从本地加载 KaTeX，请确保文件存在于 public/katex/ 目录')
        reject(new Error('无法从本地加载 KaTeX，请确保文件存在于 public/katex/ 目录。请参考 public/katex/README.md 获取安装说明。'))
        return
      }
      
      const script = document.createElement('script')
      script.src = localPaths[currentPathIndex] || ''
      script.onerror = () => {
        currentPathIndex++
        tryLoadScript()
      }
      script.onload = () => {
        debug('KaTeX JavaScript 加载成功:', localPaths[currentPathIndex])
        
        // 加载 CSS（从 public 目录，路径与 JS 对应）
        const cssPaths = [
          '/katex/katex.min.css',        // 优先：根目录（GitHub 发布版本）
          '/katex/dist/katex.min.css',   // 备选：dist 目录结构（npm 包）
        ]
        
        let cssPathIndex = 0
        const tryLoadCSS = () => {
          if (cssPathIndex >= cssPaths.length) {
            logError('无法从本地加载 KaTeX CSS，请确保文件存在于 public/katex/ 目录')
            reject(new Error('无法从本地加载 KaTeX CSS，请确保文件存在于 public/katex/ 目录。请参考 public/katex/README.md 获取安装说明。'))
            return
          }
          
          const link = document.createElement('link')
          link.rel = 'stylesheet'
          link.href = cssPaths[cssPathIndex] || ''
          link.onerror = () => {
            cssPathIndex++
            tryLoadCSS()
          }
          link.onload = () => {
            debug('KaTeX CSS 加载成功:', cssPaths[cssPathIndex])
            
            // 等待一小段时间确保 CSS 完全应用
            setTimeout(() => {
              // 验证 CSS 是否真正加载（检查是否有 KaTeX 样式规则）
              const testElement = document.createElement('span')
              testElement.className = 'katex'
              testElement.style.position = 'absolute'
              testElement.style.visibility = 'hidden'
              document.body.appendChild(testElement)
              
              const computedStyle = window.getComputedStyle(testElement)
              const hasKatexStyles = computedStyle.fontFamily && computedStyle.fontFamily.includes('KaTeX')
              
              if (hasKatexStyles) {
                debug('✅ KaTeX CSS 已正确加载并应用')
              } else {
                debug('⚠️ KaTeX CSS 可能未正确加载，公式可能无法正确显示')
              }
              
              document.body.removeChild(testElement)
              
              resolve()
            }, 50)
          }
          document.head.appendChild(link)
        }
        tryLoadCSS()
      }
      document.head.appendChild(script)
    }
    
    tryLoadScript()
  })
}

// 渲染 KaTeX 数学公式
const renderKaTeX = async (container: HTMLElement) => {
  // 动态加载 KaTeX（仅从本地）
  if (typeof (window as any).katex === 'undefined') {
    try {
      // 先加载 KaTeX JS 和 CSS
      await loadKaTeX()
      
      // 验证 CSS 是否已加载
      const cssLoaded = document.querySelector('link[href*="katex.min.css"]') !== null
      if (!cssLoaded) {
        logError('⚠️ KaTeX CSS 可能未加载，公式可能无法正确显示')
      }
      
      // 使用 setTimeout 确保 KaTeX 和 CSS 完全加载后再渲染
      setTimeout(() => {
        renderKaTeXFormulas(container)
      }, 150) // 增加延迟，确保 CSS 完全应用
    } catch (err) {
      logError('无法加载 KaTeX:', err)
    }
  } else {
    // 检查 CSS 是否已加载
    const cssLoaded = document.querySelector('link[href*="katex.min.css"]') !== null
    if (!cssLoaded) {
      logError('⚠️ KaTeX CSS 未加载，尝试重新加载...')
      // 如果 CSS 未加载，尝试加载
      try {
        const link = document.createElement('link')
        link.rel = 'stylesheet'
        link.href = '/katex/katex.min.css'
        link.onload = () => {
          debug('KaTeX CSS 加载成功')
          setTimeout(() => {
            renderKaTeXFormulas(container)
          }, 50)
        }
        link.onerror = () => {
          // 尝试备选路径
          const link2 = document.createElement('link')
          link2.rel = 'stylesheet'
          link2.href = '/katex/dist/katex.min.css'
          link2.onload = () => {
            debug('KaTeX CSS 加载成功（备选路径）')
            setTimeout(() => {
              renderKaTeXFormulas(container)
            }, 50)
          }
          link2.onerror = () => {
            logError('无法加载 KaTeX CSS，公式可能无法正确显示')
            // 即使 CSS 加载失败，也尝试渲染（可能部分功能可用）
            setTimeout(() => {
              renderKaTeXFormulas(container)
            }, 50)
          }
          document.head.appendChild(link2)
        }
        document.head.appendChild(link)
      } catch (err) {
        logError('加载 KaTeX CSS 失败:', err)
        // 即使失败也尝试渲染
        setTimeout(() => {
          renderKaTeXFormulas(container)
        }, 50)
      }
    } else {
      // CSS 已加载，直接渲染
      setTimeout(() => {
        renderKaTeXFormulas(container)
      }, 50)
    }
  }
}

// 渲染 KaTeX 公式（使用防抖和错误处理，避免卡住）
// 修复：确保所有公式都能正确渲染，避免重复渲染
const renderKaTeXFormulas = (container: HTMLElement) => {
  const katex = (window as any).katex
  if (!katex) {
    debug('KaTeX 未加载，跳过公式渲染')
    return
  }
  
  // 使用 Promise 包装，确保异步执行，避免阻塞 DOM 更新
  Promise.resolve().then(() => {
    try {
      // 标记已处理的元素，避免重复渲染
      const processedElements = new WeakSet<Element>()
      
      // 第一步：处理 markdown-it-katex 已生成的元素
      // markdown-it-katex 会生成带有特定类名的元素，但可能还没有渲染内容
      const katexElements = container.querySelectorAll('.katex-display, .katex-block, .katex-inline, .katex:not(.katex-display):not(.katex-block)')
      
      katexElements.forEach((el) => {
        try {
          // 如果元素已经被处理过，跳过
          if (processedElements.has(el)) {
            return
          }
          
          // 如果元素已经有渲染内容（有 .katex 子元素），标记为已处理并跳过
          if (el.querySelector('.katex')) {
            processedElements.add(el)
            return
          }
          
          const formula = (el as HTMLElement).dataset.formula || el.textContent || ''
          if (!formula.trim()) {
            return
          }
          
          // 验证公式是否有效（不包含 HTML 标签）
          if (formula.includes('<') || formula.includes('>')) {
            return
          }
          
          // 判断是块级还是行内公式
          const isDisplay = el.classList.contains('katex-display') || el.classList.contains('katex-block')
          
          // 渲染公式
          katex.render(formula.trim(), el as HTMLElement, { 
            displayMode: isDisplay, 
            throwOnError: false 
          })
          
          // 标记为已处理
          processedElements.add(el)
        } catch (e) {
          debug('KaTeX 公式渲染失败:', e)
        }
      })
      
      // 第二步：查找并渲染未被 markdown-it-katex 处理的公式
      // 查找所有文本节点，查找 $$...$$ 和 $...$ 模式
      // 但需要跳过已经被 markdown-it-katex 处理的区域
      const walker = document.createTreeWalker(
        container,
        NodeFilter.SHOW_TEXT,
        {
          acceptNode: (node) => {
            // 跳过代码块内的文本
            const parent = node.parentElement
            if (!parent) return NodeFilter.FILTER_REJECT
            
            // 跳过代码块
            if (parent.closest('pre') || parent.closest('code')) {
              return NodeFilter.FILTER_REJECT
            }
            
            // 跳过已经被 markdown-it-katex 处理的区域
            // 如果父元素或祖先元素包含 .katex 类，说明已经被处理
            if (parent.closest('.katex, .katex-display, .katex-block, .katex-inline')) {
              return NodeFilter.FILTER_REJECT
            }
            
            // 检查文本是否包含公式标记
            const text = node.textContent || ''
            if (text.includes('$$') || text.includes('$')) {
              return NodeFilter.FILTER_ACCEPT
            }
            
            return NodeFilter.FILTER_REJECT
          }
        }
      )
      
      const textNodes: Text[] = []
      let node
      while (node = walker.nextNode()) {
        textNodes.push(node as Text)
      }
      
      // 处理找到的文本节点（从后往前处理，避免索引问题）
      for (let i = textNodes.length - 1; i >= 0; i--) {
        const textNode = textNodes[i]
        if (!textNode) continue
        
        const text = textNode.textContent || ''
        const parent = textNode.parentElement
        
        if (!parent) continue
        
        // 再次检查：如果父元素已经被处理，跳过
        if (parent.closest('.katex, .katex-display, .katex-block, .katex-inline')) {
          continue
        }
        
        // 处理块级公式 $$...$$
        // 使用非贪婪匹配，避免匹配多个公式
        const blockMatches = text.match(/\$\$([^$]+?)\$\$/g)
        if (blockMatches && blockMatches.length > 0) {
          // 从后往前处理，避免索引变化
          for (let j = blockMatches.length - 1; j >= 0; j--) {
            const match = blockMatches[j]
            if (!match) continue
            
            const formula = match.replace(/\$\$/g, '').trim()
            if (formula) {
              try {
                const span = document.createElement('span')
                span.className = 'katex-display'
                katex.render(formula, span, { displayMode: true, throwOnError: false })
                
                // 替换文本节点中的匹配部分
                const matchIndex = text.indexOf(match)
                if (matchIndex >= 0) {
                  const newText = text.substring(0, matchIndex) + 
                                span.outerHTML + 
                                text.substring(matchIndex + match.length)
                  
                  // 创建新的文本节点替换
                  const fragment = document.createDocumentFragment()
                  const tempDiv = document.createElement('div')
                  tempDiv.innerHTML = newText
                  while (tempDiv.firstChild) {
                    fragment.appendChild(tempDiv.firstChild)
                  }
                  
                  // 替换原文本节点
                  parent.replaceChild(fragment, textNode)
                  
                  // 只处理第一个匹配，避免重复
                  break
                }
              } catch (e) {
                debug('KaTeX 块级公式渲染失败:', e)
              }
            }
          }
          continue
        }
        
        // 处理行内公式 $...$（但要避免误识别）
        // 使用非贪婪匹配，避免匹配多个公式
        const inlineMatches = text.match(/\$([^$\n]+?)\$/g)
        if (inlineMatches && inlineMatches.length > 0) {
          // 从后往前处理，避免索引变化
          for (let j = inlineMatches.length - 1; j >= 0; j--) {
            const match = inlineMatches[j]
            if (!match) continue
            
            const formula = match.replace(/\$/g, '').trim()
            
            // 验证是否像数学公式（包含数学符号）
            const hasMathSymbols = /[+\-*/=()\[\]{},.^_\\]/.test(formula)
            if (formula.length >= 2 && hasMathSymbols) {
              try {
                const span = document.createElement('span')
                span.className = 'katex-inline'
                katex.render(formula, span, { displayMode: false, throwOnError: false })
                
                // 替换文本节点中的匹配部分
                const matchIndex = text.indexOf(match)
                if (matchIndex >= 0) {
                  const newText = text.substring(0, matchIndex) + 
                                span.outerHTML + 
                                text.substring(matchIndex + match.length)
                  
                  // 创建新的文本节点替换
                  const fragment = document.createDocumentFragment()
                  const tempDiv = document.createElement('div')
                  tempDiv.innerHTML = newText
                  while (tempDiv.firstChild) {
                    fragment.appendChild(tempDiv.firstChild)
                  }
                  
                  // 替换原文本节点
                  parent.replaceChild(fragment, textNode)
                  
                  // 只处理第一个匹配，避免重复
                  break
                }
              } catch (e) {
                debug('KaTeX 行内公式渲染失败:', e)
              }
            }
          }
        }
      }
    } catch (err) {
      logError('KaTeX 公式渲染出错:', err)
    }
  })
}

// 已移除终端样式和行号功能，使用普通 GitHub 暗色样式

// 为代码块添加复制按钮（普通 GitHub 样式）
const addCopyButtonsToCodeBlocks = (container: HTMLElement) => {
  // 获取所有代码块
  const codeBlocks = container.querySelectorAll('pre code')
  
  codeBlocks.forEach((codeElement) => {
    const preElement = codeElement.parentElement as HTMLElement
    if (!preElement || preElement.classList.contains('has-copy-button')) {
      return
    }
    
    preElement.classList.add('has-copy-button')
    preElement.style.position = 'relative'
    
    const copyButton = document.createElement('button')
    copyButton.className = 'code-copy-button'
    copyButton.innerHTML = '<span class="copy-icon">📋</span><span class="copy-text">复制</span>'
    copyButton.title = '复制代码'
    
    const codeText = codeElement.textContent || ''
    
    copyButton.addEventListener('click', async (e) => {
      e.stopPropagation()
      e.preventDefault()
      
      try {
        await navigator.clipboard.writeText(codeText)
        copyButton.innerHTML = '<span class="copy-icon">✓</span><span class="copy-text">已复制</span>'
        copyButton.classList.add('copied')
        setTimeout(() => {
          copyButton.innerHTML = '<span class="copy-icon">📋</span><span class="copy-text">复制</span>'
          copyButton.classList.remove('copied')
        }, 2000)
      } catch (err) {
        // 降级方案：使用 document.execCommand
        const textArea = document.createElement('textarea')
        textArea.value = codeText
        textArea.style.position = 'fixed'
        textArea.style.left = '-9999px'
        textArea.style.top = '0'
        textArea.style.opacity = '0'
        document.body.appendChild(textArea)
        textArea.focus()
        textArea.select()
        try {
          const successful = document.execCommand('copy')
          if (successful) {
            copyButton.innerHTML = '<span class="copy-icon">✓</span><span class="copy-text">已复制</span>'
            copyButton.classList.add('copied')
            setTimeout(() => {
              copyButton.innerHTML = '<span class="copy-icon">📋</span><span class="copy-text">复制</span>'
              copyButton.classList.remove('copied')
            }, 2000)
          } else {
            console.error('复制失败: execCommand 返回 false')
          }
        } catch (e) {
          console.error('复制失败:', e)
        }
        document.body.removeChild(textArea)
      }
    })
    
    preElement.appendChild(copyButton)
  })
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
  debug('loadWikiContent 被调用:', { 
    currentFilePath: currentFilePath.value, 
    currentToolId: currentToolId.value, 
    currentToolName: currentToolName.value,
    isInitialLoad: isInitialLoad.value,
    propsFilePath: props.filePath,
    propsToolId: props.toolId,
    propsToolName: props.toolName,
    contentHtmlLength: contentHtml.value.length,
    loading: loading.value,
    error: error.value,
    isMounted: isMounted.value
  })
  
  // 如果内容为空或者是初始加载，显示加载状态
  // 如果已有内容，说明是切换，不显示加载页面（使用淡入淡出效果）
  if (isInitialLoad.value || contentHtml.value === '') {
    loading.value = true
  }
  error.value = null
  
  try {
    // 确定要加载的文件
    // 注意：空字符串表示要加载首页，undefined 也表示未指定
    // console.log('检查加载条件:', {
    //   hasFilePath: currentFilePath.value !== undefined && currentFilePath.value !== '',
    //   filePath: currentFilePath.value,
    //   hasToolId: !!currentToolId.value,
    //   toolId: currentToolId.value
    // })
    if (currentFilePath.value !== undefined && currentFilePath.value !== '') {
      // console.log('有 filePath，加载文件:', currentFilePath.value)
      debug('有 filePath，加载文件:', currentFilePath.value)
      await loadWikiFile(currentFilePath.value)
    } else if (currentToolId.value) {
      debug('没有 filePath，但有 toolId，尝试查找:', currentToolId.value)
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
            isInitialLoad.value = false
          }
        } else {
          // 如果没有 invoker，显示错误
          error.value = 'Tauri API 不可用'
          loading.value = false
          isInitialLoad.value = false
        }
      } catch (err) {
        error.value = err instanceof Error ? err.message : String(err)
        loading.value = false
        isInitialLoad.value = false
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
    })
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    loading.value = false
    isInitialLoad.value = false
    logError('loadWikiContent 失败:', err)
  }
}

// 重试
const retry = () => {
  loadWikiContent()
}

// 初始化
// 更新面包屑导航
const updateBreadcrumbs = (filePath: string) => {
  const parts = filePath.split('/').filter(p => p)
  breadcrumbs.value = parts.map((part, index) => ({
    name: part.replace(/\.md$/, ''),
    path: parts.slice(0, index + 1).join('/')
  }))
}

// 导航到指定路径
const navigateToPath = (path: string) => {
  loadFile(path + (path.endsWith('.md') ? '' : '.md'))
}

// 返回首页
const goHome = () => {
  loadFile('README.md')
}


// 检测移动端
const checkMobile = () => {
  isMobile.value = window.innerWidth < 768
}

// 键盘导航
const handleKeyboardNavigation = (e: KeyboardEvent) => {
  // Ctrl/Cmd + K: 聚焦搜索框
  if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
    e.preventDefault()
    const searchInput = document.getElementById('search-input') as HTMLInputElement
    if (searchInput) {
      searchInput.focus()
    }
  }
  
  // 上下箭头：滚动
  if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault()
      const scrollAmount = 100
      window.scrollBy({
        top: e.key === 'ArrowUp' ? -scrollAmount : scrollAmount,
        behavior: 'smooth'
      })
    }
  }
  
  // Home: 返回顶部
  if (e.key === 'Home' && (e.ctrlKey || e.metaKey)) {
    e.preventDefault()
    window.scrollTo({ top: 0, behavior: 'smooth' })
  }
}

onMounted(async () => {
  // 调试代码已注释
  // console.log('========== WikiView onMounted ==========')
  // console.log('WikiView 组件挂载:', {
  //   propsFilePath: props.filePath,
  //   propsToolId: props.toolId,
  //   propsToolName: props.toolName,
  //   isModal: props.isModal,
  //   currentFilePath: currentFilePath.value,
  //   currentToolId: currentToolId.value,
  //   contentHtmlLength: contentHtml.value.length
  // })
  debug('WikiView onMounted')
  
  isMounted.value = true
  
  // 尝试恢复保存的状态（刷新后恢复）
  const restored = restoreWikiState()
  if (restored) {
    debug('已从 sessionStorage 恢复 Wiki 状态')
  }
  
  // 重置状态，确保每次挂载时都是干净的状态
  // 如果内容为空，说明是新的加载，应该显示加载页面
  if (contentHtml.value === '') {
    isInitialLoad.value = true
    loading.value = true
    debug('设置初始加载状态: isInitialLoad=true, loading=true')
  }
  
  // 加载文件树
  await loadFileTree()
  
  // 加载内置主题
  loadBuiltinTheme()
  
  // 加载 highlight.js 并在页面加载后调用 highlightAll()
  try {
    const hljs = await loadHighlightJS()
    if (hljs) {
      // 等待 DOM 完全渲染后调用 highlightAll()
      await nextTick()
      hljs.highlightAll()
      debug('highlightAll() 调用完成')
    }
  } catch (err) {
    logError('加载 highlight.js 失败:', err)
  }
  
  debug('准备调用 loadWikiContent')
  await loadWikiContent()
  // console.log('loadWikiContent 调用完成')
  
  // 默认设置：不显示行号，使用 GitHub 主题，字体大小 16px，不高对比
  // 已移除用户设置加载，使用默认值
  
  // 检测移动端
  checkMobile()
  window.addEventListener('resize', checkMobile)
  
  // 键盘导航
  window.addEventListener('keydown', handleKeyboardNavigation)
})

// 组件卸载时清理
onUnmounted(() => {
  isMounted.value = false
  
  // 取消所有未完成的异步操作
  if (currentAbortController) {
    currentAbortController.abort()
    currentAbortController = null
  }
  
  // 清除搜索防抖定时器
  if (searchDebounceTimer.value !== null) {
    clearTimeout(searchDebounceTimer.value)
    searchDebounceTimer.value = null
  }
  
  // 移除事件监听器
  window.removeEventListener('resize', checkMobile)
  window.removeEventListener('keydown', handleKeyboardNavigation)
})
</script>

<style scoped>
.wiki-view {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-color, #090c10);
  color: var(--text-color, #f1f3f6);
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Cantarell", "Fira Sans", "Droid Sans", "Helvetica Neue", Arial, sans-serif;
}

.wiki-loading,
.wiki-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 40px;
  min-height: 300px;
  background: transparent;
  color: #333;
  position: relative;
}

.spinner {
  width: 56px;
  height: 56px;
  border: 5px solid rgba(255, 158, 200, 0.3);
  border-top-color: #FF9EC8;
  border-right-color: #FF77CC;
  border-radius: 50%;
  animation: spin 0.8s cubic-bezier(0.5, 0, 0.5, 1) infinite;
  margin-bottom: 24px;
  box-shadow: 0 4px 16px rgba(255, 158, 200, 0.4);
  position: relative;
}

.spinner::before {
  content: '';
  position: absolute;
  top: -5px;
  left: -5px;
  right: -5px;
  bottom: -5px;
  border-radius: 50%;
  background: radial-gradient(circle, 
    rgba(255, 158, 200, 0.2) 0%,
    transparent 70%);
  animation: pulse 2s ease-in-out infinite;
}

@keyframes spin {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}

@keyframes pulse {
  0%, 100% {
    opacity: 0.5;
    transform: scale(1);
  }
  50% {
    opacity: 1;
    transform: scale(1.1);
  }
}

.wiki-loading p {
  margin: 0;
  font-size: 16px;
  color: #FF9EC8;
  font-weight: 600;
  letter-spacing: 0.5px;
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
  text-shadow: 0 2px 4px rgba(255, 158, 200, 0.4);
}

.wiki-error {
  background: transparent;
}

.wiki-error p {
  color: #ef4444;
  margin-bottom: 24px;
  font-size: 16px;
  font-weight: 600;
  line-height: 1.6;
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
  text-shadow: 0 1px 2px rgba(239, 68, 68, 0.2);
}

.wiki-container {
  display: grid;
  grid-template-columns: 280px 1fr;
  grid-template-rows: 1fr;
  grid-template-areas: "sidebar-left content";
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
  width: 300px;
  background: linear-gradient(to bottom, 
    rgba(9, 12, 16, 0.98) 0%, 
    rgba(9, 12, 16, 0.95) 30%,
    rgba(10, 13, 18, 0.92) 60%,
    rgba(10, 13, 18, 0.95) 100%);
  border-right: 2px solid rgba(255, 158, 200, 0.3);
  box-shadow: 4px 0 20px rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(20px);
  position: relative;
  overflow: hidden;
}

.wiki-sidebar-left::before {
  content: '';
  position: absolute;
  top: 0;
  right: 0;
  width: 2px;
  height: 100%;
  background: linear-gradient(to bottom, 
    rgba(255, 158, 200, 0.5) 0%,
    rgba(255, 119, 204, 0.5) 50%,
    rgba(255, 158, 200, 0.5) 100%);
  opacity: 0.6;
  z-index: 1;
}

.wiki-view-modal .wiki-sidebar-left {
  height: 100%;
}

.wiki-sidebar-header {
  padding: 20px 20px 18px 20px;
  background: linear-gradient(to bottom, 
    rgba(255, 158, 200, 0.1) 0%, 
    rgba(9, 12, 16, 0.06) 100%);
  border-bottom: 1px solid rgba(255, 158, 200, 0.2);
  margin-bottom: 0;
  flex-shrink: 0;
  position: relative;
  overflow: hidden;
  transition: all 0.3s ease;
  z-index: 2;
}

.wiki-sidebar-header::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(to right,
    transparent 0%,
    rgba(255, 158, 200, 0.3) 30%,
    rgba(255, 119, 204, 0.3) 50%,
    rgba(255, 158, 200, 0.3) 70%,
    transparent 100%);
  opacity: 0.6;
}

.wiki-sidebar-header h2 {
  font-size: 20px;
  font-weight: 700;
  margin: 0;
  color: #FF9EC8;
  letter-spacing: 0.5px;
  text-transform: uppercase;
  text-shadow: 0 2px 4px rgba(255, 158, 200, 0.4);
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
  position: relative;
  z-index: 1;
  line-height: 1.4;
  display: flex;
  align-items: center;
  gap: 8px;
}

.wiki-sidebar-header h2::after {
  content: '';
  flex: 1;
  height: 1px;
  background: linear-gradient(to right,
    rgba(255, 158, 200, 0.3) 0%,
    transparent 100%);
  margin-left: 12px;
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

/* 搜索栏（文件导航上侧） - 暗色背景，粉色主题，无缝衔接 */
.wiki-search-section {
  padding: 16px 20px;
  border-bottom: 1px solid rgba(255, 158, 200, 0.2);
  border-top: 1px solid rgba(255, 158, 200, 0.15);
  flex-shrink: 0;
  background: linear-gradient(to bottom,
    rgba(9, 12, 16, 0.92) 0%,
    rgba(10, 13, 18, 0.88) 100%);
  position: relative;
  z-index: 2;
  transition: all 0.3s ease;
}

.wiki-search-section::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(to right,
    transparent 0%,
    rgba(255, 158, 200, 0.2) 50%,
    transparent 100%);
}

.wiki-search-wrapper {
  position: relative;
  width: 100%;
}

.search-input-container {
  position: relative;
  display: flex;
  align-items: center;
}

.search-icon {
  position: absolute;
  left: 12px;
  color: #FF9EC8;
  font-size: 16px;
  pointer-events: none;
  z-index: 1;
  filter: drop-shadow(0 1px 2px rgba(255, 158, 200, 0.4));
}

.wiki-search-wrapper input {
  width: 100%;
  padding: 12px 40px 12px 40px;
  border: 2px solid rgba(255, 158, 200, 0.3);
  border-radius: 12px;
  font-size: 14px;
  background: rgba(9, 12, 16, 0.85);
  color: #f1f3f6;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  box-sizing: border-box;
  backdrop-filter: blur(10px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
  font-weight: 500;
}

.wiki-search-wrapper input::placeholder {
  color: rgba(255, 158, 200, 0.5);
}

.wiki-search-wrapper input:focus {
  outline: none;
  border-color: rgba(255, 119, 204, 0.6);
  background: rgba(9, 12, 16, 0.95);
  box-shadow: 0 0 0 4px rgba(255, 158, 200, 0.15), 
              0 4px 16px rgba(255, 119, 204, 0.25);
  transform: translateY(-2px);
}

.search-clear-btn {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  background: rgba(255, 158, 200, 0.3);
  color: #FF9EC8;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  line-height: 1;
  transition: all 0.2s;
  z-index: 2;
  font-weight: bold;
}

.search-clear-btn:hover {
  background: rgba(255, 119, 204, 0.7);
  color: white;
  transform: translateY(-50%) scale(1.1);
  box-shadow: 0 2px 8px rgba(255, 119, 204, 0.4);
}

.search-results-dropdown {
  position: absolute;
  top: calc(100% + 8px);
  left: 0;
  right: 0;
  background: rgba(9, 12, 16, 0.98);
  backdrop-filter: blur(20px);
  border: 2px solid rgba(255, 158, 200, 0.3);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.7),
              0 2px 8px rgba(255, 158, 200, 0.15);
  max-height: 400px;
  overflow: hidden;
  z-index: 100;
  display: flex;
  flex-direction: column;
}

.search-results-header {
  padding: 12px 16px;
  border-bottom: 2px solid rgba(255, 158, 200, 0.3);
  font-size: 13px;
  color: #FF9EC8;
  font-weight: 600;
  background: linear-gradient(to bottom,
    rgba(9, 12, 16, 0.98) 0%,
    rgba(10, 13, 18, 0.95) 100%);
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
}

.search-results-list {
  list-style: none;
  padding: 4px;
  margin: 0;
  overflow-y: auto;
  flex: 1;
}

.search-result-item {
  margin: 2px 0;
}

.search-result-item a {
  color: #f1f3f6;
  text-decoration: none;
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-radius: 8px;
  font-size: 14px;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  gap: 12px;
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
  font-weight: 500;
}

.search-result-item a:hover {
  background: linear-gradient(to right,
    rgba(255, 158, 200, 0.15) 0%,
    rgba(255, 119, 204, 0.1) 100%);
  color: #FF9EC8;
  transform: translateX(4px);
  box-shadow: 0 2px 8px rgba(255, 158, 200, 0.3);
}

.result-icon {
  font-size: 16px;
  flex-shrink: 0;
  opacity: 0.7;
}

.result-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.result-title {
  font-weight: 600;
  color: #f1f3f6;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-title :deep(.search-match) {
  background: rgba(255, 158, 200, 0.3);
  color: #FF9EC8;
  font-weight: 700;
  padding: 2px 4px;
  border-radius: 4px;
  border: 1px solid rgba(255, 158, 200, 0.5);
}

.result-path {
  font-size: 12px;
  color: rgba(255, 158, 200, 0.7);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: "Consolas", monospace;
}

.search-no-results,
.search-loading {
  padding: 20px;
  text-align: center;
  color: rgba(229, 231, 235, 0.6);
  font-size: 13px;
}

.wiki-file-tree {
  flex: 1;
  padding: 20px;
  overflow: hidden;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: linear-gradient(to bottom,
    rgba(9, 12, 16, 0.88) 0%,
    rgba(10, 13, 18, 0.85) 100%);
  border-top: 1px solid rgba(255, 158, 200, 0.15);
  border-left: none;
  border-right: none;
  border-bottom: none;
  border-radius: 0;
  margin: 0;
  box-shadow: none;
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
  backdrop-filter: blur(20px);
  position: relative;
  z-index: 1;
  transition: all 0.3s ease;
}

.wiki-file-tree::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(to right,
    transparent 0%,
    rgba(255, 158, 200, 0.2) 50%,
    transparent 100%);
}

.wiki-file-tree h3 {
  font-size: 16px;
  font-weight: 700;
  margin: 0 0 20px 0;
  color: #FF9EC8;
  text-transform: uppercase;
  letter-spacing: 1.5px;
  padding-bottom: 12px;
  border-bottom: 1px solid rgba(255, 158, 200, 0.25);
  flex-shrink: 0;
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
  text-shadow: 0 2px 4px rgba(255, 158, 200, 0.4);
  position: relative;
}

.wiki-file-tree h3::after {
  content: '';
  position: absolute;
  bottom: -1px;
  left: 0;
  width: 60px;
  height: 1px;
  background: linear-gradient(to right, rgba(255, 158, 200, 0.5), transparent);
}

.wiki-file-tree .wiki-tree-list {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding-right: 8px;
  margin-top: 4px;
}

/* 优化左侧栏整体衔接 - 添加连接装饰 */
.wiki-sidebar-left::after {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 1px;
  background: linear-gradient(to bottom,
    rgba(255, 158, 200, 0.2) 0%,
    rgba(255, 158, 200, 0.15) 20%,
    rgba(255, 158, 200, 0.15) 80%,
    rgba(255, 158, 200, 0.2) 100%);
  opacity: 0.5;
  z-index: 0;
}

.wiki-file-tree .wiki-tree-list {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding-right: 8px;
}


/* Mermaid 图表样式 */
/* Mermaid 图表样式 - 确保文字可见 */
/* Mermaid 图表样式 - 确保文字可见 */
.markdown-body .mermaid,
.mermaid {
  background: var(--bg-color, #090c10);
  color: var(--text-color, #f1f3f6);
  margin: 1.5em 0;
  text-align: center;
  padding: 16px;
  border-radius: 8px;
  border: 1px solid var(--color-border-secondary, #79828e);
}

/* Mermaid SVG 文字样式 */
.markdown-body .mermaid svg,
.mermaid svg {
  max-width: 100%;
  height: auto;
}

.markdown-body .mermaid svg text,
.mermaid svg text {
  fill: var(--text-color, #f1f3f6) !important;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Cantarell", "Fira Sans", "Droid Sans", "Helvetica Neue", Arial, sans-serif;
  font-size: 14px;
}

/* Mermaid 节点文字 */
.markdown-body .mermaid .nodeLabel,
.markdown-body .mermaid .edgeLabel,
.markdown-body .mermaid .cluster-label,
.mermaid .nodeLabel,
.mermaid .edgeLabel,
.mermaid .cluster-label {
  color: var(--text-color, #f1f3f6) !important;
  fill: var(--text-color, #f1f3f6) !important;
}

/* Mermaid 节点背景 */
.markdown-body .mermaid .node rect,
.markdown-body .mermaid .node circle,
.markdown-body .mermaid .node ellipse,
.markdown-body .mermaid .node polygon,
.mermaid .node rect,
.mermaid .node circle,
.mermaid .node ellipse,
.mermaid .node polygon {
  fill: var(--item-hover-bg-color, #272b33) !important;
  stroke: var(--color-border-secondary, #79828e) !important;
}

/* Mermaid 连接线 */
.markdown-body .mermaid .edgePath path,
.mermaid .edgePath path {
  stroke: var(--primary-color, #f9826c) !important;
}

.markdown-body .mermaid .arrowheadPath,
.mermaid .arrowheadPath {
  fill: var(--primary-color, #f9826c) !important;
}

.wiki-content {
  grid-area: content;
  overflow-y: auto;
  padding: 0;
  background: transparent;
  position: relative;
  height: 100%;
  min-width: 0; /* 防止 flex 子元素溢出 */
  scroll-behavior: smooth;
}

/* 面包屑导航 - 暗色背景，粉色主题，与Wiki标题行衔接 */
.wiki-breadcrumb {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 20px 24px 18px 24px;
  background: linear-gradient(to bottom, 
    rgba(255, 158, 200, 0.1) 0%, 
    rgba(9, 12, 16, 0.06) 100%);
  border-bottom: 1px solid rgba(255, 158, 200, 0.2);
  font-size: 14px;
  box-shadow: none;
  backdrop-filter: blur(20px);
  position: sticky;
  top: 0;
  z-index: 10;
  position: relative;
}

.wiki-breadcrumb::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(to right,
    transparent 0%,
    rgba(255, 158, 200, 0.3) 30%,
    rgba(255, 119, 204, 0.3) 50%,
    rgba(255, 158, 200, 0.3) 70%,
    transparent 100%);
  opacity: 0.6;
}

.wiki-breadcrumb::after {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 1px;
  background: linear-gradient(to bottom,
    rgba(255, 158, 200, 0.3) 0%,
    transparent 50%,
    rgba(255, 158, 200, 0.3) 100%);
  opacity: 0.4;
}

.breadcrumb-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid rgba(255, 158, 200, 0.3);
  border-radius: 8px;
  background: rgba(9, 12, 16, 0.6);
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  flex-shrink: 0;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
}

.breadcrumb-btn:hover {
  background: rgba(255, 119, 204, 0.7);
  border-color: rgba(255, 119, 204, 0.8);
  box-shadow: 0 4px 16px rgba(255, 119, 204, 0.4);
  transform: translateY(-2px) scale(1.05);
}

.breadcrumb-btn svg path {
  fill: #FF9EC8;
  transition: fill 0.2s;
}

.breadcrumb-btn:hover svg path {
  fill: white;
}

.breadcrumb-btn:active {
  transform: translateY(0);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
}

.breadcrumb-btn svg {
  width: 14px;
  height: 14px;
}

.breadcrumb-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.breadcrumb-separator {
  color: rgba(255, 158, 200, 0.5);
  margin: 0 4px;
  font-weight: 400;
  font-size: 14px;
}

.breadcrumb-link {
  color: #FF9EC8;
  text-decoration: none;
  cursor: pointer;
  background: none;
  border: none;
  padding: 6px 12px;
  font-size: 13px;
  border-radius: 6px;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  font-weight: 500;
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
}

.breadcrumb-link:hover {
  color: white;
  background: linear-gradient(135deg, 
    rgba(255, 158, 200, 0.7) 0%,
    rgba(255, 119, 204, 0.7) 100%);
  text-decoration: none;
  transform: translateX(2px);
  box-shadow: 0 2px 8px rgba(255, 119, 204, 0.4);
}

.breadcrumb-current {
  color: #FF9EC8;
  font-weight: 600;
  padding: 6px 12px;
  background: linear-gradient(135deg,
    rgba(255, 158, 200, 0.12) 0%,
    rgba(255, 119, 204, 0.08) 100%);
  border-radius: 6px;
  border: 1px solid rgba(255, 158, 200, 0.3);
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
  font-size: 13px;
}

.markdown-body {
  max-width: 1000px;
  margin: 0 auto;
  padding: 56px 72px 140px;
  /* 完全透明背景，与整体背景一体化 */
  background: transparent;
  min-height: 100%;
  box-sizing: border-box;
  width: 100%;
  font-size: 16px;
  line-height: 1.75;
  /* 浅色文字确保在暗色背景上的可读性 */
  color: #f1f3f6;
  font-family: "仿宋", "FangSong", serif;
  font-weight: bold;
  letter-spacing: 0.01em;
  border-radius: 0;
  box-shadow: none;
  position: relative;
}

@media only screen and (min-width: 1400px) {
  .markdown-body {
    max-width: 1000px;
    padding: 64px 80px 160px;
  }
}

@media only screen and (min-width: 1800px) {
  .markdown-body {
    max-width: 1100px;
    padding: 72px 96px 180px;
  }
}

/* 优化段落间距和文字颜色 */
.markdown-body p {
  margin: 1.4em 0;
  line-height: 1.8;
  color: var(--text-color, #f1f3f6);
  text-align: justify;
  text-justify: inter-ideograph;
}

/* 美化整体显示 - 暗黑色背景，保留 PinkFairy 主题样式 */
.wiki-view {
  background: linear-gradient(135deg, 
    #090c10 0%, 
    #0a0d12 50%, 
    #080b0f 100%);
  color: #f1f3f6;
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
  min-height: 100vh;
  position: relative;
  overflow: hidden;
}

/* 添加背景装饰 - 暗色背景上的粉色点缀 */
.wiki-view::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: 
    radial-gradient(circle at 20% 50%, rgba(255, 158, 200, 0.08) 0%, transparent 50%),
    radial-gradient(circle at 80% 80%, rgba(255, 119, 204, 0.08) 0%, transparent 50%),
    radial-gradient(circle at 40% 20%, rgba(255, 170, 255, 0.05) 0%, transparent 50%);
  pointer-events: none;
  z-index: 0;
}

.wiki-container {
  position: relative;
  z-index: 1;
  /* 确保容器与背景一体化 */
  background: transparent;
}

.wiki-content {
  scroll-behavior: smooth;
  background: transparent;
  /* 确保内容区域与整体背景一体化 */
  position: relative;
  z-index: 1;
}

/* 美化滚动条 - 暗色背景，粉色主题 */
.wiki-content::-webkit-scrollbar {
  width: 12px;
}

.wiki-content::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.3);
  border-radius: 6px;
  border: 1px solid rgba(255, 158, 200, 0.2);
}

.wiki-content::-webkit-scrollbar-thumb {
  background: linear-gradient(to bottom,
    rgba(255, 158, 200, 0.6) 0%,
    rgba(255, 119, 204, 0.6) 100%);
  border-radius: 6px;
  border: 2px solid rgba(0, 0, 0, 0.3);
  box-shadow: inset 0 1px 2px rgba(255, 158, 200, 0.3);
}

.wiki-content::-webkit-scrollbar-thumb:hover {
  background: linear-gradient(to bottom,
    rgba(255, 119, 204, 0.8) 0%,
    rgba(255, 158, 200, 0.8) 100%);
  box-shadow: 0 2px 8px rgba(255, 119, 204, 0.4);
}

/* 优化表格显示 - 暗色背景，保留 PinkFairy 主题，高对比度 */
.markdown-body table {
  border-collapse: collapse;
  width: 100%;
  margin: 2em 0;
  overflow-x: auto;
  display: block;
  border: 2px solid rgba(255, 158, 200, 0.3);
  border-radius: 12px;
  background: rgba(9, 12, 16, 0.75);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.6),
              inset 0 1px 0 rgba(255, 158, 200, 0.1);
  backdrop-filter: blur(20px);
  transition: all 0.3s ease;
}

.markdown-body table:hover {
  border-color: rgba(255, 158, 200, 0.4);
  box-shadow: 0 6px 24px rgba(255, 158, 200, 0.15),
              inset 0 1px 0 rgba(255, 158, 200, 0.15);
}

.markdown-body table thead {
  background: linear-gradient(to bottom, 
    rgba(255, 158, 200, 0.15) 0%, 
    rgba(255, 119, 204, 0.12) 100%);
}

.markdown-body table th {
  font-weight: 700;
  padding: 16px 20px;
  text-align: left;
  border-bottom: 3px solid rgba(255, 158, 200, 0.5);
  border-right: 1px solid rgba(255, 158, 200, 0.4);
  color: #FF77CC;
  letter-spacing: 0.5px;
  text-transform: uppercase;
  font-size: 14px;
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
}

.markdown-body table th:last-child {
  border-right: none;
}

.markdown-body table td {
  padding: 14px 20px;
  border-bottom: 1px solid rgba(255, 158, 200, 0.3);
  border-right: 1px solid rgba(255, 158, 200, 0.3);
  color: #f1f3f6;
  transition: all 0.2s ease;
  font-family: "楷体", "KaiTi", "Consolas", monospace;
}

.markdown-body table td:last-child {
  border-right: none;
}

.markdown-body table tbody tr:nth-child(even) {
  background: rgba(255, 158, 200, 0.05);
}

.markdown-body table tbody tr:hover {
  background: linear-gradient(to right,
    rgba(255, 158, 200, 0.15) 0%,
    rgba(255, 119, 204, 0.12) 100%);
  transform: scale(1.005);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 2px 8px rgba(255, 158, 200, 0.25);
  border-left: 2px solid rgba(255, 158, 200, 0.4);
}

/* 优化引用块 - 暗色背景，保留 PinkFairy 主题，高对比度 */
.markdown-body blockquote {
  display: block;
  font-size: 1em;
  overflow: visible;
  border-left: 10px solid #ffaaff;
  padding: 15px 30px 15px 20px;
  margin-bottom: 20px;
  margin-top: 20px;
  background-color: #2a1631;
  color: #FF77CC;
  border-radius: 8px;
  font-style: italic;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.7),
              inset 0 1px 0 rgba(255, 158, 200, 0.1);
  backdrop-filter: blur(20px);
  position: relative;
  transition: all 0.3s ease;
}

/* 引用块内的删除线样式 - 确保在暗色背景下可见 */
.markdown-body blockquote del,
.markdown-body blockquote s {
  text-decoration: line-through;
  text-decoration-color: rgba(255, 158, 200, 0.5);
  color: rgba(255, 119, 204, 0.6);
  background: linear-gradient(to right,
    rgba(109, 106, 167, 0.2) 0%,
    rgba(9, 12, 16, 0.4) 100%);
  padding: 2px 4px;
  border-radius: 3px;
  border: 1px solid rgba(109, 106, 167, 0.4);
  transition: all 0.3s ease;
  position: relative;
}

.markdown-body blockquote del:hover,
.markdown-body blockquote s:hover {
  background: linear-gradient(to right,
    rgba(255, 238, 248, 0.25) 0%,
    rgba(255, 158, 200, 0.15) 100%);
  color: rgba(255, 158, 200, 0.8);
  border-color: rgba(255, 158, 200, 0.5);
  text-decoration-color: rgba(255, 158, 200, 0.7);
  border-style: dashed;
  transform: translateY(-1px);
  box-shadow: 0 2px 4px rgba(255, 158, 200, 0.2);
}

.markdown-body blockquote:hover {
  border-left-color: #FF77CC;
  box-shadow: 0 6px 24px rgba(255, 158, 200, 0.2),
              inset 0 1px 0 rgba(255, 158, 200, 0.2);
  transform: translateX(2px);
}

/* 嵌套引用块样式 - 暗色背景 */
.markdown-body blockquote blockquote {
  border-left: 8px solid rgba(255, 170, 255, 0.5);
  margin-left: 20px;
  margin-right: 0;
  padding: 16px 20px;
  background: linear-gradient(to right, 
    rgba(255, 238, 248, 0.02) 0%, 
    rgba(9, 12, 16, 0.93) 100%);
  border-right: 1px solid rgba(255, 158, 200, 0.2);
  border-top: 1px solid rgba(255, 158, 200, 0.2);
  border-bottom: 1px solid rgba(255, 158, 200, 0.2);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.6),
              inset 0 1px 0 rgba(255, 158, 200, 0.08);
}

.markdown-body blockquote blockquote blockquote {
  border-left: 6px solid rgba(255, 170, 255, 0.35);
  margin-left: 16px;
  padding: 12px 16px;
  background: linear-gradient(to right, 
    rgba(255, 238, 248, 0.01) 0%, 
    rgba(9, 12, 16, 0.95) 100%);
  border-right: 1px solid rgba(255, 158, 200, 0.15);
  border-top: 1px solid rgba(255, 158, 200, 0.15);
  border-bottom: 1px solid rgba(255, 158, 200, 0.15);
  transition: all 0.3s ease;
}

.markdown-body blockquote blockquote blockquote:hover {
  border-left-color: rgba(255, 170, 255, 0.6);
  transform: translateX(2px);
}

.markdown-body blockquote::before {
  content: '"';
  position: absolute;
  left: 16px;
  top: 12px;
  font-size: 56px;
  color: rgba(255, 158, 200, 0.25);
  font-family: Georgia, serif;
  line-height: 1;
  font-weight: bold;
  transition: color 0.3s ease;
}

.markdown-body blockquote:hover::before {
  color: rgba(255, 158, 200, 0.4);
}

/* markdown-it-container 自定义容器样式 - 淡绿色主题 */
.markdown-body .tip,
.markdown-body .note,
.markdown-body .info,
.markdown-body .warning,
.markdown-body .danger,
.markdown-body .caution {
  margin: 2em 0;
  padding: 20px 24px;
  border-radius: 16px;
  border-left: 10px solid;
  background: rgba(9, 12, 16, 0.75);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.6),
              inset 0 1px 0 rgba(255, 158, 200, 0.15);
  position: relative;
  backdrop-filter: blur(20px);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  border-right: 2px solid;
  border-top: 2px solid;
  border-bottom: 2px solid;
  color: #f1f3f6;
}

.markdown-body .tip:hover,
.markdown-body .note:hover,
.markdown-body .info:hover,
.markdown-body .warning:hover,
.markdown-body .danger:hover,
.markdown-body .caution:hover {
  transform: translateX(4px) translateY(-2px);
  box-shadow: 0 8px 32px rgba(255, 158, 200, 0.25),
              inset 0 1px 0 rgba(255, 158, 200, 0.2);
  border-left-width: 12px;
}

.markdown-body .tip {
  border-left-color: #10b981;
  border-right-color: rgba(16, 185, 129, 0.3);
  border-top-color: rgba(16, 185, 129, 0.3);
  border-bottom-color: rgba(16, 185, 129, 0.3);
  background: linear-gradient(to right, 
    rgba(16, 185, 129, 0.12), 
    rgba(9, 12, 16, 0.75));
  color: #f1f3f6;
}

.markdown-body .note,
.markdown-body .info {
  border-left-color: #FF9EC8;
  border-right-color: rgba(255, 158, 200, 0.3);
  border-top-color: rgba(255, 158, 200, 0.3);
  border-bottom-color: rgba(255, 158, 200, 0.3);
  background: linear-gradient(to right, 
    rgba(255, 158, 200, 0.12), 
    rgba(9, 12, 16, 0.75));
  color: #f1f3f6;
}

.markdown-body .warning {
  border-left-color: #f59e0b;
  border-right-color: rgba(245, 158, 11, 0.3);
  border-top-color: rgba(245, 158, 11, 0.3);
  border-bottom-color: rgba(245, 158, 11, 0.3);
  background: linear-gradient(to right, 
    rgba(245, 158, 11, 0.12), 
    rgba(9, 12, 16, 0.75));
  color: #f1f3f6;
}

.markdown-body .danger,
.markdown-body .caution {
  border-left-color: #ef4444;
  border-right-color: rgba(239, 68, 68, 0.4);
  border-top-color: rgba(239, 68, 68, 0.4);
  border-bottom-color: rgba(239, 68, 68, 0.4);
  background: linear-gradient(to right, 
    rgba(239, 68, 68, 0.15), 
    rgba(27, 31, 35, 0.6));
  color: #f1f3f6;
}

/* 容器内段落和列表样式 */
.markdown-body .tip p,
.markdown-body .note p,
.markdown-body .info p,
.markdown-body .warning p,
.markdown-body .danger p,
.markdown-body .caution p {
  margin: 0.8em 0;
}

.markdown-body .tip p:first-child,
.markdown-body .note p:first-child,
.markdown-body .info p:first-child,
.markdown-body .warning p:first-child,
.markdown-body .danger p:first-child,
.markdown-body .caution p:first-child {
  margin-top: 0;
}

.markdown-body .tip p:last-child,
.markdown-body .note p:last-child,
.markdown-body .info p:last-child,
.markdown-body .warning p:last-child,
.markdown-body .danger p:last-child,
.markdown-body .caution p:last-child {
  margin-bottom: 0;
}

/* 优化列表 - 缩进明确，支持嵌套，暗色背景高对比度 */
.markdown-body ul,
.markdown-body ol {
  padding-left: 2em;
  margin: 1.2em 0;
  line-height: 1.5;
  color: #f1f3f6;
}

.markdown-body li {
  margin: 0.6em 0;
  line-height: 1.8;
  color: #f1f3f6;
}

.markdown-body ul li::marker {
  color: #FF9EC8;
}

.markdown-body ol li::marker {
  color: #FF9EC8;
  font-weight: 600;
}

.markdown-body li > ul,
.markdown-body li > ol {
  margin-top: 0.5em;
  margin-bottom: 0.5em;
}

/* 任务列表可点击 */
.markdown-body .task-list-item {
  list-style-type: none;
  margin-left: -1.5em;
}

.markdown-body .task-list-item input[type="checkbox"] {
  margin-right: 0.5em;
  cursor: pointer;
  width: 16px;
  height: 16px;
}

/* 优化链接 - 暗色背景，保留 PinkFairy 主题，高对比度 */
.markdown-body a {
  color: #ffaaff;
  text-decoration: underline;
  text-decoration-color: rgba(255, 158, 200, 0.5);
  text-underline-offset: 2px;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  font-weight: 600;
  position: relative;
}

.markdown-body a::before {
  content: '';
  position: absolute;
  bottom: -1px;
  left: 0;
  width: 0;
  height: 2px;
  background: linear-gradient(to right, #FF77CC, #FF9EC8);
  transition: width 0.3s ease;
  z-index: -1;
}

.markdown-body a:hover {
  color: #FF77CC;
  text-decoration-color: transparent;
  text-decoration-thickness: 2px;
  transform: translateY(-1px);
  text-shadow: 0 2px 6px rgba(255, 119, 204, 0.4);
}

.markdown-body a:hover::before {
  width: 100%;
}

/* 内部链接高亮 */
.markdown-body a.internal-link {
  color: #FF77CC;
  font-weight: 700;
  border-bottom: 2px dashed rgba(255, 158, 200, 0.5);
  text-decoration: none;
  padding-bottom: 2px;
  transition: all 0.3s ease;
}

.markdown-body a.internal-link::before {
  display: none;
}

.markdown-body a.internal-link:hover {
  color: #FF9EC8;
  border-bottom-color: rgba(255, 119, 204, 0.8);
  border-bottom-style: solid;
  transform: translateY(-1px);
  text-shadow: 0 2px 6px rgba(255, 119, 204, 0.4);
}

/* 外部链接带图标 */
.markdown-body a.external-link {
  position: relative;
  padding-right: 16px;
}

.markdown-body a.external-link .external-link-icon {
  position: absolute;
  right: 0;
  top: 0;
  font-size: 0.85em;
  opacity: 0.6;
  transition: opacity 0.2s;
}

.markdown-body a.external-link:hover .external-link-icon {
  opacity: 1;
}

/* 优化图片 - 暗色背景，保留 PinkFairy 主题，高对比度 */
.markdown-body img {
  max-width: 90%;
  height: auto;
  border-radius: 16px;
  box-shadow: 
    -4px 0 16px rgba(255, 158, 200, 0.3),
    0 8px 24px rgba(255, 119, 204, 0.25);
  margin: 2em auto;
  display: block;
  object-fit: contain;
  border: 2px solid rgba(255, 158, 200, 0.4);
  transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  background: rgba(9, 12, 16, 0.8);
  padding: 4px;
}

.markdown-body img:hover {
  transform: scale(1.02) translateY(-3px);
  box-shadow: 
    -6px 0 24px rgba(255, 158, 200, 0.4),
    0 12px 32px rgba(255, 119, 204, 0.35);
  border-color: rgba(255, 119, 204, 0.6);
  filter: brightness(1.05);
}

/* 优化水平线 */
.markdown-body hr {
  border: none;
  border-top: 2px solid;
  border-image: linear-gradient(to right, transparent, rgba(249, 130, 108, 0.5), transparent) 1;
  margin: 3em 0;
  height: 0;
  position: relative;
}

.markdown-body hr::after {
  content: '';
  position: absolute;
  top: -1px;
  left: 50%;
  transform: translateX(-50%);
  width: 60px;
  height: 2px;
  background: rgba(249, 130, 108, 0.8);
  border-radius: 2px;
}

/* 折叠内容块 */
.markdown-body .collapsible-block {
  margin: 1.5em 0;
  border: 1px solid rgba(255, 158, 200, 0.3);
  border-radius: 8px;
  overflow: hidden;
  background: rgba(9, 12, 16, 0.85);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
}

.markdown-body .collapsible-header {
  padding: 12px 16px;
  background: rgba(9, 12, 16, 0.9);
  color: #f1f3f6;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  user-select: none;
  transition: all 0.3s ease;
  border-bottom: 1px solid rgba(255, 158, 200, 0.2);
}

.markdown-body .collapsible-header:hover {
  background: rgba(255, 158, 200, 0.12);
  color: #FF9EC8;
}

.markdown-body .collapsible-icon {
  font-size: 12px;
  transition: transform 0.2s;
}

.markdown-body .collapsible-block.collapsed .collapsible-icon {
  transform: rotate(-90deg);
}

.markdown-body .collapsible-content {
  padding: 16px;
  background: rgba(9, 12, 16, 0.9);
}

/* 自定义按钮和标签 */
.markdown-body .wiki-button {
  display: inline-block;
  padding: 8px 16px;
  background: rgba(255, 119, 204, 0.3);
  color: #FF9EC8;
  border: 1px solid rgba(255, 158, 200, 0.4);
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.2s;
  text-decoration: none;
}

.markdown-body .wiki-button:hover {
  background: rgba(255, 119, 204, 0.4);
  color: #FF77CC;
  border-color: rgba(255, 119, 204, 0.6);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(255, 119, 204, 0.3);
}

.markdown-body .wiki-tag {
  display: inline-block;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
  margin: 0 4px;
}

.markdown-body .wiki-tag-info {
  background: rgba(77, 163, 255, 0.2);
  color: #4da3ff;
  border: 1px solid rgba(77, 163, 255, 0.3);
}

.markdown-body .wiki-tag-success {
  background: rgba(16, 185, 129, 0.2);
  color: #10b981;
  border: 1px solid rgba(16, 185, 129, 0.3);
}

.markdown-body .wiki-tag-warning {
  background: rgba(245, 158, 11, 0.2);
  color: #f59e0b;
  border: 1px solid rgba(245, 158, 11, 0.3);
}

.markdown-body .wiki-tag-danger {
  background: rgba(239, 68, 68, 0.2);
  color: #ef4444;
  border: 1px solid rgba(239, 68, 68, 0.3);
}

/* 优化任务列表 */
.markdown-body .task-list-item {
  list-style-type: none;
  margin-left: -1.5em;
}

/* 脚注样式 */
.markdown-body .footnote-ref {
  font-size: 0.8em;
  vertical-align: super;
  margin-left: 2px;
}

.markdown-body .footnote-ref a {
  color: #5e81ac;
  text-decoration: none;
  background: rgba(94, 129, 172, 0.2);
  padding: 1px 4px;
  border-radius: 2px;
}

.markdown-body .footnote-ref a:hover {
  background: rgba(94, 129, 172, 0.4);
}

.markdown-body .footnotes {
  margin-top: 40px;
  padding-top: 20px;
  border-top: 1px solid rgba(255, 158, 200, 0.15);
}

.markdown-body .footnotes h4 {
  font-size: 1.1em;
  margin-bottom: 16px;
  color: #e5e7eb;
}

.markdown-body .footnote-item {
  margin-bottom: 12px;
  padding-left: 24px;
  position: relative;
  font-size: 0.9em;
  line-height: 1.6;
  color: rgba(229, 231, 235, 0.8);
}

.markdown-body .footnote-number {
  position: absolute;
  left: 0;
  color: #5e81ac;
  font-weight: 500;
}

.markdown-body .footnote-content {
  display: inline;
}

.markdown-body .footnote-backref {
  margin-left: 4px;
  color: #5e81ac;
  text-decoration: none;
  font-size: 0.9em;
}

.markdown-body .footnote-backref:hover {
  text-decoration: underline;
}

/* 代码块行号样式 */
.markdown-body pre.code-with-lines {
  position: relative;
  padding-left: 48px;
}

.markdown-body pre.code-with-lines .line-number {
  position: absolute;
  left: 0;
  width: 40px;
  padding-right: 8px;
  text-align: right;
  color: rgba(229, 231, 235, 0.4);
  user-select: none;
  font-size: 0.9em;
}

.markdown-body pre.code-with-lines .line-content {
  display: inline-block;
  width: 100%;
}

/* 下划线样式 */
.markdown-body u {
  text-decoration: underline;
  text-decoration-color: rgba(94, 129, 172, 0.6);
  text-underline-offset: 2px;
}

/* HTML 标签样式 */
.markdown-body kbd {
  display: inline-block;
  padding: 2px 6px;
  font-size: 0.85em;
  font-family: 'JetBrains Mono', monospace;
  color: #e5e7eb;
  background: rgba(110, 118, 129, 0.3);
  border: 1px solid rgba(110, 118, 129, 0.5);
  border-radius: 3px;
  box-shadow: 0 1px 0 rgba(0, 0, 0, 0.2);
}

.markdown-body mark {
  background: rgba(255, 193, 7, 0.25);
  color: #ffc107;
  padding: 2px 6px;
  border-radius: 3px;
  border: 1px solid rgba(255, 193, 7, 0.4);
  border-radius: 2px;
}

.markdown-body abbr {
  border-bottom: 1px dotted rgba(229, 231, 235, 0.5);
  cursor: help;
}

/* 删除线样式 - 适配暗黑主题和 PinkFairy 主题 */
.markdown-body del,
.markdown-body s {
  text-decoration: line-through;
  text-decoration-color: rgba(255, 158, 200, 0.4);
  color: rgba(241, 243, 246, 0.4);
  background: linear-gradient(to right,
    rgba(109, 106, 167, 0.15) 0%,
    rgba(9, 12, 16, 0.3) 100%);
  padding: 2px 4px;
  border-radius: 3px;
  border: 1px solid rgba(109, 106, 167, 0.3);
  transition: all 0.3s ease;
  position: relative;
}

.markdown-body del:hover,
.markdown-body s:hover {
  background: linear-gradient(to right,
    rgba(255, 238, 248, 0.2) 0%,
    rgba(255, 158, 200, 0.1) 100%);
  color: rgba(255, 158, 200, 0.7);
  border-color: rgba(255, 158, 200, 0.4);
  text-decoration-color: rgba(255, 158, 200, 0.6);
  border-style: dashed;
  transform: translateY(-1px);
  box-shadow: 0 2px 4px rgba(255, 158, 200, 0.2);
}

.markdown-body ins {
  text-decoration: underline;
  text-decoration-color: rgba(16, 185, 129, 0.6);
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
  padding: 2px 6px;
  border-radius: 3px;
  border: 1px solid rgba(16, 185, 129, 0.3);
}

.markdown-body sub,
.markdown-body sup {
  font-size: 0.75em;
  line-height: 0;
  position: relative;
  vertical-align: baseline;
}

.markdown-body sup {
  top: -0.5em;
}

.markdown-body sub {
  bottom: -0.25em;
}

/* 已移除代码主题选择器、行号切换、字体大小控制和高对比切换的 UI */

/* 优化滚动条样式 - 暗色背景，粉色主题 */
.wiki-sidebar::-webkit-scrollbar {
  width: 10px;
}

.wiki-sidebar::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.3);
  border-radius: 5px;
}

.wiki-sidebar::-webkit-scrollbar-thumb {
  background: linear-gradient(to bottom,
    rgba(255, 158, 200, 0.6) 0%,
    rgba(255, 119, 204, 0.6) 100%);
  border-radius: 5px;
  border: 1px solid rgba(0, 0, 0, 0.3);
}

.wiki-sidebar::-webkit-scrollbar-thumb:hover {
  background: linear-gradient(to bottom,
    rgba(255, 119, 204, 0.8) 0%,
    rgba(255, 158, 200, 0.8) 100%);
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


/* Mermaid 图表样式 - 确保文字可见（补充样式） */
.markdown-body .mermaid {
  background: var(--bg-color, #090c10);
  color: var(--text-color, #f1f3f6);
  margin: 1.5em 0;
  text-align: center;
  padding: 16px;
  border-radius: 8px;
  border: 1px solid var(--color-border-secondary, #79828e);
}

/* Mermaid SVG 文字样式 */
.markdown-body .mermaid svg {
  max-width: 100%;
  height: auto;
}

.markdown-body .mermaid svg text {
  fill: var(--text-color, #f1f3f6) !important;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Cantarell", "Fira Sans", "Droid Sans", "Helvetica Neue", Arial, sans-serif;
  font-size: 14px;
}

/* Mermaid 节点文字 */
.markdown-body .mermaid .nodeLabel,
.markdown-body .mermaid .edgeLabel,
.markdown-body .mermaid .cluster-label {
  color: var(--text-color, #f1f3f6) !important;
  fill: var(--text-color, #f1f3f6) !important;
}

/* Mermaid 节点背景 */
.markdown-body .mermaid .node rect,
.markdown-body .mermaid .node circle,
.markdown-body .mermaid .node ellipse,
.markdown-body .mermaid .node polygon {
  fill: var(--item-hover-bg-color, #272b33) !important;
  stroke: var(--color-border-secondary, #79828e) !important;
}

/* Mermaid 连接线 */
.markdown-body .mermaid .edgePath path {
  stroke: var(--primary-color, #f9826c) !important;
}

.markdown-body .mermaid .arrowheadPath {
  fill: var(--primary-color, #f9826c) !important;
}

/* KaTeX 公式样式优化 */
.markdown-body .katex {
  font-size: 1.1em;
  color: var(--text-color, #f1f3f6);
}

.markdown-body .katex-display {
  display: block;
  margin: 2em auto;
  padding: 1em 1.5em;
  background: rgba(9, 12, 16, 0.85);
  border-radius: 12px;
  border: 1px solid rgba(255, 158, 200, 0.3);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.6), inset 0 1px 0 rgba(255, 158, 200, 0.1);
  backdrop-filter: blur(10px);
  overflow-x: auto;
  overflow-y: hidden;
  text-align: center;
  position: relative;
}

.markdown-body .katex-display::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 4px;
  background: linear-gradient(to bottom, rgba(249, 130, 108, 0.6), rgba(77, 163, 255, 0.6));
  border-radius: 12px 0 0 12px;
}

.markdown-body .katex-display > .katex {
  display: inline-block;
  text-align: center;
  margin: 0;
  padding: 0.5em 0;
}

.markdown-body .katex-inline {
  display: inline-block;
  margin: 0 0.2em;
  padding: 0.2em 0.4em;
  background: rgba(39, 43, 51, 0.4);
  border-radius: 4px;
  vertical-align: baseline;
  line-height: 1.6;
}

.markdown-body .katex .base {
  color: var(--text-color, #f1f3f6);
}

.markdown-body .katex .mord {
  color: var(--text-color, #f1f3f6);
}

.markdown-body .katex .mop {
  color: rgba(249, 130, 108, 0.9);
}

.markdown-body .katex .mbin {
  color: rgba(77, 163, 255, 0.9);
}

.markdown-body .katex .mrel {
  color: rgba(249, 130, 108, 0.9);
}

.markdown-body .katex .mopen,
.markdown-body .katex .mclose {
  color: rgba(229, 231, 235, 0.8);
}

.markdown-body .katex .mfrac {
  border-color: rgba(121, 130, 142, 0.5);
}

.markdown-body .katex .sqrt > .vlist-t {
  border-color: rgba(121, 130, 142, 0.5);
}

/* 确保 KaTeX 公式在暗色主题下可见 */
.markdown-body .katex .mathnormal {
  color: var(--text-color, #f1f3f6);
}

.markdown-body .katex .mathit {
  color: rgba(249, 130, 108, 0.9);
  font-style: italic;
}

.markdown-body .katex .mathbf {
  color: var(--text-color, #f1f3f6);
  font-weight: 700;
}

.markdown-body .katex .boldsymbol {
  color: rgba(249, 130, 108, 0.9);
  font-weight: 700;
}

/* 统一使用暗色主题，移除 prefers-color-scheme 媒体查询 */
/* 所有样式已在上面统一定义为暗色主题（#020617 背景） */
</style>

