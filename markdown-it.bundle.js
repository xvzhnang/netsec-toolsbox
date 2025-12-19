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

// Debug 开关：控制是否输出详细日志
const DEBUG = true

// 调试日志函数
const debugLog = (...args) => {
  if (DEBUG) {
    console.log(...args)
  }
}

const debugWarn = (...args) => {
  if (DEBUG) {
    console.warn(...args)
  }
}

const debugError = (...args) => {
  // 错误始终输出，不受 DEBUG 控制
  console.error(...args)
}

// 解包插件函数
const katex = (katexModule.default || katexModule)

// 统一的 Mermaid 插件解析函数（封装多版本兼容逻辑）
const resolveMermaidPlugin = (mod) => {
  if (!mod) return null
  if (typeof mod === 'function') return mod
  
  // 处理 default 导出（可能是函数或对象）
  if (mod.default) {
    if (typeof mod.default === 'function') {
      return mod.default
    }
    // 如果 default 是对象，递归查找
    if (typeof mod.default === 'object') {
      const resolved = resolveMermaidPlugin(mod.default)
      if (resolved) return resolved
    }
  }
  
  if (mod.plugin && typeof mod.plugin === 'function') return mod.plugin
  
  if (typeof mod === 'object') {
    // 尝试查找任何函数属性
    for (const key in mod) {
      if (key === 'default' || key === 'plugin') continue // 已处理过
      if (typeof mod[key] === 'function') {
        debugLog(`[markdown-it] 找到 mermaid 插件函数: ${key}`)
        return mod[key]
      }
    }
  }
  return null
}

// 使用统一的解析函数
const mermaid = resolveMermaidPlugin(mermaidModule)

// 初始化 markdown-it
const md = new MarkdownIt({
  html: true,           // 允许 HTML 标签
  breaks: true,         // 支持 GitHub 风格的换行
  linkify: true,        // 自动转换 URL 为链接
  typographer: false,   // 不使用智能标点
  highlight: function(code, lang) {
    // 排除 mermaid 语言，让 markdown-it-mermaid 插件处理
    // 对于 mermaid 代码块，返回 null 让 markdown-it 使用默认处理，这样插件可以拦截
    if (lang && lang.toLowerCase() === 'mermaid') {
      return null // 返回 null，让 markdown-it 使用默认处理，插件会拦截
    }
    
    // 获取全局的 hljs（从 public 目录加载的 highlight.js-11.11.1）
    const hljs = (typeof window !== 'undefined' && window.hljs) ? window.hljs : null
    
    // 如果 hljs 未加载，返回 null 让 markdown-it 使用默认处理
    // 这样代码块会有正确的结构，后续可以通过 highlightElement 处理
    if (!hljs) {
      debugWarn('[markdown-it] highlight.js 未加载，代码块将使用默认样式')
      return null
    }
    
    // 转义 HTML 以防止 XSS 攻击
    const escapeHtml = (text) => {
      const div = document.createElement('div')
      div.textContent = text
      return div.innerHTML
    }
    
    // 先转义代码内容（防止 XSS）
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
    
    // 如果指定了语言且 highlight.js 支持，进行高亮
    if (normalizedLang && hljs.getLanguage(normalizedLang)) {
      try {
        const result = hljs.highlight(escapedCode, { language: normalizedLang })
        // 返回高亮后的 HTML
        // markdown-it 会自动包装在 <pre><code> 中，所以我们只返回 <code> 标签内的内容
        // 注意：必须包含 hljs 类和 language-xxx 类，这样后续的 highlightElement 也能识别
        return `<code class="hljs language-${normalizedLang}">${result.value}</code>`
      } catch (err) {
        debugWarn('[markdown-it] highlight.js 高亮失败:', err, '语言:', normalizedLang)
        // 如果高亮失败，返回 null 让 markdown-it 使用默认处理
        return null
      }
    }
    
    // 自动检测语言（仅在未指定语言时）
    // 注意：highlightAuto 可能较慢，大文件建议禁用或缓存结果
    if (!lang) {
      try {
        const result = hljs.highlightAuto(escapedCode)
        const detectedLang = result.language || 'plaintext'
        // 返回高亮后的 HTML，包含 hljs 类和 language-xxx 类
        return `<code class="hljs language-${detectedLang}">${result.value}</code>`
      } catch (err) {
        debugWarn('[markdown-it] highlight.js 自动检测失败:', err)
        // 如果自动检测失败，返回 null 让 markdown-it 使用默认处理
        return null
      }
    }
    
    // 如果语言不支持，返回 null 让 markdown-it 使用默认处理
    // 这样代码块会有正确的结构（<pre><code class="language-xxx">），后续可以通过 highlightElement 处理
    debugWarn('[markdown-it] 语言不支持:', lang, '代码块将使用默认样式，后续可通过 highlightElement 处理')
    return null
  }
})

// 配置并使用插件
// 重要：插件加载顺序很关键
// 1. 先加载 mermaid 插件（必须在 highlight 之前，以便拦截 mermaid 代码块）
// 2. 然后加载其他插件
// 3. 最后加载 katex（数学公式）

// mermaid 插件（图表）- 必须在最前面加载
// markdown-it-mermaid 会将 ```mermaid 代码块转换为 <div class="mermaid"> 或 <pre class="mermaid">
if (mermaid && typeof mermaid === 'function') {
  try {
    md.use(mermaid)
    debugLog('✅ [markdown-it] mermaid 插件已加载')
  } catch (err) {
    debugError('❌ [markdown-it] mermaid 插件加载失败:', err)
    debugError('错误详情:', err.message, err.stack)
  }
} else {
  debugError('❌ [markdown-it] mermaid 插件未加载，Mermaid 图表可能无法渲染')
  debugError('mermaidModule:', mermaidModule)
  debugError('mermaidModule 类型:', typeof mermaidModule)
  if (mermaidModule && typeof mermaidModule === 'object') {
    debugError('mermaidModule 键:', Object.keys(mermaidModule))
  }
}

// 其他插件
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
md.use(attrs, {
  leftDelimiter: '{',
  rightDelimiter: '}',
  allowedAttributes: ['id', 'class', 'style']
})

md.use(footnote)

md.use(emoji)

// katex 插件（数学公式）- 已禁用
// 重要：完全禁用 markdown-it-katex 的自动渲染，避免重复渲染问题
// 
// 问题：markdown-it-katex 会自动调用 katex.render() 渲染公式，生成 KaTeX HTML
// 但 WikiView.vue 中的 renderKaTeXFormulas 又会再次渲染，导致：
// 1. KaTeX 元素重复嵌套
// 2. 公式被渲染 2-3 次
// 3. 错误的 LaTeX 语法被原样保留
// 
// 解决方案：不加载 markdown-it-katex，完全由 WikiView.vue 中的 renderKaTeXFormulas 统一处理
// 正确的渲染链路：
// Markdown 原文（包含 $...$ / $$...$$）
//  ↓
// Markdown 解析（不生成 KaTeX HTML，保留原始公式文本）
//  ↓
// KaTeX 只渲染一次（renderKaTeXFormulas）
//  ↓
// HTML 输出
//  ↓
// 直接展示（不再碰公式）
//
// 注释掉 katex 插件，公式将由 renderKaTeXFormulas 统一处理
/*
if (katex && typeof katex === 'function') {
  try {
    md.use(katex, {
      throwOnError: false,
      errorColor: '#cc0000',
      delimiters: [
        {left: '$$', right: '$$', display: true},
        {left: '$', right: '$', display: false},
        {left: '\\[', right: '\\]', display: true},
        {left: '\\(', right: '\\)', display: false}
      ],
      strict: false
    })
    debugLog('✅ [markdown-it] katex 插件已加载')
  } catch (err) {
    debugError('❌ [markdown-it] katex 插件加载失败:', err)
  }
} else {
  debugError('❌ [markdown-it] katex 插件未加载')
}
*/
debugLog('ℹ️ [markdown-it] katex 插件已禁用，公式将由 WikiView.vue 统一渲染（避免重复渲染）')

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
  debugLog('🚀 [markdown-it bundle] 全局变量已设置', { 
    markdownit: !!window.markdownit, 
    MarkdownIt: !!window.MarkdownIt,
    hasMermaid: !!mermaid,
    hasKatex: !!katex
  })
  
  // 暴露配置接口（可选增强）
  // 允许外部设置 Mermaid 主题、KaTeX 配置等
  if (typeof window !== 'undefined') {
    window.markdownitConfig = {
      setMermaidTheme: (theme) => {
        if (typeof mermaid !== 'undefined') {
          mermaid.initialize({ theme, startOnLoad: false })
          debugLog('✅ [markdown-it] Mermaid 主题已更新:', theme)
        }
      },
      setDebug: (enabled) => {
        // 注意：这个函数在 bundle 中无法直接修改 DEBUG 常量
        // 但可以通过全局变量控制
        window.markdownitDebug = enabled
        debugLog('✅ [markdown-it] Debug 模式:', enabled ? '开启' : '关闭')
      },
      addPlugin: (plugin, options) => {
        if (md && typeof plugin === 'function') {
          md.use(plugin, options)
          debugLog('✅ [markdown-it] 插件已添加')
        } else {
          debugError('❌ [markdown-it] 插件添加失败: 插件必须是函数')
        }
      }
    }
  }
}

// 也支持 CommonJS 导出（如果需要，但在这个 ES 模块环境中不需要）


