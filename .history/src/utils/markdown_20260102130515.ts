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
    
    // ==========================================
    // 恢复公式
    // ==========================================
    formulaMap.forEach((formula, id) => {
      // 全局替换回原始公式
      html = html.replace(new RegExp(id, 'g'), formula)
    })
    
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
    
    // 查找所有 mermaid 元素
    // 我们在 markdown-it highlight 中生成的是 <div class="mermaid">
    // 但为了兼容性，也查找其他可能的格式
    const elements = container 
      ? container.querySelectorAll('.mermaid')
      : document.querySelectorAll('.mermaid')
    
    if (elements.length === 0) {
      return
    }
    
    debugLog(`[Mermaid] 找到 ${elements.length} 个 Mermaid 元素`)
    
    // 遍历处理每个元素
    for (let i = 0; i < elements.length; i++) {
      const element = elements[i] as HTMLElement
      
      // 跳过已经渲染过的（包含 SVG）
      if (element.querySelector('svg')) continue
      
      // 获取源代码
      // 优先使用 textContent，它会自动解码 HTML 实体
      let code = element.textContent || ''
      if (!code.trim()) continue
      
      // 简单的清理：解码可能存在的 HTML 实体（以防万一），处理非标准空格
      code = code
        .replace(/&gt;/g, '>')
        .replace(/&lt;/g, '<')
        .replace(/&amp;/g, '&')
        .replace(/\u00A0/g, ' ') // 替换 &nbsp;
      
      // 生成唯一 ID
      const id = `mermaid-${Date.now()}-${i}`
      
      try {
        // 使用 mermaid.render 生成 SVG
        // 注意：mermaid.render 返回 Promise<{svg: string}>
        const { svg } = await mermaid.render(id, code)
        element.innerHTML = svg
        // 标记为已渲染，防止重复处理
        element.setAttribute('data-processed', 'true')
      } catch (err) {
        console.error(`[Mermaid] 渲染失败 (ID: ${id}):`, err)
        // 显示错误信息给用户，而不是让它留白
        element.innerHTML = `<div style="color: red; border: 1px solid red; padding: 8px;">
          <p>Mermaid 渲染错误:</p>
          <pre>${err instanceof Error ? err.message : String(err)}</pre>
          <pre style="background: #333; color: #ccc; padding: 4px; margin-top: 4px;">${code}</pre>
        </div>`
      }
    }
  } catch (error) {
    console.error('Mermaid 渲染过程出错:', error)
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
