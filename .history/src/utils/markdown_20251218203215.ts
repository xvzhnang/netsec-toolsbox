/**
 * Markdown 渲染工具
 * 纯函数式渲染，不依赖任何外部服务
 * 为 AI 预留接口：AI 可以生成 Markdown，直接使用此函数渲染
 */

import { marked } from 'marked'
import hljs from 'highlight.js'
import 'highlight.js/styles/github.min.css'
import mermaid from 'mermaid'

// 配置 marked - 支持完整的 GitHub 风格 Markdown
marked.setOptions({
  breaks: true,        // 支持 GitHub 风格的换行（单个换行也会换行）
  gfm: true,           // GitHub 风格 Markdown（任务列表、表格、删除线等）
  mangle: false,       // 不混淆邮箱地址
  pedantic: false,     // 不使用原始 Markdown.pl 的行为
  sanitize: false,     // 不清理 HTML（允许 HTML 标签）
  silent: false,       // 不静默忽略错误
  smartLists: true,    // 使用更智能的列表行为
  smartypants: false,  // 不使用智能标点符号
  highlight: function(code: string, lang?: string) {
    // 如果指定了语言且 highlight.js 支持该语言
    if (lang && hljs.getLanguage(lang)) {
      try {
        return hljs.highlight(code, { 
          language: lang,
          ignoreIllegals: true  // 忽略无法识别的代码
        }).value
      } catch (err) {
        console.warn(`代码高亮失败 (语言: ${lang}):`, err)
        // 如果指定语言失败，尝试自动检测
        return hljs.highlightAuto(code).value
      }
    }
    // 如果没有指定语言或语言不支持，自动检测
    return hljs.highlightAuto(code).value
  }
} as any)

/**
 * 纯函数：渲染 Markdown 为 HTML
 * @param markdownText Markdown 文本内容
 * @returns 渲染后的 HTML 字符串
 * 
 * 注意：此函数是纯函数，不依赖：
 * - 文件路径
 * - UI 状态
 * - 外部服务
 * 
 * 为 AI 预留：AI 可以生成 Markdown，直接调用此函数渲染
 */
// 初始化 Mermaid
mermaid.initialize({
  startOnLoad: false,
  theme: 'default',
  securityLevel: 'loose',
  flowchart: {
    useMaxWidth: true,
    htmlLabels: true,
  },
})

/**
 * 纯函数：渲染 Markdown 为 HTML
 * @param markdownText Markdown 文本内容
 * @param basePath 基础路径（用于处理相对路径的图片和链接）
 * @returns 渲染后的 HTML 字符串
 * 
 * 注意：此函数是纯函数，不依赖：
 * - 文件路径
 * - UI 状态
 * - 外部服务
 * 
 * 为 AI 预留：AI 可以生成 Markdown，直接调用此函数渲染
 */
export function renderMarkdown(markdownText: string, basePath?: string): string {
  if (!markdownText || !markdownText.trim()) {
    return '<p>内容为空</p>'
  }
  
  try {
    // 使用 marked 渲染 Markdown
    let html = marked.parse(markdownText) as string
    
    // 处理折叠内容块（格式：<details>...</details>）
    html = html.replace(/<details>([\s\S]*?)<\/details>/gi, (_match, content) => {
      const summaryMatch = content.match(/<summary>([\s\S]*?)<\/summary>/i)
      const summary = summaryMatch ? summaryMatch[1] : '点击展开'
      const detailsContent = content.replace(/<summary>[\s\S]*?<\/summary>/i, '')
      return `<div class="collapsible-block"><div class="collapsible-header"><span class="collapsible-icon">▼</span><span class="collapsible-title">${summary}</span></div><div class="collapsible-content">${detailsContent}</div></div>`
    })
    
    // 处理自定义按钮/标签（格式：`[button:文本]` 或 `[tag:文本:类型]`）
    html = html.replace(/\[button:([^\]]+)\]/g, '<button class="wiki-button">$1</button>')
    html = html.replace(/\[tag:([^\]]+):([^\]]+)\]/g, '<span class="wiki-tag wiki-tag-$2">$1</span>')
    
    // 处理特殊引用块（警告、提示、注意事项）
    // 格式：> [!WARNING] 或 > [!NOTE] 或 > [!TIP] 或 > [!CAUTION]
    html = html.replace(/<blockquote>\s*<p>\s*\[!(WARNING|NOTE|TIP|CAUTION|INFO)\]\s*(.*?)<\/p>\s*<\/blockquote>/g, (_match, type, content) => {
      const typeClass = type.toLowerCase()
      return `<div class="admonition admonition-${typeClass}"><div class="admonition-title">${getAdmonitionTitle(type)}</div><div class="admonition-content">${content}</div></div>`
    })
    
    // 处理 KaTeX 数学公式（行内：$...$，块级：$$...$$）
    // 先处理块级公式（$$...$$），避免与行内公式冲突
    // 排除代码块中的 $ 符号
    html = html.replace(/\$\$([\s\S]*?)\$\$/g, (match, formula) => {
      // 检查是否在代码块中
      if (match.includes('<code') || match.includes('</code>') || match.includes('<pre')) {
        return match
      }
      return `<div class="katex-block" data-formula="${escapeHtml(formula.trim())}">${formula.trim()}</div>`
    })
    // 处理行内公式，需要更精确的匹配，避免匹配代码中的 $ 符号
    // 只匹配不在代码块中的 $...$，且公式内容看起来像数学表达式
    html = html.replace(/\$([^\$\n]+?)\$/g, (match, formula) => {
      // 检查是否在代码块中
      const beforeMatch = html.substring(0, html.indexOf(match))
      const afterMatch = html.substring(html.indexOf(match) + match.length)
      // 检查前后是否有代码标签
      const beforeCode = beforeMatch.lastIndexOf('<code') > beforeMatch.lastIndexOf('</code>')
      const afterCode = afterMatch.indexOf('</code>') < afterMatch.indexOf('<code')
      if (beforeCode || afterCode) {
        return match
      }
      // 只处理看起来像数学公式的内容（包含数学符号或字母数字组合）
      const mathPattern = /[a-zA-Z0-9\s+\-*/=()\[\]{},.^_\\]/
      if (!mathPattern.test(formula.trim())) {
        return match
      }
      return `<span class="katex-inline" data-formula="${escapeHtml(formula.trim())}">${formula.trim()}</span>`
    })
    
    // 处理 Mermaid 图表
    // 查找所有 mermaid 代码块并替换为占位符
    html = html.replace(/<pre><code class="language-mermaid">([\s\S]*?)<\/code><\/pre>/g, (match, code) => {
      const id = `mermaid-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
      return `<div class="mermaid" id="${id}">${code.trim()}</div>`
    })
    
    // 处理相对路径的图片和链接（如果提供了 basePath）
    if (basePath) {
      const baseDir = basePath.substring(0, basePath.lastIndexOf('/') + 1)
      
      // 处理相对路径的图片
      html = html.replace(/<img([^>]*?)src="([^"]+)"([^>]*?)>/g, (match, before, src, after) => {
        // 如果是相对路径且不是 data: 或 http(s):// 开头
        if (!src.startsWith('data:') && !src.startsWith('http://') && !src.startsWith('https://') && !src.startsWith('/')) {
          // 构建相对于 wiki 根目录的路径
          const resolvedPath = baseDir + src.replace(/^\.\//, '')
          return `<img${before}src="${resolvedPath}"${after} data-wiki-image="${resolvedPath}">`
        }
        return match
      })
      
      // 处理相对路径的链接（Markdown 内部链接）
      html = html.replace(/<a([^>]*?)href="([^"]+)"([^>]*?)>/g, (match, before, href, after) => {
        // 如果是相对路径且不是 http(s):// 或 # 开头
        if (!href.startsWith('http://') && !href.startsWith('https://') && !href.startsWith('#') && !href.startsWith('/')) {
          // 如果是 .md 文件，转换为路由链接
          if (href.endsWith('.md') || href.endsWith('.markdown')) {
            const resolvedPath = baseDir + href.replace(/^\.\//, '')
            return `<a${before}href="#" data-wiki-link="${resolvedPath}" class="wiki-internal-link"${after}>`
          }
        }
        return match
      })
    }
    
    return html
  } catch (error) {
    console.error('Markdown 渲染失败:', error)
    return `<p>渲染失败: ${error instanceof Error ? error.message : String(error)}</p>`
  }
}

/**
 * 渲染 Mermaid 图表（需要在 DOM 更新后调用）
 * @param container 容器元素，如果为 null 则在 document 中查找所有 .mermaid 元素
 */
export async function renderMermaidCharts(container: HTMLElement | null = null): Promise<void> {
  try {
    const elements = container 
      ? container.querySelectorAll('.mermaid')
      : document.querySelectorAll('.mermaid')
    
    if (elements.length === 0) return
    
    // 为每个 Mermaid 元素生成唯一 ID
    elements.forEach((element, index) => {
      if (!element.id) {
        element.id = `mermaid-${Date.now()}-${index}`
      }
    })
    
    // 渲染所有 Mermaid 图表
    await mermaid.run({
      nodes: Array.from(elements) as HTMLElement[],
    })
  } catch (error) {
    console.error('Mermaid 渲染失败:', error)
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
 * 转义 HTML
 */
function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

/**
 * 获取引用块标题
 */
function getAdmonitionTitle(type: string): string {
  const titles: Record<string, string> = {
    'WARNING': '⚠️ 警告',
    'NOTE': 'ℹ️ 提示',
    'TIP': '💡 技巧',
    'CAUTION': '⚠️ 注意',
    'INFO': 'ℹ️ 信息',
  }
  return titles[type] || '提示'
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
    
    // 生成 ID（与 marked 的 headerIds 保持一致）
    const id = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-')
      .replace(/-+/g, '-')
      .trim()
    
    const item: TocItem = {
      level,
      id,
      text,
    }
    
    // 构建层级结构
    while (stack.length > 0 && stack[stack.length - 1].level >= level) {
      stack.pop()
    }
    
    if (stack.length === 0) {
      toc.push(item)
    } else {
      const parent = stack[stack.length - 1]
      if (!parent.children) {
        parent.children = []
      }
      parent.children.push(item)
    }
    
    stack.push(item)
  }
  
  return toc
}

