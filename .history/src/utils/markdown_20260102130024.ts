/**
 * Markdown 渲染工具
 * 使用 markdown-it 及其插件进行渲染
 * 为 AI 预留接口：AI 可以生成 Markdown，直接使用此函数渲染
 */

import mermaid from 'mermaid'
import { debug, warn, error } from './logger'

// markdown-it 将从 public 目录动态加载（打包后的 bundle）

// markdown-it 实例（动态加载后设置）
let mdInstance: any = null

// Debug 开关：控制是否输出详细日志
// 生产环境可以设置为 false，减少控制台输出
const DEBUG = false

// 调试日志函数
const debugLog = (...args: any[]) => {
  if (DEBUG) {
    debug('[Markdown]', ...args)
  }
}

const debugWarn = (...args: any[]) => {
  if (DEBUG) {
    warn('[Markdown]', ...args)
  }
}

const debugError = (...args: any[]) => {
  // 错误始终输出，不受 DEBUG 控制
  error('[Markdown]', ...args)
}

// 加载 markdown-it bundle（仅从本地 public 目录）
const loadMarkdownIt = (): Promise<any> => {
  return new Promise((resolve, reject) => {
    // 检查是否已经加载
    if (mdInstance && typeof (window as any).markdownit !== 'undefined') {
      mdInstance = (window as any).markdownit
      resolve(mdInstance)
      return
    }
    
    // 仅从 public 目录加载
    const localPaths = [
      '/markdown-it/markdown-it.min.js',        // 标准压缩版
      '/markdown-it/markdown-it.js',            // 标准版
      '/markdown-it/markdown-it.bundle.min.js', // 兼容旧路径
    ]
    
    let currentPathIndex = 0
    
    const tryLoadScript = () => {
      if (currentPathIndex >= localPaths.length) {
        // 如果本地加载失败，尝试 CDN 兜底
        debugWarn('⚠️ [markdown-it] 本地加载失败，尝试使用 CDN')
        loadFromCDN()
        return
      }
      
      const script = document.createElement('script')
      script.src = localPaths[currentPathIndex] || ''
      script.onerror = () => {
        currentPathIndex++
        tryLoadScript()
      }
      script.onload = () => {
        checkLoaded()
      }
      document.head.appendChild(script)
    }

    const loadFromCDN = () => {
      const script = document.createElement('script')
      script.src = 'https://cdn.jsdelivr.net/npm/markdown-it@14.0.0/dist/markdown-it.min.js'
      script.onload = () => checkLoaded()
      script.onerror = () => {
        debugError('❌ [markdown-it] CDN 加载也失败了')
        reject(new Error('无法加载 markdown-it'))
      }
      document.head.appendChild(script)
    }

    const checkLoaded = () => {
      setTimeout(() => {
        const win = window as any
        if (typeof win.markdownit !== 'undefined') {
          // 注意：这里保存构造函数，而不是实例
          mdInstance = win.markdownit
          debugLog('✅ [markdown-it] 加载成功')
          resolve(mdInstance)
        } else {
          debugError('❌ [markdown-it] 脚本加载但未找到全局变量')
          currentPathIndex++
          tryLoadScript()
        }
      }, 50)
    }
    
    tryLoadScript()
  })
}

// 初始化 markdown-it 实例
// 注意：mdInstance 实际上是构造函数
let mdRenderer: any = null

const initMarkdownIt = async (): Promise<any> => {
  if (mdRenderer) return mdRenderer

  const MarkdownIt = await loadMarkdownIt()
  
  // 实例化 markdown-it
  mdRenderer = new MarkdownIt({
    html: true,        // 启用 HTML 标签支持
    breaks: true,      // 转换 \n 为 <br>
    linkify: true,     // 自动转换 URL 为链接
    typographer: true, // 启用一些语言优化
    highlight: function (str: string, lang: string) {
      // Mermaid 特殊处理：保留原始内容，不转义 HTML，直接包裹在 div 中
      // 这样可以保留换行符，供后续 renderMermaidCharts 处理
      if (lang && lang.toLowerCase() === 'mermaid') {
        return '<div class="mermaid">' + str + '</div>';
      }

      // 其他语言：使用标准的高亮结构
      if (lang && (window as any).hljs) {
        try {
          return '<pre><code class="hljs language-' + lang + '">' +
                 (window as any).hljs.highlight(str, { language: lang, ignoreIllegals: true }).value +
                 '</code></pre>';
        } catch (__) {}
      }

      // 默认处理（转义 HTML）
      return '<pre><code class="hljs language-' + lang + '">' + mdRenderer.utils.escapeHtml(str) + '</code></pre>';
    }
  })
  
  return mdRenderer
}

// 初始化 Mermaid
mermaid.initialize({
  startOnLoad: false,
  theme: 'dark', // 使用暗色主题，确保文字可见
  securityLevel: 'loose',
  flowchart: {
    useMaxWidth: true,
    htmlLabels: true, // 确保 HTML 标签正确渲染，支持中文节点
  },
})

/**
 * 纯函数：渲染 Markdown 为 HTML
 * @param markdownText Markdown 文本内容
 * @param basePath 基础路径（用于处理相对路径的图片和链接）
 * @returns 渲染后的 HTML 字符串（Promise）
 * 
 * 注意：此函数现在是异步的，因为需要动态加载 markdown-it
 */
export async function renderMarkdown(markdownText: string, basePath?: string): Promise<string> {
  if (!markdownText || !markdownText.trim()) {
    return '<p>内容为空</p>'
  }
  
  try {
    // 确保 markdown-it 已加载
    const md = await initMarkdownIt()
    if (!md) {
      return '<p>markdown-it 加载失败，请确保文件存在于 public/markdown-it/ 目录。请先运行 npm run build:markdown-it 构建 bundle。</p>'
    }
    
    // 预处理：将 GitHub 风格的 admonition 语法转换为 markdown-it-container 语法
    let processedText = markdownText
    
    // ==========================================
    // 公式保护逻辑
    // ==========================================
    // 为了防止 markdown-it 转义 LaTeX 公式中的特殊字符（如 _ *），我们先将公式替换为占位符
    const formulaMap = new Map<string, string>()
    let formulaCounter = 0
    
    // 保护块级公式 $$...$$
    processedText = processedText.replace(/\$\$([\s\S]*?)\$\$/g, (match) => {
      const id = `__FORMULA_BLOCK_${formulaCounter++}__`
      formulaMap.set(id, match)
      return id
    })
    
    // 保护行内公式 $...$ (注意不匹配 \$)
    // 简单的正则，可能无法处理嵌套或复杂情况，但对大多数情况有效
    processedText = processedText.replace(/(?<!\\)\$(.+?)(?<!\\)\$/g, (match) => {
      const id = `__FORMULA_INLINE_${formulaCounter++}__`
      formulaMap.set(id, match)
      return id
    })

    const admonitionMap: Record<string, string> = {
      'NOTE': 'note',
      'TIP': 'tip',
      'IMPORTANT': 'info',
      'WARNING': 'warning',
      'CAUTION': 'danger',
      'DANGER': 'danger',
    }
    
    // 使用正则表达式匹配完整的 admonition 块
    // 匹配模式：> [!XXX] 开头，后面跟着多行以 > 开头的内容，直到遇到空行或非 > 开头的行
    processedText = processedText.replace(/^>\s*\[!(\w+)\]\s*$\n((?:>\s*.*\n?)*)/gm, (_match, type, content) => {
      const normalizedType = admonitionMap[type.toUpperCase()] || type.toLowerCase()
      // 移除每行的 > 前缀和首尾空白
      const cleanedContent = content
        .split('\n')
        .map((line: string) => line.replace(/^>\s*/, ''))
        .join('\n')
        .trim()
      return `::: ${normalizedType}\n${cleanedContent}\n:::`
    })
    
    // 使用 markdown-it 渲染 Markdown
    let html = md.render(processedText)
    
    // 处理相对路径的图片和链接（如果提供了 basePath）
    if (basePath) {
      // 使用 URL API 构建基础路径，更安全可靠
      const baseUrl = new URL(basePath, window.location.origin)
      const baseDir = baseUrl.pathname.substring(0, baseUrl.pathname.lastIndexOf('/') + 1)
      
      // 处理相对路径的图片
      html = html.replace(/<img([^>]*?)src="([^"]+)"([^>]*?)>/g, (match: string, before: string, src: string, after: string) => {
        // 如果是相对路径且不是 data: 或 http(s):// 开头
        if (!src.startsWith('data:') && !src.startsWith('http://') && !src.startsWith('https://') && !src.startsWith('/')) {
          // 使用 URL API 解析相对路径
          try {
            const resolvedUrl = new URL(src.replace(/^\.\//, ''), window.location.origin + baseDir)
            const resolvedPath = resolvedUrl.pathname
            return `<img${before}src="${resolvedPath}"${after} data-wiki-image="${resolvedPath}">`
          } catch (e) {
            // 如果 URL 解析失败，使用旧方法
            const resolvedPath = baseDir + src.replace(/^\.\//, '')
            return `<img${before}src="${resolvedPath}"${after} data-wiki-image="${resolvedPath}">`
          }
        }
        return match
      })
      
      // 处理相对路径的链接（Markdown 内部链接）
      html = html.replace(/<a([^>]*?)href="([^"]+)"([^>]*?)>/g, (match: string, before: string, href: string, after: string) => {
        // 如果是相对路径且不是 http(s):// 或 # 开头
        if (!href.startsWith('http://') && !href.startsWith('https://') && !href.startsWith('#') && !href.startsWith('/')) {
          // 只处理 .md 或 .markdown 文件，忽略纯文本链接
          if (href.endsWith('.md') || href.endsWith('.markdown')) {
            try {
              const resolvedUrl = new URL(href.replace(/^\.\//, ''), window.location.origin + baseDir)
              const resolvedPath = resolvedUrl.pathname
              return `<a${before}href="#" data-wiki-link="${resolvedPath}" class="wiki-internal-link"${after}>`
            } catch (e) {
              // 如果 URL 解析失败，使用旧方法
              const resolvedPath = baseDir + href.replace(/^\.\//, '')
              return `<a${before}href="#" data-wiki-link="${resolvedPath}" class="wiki-internal-link"${after}>`
            }
          }
          // 对于非 .md 文件的相对路径，保持原样（可能是锚点或其他资源）
        }
        return match
      })
    }
    
    return html
  } catch (error) {
    // console.error('Markdown 渲染失败:', error)
    return `<p>渲染失败: ${error instanceof Error ? error.message : String(error)}</p>`
  }
}

/**
 * 渲染 Mermaid 图表（需要在 DOM 更新后调用）
 * @param container 容器元素，如果为 null 则在 document 中查找所有 .mermaid 元素
 */
export async function renderMermaidCharts(container: HTMLElement | null = null): Promise<void> {
  try {
    // 确保 mermaid 已加载
    if (typeof mermaid === 'undefined') {
      console.error('❌ [Mermaid] 未加载，无法渲染图表')
      return
    }
    
    // 查找所有 mermaid 元素（可能是 <pre class="mermaid"> 或 <div class="mermaid"> 或 <code class="language-mermaid">）
    const elements = container 
      ? container.querySelectorAll('.mermaid, pre.mermaid, div.mermaid, code.language-mermaid, pre code.language-mermaid')
      : document.querySelectorAll('.mermaid, pre.mermaid, div.mermaid, code.language-mermaid, pre code.language-mermaid')
    
    if (elements.length === 0) {
      debugLog('[Mermaid] 未找到 Mermaid 元素')
      return
    }
    
    debugLog(`[Mermaid] 找到 ${elements.length} 个 Mermaid 元素`)
    
    // 为每个 Mermaid 元素生成唯一 ID 并准备内容
    const mermaidElements: HTMLElement[] = []
    elements.forEach((element, index) => {
      const el = element as HTMLElement
      
      // 跳过已经渲染过的元素（有 mermaid 生成的 SVG）
      if (el.querySelector('svg')) {
        debugLog(`[Mermaid] 跳过已渲染的元素 ${index}`)
        return
      }
      
      // 统一的 Mermaid 代码清理函数（合并了 cleanMermaidCode 和 finalCleanMermaidCode）
      // 关键：确保保留所有换行符，特别是流程图声明后的换行符
      // 支持中文节点（使用 [^\]]* 匹配方括号内容）
      const cleanMermaidCode = (code: string): string => {
        if (!code) return ''
        
        // 移除首尾空白（但保留内部换行符）
        let cleaned = code.trim()
        
        // 移除可能的 HTML 实体编码
        cleaned = cleaned.replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&')
        cleaned = cleaned.replace(/&nbsp;/g, ' ')
        
        // 规范化换行符（统一为 \n）
        // 重要：这一步必须在其他处理之前，确保换行符统一
        cleaned = cleaned.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
        
        // 关键修复：确保流程图声明后必须有换行符
        // 问题：markdown-it-mermaid 可能生成 "flowchart TD    A[开始]" 这样的单行文本
        // 解决：强制在流程图声明和第一个节点之间添加换行符
        // 匹配模式：
        // - flowchart TD    A[开始] -> flowchart TD\nA[开始]
        // - graph LR    B{判断} -> graph LR\nB{判断}
        // 注意：[^\]]* 会匹配方括号内的所有内容，包括中文字符（如"开始"）
        cleaned = cleaned.replace(/(flowchart\s+[A-Z]{2})\s+([A-Z]\[[^\]]*)/gi, '$1\n$2')
        cleaned = cleaned.replace(/(graph\s+[A-Z]{2})\s+([A-Z]\[[^\]]*)/gi, '$1\n$2')
        // 也处理没有方括号的情况（较少见，但也要处理）
        cleaned = cleaned.replace(/(flowchart\s+[A-Z]{2})\s+([A-Z])(?![A-Za-z0-9_\[])/gi, '$1\n$2')
        cleaned = cleaned.replace(/(graph\s+[A-Z]{2})\s+([A-Z])(?![A-Za-z0-9_\[])/gi, '$1\n$2')
        
        // 处理每行：保留行首空格（缩进），移除行尾空格
        // 重要：保留缩进，因为 Mermaid 的某些语法依赖缩进
        cleaned = cleaned.split('\n').map(line => {
          // 移除行尾空格和制表符，但保留行首空格（用于缩进）
          return line.replace(/[\s\t]+$/, '')
        }).join('\n')
        
        // 移除连续的空行（最多保留一个空行）
        // 注意：Mermaid 不需要多个空行，但保留一个空行有助于可读性
        cleaned = cleaned.replace(/\n{3,}/g, '\n\n')
        
        // 移除第一行的前导空格（流程图声明不应该有前导空格）
        // 但保留后续行的缩进（用于子节点等）
        const lines = cleaned.split('\n')
        if (lines.length > 0 && lines[0]) {
          // 第一行通常是 "flowchart TD" 或 "graph LR"，不应该有前导空格
          lines[0] = lines[0].trimStart()
          cleaned = lines.join('\n')
        }
        
        // 确保以换行符结尾（如果内容不为空）
        // 这有助于 Mermaid 正确解析最后一行
        if (cleaned && !cleaned.endsWith('\n')) {
          cleaned += '\n'
        }
        
        return cleaned
      }
      
      // 提取文本的辅助函数，确保保留换行符
      const extractTextWithNewlines = (element: HTMLElement): string => {
        // 对于 <pre> 或 <code> 元素，textContent 应该保留换行符
        // 但如果是从 markdown-it-mermaid 生成的 <div>，可能需要特殊处理
        let text = element.textContent || element.innerText || ''
        
        // 如果 textContent 没有换行符，但 innerHTML 有 <br> 标签，转换它们
        if (!text.includes('\n') && element.innerHTML) {
          text = element.innerHTML
            .replace(/<br\s*\/?>/gi, '\n')
            .replace(/<\/?[^>]+>/g, '') // 移除所有 HTML 标签
        }
        
        return text
      }
      
      // 如果是 <code class="language-mermaid">，需要找到父元素 <pre>
      if (el.tagName === 'CODE' && el.classList.contains('language-mermaid')) {
        const pre = el.parentElement
        if (pre && pre.tagName === 'PRE') {
          const rawText = extractTextWithNewlines(pre) // 从 <pre> 提取，保留换行符
          const textContent = cleanMermaidCode(rawText)
          if (!textContent.trim()) {
            debugWarn(`Mermaid 代码块 ${index} 内容为空`)
            return
          }
          debugLog(`[Mermaid] CODE 元素原始内容:`, JSON.stringify(rawText.substring(0, 100)))
          debugLog(`[Mermaid] CODE 元素清理后:`, JSON.stringify(textContent.substring(0, 100)))
          const div = document.createElement('div')
          div.className = 'mermaid'
          div.id = `mermaid-${Date.now()}-${index}`
          div.textContent = textContent
          pre.parentNode?.replaceChild(div, pre)
          mermaidElements.push(div)
          debugLog(`[Mermaid] 转换 CODE 元素为 DIV: ${div.id}`)
          return
        }
      }
      
      // 如果是 <pre class="mermaid">，需要提取文本内容
      if (el.tagName === 'PRE' && el.classList.contains('mermaid')) {
        const rawText = extractTextWithNewlines(el)
        const textContent = cleanMermaidCode(rawText)
        if (!textContent.trim()) {
          debugWarn(`Mermaid PRE 元素 ${index} 内容为空`)
          return
        }
        debugLog(`[Mermaid] PRE 元素原始内容:`, JSON.stringify(rawText.substring(0, 100)))
        debugLog(`[Mermaid] PRE 元素清理后:`, JSON.stringify(textContent.substring(0, 100)))
        const div = document.createElement('div')
        div.className = 'mermaid'
        div.id = `mermaid-${Date.now()}-${index}`
        div.textContent = textContent
        el.parentNode?.replaceChild(div, el)
        mermaidElements.push(div)
        debugLog(`[Mermaid] 转换 PRE 元素为 DIV: ${div.id}`)
      } else if (el.tagName === 'DIV' && el.classList.contains('mermaid')) {
        // 如果是 <div class="mermaid">，直接使用
        if (!el.id) {
          el.id = `mermaid-${Date.now()}-${index}`
        }
        const rawText = extractTextWithNewlines(el)
        const textContent = cleanMermaidCode(rawText)
        if (!textContent.trim()) {
          debugWarn(`Mermaid DIV 元素 ${index} 内容为空`)
          return
        }
        debugLog(`[Mermaid] DIV 元素原始内容:`, JSON.stringify(rawText.substring(0, 100)))
        debugLog(`[Mermaid] DIV 元素清理后:`, JSON.stringify(textContent.substring(0, 100)))
        // 更新元素内容为清理后的内容
        el.textContent = textContent
        mermaidElements.push(el)
        debugLog(`[Mermaid] 使用现有 DIV 元素: ${el.id}`)
      }
    })
    
    // 渲染所有 Mermaid 图表
    if (mermaidElements.length > 0) {
      debugLog(`[Mermaid] 准备渲染 ${mermaidElements.length} 个图表`)
      
      // 再次清理函数（用于渲染前最后清理）
      const finalCleanMermaidCode = (code: string): string => {
        if (!code) return ''
        let cleaned = code.trim()
        cleaned = cleaned.replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&')
        cleaned = cleaned.replace(/&nbsp;/g, ' ')
        cleaned = cleaned.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
        // 关键：确保流程图声明后必须有换行符
        // 先处理带方括号的节点定义
        cleaned = cleaned.replace(/(flowchart\s+[A-Z]{2})\s+([A-Z]\[[^\]]*)/gi, '$1\n$2')
        cleaned = cleaned.replace(/(graph\s+[A-Z]{2})\s+([A-Z]\[[^\]]*)/gi, '$1\n$2')
        // 再处理没有方括号的情况
        cleaned = cleaned.replace(/(flowchart\s+[A-Z]{2})\s+([A-Z])(?![A-Za-z0-9_])/gi, '$1\n$2')
        cleaned = cleaned.replace(/(graph\s+[A-Z]{2})\s+([A-Z])(?![A-Za-z0-9_])/gi, '$1\n$2')
        cleaned = cleaned.split('\n').map(line => line.replace(/[\s\t]+$/, '')).join('\n')
        cleaned = cleaned.replace(/\n{3,}/g, '\n\n')
        const lines = cleaned.split('\n')
        if (lines.length > 0 && lines[0]) {
          lines[0] = lines[0].trimStart()
          cleaned = lines.join('\n')
        }
        if (cleaned && !cleaned.endsWith('\n')) {
          cleaned += '\n'
        }
        return cleaned
      }
      
      // 逐个渲染，以便更好地捕获错误
      for (const node of mermaidElements) {
        try {
          // 确保节点有 ID
          if (!node.id) {
            node.id = `mermaid-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
          }
          
          // 获取并验证内容，最后清理一次确保格式正确
          let content = node.textContent || ''
          if (!content.trim()) {
            debugWarn(`[Mermaid] 元素 ${node.id} 内容为空，跳过`)
            continue
          }
          
          // 最后清理，确保格式正确
          const originalContent = content
          content = finalCleanMermaidCode(content)
          
          // 调试：如果内容被修改，记录日志
          if (originalContent !== content) {
            debugLog(`[Mermaid] 内容已清理:`, {
              original: originalContent.substring(0, 100),
              cleaned: content.substring(0, 100)
            })
          }
          
          // 更新节点内容
          node.textContent = content
          
          debugLog(`[Mermaid] 渲染元素 ${node.id}:`, content.substring(0, 100))
          
          await mermaid.run({
            nodes: [node]
          })
          debugLog(`✅ [Mermaid] 成功渲染元素: ${node.id}`)
        } catch (error) {
          debugError(`❌ [Mermaid] 渲染元素 ${node.id} 失败:`, error)
          // 失败时，保留原始内容并显示错误
          const errorMessage = error instanceof Error ? error.message : String(error)
          node.innerHTML = `
            <div class="mermaid-error">
              <div class="mermaid-error-title">Mermaid 渲染失败</div>
              <pre class="mermaid-error-content">${node.textContent}</pre>
              <div class="mermaid-error-message">${errorMessage}</div>
            </div>
          `
          node.classList.add('render-error')
        }
      }
      
      debugLog(`[Mermaid] 渲染流程完成，处理了 ${mermaidElements.length} 个元素`)
    } else {
      debugLog('[Mermaid] 没有需要渲染的元素')
    }
  } catch (error) {
    debugError('❌ [Mermaid] 渲染失败:', error)
    debugError('错误堆栈:', error instanceof Error ? error.stack : '无堆栈信息')
  }
}

/**
 * 从 Markdown 文本提取标题
 * @param markdownText Markdown 文本
 * @returns 第一个一级或二级标题，如果没有则返回 null
 */
export function extractTitle(markdownText: string): string | null {
  if (!markdownText) return null
  
  const lines = markdownText.trim().split('\n')
  for (const line of lines) {
    const trimmed = line.trim()
    if (trimmed.startsWith('# ')) {
      return trimmed.substring(2).trim()
    } else if (trimmed.startsWith('## ')) {
      return trimmed.substring(3).trim()
    }
  }
  
  return null
}

/**
 * 从 Markdown 文本提取目录结构
 * @param markdownText Markdown 文本
 * @returns 目录项数组
 */
export interface TocItem {
  level: number
  id: string
  text: string
  children?: TocItem[]
}

export function extractTOC(markdownText: string): TocItem[] {
  if (!markdownText) return []
  
  const lines = markdownText.split('\n')
  const toc: TocItem[] = []
  const stack: TocItem[] = []
  
  for (const line of lines) {
    const trimmed = line.trim()
    if (!trimmed.startsWith('#')) continue
    
    // 计算标题级别
    let level = 0
    while (level < trimmed.length && trimmed[level] === '#') {
      level++
    }
    
    if (level > 6) continue // 只支持 h1-h6
    
    const text = trimmed.substring(level).trim()
    if (!text) continue
    
    // 生成 ID（与 markdown-it-anchor 保持一致）
    // markdown-it-anchor 的 ID 生成规则：小写、移除特殊字符、空格转连字符、合并多个连字符
    const id = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')  // 移除特殊字符，保留字母、数字、空格、连字符
      .replace(/\s+/g, '-')      // 空格替换为连字符
      .replace(/-+/g, '-')       // 多个连字符合并为一个
      .replace(/^-|-$/g, '')     // 移除首尾连字符
      .trim()
    
    const item: TocItem = {
      level,
      id,
      text,
    }
    
    // 构建层级结构
    while (stack.length > 0) {
      const top = stack[stack.length - 1]
      if (top && top.level >= level) {
        stack.pop()
      } else {
        break
      }
    }
    
    if (stack.length === 0) {
      toc.push(item)
    } else {
      const parent = stack[stack.length - 1]
      if (parent) {
        if (!parent.children) {
          parent.children = []
        }
        parent.children.push(item)
      }
    }
    
    stack.push(item)
  }
  
  return toc
}
