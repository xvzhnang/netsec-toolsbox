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
import { getTauriInvoke } from '../utils/tauri'
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
  // 调试代码已注释
  // console.log('========== WikiView props.filePath 变化 ==========')
  // console.log('WikiView props.filePath 变化:', { 
  //   newFilePath, 
  //   current: currentFilePath.value,
  //   isInitialLoad: isInitialLoad.value,
  //   loading: loading.value,
  //   contentHtmlLength: contentHtml.value.length,
  //   propsToolId: props.toolId,
  //   propsToolName: props.toolName,
  //   isMounted: isMounted.value
  // })
  debug('WikiView props.filePath 变化')
  // 注意：undefined 和空字符串都表示未指定，需要区分
  const normalizedNew = newFilePath !== undefined ? newFilePath : undefined
  // console.log('规范化后的路径:', { normalizedNew, current: currentFilePath.value, 是否相同: normalizedNew === currentFilePath.value })
  if (normalizedNew !== currentFilePath.value) {
    // console.log('路径不同，更新 currentFilePath')
    currentFilePath.value = normalizedNew
    // 当 props 变化时，重置初始加载状态，确保显示加载状态
    // 如果内容为空，说明是新的加载，应该显示加载页面
    if (contentHtml.value === '' || normalizedNew !== undefined) {
      isInitialLoad.value = true
      loading.value = true
      error.value = null
      // console.log('重置加载状态: isInitialLoad=true, loading=true')
    }
    // console.log('触发 loadWikiContent，因为 filePath 变化:', { 
    //   from: currentFilePath.value, 
    //   to: normalizedNew,
    //   isInitialLoad: isInitialLoad.value,
    //   loading: loading.value
    // })
    debug('触发 loadWikiContent，因为 filePath 变化')
    // 使用 nextTick 确保状态更新后再加载
    nextTick(() => {
      // console.log('nextTick 中调用 loadWikiContent')
      loadWikiContent()
    })
  } else {
    // console.log('路径相同，不触发加载')
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

// 监听路由变化
watch(() => route.query, (newQuery) => {
  const queryFilePath = newQuery.filePath as string | undefined
  const queryToolId = newQuery.toolId as string | undefined
  const queryToolName = newQuery.toolName as string | undefined
  
  debug('WikiView 路由查询参数变化:', { queryFilePath, queryToolId, queryToolName, propsFilePath: props.filePath })
  
  // 优先使用路由参数，如果没有则使用 props
  currentFilePath.value = queryFilePath !== undefined ? queryFilePath : props.filePath
  currentToolId.value = queryToolId !== undefined && queryToolId !== '' ? queryToolId : (props.toolId !== undefined && props.toolId !== '' ? props.toolId : undefined)
  currentToolName.value = queryToolName !== undefined && queryToolName !== '' ? queryToolName : (props.toolName !== undefined && props.toolName !== '' ? props.toolName : undefined)
  
  debug('更新后的值:', { currentFilePath: currentFilePath.value, currentToolId: currentToolId.value, currentToolName: currentToolName.value })
  
  // 重新加载 Wiki 内容（只要有 filePath 或 toolId）
  if (currentFilePath.value !== undefined || currentToolId.value) {
    loadWikiContent()
  }
}, { deep: true })


// 加载 Wiki 文件（纯前端渲染）
const loadWikiFile = async (filePath: string) => {
  // 调试代码已注释
  // console.log('========== loadWikiFile 开始 ==========')
  // console.log('loadWikiFile 被调用:', { 
  //   filePath,
  //   currentFilePath: currentFilePath.value,
  //   loading: loading.value,
  //   isInitialLoad: isInitialLoad.value,
  //   contentHtmlLength: contentHtml.value.length,
  //   isMounted: isMounted.value
  // })
  
  // 如果正在加载相同文件，直接返回
  // 但是，如果内容为空（初始加载），即使路径相同也要加载
  const isSameFile = currentFilePath.value === filePath
  const hasContent = contentHtml.value !== ''
  const shouldSkip = loading.value && isSameFile && hasContent
  // console.log('检查是否正在加载相同文件:', { 
  //   loading: loading.value, 
  //   currentFilePath: currentFilePath.value, 
  //   filePath, 
  //   isSameFile,
  //   hasContent,
  //   shouldSkip
  // })
  if (shouldSkip) {
    // console.log('正在加载相同文件且有内容，跳过:', filePath)
    debug('正在加载相同文件且有内容，跳过')
    return
  }
  // console.log('继续加载文件（初始加载或不同文件）')
  
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
    console.log('========== 调用 Tauri read_wiki_file ==========')
    console.log('正在读取 Wiki 文件:', filePath)
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
    const html = renderMarkdown(markdownText, filePath)
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
      // console.log('开始应用代码高亮...')
      await applyCodeHighlighting(element)
      // console.log('代码高亮完成')
      
      // 渲染 Mermaid 图表
      // console.log('开始渲染 Mermaid 图表...')
      await renderMermaidCharts(element)
      // console.log('Mermaid 图表渲染完成')
      
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
      // console.log('开始渲染 KaTeX 公式...')
      renderKaTeX(element)
      // console.log('KaTeX 公式渲染完成')
      
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
  const codeBlocks = container.querySelectorAll('pre code:not(.hljs)')
  // console.log('找到未高亮的代码块数量:', codeBlocks.length)
  
  codeBlocks.forEach((codeElement, index) => {
    try {
      // highlightElement 会自动识别 language-xxx 类并应用高亮
      // 如果没有 language-xxx 类，会自动检测语言
      // console.log(`处理代码块 ${index}:`, { 
      //   className: codeElement.className,
      //   hasLanguageClass: /language-\w+/.test(codeElement.className)
      // })
      hljs.highlightElement(codeElement as HTMLElement)
      // console.log(`代码块 ${index} 高亮完成`)
    } catch (err) {
      // console.error(`代码块 ${index} 高亮失败:`, err)
      debug(`代码块 ${index} 高亮失败`)
    }
  })
  
  console.log('所有代码块处理完成')
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

// 加载内置主题 CSS
const loadBuiltinTheme = () => {
  // 内置主题 CSS（GitHub 风格 - 完整版）
  const builtinThemeCSS = `
/* JetBrains Mono 字体声明 */
@font-face {
    font-family: 'JetBrains Mono';
    src: url('/fonts/fonts/webfonts/JetBrainsMono-Regular.woff2') format('woff2'),
         url('/fonts/fonts/ttf/JetBrainsMono-Regular.ttf') format('truetype');
    font-weight: 400;
    font-style: normal;
    font-display: swap;
}

@font-face {
    font-family: 'JetBrains Mono';
    src: url('/fonts/fonts/webfonts/JetBrainsMono-Bold.woff2') format('woff2'),
         url('/fonts/fonts/ttf/JetBrainsMono-Bold.ttf') format('truetype');
    font-weight: 700;
    font-style: normal;
    font-display: swap;
}

@font-face {
    font-family: 'JetBrains Mono';
    src: url('/fonts/fonts/webfonts/JetBrainsMono-Italic.woff2') format('woff2'),
         url('/fonts/fonts/ttf/JetBrainsMono-Italic.ttf') format('truetype');
    font-weight: 400;
    font-style: italic;
    font-display: swap;
}

@font-face {
    font-family: 'JetBrains Mono';
    src: url('/fonts/fonts/webfonts/JetBrainsMono-BoldItalic.woff2') format('woff2'),
         url('/fonts/fonts/ttf/JetBrainsMono-BoldItalic.ttf') format('truetype');
    font-weight: 700;
    font-style: italic;
    font-display: swap;
}

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
    color: #e5e7eb;
    line-height: 1.6;
    background: #020617;
}

/* 英文和代码使用 JetBrains Mono */
.markdown-body :lang(en),
.markdown-body code,
.markdown-body pre,
.markdown-body kbd,
.markdown-body samp {
    font-family: 'JetBrains Mono', 'SF Mono', 'Monaco', 'Inconsolata', 'Fira Code', 'Droid Sans Mono', 'Source Code Pro', monospace;
}

/* 适配我们的 HTML 结构：将 #write 改为 .markdown-body */
/* 注意：此样式已被后面的 .markdown-body 样式覆盖，保留仅用于兼容 */

.markdown-body > ul:first-child,
.markdown-body > ol:first-child {
    margin-top: 30px;
}

a {
    color: #4da3ff;
    text-decoration: none;
    transition: color 0.2s ease;
}

a:hover {
    color: #6bb3ff;
    text-decoration: underline;
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
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    color: #e5e7eb;
    padding-bottom: 0.3em;
    margin-top: 1.5em;
    margin-bottom: 1em;
}

h2 {
    font-size: 1.75em;
    line-height: 1.225;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    color: #e5e7eb;
    padding-bottom: 0.3em;
    margin-top: 1.3em;
    margin-bottom: 0.8em;
}

h3 {
    font-size: 1.5em;
    line-height: 1.43;
    color: #e5e7eb;
    margin-top: 1.2em;
    margin-bottom: 0.7em;
}

h4 {
    font-size: 1.25em;
    color: #d1d5db;
    margin-top: 1em;
    margin-bottom: 0.6em;
}

h5 {
    font-size: 1em;
    color: #d1d5db;
    margin-top: 0.9em;
    margin-bottom: 0.5em;
}

h6 {
    font-size: 1em;
    color: rgba(229, 231, 235, 0.7);
    margin-top: 0.8em;
    margin-bottom: 0.4em;
}

p, blockquote, ul, ol, dl, table {
    margin: 0.8em 0;
}

li>ol, li>ul {
    margin: 0 0;
}

hr {
    height: 1px;
    padding: 0;
    margin: 24px 0;
    background: linear-gradient(to right, transparent, rgba(255, 255, 255, 0.2), transparent);
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
    border-left: 4px solid rgba(77, 163, 255, 0.5);
    color: rgba(229, 231, 235, 0.8);
    padding: 0 1em;
    margin: 1.5em 0;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 0 6px 6px 0;
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
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    overflow: hidden;
}

table tr {
    border: 1px solid rgba(255, 255, 255, 0.1);
    margin: 0;
    padding: 0;
}

table tr:nth-child(2n), thead {
    background-color: rgba(0, 0, 0, 0.2);
}

table th {
    font-weight: bold;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-bottom: 0;
    margin: 0;
    padding: 6px 13px;
    color: #e5e7eb;
    background: rgba(0, 0, 0, 0.3);
}

table td {
    border: 1px solid rgba(255, 255, 255, 0.1);
    margin: 0;
    padding: 6px 13px;
    color: rgba(229, 231, 235, 0.9);
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

/* macOS 风格代码框 */
.md-fences, code, tt {
    font-size: 0.9em;
}

code {
    background-color: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 3px;
    padding: 0.2em 0.4em;
    font-family: 'JetBrains Mono', 'SF Mono', 'Monaco', 'Inconsolata', 'Fira Code', 'Droid Sans Mono', 'Source Code Pro', monospace;
    font-size: 85%;
    color: #e5e7eb;
}

.md-fences {
    margin-bottom: 15px;
    margin-top: 15px;
    padding: 0;
    border-radius: 8px;
    overflow: hidden;
    background: #1e1e1e;
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.md-fences pre {
    margin: 0;
    padding: 16px;
    background: #1e1e1e;
    overflow-x: auto;
    font-family: 'JetBrains Mono', 'SF Mono', 'Monaco', 'Inconsolata', 'Fira Code', 'Droid Sans Mono', 'Source Code Pro', monospace;
    font-size: 13px;
    line-height: 1.6;
    color: #d4d4d4;
}

.md-fences pre code {
    background: transparent;
    padding: 0;
    border-radius: 0;
    color: inherit;
    font-size: inherit;
    font-family: inherit;
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

/* GitHub 暗色风格代码块 - 基础样式 */
/* 代码块样式已在下方的 highlight.js 代码高亮样式部分统一定义，此处移除重复 */

/* highlight.js 代码高亮样式 - GitHub 暗色风格 */
/* 统一代码块样式 - 只有一个框，不分内外 */
.markdown-body pre {
    background: #161b22;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 16px;
    overflow-x: auto;
    margin: 1em 0;
    position: relative;
}

/* code 元素样式 - 与 pre 融为一体，不分内外框 */
.markdown-body pre code {
    background: transparent !important;
    border: none !important;
    padding: 0;
    margin: 0;
    color: #e5e7eb;
    font-family: 'JetBrains Mono', 'SF Mono', 'Monaco', 'Inconsolata', 'Fira Code', 'Droid Sans Mono', 'Source Code Pro', monospace !important;
    font-size: 14px;
    line-height: 1.6;
    display: block;
}

/* 确保代码高亮正常工作 - hljs 类应用在 code 元素上 */
.markdown-body pre code.hljs {
    background: transparent !important;
    border: none !important;
    padding: 0;
    margin: 0;
    color: #e5e7eb;
    display: block;
    overflow-x: auto;
    font-family: 'JetBrains Mono', 'SF Mono', 'Monaco', 'Inconsolata', 'Fira Code', 'Droid Sans Mono', 'Source Code Pro', monospace !important;
}

/* 行内代码样式 */
.markdown-body code:not(pre code) {
    background: rgba(110, 118, 129, 0.2);
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 0.9em;
    color: #e5e7eb;
    font-family: 'JetBrains Mono', 'SF Mono', 'Monaco', 'Inconsolata', 'Fira Code', 'Droid Sans Mono', 'Source Code Pro', monospace;
}

/* GitHub 暗色风格代码高亮颜色 */
.markdown-body .hljs {
    color: #e5e7eb;
}

.markdown-body .hljs-keyword,
.markdown-body .hljs-selector-tag,
.markdown-body .hljs-literal,
.markdown-body .hljs-doctag,
.markdown-body .hljs-title,
.markdown-body .hljs-section,
.markdown-body .hljs-type,
.markdown-body .hljs-name,
.markdown-body .hljs-strong {
    font-weight: bold;
    color: #ff7b72;
}

.markdown-body .hljs-string,
.markdown-body .hljs-attr,
.markdown-body .hljs-attribute,
.markdown-body .hljs-symbol,
.markdown-body .hljs-bullet,
.markdown-body .hljs-addition,
.markdown-body .hljs-variable,
.markdown-body .hljs-template-tag,
.markdown-body .hljs-template-variable {
    color: #a5d6ff;
}

.markdown-body .hljs-comment,
.markdown-body .hljs-quote,
.markdown-body .hljs-deletion,
.markdown-body .hljs-meta {
    color: #8b949e;
}

.markdown-body .hljs-number {
    color: #79c0ff;
}

.markdown-body .hljs-function,
.markdown-body .hljs-title.function_ {
    color: #d2a8ff;
}

.markdown-body .hljs-params {
    color: #c9d1d9;
}

.markdown-body .hljs-emphasis {
    font-style: italic;
}

.markdown-body .hljs-built_in,
.markdown-body .hljs-class {
    color: #ffa657;
}

.markdown-body .hljs-tag,
.markdown-body .hljs-name {
    color: #7ee787;
}

/* 代码块 position 已在上面统一定义，此处移除重复 */

/* 代码块复制按钮样式 */

.code-copy-button {
    position: absolute;
    top: 8px;
    right: 8px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    color: #e5e7eb;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s ease;
    z-index: 10;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
    opacity: 0;
    pointer-events: none;
    backdrop-filter: blur(10px);
}

.markdown-body pre:hover .code-copy-button {
    opacity: 1;
    pointer-events: all;
}

.code-copy-button:hover {
    background: rgba(255, 255, 255, 0.15);
    border-color: rgba(255, 255, 255, 0.25);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.code-copy-button:active {
    transform: translateY(0);
}

.code-copy-button.copied {
    background: rgba(56, 139, 253, 0.2);
    border-color: rgba(56, 139, 253, 0.4);
    color: #58a6ff;
}

.code-copy-button .copy-icon {
    font-size: 14px;
    line-height: 1;
}

.code-copy-button .copy-text {
    font-weight: 500;
}

/* 移除浅色主题样式，统一使用暗色主题 */
.markdown-body pre.md-meta-block {
    padding: 1rem;
    font-size: 85%;
    line-height: 1.45;
    background-color: #161b22;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    color: rgba(229, 231, 235, 0.8);
    margin-top: 0 !important;
}

.mathjax-block>.code-tooltip {
    bottom: .375rem;
}

.md-mathjax-midline {
    background: rgba(0, 0, 0, 0.2);
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
    }
    // 如果是内部链接（已由 markdown.ts 处理）
    else if (link.classList.contains('wiki-internal-link')) {
      link.classList.add('internal-link')
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
        // 构建后的样式文件在 build/demo/styles/ 目录下，或者使用 src/styles/
        const cssPaths = [
          '/highlight.js-11.11.1/build/demo/styles/github-dark.css',
          '/highlight.js-11.11.1/src/styles/github-dark.css',
          '/highlight.js-11.11.1/src/styles/github-dark.min.css',
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

// 渲染 KaTeX 数学公式
const renderKaTeX = async (container: HTMLElement) => {
  // 动态加载 KaTeX
  if (typeof (window as any).katex === 'undefined') {
    const script = document.createElement('script')
    script.src = 'https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js'
    script.onload = () => {
      const link = document.createElement('link')
      link.rel = 'stylesheet'
      link.href = 'https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css'
      document.head.appendChild(link)
      renderKaTeXFormulas(container)
    }
    document.head.appendChild(script)
  } else {
    renderKaTeXFormulas(container)
  }
}

// 渲染 KaTeX 公式
const renderKaTeXFormulas = (container: HTMLElement) => {
  const katex = (window as any).katex
  if (!katex) return
  
  // 渲染块级公式
  const blockFormulas = container.querySelectorAll('.katex-block')
  blockFormulas.forEach((el) => {
    const formula = (el as HTMLElement).dataset.formula
    if (formula && formula.trim()) {
      try {
        // 验证公式是否有效（不包含代码块标记）
        if (formula.includes('<code') || formula.includes('</code>') || formula.includes('<pre')) {
          return
        }
        katex.render(formula, el as HTMLElement, { displayMode: true, throwOnError: false })
      } catch (e) {
        // 静默失败，不输出错误（可能是误识别的非数学公式）
        console.debug('KaTeX 渲染跳过（可能是误识别）:', formula.substring(0, 20))
      }
    }
  })
  
  // 渲染行内公式
  const inlineFormulas = container.querySelectorAll('.katex-inline')
  inlineFormulas.forEach((el) => {
    const formula = (el as HTMLElement).dataset.formula
    if (formula && formula.trim()) {
      try {
        // 验证公式是否有效（不包含代码块标记）
        if (formula.includes('<code') || formula.includes('</code>') || formula.includes('<pre')) {
          return
        }
        // 验证是否看起来像数学公式（包含数学符号）
        const hasMathSymbols = /[+\-*/=()\[\]{},.^_\\]/.test(formula)
        if (!hasMathSymbols && formula.length < 3) {
          return // 太短且没有数学符号，可能是误识别
        }
        katex.render(formula, el as HTMLElement, { displayMode: false, throwOnError: false })
      } catch (e) {
        // 静默失败，不输出错误（可能是误识别的非数学公式）
        console.debug('KaTeX 渲染跳过（可能是误识别）:', formula.substring(0, 20))
      }
    }
  })
}

// 已移除终端样式和行号功能，使用普通 GitHub 暗色样式

// 为代码块添加复制按钮（普通 GitHub 样式）
const addCopyButtonsToCodeBlocks = (container: HTMLElement) => {
  // console.log('addCopyButtonsToCodeBlocks 开始:', { containerTag: container.tagName })
  // 获取所有代码块
  const codeBlocks = container.querySelectorAll('pre code')
  // console.log('找到代码块数量:', codeBlocks.length)
  
  codeBlocks.forEach((codeElement, index) => {
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
  console.log('========== loadWikiContent 开始 ==========')
  console.log('loadWikiContent 被调用:', { 
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
  // console.log('设置 isMounted = true')
  
  // 重置状态，确保每次挂载时都是干净的状态
  // 如果内容为空，说明是新的加载，应该显示加载页面
  if (contentHtml.value === '') {
    isInitialLoad.value = true
    loading.value = true
    // console.log('设置初始加载状态: isInitialLoad=true, loading=true')
    debug('设置初始加载状态: isInitialLoad=true, loading=true')
  }
  
  // console.log('开始加载文件树...')
  await loadFileTree()
  // console.log('文件树加载完成')
  
  // console.log('加载内置主题...')
  loadBuiltinTheme()
  // console.log('内置主题加载完成')
  
  // 加载 highlight.js 并在页面加载后调用 highlightAll()
  // console.log('加载 highlight.js...')
  try {
    const hljs = await loadHighlightJS()
    if (hljs) {
      // console.log('highlight.js 加载成功，调用 highlightAll()')
      // 等待 DOM 完全渲染后调用 highlightAll()
      await nextTick()
      hljs.highlightAll()
      // console.log('highlightAll() 调用完成')
      debug('highlightAll() 调用完成')
    }
  } catch (err) {
    // console.error('加载 highlight.js 失败:', err)
    logError('加载 highlight.js 失败:', err)
  }
  
  // 只有在有 props 或路由参数时才加载内容
  // 如果没有，loadWikiContent 会加载首页
  // console.log('准备调用 loadWikiContent...')
  // console.log('当前状态:', {
  //   currentFilePath: currentFilePath.value,
  //   currentToolId: currentToolId.value,
  //   currentToolName: currentToolName.value,
  //   propsFilePath: props.filePath,
  //   propsToolId: props.toolId,
  //   propsToolName: props.toolName
  // })
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
  background: #020617;
  color: #e5e7eb;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
}

.wiki-loading,
.wiki-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 40px;
  min-height: 300px;
  background: #020617;
  color: #e5e7eb;
}

.spinner {
  width: 48px;
  height: 48px;
  border: 4px solid rgba(77, 163, 255, 0.2);
  border-top-color: #4da3ff;
  border-right-color: #4da3ff;
  border-radius: 50%;
  animation: spin 0.8s cubic-bezier(0.5, 0, 0.5, 1) infinite;
  margin-bottom: 24px;
  box-shadow: 0 2px 8px rgba(77, 163, 255, 0.3);
}

@keyframes spin {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}

.wiki-loading p {
  margin: 0;
  font-size: 15px;
  color: rgba(229, 231, 235, 0.9);
  font-weight: 500;
  letter-spacing: 0.3px;
}

.wiki-error {
  background: #020617;
}

.wiki-error p {
  color: #ff6b6b;
  margin-bottom: 24px;
  font-size: 16px;
  font-weight: 500;
  line-height: 1.6;
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
  width: 280px;
  background: linear-gradient(180deg, #1a1a1a 0%, #2d2d2d 100%);
  border-right: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.3);
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
  font-weight: 600;
  margin: 0;
  color: #e5e7eb;
  letter-spacing: 0.3px;
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

/* 搜索栏（文件导航上侧） */
.wiki-search-section {
  padding: 16px 20px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  flex-shrink: 0;
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
  color: rgba(229, 231, 235, 0.6);
  font-size: 14px;
  pointer-events: none;
  z-index: 1;
}

.wiki-search-wrapper input {
  width: 100%;
  padding: 10px 36px 10px 36px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 6px;
  font-size: 13px;
  background: rgba(255, 255, 255, 0.1);
  color: #e5e7eb;
  transition: all 0.2s;
  box-sizing: border-box;
}

.wiki-search-wrapper input::placeholder {
  color: rgba(229, 231, 235, 0.5);
}

.wiki-search-wrapper input:focus {
  outline: none;
  border-color: rgba(77, 163, 255, 0.6);
  background: rgba(255, 255, 255, 0.15);
  box-shadow: 0 0 0 3px rgba(77, 163, 255, 0.1);
}

.search-clear-btn {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  width: 20px;
  height: 20px;
  padding: 0;
  border: none;
  background: rgba(255, 255, 255, 0.2);
  color: rgba(229, 231, 235, 0.8);
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  line-height: 1;
  transition: all 0.2s;
  z-index: 2;
}

.search-clear-btn:hover {
  background: rgba(255, 255, 255, 0.3);
  color: #e5e7eb;
}

.search-results-dropdown {
  position: absolute;
  top: calc(100% + 8px);
  left: 0;
  right: 0;
  background: rgba(30, 30, 30, 0.98);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  max-height: 400px;
  overflow: hidden;
  z-index: 100;
  display: flex;
  flex-direction: column;
}

.search-results-header {
  padding: 10px 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  font-size: 12px;
  color: rgba(229, 231, 235, 0.7);
  font-weight: 500;
  background: rgba(0, 0, 0, 0.2);
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
  color: rgba(229, 231, 235, 0.9);
  text-decoration: none;
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border-radius: 6px;
  font-size: 13px;
  transition: all 0.15s;
  gap: 10px;
}

.search-result-item a:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #4da3ff;
  transform: translateX(2px);
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
  font-weight: 500;
  color: #e5e7eb;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-title :deep(.search-match) {
  background: rgba(255, 235, 59, 0.3);
  color: #ffeb3b;
  font-weight: 600;
  padding: 0 2px;
  border-radius: 2px;
}

.result-path {
  font-size: 11px;
  color: rgba(229, 231, 235, 0.6);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
  padding: 16px;
  overflow: hidden;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 8px;
  margin: 12px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
}

.wiki-file-tree h3 {
  font-size: 13px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: rgba(229, 231, 235, 0.9);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding-bottom: 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.15);
  flex-shrink: 0;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
}

.wiki-file-tree .wiki-tree-list {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding-right: 8px;
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
  background: #020617;
  position: relative;
  height: 100%;
  min-width: 0; /* 防止 flex 子元素溢出 */
  scroll-behavior: smooth;
}

/* 面包屑导航 */
.wiki-breadcrumb {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 20px;
  background: rgba(0, 0, 0, 0.2);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  font-size: 14px;
  box-shadow: 0 1px 0 rgba(0, 0, 0, 0.2);
}

.breadcrumb-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.05);
  cursor: pointer;
  transition: all 0.2s ease;
  flex-shrink: 0;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
}

.breadcrumb-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(77, 163, 255, 0.6);
  box-shadow: 0 2px 4px rgba(77, 163, 255, 0.2);
  transform: translateY(-1px);
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
  color: rgba(229, 231, 235, 0.5);
  margin: 0 4px;
  font-weight: 300;
}

.breadcrumb-link {
  color: #4da3ff;
  text-decoration: none;
  cursor: pointer;
  background: none;
  border: none;
  padding: 4px 8px;
  font-size: 14px;
  border-radius: 4px;
  transition: all 0.2s ease;
}

.breadcrumb-link:hover {
  color: #6bb3ff;
  background: rgba(255, 255, 255, 0.1);
  text-decoration: none;
}

.breadcrumb-current {
  color: rgba(229, 231, 235, 0.9);
  font-weight: 500;
  padding: 4px 8px;
}

.markdown-body {
  max-width: 860px;
  margin: 0 auto;
  padding: 48px 64px 120px;
  background: #020617;
  min-height: 100%;
  box-sizing: border-box;
  width: 100%;
  font-size: 16px;
  line-height: 1.8;
  color: #e5e7eb;
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

/* 优化段落间距和文字颜色 */
.markdown-body p {
  margin: 1.2em 0;
  line-height: 1.8;
  color: rgba(229, 231, 235, 0.9);
}

/* 美化整体显示 */
.wiki-view {
  background: #020617;
  color: #e5e7eb;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
}

.wiki-content {
  scroll-behavior: smooth;
  background: #020617;
}

/* 美化滚动条 */
.wiki-content::-webkit-scrollbar {
  width: 10px;
}

.wiki-content::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.2);
  border-radius: 5px;
}

.wiki-content::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 5px;
  border: 2px solid rgba(0, 0, 0, 0.2);
}

.wiki-content::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}

/* 优化表格显示 - 边框清晰，交替行背景 */
.markdown-body table {
  border-collapse: collapse;
  width: 100%;
  margin: 1.5em 0;
  overflow-x: auto;
  display: block;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.2);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.markdown-body table thead {
  background: rgba(0, 0, 0, 0.3);
}

.markdown-body table th {
  font-weight: 600;
  padding: 12px 16px;
  text-align: left;
  border-bottom: 2px solid rgba(255, 255, 255, 0.15);
  border-right: 1px solid rgba(255, 255, 255, 0.1);
  color: #e5e7eb;
}

.markdown-body table th:last-child {
  border-right: none;
}

.markdown-body table td {
  padding: 12px 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  border-right: 1px solid rgba(255, 255, 255, 0.1);
  color: rgba(229, 231, 235, 0.9);
}

.markdown-body table td:last-child {
  border-right: none;
}

.markdown-body table tbody tr:nth-child(even) {
  background: rgba(0, 0, 0, 0.15);
}

.markdown-body table tbody tr:hover {
  background: rgba(77, 163, 255, 0.15);
}

/* 优化引用块 - 卡片式 */
.markdown-body blockquote {
  border-left: 4px solid rgba(77, 163, 255, 0.6);
  padding: 16px 20px;
  color: rgba(229, 231, 235, 0.85);
  background: rgba(0, 0, 0, 0.2);
  border-radius: 0 8px 8px 0;
  margin: 2em 0;
  font-style: italic;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

/* 特殊引用块（警告、提示、注意事项） */
.markdown-body .admonition {
  margin: 1.5em 0;
  border-radius: 8px;
  border-left: 4px solid;
  padding: 12px 16px;
  background: rgba(0, 0, 0, 0.2);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.markdown-body .admonition-title {
  font-weight: 600;
  margin-bottom: 8px;
  font-size: 0.95em;
}

.markdown-body .admonition-content {
  line-height: 1.7;
}

.markdown-body .admonition-warning {
  border-left-color: #f59e0b;
  background: rgba(245, 158, 11, 0.15);
  color: #fbbf24;
}

.markdown-body .admonition-note,
.markdown-body .admonition-info {
  border-left-color: #3b82f6;
  background: rgba(59, 130, 246, 0.15);
  color: #60a5fa;
}

.markdown-body .admonition-tip {
  border-left-color: #10b981;
  background: rgba(16, 185, 129, 0.15);
  color: #34d399;
}

.markdown-body .admonition-caution {
  border-left-color: #ef4444;
  background: rgba(239, 68, 68, 0.15);
  color: #f87171;
}

/* 优化列表 - 缩进明确，支持嵌套 */
.markdown-body ul,
.markdown-body ol {
  padding-left: 2em;
  margin: 1.2em 0;
  line-height: 1.8;
  color: rgba(229, 231, 235, 0.9);
}

.markdown-body li {
  margin: 0.6em 0;
  line-height: 1.8;
  color: rgba(229, 231, 235, 0.9);
}

.markdown-body ul li::marker {
  color: rgba(77, 163, 255, 0.6);
}

.markdown-body ol li::marker {
  color: rgba(77, 163, 255, 0.6);
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

/* 优化链接 */
.markdown-body a {
  color: #4da3ff;
  text-decoration: none;
  transition: color 0.2s;
}

.markdown-body a:hover {
  color: #6bb3ff;
  text-decoration: underline;
}

/* 内部链接高亮 */
.markdown-body a.internal-link {
  color: #4da3ff;
  font-weight: 500;
  border-bottom: 1px dashed rgba(77, 163, 255, 0.6);
}

.markdown-body a.internal-link:hover {
  color: #6bb3ff;
  border-bottom-color: #6bb3ff;
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

/* 优化图片 - 保持原始宽高比例或自适应宽度 */
.markdown-body img {
  max-width: 100%;
  height: auto;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  margin: 1.5em 0;
  display: block;
  object-fit: contain;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

/* 优化水平线 */
.markdown-body hr {
  border: none;
  border-top: 1px solid rgba(255, 255, 255, 0.2);
  margin: 2.5em 0;
  height: 0;
  background: linear-gradient(to right, transparent, rgba(255, 255, 255, 0.2), transparent);
}

/* 折叠内容块 */
.markdown-body .collapsible-block {
  margin: 1.5em 0;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.2);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.markdown-body .collapsible-header {
  padding: 12px 16px;
  background: rgba(0, 0, 0, 0.3);
  color: #e5e7eb;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  user-select: none;
  transition: background 0.2s;
}

.markdown-body .collapsible-header:hover {
  background: rgba(77, 163, 255, 0.15);
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
  background: #ffffff;
}

/* 自定义按钮和标签 */
.markdown-body .wiki-button {
  display: inline-block;
  padding: 8px 16px;
  background: #0969da;
  color: #ffffff;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.2s;
  text-decoration: none;
}

.markdown-body .wiki-button:hover {
  background: #0550ae;
  transform: translateY(-1px);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
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
  background: #dbeafe;
  color: #1e40af;
}

.markdown-body .wiki-tag-success {
  background: #d1fae5;
  color: #065f46;
}

.markdown-body .wiki-tag-warning {
  background: #fef3c7;
  color: #92400e;
}

.markdown-body .wiki-tag-danger {
  background: #fee2e2;
  color: #991b1b;
}

/* 优化任务列表 */
.markdown-body .task-list-item {
  list-style-type: none;
  margin-left: -1.5em;
}

/* 已移除代码主题选择器、行号切换、字体大小控制和高对比切换的 UI */

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


/* 统一使用暗色主题，移除 prefers-color-scheme 媒体查询 */
/* 所有样式已在上面统一定义为暗色主题（#020617 背景） */
</style>

