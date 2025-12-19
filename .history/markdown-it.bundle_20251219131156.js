// markdown-it 打包入口文件
// 用于将所有 markdown-it 插件打包成一个浏览器可用的文件

import MarkdownIt from 'markdown-it'
import anchor from 'markdown-it-anchor'
import toc from 'markdown-it-toc-done-right'
import taskLists from 'markdown-it-task-lists'
import attrs from 'markdown-it-attrs'
import footnote from 'markdown-it-footnote'
import { full as emoji } from 'markdown-it-emoji'
import hljs from 'highlight.js'

// 这些插件可能需要特殊处理
let katex, container, mermaid

try {
  katex = require('markdown-it-katex')
} catch (e) {
  // 如果 require 失败，尝试 import
  const katexModule = await import('markdown-it-katex')
  katex = katexModule.default || katexModule
}

try {
  container = require('markdown-it-container')
} catch (e) {
  const containerModule = await import('markdown-it-container')
  container = containerModule.default || containerModule
}

try {
  mermaid = require('markdown-it-mermaid')
} catch (e) {
  const mermaidModule = await import('markdown-it-mermaid')
  mermaid = mermaidModule.default || mermaidModule
}

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

// 导出到全局对象（必须在最后执行）
// 注意：esbuild 会将整个文件包装在 IIFE 中，所以这里的代码会在加载时立即执行
if (typeof window !== 'undefined') {
  window.markdownit = md
  window.MarkdownIt = MarkdownIt
  // 调试信息
  console.debug('markdown-it bundle: 全局变量已设置', { markdownit: !!window.markdownit, MarkdownIt: !!window.MarkdownIt })
}

// 也支持 CommonJS 导出（如果需要，但在这个 ES 模块环境中不需要）


