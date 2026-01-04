/**
 * Markdown 渲染工具
 * 使用 markdown-it 及其插件进行渲染
 * 遵循 "Markdown -> markdown-it + plugins -> HTML" 的标准流程
 */

import MarkdownIt from 'markdown-it'
// @ts-ignore
import anchor from 'markdown-it-anchor'
// @ts-ignore
import attrs from 'markdown-it-attrs'
// @ts-ignore
import container from 'markdown-it-container'
// @ts-ignore
import katex from 'markdown-it-katex'
// @ts-ignore
import mermaidPlugin from 'markdown-it-mermaid'
// @ts-ignore
import taskLists from 'markdown-it-task-lists'
// @ts-ignore
import toc from 'markdown-it-toc-done-right'
// @ts-ignore
import deflist from 'markdown-it-deflist'
// @ts-ignore
import multimdTable from 'markdown-it-multimd-table'
// @ts-ignore
import emoji from 'markdown-it-emoji'
// @ts-ignore
import footnote from 'markdown-it-footnote'

import mermaid from 'mermaid'
import hljs from 'highlight.js'
import 'katex/dist/katex.min.css'
import { debug, warn, error } from './logger'

// 初始化 Mermaid
mermaid.initialize({
  startOnLoad: false,
  theme: 'dark',
  securityLevel: 'loose',
  flowchart: {
    useMaxWidth: true,
    htmlLabels: true,
  },
})

// 创建 markdown-it 实例
const md = new MarkdownIt({
  html: true,        // 启用 HTML 标签支持
  breaks: true,      // 转换 \n 为 <br>
  linkify: true,     // 自动转换 URL 为链接
  typographer: true, // 启用一些语言优化
  highlight: function (str, lang) {
    // Mermaid 由插件处理，或者忽略
    if (lang === 'mermaid') {
      return `<div class="mermaid">${str}</div>`
    }

    if (lang && hljs.getLanguage(lang)) {
      try {
        return '<pre><code class="hljs language-' + lang + '">' +
               hljs.highlight(str, { language: lang, ignoreIllegals: true }).value +
               '</code></pre>';
      } catch (__) {}
    }

    return '<pre><code class="hljs language-' + lang + '">' + md.utils.escapeHtml(str) + '</code></pre>';
  }
})

// ==========================================
// 插件配置
// ==========================================

// 1. 锚点 (必须在 TOC 之前)
md.use(anchor, {
  permalink: anchor.permalink.headerLink()
})

// 2. 目录
md.use(toc)

// 3. 任务列表
md.use(taskLists, {
  label: true,
  labelAfter: true
})

// 4. 数学公式 (KaTeX)
md.use(katex)

// 5. 属性语法 {.class #id}
md.use(attrs)

// 6. 定义列表
md.use(deflist)

// 7. 表格扩展
md.use(multimdTable)

// 8. Emoji
md.use(emoji)

// 9. 脚注
md.use(footnote)

// 10. Mermaid
// 注意：markdown-it-mermaid 通常会接管 mermaid 代码块
// 如果插件未生效，我们的 highlight 函数会作为兜底生成 <div class="mermaid">
md.use(mermaidPlugin)

// 11. 容器 (::: info 等)
const containers = ['info', 'tip', 'warning', 'danger', 'note', 'success']
containers.forEach(name => {
  md.use(container, name)
})

/**
 * 渲染 Markdown 为 HTML
 * @param markdownText Markdown 文本内容
 * @returns 渲染后的 HTML 字符串
 */
export async function renderMarkdown(markdownText: string, basePath?: string): Promise<string> {
  if (!markdownText || !markdownText.trim()) {
    return '<div class="empty-content">暂无内容</div>'
  }
  
  try {
    // 直接使用同步渲染
    // markdown-it 的 render 是同步的，但为了兼容之前的接口保持 async
    return md.render(markdownText)
  } catch (err) {
    error('[Markdown] 渲染失败:', err)
    return `<div class="render-error">
      <h3>渲染错误</h3>
      <pre>${err instanceof Error ? err.message : String(err)}</pre>
      <pre>${md.utils.escapeHtml(markdownText)}</pre>
    </div>`
  }
}

/**
 * 初始化 Mermaid 图表
 * 在 HTML 插入 DOM 后调用
 */
export async function initMermaid() {
  try {
    const nodes = document.querySelectorAll('.mermaid')
    if (nodes.length > 0) {
      await mermaid.run({
        nodes: nodes as any
      })
    }
  } catch (err) {
    error('[Mermaid] 初始化失败:', err)
  }
}

// ==========================================
// 遗留辅助函数 (保持兼容性)
// ==========================================

/**
 * 从 Markdown 文本提取标题
 * (简单的正则实现，用于不需要完整渲染的场景)
 */
export function extractTitle(markdownText: string): string | null {
  if (!markdownText) return null
  const match = markdownText.match(/^#\s+(.+)$/m)
  return match ? match[1] : null
}

/**
 * 从 Markdown 文本提取目录结构
 * (简单的正则实现，用于不需要完整渲染的场景)
 */
export interface TocItem {
  level: number
  id: string
  text: string
  children?: TocItem[]
}

export function extractToc(markdownText: string): TocItem[] {
  // 如果需要基于 markdown-it 生成 TOC 数据，可以使用 markdown-it-toc-done-right 的 callback
  // 这里保留旧的正则实现以兼容 WikiModal
  const toc: TocItem[] = []
  const stack: TocItem[] = []
  const lines = markdownText.split('\n')
  
  // 简单的 slugify
  const slugify = (s: string) => s.toLowerCase().replace(/\s+/g, '-').replace(/[^\w\u4e00-\u9fa5-]/g, '')

  for (const line of lines) {
    const match = line.match(/^(#{1,6})\s+(.+)$/)
    if (!match) continue
    
    const level = match[1].length
    const text = match[2].trim()
    const id = slugify(text)
    
    const item: TocItem = { level, id, text }
    
    while (stack.length > 0 && stack[stack.length - 1].level >= level) {
      stack.pop()
    }
    
    if (stack.length === 0) {
      toc.push(item)
    } else {
      const parent = stack[stack.length - 1]
      if (!parent.children) parent.children = []
      parent.children.push(item)
    }
    stack.push(item)
  }
  
  return toc
}

// 导出 renderMermaidCharts 为 initMermaid 的别名，兼容旧代码
export const renderMermaidCharts = async (_container?: HTMLElement) => {
  await initMermaid()
}
