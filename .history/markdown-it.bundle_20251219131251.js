// markdown-it 打包入口文件
// 用于将所有 markdown-it 插件打包成一个浏览器可用的文件

import MarkdownIt from 'markdown-it'
import anchor from 'markdown-it-anchor'
import toc from 'markdown-it-toc-done-right'
import taskLists from 'markdown-it-task-lists'
import attrs from 'markdown-it-attrs'
import footnote from 'markdown-it-footnote'
import { full as emoji } from 'markdown-it-emoji'
import container from 'markdown-it-container'
import hljs from 'highlight.js'

// markdown-it-katex 和 markdown-it-mermaid 是 CommonJS 模块
// 使用动态 import 或 require，esbuild 会自动处理
let katex, mermaid

// 尝试不同的导入方式
try {
  // 方式1: 直接 require
  katex = require('markdown-it-katex')
  if (typeof katex !== 'function' && katex.default) {
    katex = katex.default
  }
} catch (e) {
  console.warn('无法加载 markdown-it-katex:', e)
}

try {
  mermaid = require('markdown-it-mermaid')
  if (typeof mermaid !== 'function' && mermaid.default) {
    mermaid = mermaid.default
  }
} catch (e) {
  console.warn('无法加载 markdown-it-mermaid:', e)
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

// katex 插件（数学公式）
if (katex && typeof katex === 'function') {
  md.use(katex)
} else {
  console.warn('markdown-it-katex 插件未加载，数学公式可能无法渲染')
}

// 自定义容器：tip, info, warning, danger
md.use(container, 'tip')
md.use(container, 'info')
md.use(container, 'warning')
md.use(container, 'danger')
md.use(container, 'note')

// mermaid 插件（图表）
if (mermaid && typeof mermaid === 'function') {
  md.use(mermaid)
} else {
  console.warn('markdown-it-mermaid 插件未加载，Mermaid 图表可能无法渲染')
}

// 导出到全局对象（必须在最后执行）
// 注意：esbuild 会将整个文件包装在 IIFE 中，所以这里的代码会在加载时立即执行
if (typeof window !== 'undefined') {
  window.markdownit = md
  window.MarkdownIt = MarkdownIt
  // 调试信息
  console.debug('markdown-it bundle: 全局变量已设置', { markdownit: !!window.markdownit, MarkdownIt: !!window.MarkdownIt })
}

// 也支持 CommonJS 导出（如果需要，但在这个 ES 模块环境中不需要）


