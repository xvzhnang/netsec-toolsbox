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
// markdown-it-mermaid 0.2.5 是 CommonJS 模块，可能导出为 default 或直接导出
let mermaid = null
if (typeof mermaidModule === 'function') {
  mermaid = mermaidModule
} else if (mermaidModule && typeof mermaidModule.default === 'function') {
  mermaid = mermaidModule.default
} else if (mermaidModule && typeof mermaidModule === 'object') {
  // 尝试从对象中获取函数
  // 检查是否有 plugin 属性
  if (mermaidModule.plugin && typeof mermaidModule.plugin === 'function') {
    mermaid = mermaidModule.plugin
  } else if (mermaidModule.default) {
    mermaid = mermaidModule.default
  } else {
    // 尝试直接使用对象（某些版本可能直接导出对象）
    mermaid = mermaidModule
  }
}

// 初始化 markdown-it
const md = new MarkdownIt({
  html: true,           // 允许 HTML 标签
  breaks: true,         // 支持 GitHub 风格的换行
  linkify: true,        // 自动转换 URL 为链接
  typographer: false,   // 不使用智能标点
  highlight: function(code, lang) {
    // 排除 mermaid 语言，让 markdown-it-mermaid 插件处理
    // 对于 mermaid 代码块，返回空字符串，让插件处理
    if (lang && lang.toLowerCase() === 'mermaid') {
      return '' // 返回空字符串，让 markdown-it-mermaid 插件处理
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

// markdown-it-attrs 插件：支持 {.class #id} 语法
// 配置选项：允许在行尾添加属性
md.use(attrs, {
  // 允许的属性分隔符
  leftDelimiter: '{',
  rightDelimiter: '}',
  // 允许的属性格式：{.class} {#id} {.class #id} {.class1 .class2}
  allowedAttributes: ['id', 'class', 'style']
})

md.use(footnote)

md.use(emoji)

// mermaid 插件（图表）
// 注意：mermaid 插件需要在其他插件之前加载，以便正确拦截 mermaid 代码块
// markdown-it-mermaid 会将 ```mermaid 代码块转换为 <div class="mermaid"> 或 <pre class="mermaid">
if (mermaid) {
  try {
    if (typeof mermaid === 'function') {
      md.use(mermaid)
      console.debug('markdown-it-mermaid 插件已加载（函数形式）')
    } else if (typeof mermaid === 'object' && typeof mermaid.default === 'function') {
      md.use(mermaid.default)
      console.debug('markdown-it-mermaid 插件已加载（对象.default）')
    } else if (typeof mermaid === 'object' && typeof mermaid.plugin === 'function') {
      md.use(mermaid.plugin)
      console.debug('markdown-it-mermaid 插件已加载（对象.plugin）')
    } else {
      console.warn('markdown-it-mermaid 插件格式不正确:', typeof mermaid, mermaid)
    }
  } catch (err) {
    console.error('markdown-it-mermaid 插件加载失败:', err)
  }
} else {
  console.warn('markdown-it-mermaid 插件未加载，Mermaid 图表可能无法渲染')
  console.warn('mermaidModule:', mermaidModule)
}

// katex 插件（数学公式）
// 配置选项：支持块级公式（$$...$$）和行内公式（$...$）
if (katex && typeof katex === 'function') {
  md.use(katex, {
    throwOnError: false,
    errorColor: '#cc0000',
    delimiters: [
      {left: '$$', right: '$$', display: true},   // 块级公式
      {left: '$', right: '$', display: false},    // 行内公式
      {left: '\\[', right: '\\]', display: true}, // LaTeX 块级
      {left: '\\(', right: '\\)', display: false} // LaTeX 行内
    ]
  })
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


