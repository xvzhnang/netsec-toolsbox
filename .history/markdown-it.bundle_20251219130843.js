// markdown-it 打包入口文件
// 用于将所有 markdown-it 插件打包成一个浏览器可用的文件

import MarkdownIt from 'markdown-it'
import anchor from 'markdown-it-anchor'
import toc from 'markdown-it-toc-done-right'
import taskLists from 'markdown-it-task-lists'
import attrs from 'markdown-it-attrs'
import footnote from 'markdown-it-footnote'
import { full as emoji } from 'markdown-it-emoji'
import katex from 'markdown-it-katex'
import container from 'markdown-it-container'
import mermaid from 'markdown-it-mermaid'
import hljs from 'highlight.js'

// 初始化 markdown-it
const md = new MarkdownIt({
  html: true,           // 允许 HTML 标签
  breaks: true,         // 支持 GitHub 风格的换行
  linkify: true,        // 自动转换 URL 为链接
  typographer: false,   // 不使用智能标点
  highlight: function(code, lang) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return hljs.highlight(code, { language: lang }).value
      } catch (err) {
        return code
      }
    }
    // 自动检测语言
    try {
      return hljs.highlightAuto(code).value
    } catch (err) {
      return code
    }
  }
})

// 配置并使用插件
md.use(anchor, {
  permalink: anchor.permalink.headerLink(),
  level: [1, 2, 3, 4, 5, 6]
})

md.use(toc, {
  containerClass: 'table-of-contents',
  listType: 'ul',
  level: [1, 2, 3, 4, 5, 6]
})

md.use(taskLists, {
  enabled: true,
  label: true
})

md.use(attrs)

md.use(footnote)

md.use(emoji)

md.use(katex)

// 自定义容器：tip, info, warning, danger
md.use(container, 'tip')
md.use(container, 'info')
md.use(container, 'warning')
md.use(container, 'danger')
md.use(container, 'note')

md.use(mermaid)

// 导出到全局对象（必须在最后执行，确保在 IIFE 中也能访问）
if (typeof window !== 'undefined') {
  window.markdownit = md
  window.MarkdownIt = MarkdownIt
}

// 确保在模块加载后立即设置（防止异步问题）
if (typeof window !== 'undefined') {
  // 使用立即执行确保设置
  (function() {
    window.markdownit = md
    window.MarkdownIt = MarkdownIt
  })()
}

// 也支持 CommonJS 导出（如果需要，但在这个 ES 模块环境中不需要）


