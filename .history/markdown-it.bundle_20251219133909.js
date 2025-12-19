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
// 不使用 npm 包中的 highlight.js，而是使用从 public 目录加载的全局 hljs
// import hljs from 'highlight.js'

// markdown-it-katex 和 markdown-it-mermaid 是 CommonJS 模块
// esbuild 会自动处理 CommonJS 到 ES 模块的转换
// 使用 * as 导入整个模块，然后取 default 或直接使用
import * as katexModule from 'markdown-it-katex'
import * as mermaidModule from 'markdown-it-mermaid'

// 解包插件函数
const katex = (katexModule.default || katexModule)
// markdown-it-mermaid 可能是函数或对象，需要检查
let mermaid = null
if (mermaidModule && typeof mermaidModule === 'function') {
  mermaid = mermaidModule
} else if (mermaidModule && mermaidModule.default && typeof mermaidModule.default === 'function') {
  mermaid = mermaidModule.default
} else if (mermaidModule && typeof mermaidModule === 'object') {
  // 尝试从对象中获取函数
  mermaid = mermaidModule.default || mermaidModule
}

// 初始化 markdown-it
const md = new MarkdownIt({
  html: true,           // 允许 HTML 标签
  breaks: true,         // 支持 GitHub 风格的换行
  linkify: true,        // 自动转换 URL 为链接
  typographer: false,   // 不使用智能标点
  highlight: function(code, lang) {
    // 排除 mermaid 语言，让 markdown-it-mermaid 插件处理
    // 返回 null 让 markdown-it 使用默认处理，这样插件可以拦截
    if (lang && lang.toLowerCase() === 'mermaid') {
      return null
    }
    
    // 获取全局的 hljs（从 public 目录加载的 highlight.js-11.11.1）
    const hljs = (typeof window !== 'undefined' && window.hljs) ? window.hljs : null
    
    // 如果 hljs 未加载，返回转义后的代码
    if (!hljs) {
      const escapeHtml = (text) => {
        const div = document.createElement('div')
        div.textContent = text
        return div.innerHTML
      }
      const escapedCode = escapeHtml(code)
      return `<pre><code>${escapedCode}</code></pre>`
    }
    
    // 转义 HTML 以防止 XSS 攻击
    const escapeHtml = (text) => {
      const div = document.createElement('div')
      div.textContent = text
      return div.innerHTML
    }
    
    // 先转义代码内容
    const escapedCode = escapeHtml(code)
    
    // 处理语言别名
    // 注意：highlight.js 支持 'powershell' 但不支持 'ps1'
    // 所以我们将 powershell/pwsh/ps 都映射到 'powershell'
    const langMap = {
      'ps1': 'powershell',  // ps1 -> powershell
      'pwsh': 'powershell', // pwsh -> powershell
      'ps': 'powershell',   // ps -> powershell
      'powershell': 'powershell', // 保持 powershell
      'shell': 'bash',
      'sh': 'bash',
      'zsh': 'bash',
    }
    const normalizedLang = lang ? (langMap[lang.toLowerCase()] || lang.toLowerCase()) : null
    
    if (normalizedLang && hljs.getLanguage(normalizedLang)) {
      try {
        const result = hljs.highlight(escapedCode, { language: normalizedLang })
        return result.value
      } catch (err) {
        // 如果高亮失败，返回转义后的代码
        return `<pre><code>${escapedCode}</code></pre>`
      }
    }
    
    // 自动检测语言（仅在未指定语言时）
    if (!lang) {
      try {
        const result = hljs.highlightAuto(escapedCode)
        return result.value
      } catch (err) {
        // 如果自动检测失败，返回转义后的代码
        return `<pre><code>${escapedCode}</code></pre>`
      }
    }
    
    // 如果语言不支持，返回转义后的代码
    return `<pre><code>${escapedCode}</code></pre>`
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

// mermaid 插件（图表）
// 注意：mermaid 插件需要在其他插件之前加载，以便正确拦截 mermaid 代码块
// markdown-it-mermaid 会将 ```mermaid 代码块转换为 <div class="mermaid"> 或 <pre class="mermaid">
if (mermaid && typeof mermaid === 'function') {
  md.use(mermaid)
} else {
  console.warn('markdown-it-mermaid 插件未加载，Mermaid 图表可能无法渲染')
}

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

// 导出到全局对象（必须在最后执行）
// 注意：esbuild 会将整个文件包装在 IIFE 中，所以这里的代码会在加载时立即执行
if (typeof window !== 'undefined') {
  window.markdownit = md
  window.MarkdownIt = MarkdownIt
  // 调试信息
  console.debug('markdown-it bundle: 全局变量已设置', { markdownit: !!window.markdownit, MarkdownIt: !!window.MarkdownIt })
}

// 也支持 CommonJS 导出（如果需要，但在这个 ES 模块环境中不需要）


