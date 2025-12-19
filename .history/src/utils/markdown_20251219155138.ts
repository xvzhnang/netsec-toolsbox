/**
 * Markdown 渲染工具
 * 使用 markdown-it 及其插件进行渲染
 * 为 AI 预留接口：AI 可以生成 Markdown，直接使用此函数渲染
 */

import mermaid from 'mermaid'

// markdown-it 将从 public 目录动态加载（打包后的 bundle）

// markdown-it 实例（动态加载后设置）
let mdInstance: any = null

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
      '/markdown-it/markdown-it.bundle.min.js',  // 打包后的 bundle
      '/markdown-it/markdown-it.bundle.js',      // 未压缩版本（备选）
    ]
    
    let currentPathIndex = 0
    
    const tryLoadScript = () => {
      if (currentPathIndex >= localPaths.length) {
        console.error('❌ [markdown-it] 无法从本地加载，请确保文件存在于 public/markdown-it/ 目录')
        console.error('尝试的路径:', localPaths)
        reject(new Error('无法从本地加载 markdown-it，请确保文件存在于 public/markdown-it/ 目录。请先运行 npm run build:markdown-it 构建 bundle。'))
        return
      }
      
      const script = document.createElement('script')
      script.src = localPaths[currentPathIndex] || ''
      script.onerror = () => {
        currentPathIndex++
        tryLoadScript()
      }
      script.onload = () => {
        // 等待一小段时间确保脚本执行完成
        setTimeout(() => {
          // 检查 markdownit 是否已加载到全局对象
          const win = window as any
          if (typeof win.markdownit !== 'undefined') {
            mdInstance = win.markdownit
            console.log('✅ [markdown-it] 加载成功:', localPaths[currentPathIndex])
            resolve(mdInstance)
          } else {
            // 调试：检查所有可能的全局变量
            console.error('❌ [markdown-it] 脚本已加载，但未找到全局变量 markdownit')
            console.error('可用的全局变量:', {
              markdownit: typeof win.markdownit,
              MarkdownIt: typeof win.MarkdownIt,
              MarkdownItBundle: typeof win.MarkdownItBundle,
            })
            // 尝试下一个路径
            currentPathIndex++
            tryLoadScript()
          }
        }, 50)
      }
      document.head.appendChild(script)
    }
    
    tryLoadScript()
  })
}

// 初始化 markdown-it（延迟初始化）
const initMarkdownIt = async (): Promise<any> => {
  if (!mdInstance) {
    mdInstance = await loadMarkdownIt()
  }
  return mdInstance
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
    // GitHub 风格：> [!NOTE]\n> 内容\n> 更多内容
    // 转换为：::: note\n内容\n更多内容\n:::
    let processedText = markdownText
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
      const baseDir = basePath.substring(0, basePath.lastIndexOf('/') + 1)
      
      // 处理相对路径的图片
      html = html.replace(/<img([^>]*?)src="([^"]+)"([^>]*?)>/g, (_match, before, src, after) => {
        // 如果是相对路径且不是 data: 或 http(s):// 开头
        if (!src.startsWith('data:') && !src.startsWith('http://') && !src.startsWith('https://') && !src.startsWith('/')) {
          // 构建相对于 wiki 根目录的路径
          const resolvedPath = baseDir + src.replace(/^\.\//, '')
          return `<img${before}src="${resolvedPath}"${after} data-wiki-image="${resolvedPath}">`
        }
        return match
      })
      
      // 处理相对路径的链接（Markdown 内部链接）
      html = html.replace(/<a([^>]*?)href="([^"]+)"([^>]*?)>/g, (_match, before, href, after) => {
        // 如果是相对路径且不是 http(s):// 或 # 开头
        if (!href.startsWith('http://') && !href.startsWith('https://') && !href.startsWith('#') && !href.startsWith('/')) {
          // 只处理 .md 或 .markdown 文件，忽略纯文本链接
          if (href.endsWith('.md') || href.endsWith('.markdown')) {
            const resolvedPath = baseDir + href.replace(/^\.\//, '')
            return `<a${before}href="#" data-wiki-link="${resolvedPath}" class="wiki-internal-link"${after}>`
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
      console.log('[Mermaid] 未找到 Mermaid 元素')
      return
    }
    
    console.log(`[Mermaid] 找到 ${elements.length} 个 Mermaid 元素`)
    
    // 为每个 Mermaid 元素生成唯一 ID 并准备内容
    const mermaidElements: HTMLElement[] = []
    elements.forEach((element, index) => {
      const el = element as HTMLElement
      
      // 跳过已经渲染过的元素（有 mermaid 生成的 SVG）
      if (el.querySelector('svg')) {
        console.log(`[Mermaid] 跳过已渲染的元素 ${index}`)
        return
      }
      
      // 清理 Mermaid 代码内容的辅助函数
      // 关键：确保保留所有换行符，特别是流程图声明后的换行符
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
        if (lines.length > 0) {
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
            console.warn(`Mermaid 代码块 ${index} 内容为空`)
            return
          }
          console.log(`[Mermaid] CODE 元素原始内容:`, JSON.stringify(rawText.substring(0, 100)))
          console.log(`[Mermaid] CODE 元素清理后:`, JSON.stringify(textContent.substring(0, 100)))
          const div = document.createElement('div')
          div.className = 'mermaid'
          div.id = `mermaid-${Date.now()}-${index}`
          div.textContent = textContent
          pre.parentNode?.replaceChild(div, pre)
          mermaidElements.push(div)
          console.log(`[Mermaid] 转换 CODE 元素为 DIV: ${div.id}`)
          return
        }
      }
      
      // 如果是 <pre class="mermaid">，需要提取文本内容
      if (el.tagName === 'PRE' && el.classList.contains('mermaid')) {
        const rawText = extractTextWithNewlines(el)
        const textContent = cleanMermaidCode(rawText)
        if (!textContent.trim()) {
          console.warn(`Mermaid PRE 元素 ${index} 内容为空`)
          return
        }
        console.log(`[Mermaid] PRE 元素原始内容:`, JSON.stringify(rawText.substring(0, 100)))
        console.log(`[Mermaid] PRE 元素清理后:`, JSON.stringify(textContent.substring(0, 100)))
        const div = document.createElement('div')
        div.className = 'mermaid'
        div.id = `mermaid-${Date.now()}-${index}`
        div.textContent = textContent
        el.parentNode?.replaceChild(div, el)
        mermaidElements.push(div)
        console.log(`[Mermaid] 转换 PRE 元素为 DIV: ${div.id}`)
      } else if (el.tagName === 'DIV' && el.classList.contains('mermaid')) {
        // 如果是 <div class="mermaid">，直接使用
        if (!el.id) {
          el.id = `mermaid-${Date.now()}-${index}`
        }
        const rawText = extractTextWithNewlines(el)
        const textContent = cleanMermaidCode(rawText)
        if (!textContent.trim()) {
          console.warn(`Mermaid DIV 元素 ${index} 内容为空`)
          return
        }
        console.log(`[Mermaid] DIV 元素原始内容:`, JSON.stringify(rawText.substring(0, 100)))
        console.log(`[Mermaid] DIV 元素清理后:`, JSON.stringify(textContent.substring(0, 100)))
        // 更新元素内容为清理后的内容
        el.textContent = textContent
        mermaidElements.push(el)
        console.log(`[Mermaid] 使用现有 DIV 元素: ${el.id}`)
      }
    })
    
    // 渲染所有 Mermaid 图表
    if (mermaidElements.length > 0) {
      console.log(`[Mermaid] 准备渲染 ${mermaidElements.length} 个图表`)
      
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
        if (lines.length > 0) {
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
            console.warn(`[Mermaid] 元素 ${node.id} 内容为空，跳过`)
            continue
          }
          
          // 最后清理，确保格式正确
          const originalContent = content
          content = finalCleanMermaidCode(content)
          
          // 调试：如果内容被修改，记录日志
          if (originalContent !== content) {
            console.log(`[Mermaid] 内容已清理:`, {
              original: originalContent.substring(0, 100),
              cleaned: content.substring(0, 100)
            })
          }
          
          // 更新节点内容
          node.textContent = content
          
          console.log(`[Mermaid] 渲染元素 ${node.id}:`, content.substring(0, 100))
          
          // 使用 mermaid.run() 渲染单个图表
          await mermaid.run({ nodes: [node] })
          console.log(`✅ [Mermaid] 成功渲染元素: ${node.id}`)
        } catch (nodeError) {
          console.error(`❌ [Mermaid] 渲染元素 ${node.id} 失败:`, nodeError)
          console.error('元素内容:', node.textContent?.substring(0, 200))
          console.error('错误详情:', nodeError instanceof Error ? nodeError.message : String(nodeError))
          
          // 在元素上显示错误信息
          node.innerHTML = `<div style="padding: 16px; background: #fee; border: 1px solid #fcc; border-radius: 4px; color: #c00;">
            <strong>Mermaid 渲染错误:</strong><br>
            ${nodeError instanceof Error ? nodeError.message : String(nodeError)}<br>
            <details style="margin-top: 8px;">
              <summary>查看代码</summary>
              <pre style="margin-top: 8px; padding: 8px; background: #fff; border: 1px solid #ccc; overflow-x: auto;">${(node.textContent || '').substring(0, 500)}</pre>
            </details>
          </div>`
        }
      }
      
      console.log(`[Mermaid] 渲染流程完成，处理了 ${mermaidElements.length} 个元素`)
    } else {
      console.log('[Mermaid] 没有需要渲染的元素')
    }
  } catch (error) {
    console.error('❌ [Mermaid] 渲染失败:', error)
    console.error('错误堆栈:', error instanceof Error ? error.stack : '无堆栈信息')
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
