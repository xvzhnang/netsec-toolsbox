import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'
import { error as logError, debug } from './logger'

/**
 * 获取 Docsify 服务的基础 URL
 */
export const getDocsifyBaseUrl = async (): Promise<string> => {
  const port = await invoke<number>('start_docs_server')
  return `http://localhost:${port}`
}

/**
 * 获取指定路径的 Wiki Markdown 内容
 * @param path - 文档路径 (例如 "tools/nmap")
 */
export const fetchWikiContent = async (path: string): Promise<string> => {
  try {
    const baseUrl = await getDocsifyBaseUrl()
    // 假设文件扩展名为 .md，如果 path 没有后缀，尝试添加
    let fileUrl = `${baseUrl}/${path}`
    if (!fileUrl.toLowerCase().endsWith('.md')) {
      fileUrl += '.md'
    }
    
    debug('Fetching Wiki content from:', fileUrl)
    const response = await fetch(fileUrl)
    if (!response.ok) {
      throw new Error(`Failed to fetch wiki content: ${response.statusText}`)
    }
    let text = await response.text()

    // 修复：处理被错误转义或 JSON 序列化的 Markdown 内容

    // 1. 优先检查 HTML escape (e.g. &lt;h1&gt;)
    // 如果包含大量 HTML 实体，尝试反转义
    // 注意：某些后端可能会先 HTML 转义再 JSON 序列化，或者反之，所以顺序很重要
    // 这里先处理 HTML 转义，以防 &quot; 阻止了 JSON 解析
    if (text.includes('&lt;') || text.includes('&gt;') || text.includes('&amp;') || text.includes('&quot;')) {
      // 简单检测：如果 &lt; 的数量远多于 < (几乎没有 <)，或者包含 &quot; 但没有 "，则认为是全文转义
      const ltCount = (text.match(/</g) || []).length
      const escapedLtCount = (text.match(/&lt;/g) || []).length
      const hasEscapedQuote = text.includes('&quot;')
      
      if ((escapedLtCount > 0 && ltCount === 0) || hasEscapedQuote) {
        debug('Detected HTML escaped content, unescaping...')
        text = text
          .replace(/&lt;/g, '<')
          .replace(/&gt;/g, '>')
          .replace(/&quot;/g, '"')
          .replace(/&#39;/g, "'")
          .replace(/&amp;/g, '&')
      }
    }

    // 2. 检查是否被 JSON stringify (e.g. "content")
    // 如果内容被引号包裹，尝试 JSON.parse
    if (text.trim().startsWith('"') && text.trim().endsWith('"')) {
      try {
        const parsed = JSON.parse(text)
        if (typeof parsed === 'string') {
          debug('Detected JSON stringified content, unwrapping...')
          text = parsed
        }
      } catch (e) {
        // Ignore JSON parse error
      }
    }

    // 3. 针对性修复 "二次转义" 问题 (Markdown Double Escape)
    // 用户反馈：Markdown 核心语法 (e.g. **) 被变成了 \**
    // 这通常是因为某些层级错误地对 Markdown 符号进行了转义
    // 检测特征：存在 \** 但不存在 ** (或者 \** 数量异常多)
    // 修正：扩展到所有 Markdown 语法字符
    // 字符集包括：\ * _ [ ] ` # > - + ! | { } . : ( )
    if (text.includes('\\**') || text.includes('\\[') || text.includes('\\_') || text.includes('\\#') || text.includes('\\`')) {
      const hasDoubleEscapedBold = text.includes('\\**')
      
      // 如果发现常见语法的转义形式
      if (hasDoubleEscapedBold || text.match(/\\[\[\]_`#]/)) {
        debug('Detected double-escaped Markdown, attempting to fix...')
        // 谨慎替换：只替换常见的 Markdown 语法转义
        // 使用正则捕获组来还原被转义的字符
        // 匹配反斜杠后跟任意 Markdown 语法字符
        text = text.replace(/\\([\\*_[\]`#>\-+!|{}.:()])/g, '$1')
      }
    }

    return text
  } catch (err) {
    logError('Failed to fetch wiki content:', err)
    throw err
  }
}

/**
 * 启动 Docsify 服务器并打开指定的文档路径
 * @param path - 文档路径 (可选)，例如 "tools/nmap" 或完整 URL
 */
export const openDocsify = async (path?: string) => {
  try {
    debug('Starting Docsify server...')
    const port = await invoke<number>('start_docs_server')
    let url = `http://localhost:${port}`

    if (path) {
      if (path.startsWith('http')) {
        url = path
      } else {
        // Docsify 使用 hash 路由
        // 处理相对路径，确保不以 / 开头
        const cleanPath = path.startsWith('/') ? path.slice(1) : path
        url = `${url}/#/${cleanPath}`
      }
    }

    debug('Opening Docsify URL:', url)
    await open(url)
  } catch (err) {
    logError('Failed to open Docsify:', err)
    throw err
  }
}
