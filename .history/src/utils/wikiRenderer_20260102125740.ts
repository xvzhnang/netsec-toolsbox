import { debug, error as logError } from './logger'
import { openUrlInBrowser } from './tauri'

// ==========================================
// Highlight.js Logic
// ==========================================

// 加载 highlight.js（仅从 public 目录，不使用 CDN）
export const loadHighlightJS = (): Promise<any> => {
  return new Promise((resolve, reject) => {
    // 检查是否已经加载
    if (typeof (window as any).hljs !== 'undefined') {
      resolve((window as any).hljs)
      return
    }
    
    // 仅从 public 目录加载构建好的文件
    const localPaths = [
      '/highlight.js-11.11.1/build/highlight.min.js',
      '/highlight.js-11.11.1/build/highlight.js',
      '/highlight.js-11.11.1/dist/highlight.min.js',
      '/highlight.min.js'
    ]
    
    let currentPathIndex = 0
    
    const tryLoadLocal = () => {
      if (currentPathIndex >= localPaths.length) {
        reject(new Error('无法从本地加载 highlight.js，请确保文件存在于 public 目录'))
        return
      }
      
      const script = document.createElement('script')
      const path = localPaths[currentPathIndex]
      if (!path) {
        currentPathIndex++
        tryLoadLocal()
        return
      }
      
      script.src = path
      script.onerror = () => {
        currentPathIndex++
        tryLoadLocal()
      }
      script.onload = () => {
        // 加载 CSS
        const cssPaths = [
          '/highlight.js-11.11.1/build/demo/styles/github-dark.css',
          '/highlight.js-11.11.1/src/styles/github-dark.css',
          '/highlight.js-11.11.1/src/styles/github-dark.min.css',
          '/highlight.js-11.11.1/build/demo/styles/github-dark-dimmed.css',
          '/github-dark.css'
        ]
        
        let cssIndex = 0
        const tryLoadCSS = () => {
          if (cssIndex >= cssPaths.length) {
            // CSS 加载失败也继续
            resolve((window as any).hljs)
            return
          }
          
          const link = document.createElement('link')
          link.rel = 'stylesheet'
          const cssPath = cssPaths[cssIndex]
          if (!cssPath) {
            cssIndex++
            tryLoadCSS()
            return
          }
          link.href = cssPath
          link.onerror = () => {
            cssIndex++
            tryLoadCSS()
          }
          link.onload = () => {
            resolve((window as any).hljs)
          }
          document.head.appendChild(link)
        }
        tryLoadCSS()
      }
      document.head.appendChild(script)
    }
    
    tryLoadLocal()
  })
}

// 应用代码高亮
export const applyCodeHighlighting = async (container: HTMLElement) => {
  let hljs: any = null
  try {
    hljs = await loadHighlightJS()
  } catch (err) {
    logError('无法加载 highlight.js:', err)
    return
  }
  
  if (!hljs) return
  
  const codeBlocks = container.querySelectorAll('pre code:not(.hljs):not(.mermaid), pre:not(.mermaid) code:not(.hljs)')
  
  codeBlocks.forEach((codeElement, index) => {
    if (codeElement.closest('.mermaid') || codeElement.classList.contains('mermaid')) {
      return
    }
    try {
      const langMap: Record<string, string> = {
        'ps1': 'powershell',
        'pwsh': 'powershell',
        'ps': 'powershell',
        'powershell': 'powershell',
        'shell': 'bash',
        'sh': 'bash',
        'zsh': 'bash',
      }
      
      const classList = codeElement.classList
      for (const className of classList) {
        if (className.startsWith('language-')) {
          const lang = className.replace('language-', '')
          const normalizedLang = langMap[lang.toLowerCase()] || lang
          
          if (normalizedLang !== lang) {
            classList.remove(className)
            classList.add(`language-${normalizedLang}`)
          }
          break
        }
      }
      
      if (codeElement.innerHTML && !codeElement.classList.contains('hljs')) {
        const textContent = codeElement.textContent || (codeElement as HTMLElement).innerText || ''
        if (textContent && codeElement.innerHTML !== textContent) {
          codeElement.textContent = textContent
        }
      }
      
      hljs.highlightElement(codeElement as HTMLElement)
    } catch (err) {
      debug(`代码块 ${index} 高亮失败`)
    }
  })
}

// ==========================================
// Copy Button Logic
// ==========================================

export const addCopyButtonsToCodeBlocks = (container: HTMLElement) => {
  const codeBlocks = container.querySelectorAll('pre code')
  
  codeBlocks.forEach((codeElement) => {
    const preElement = codeElement.parentElement as HTMLElement
    if (!preElement || preElement.classList.contains('has-copy-button')) {
      return
    }
    
    preElement.classList.add('has-copy-button')
    preElement.style.position = 'relative'
    
    const copyButton = document.createElement('button')
    copyButton.className = 'code-copy-button'
    copyButton.innerHTML = '<span class="copy-icon">📋</span><span class="copy-text">复制</span>'
    copyButton.title = '复制代码'
    
    const codeText = codeElement.textContent || ''
    
    copyButton.addEventListener('click', async (e) => {
      e.stopPropagation()
      e.preventDefault()
      
      try {
        await navigator.clipboard.writeText(codeText)
        copyButton.innerHTML = '<span class="copy-icon">✓</span><span class="copy-text">已复制</span>'
        copyButton.classList.add('copied')
        setTimeout(() => {
          copyButton.innerHTML = '<span class="copy-icon">📋</span><span class="copy-text">复制</span>'
          copyButton.classList.remove('copied')
        }, 2000)
      } catch (err) {
        // 降级方案
        const textArea = document.createElement('textarea')
        textArea.value = codeText
        textArea.style.position = 'fixed'
        textArea.style.opacity = '0'
        document.body.appendChild(textArea)
        textArea.focus()
        textArea.select()
        try {
          document.execCommand('copy')
          copyButton.innerHTML = '<span class="copy-icon">✓</span><span class="copy-text">已复制</span>'
          copyButton.classList.add('copied')
          setTimeout(() => {
            copyButton.innerHTML = '<span class="copy-icon">📋</span><span class="copy-text">复制</span>'
            copyButton.classList.remove('copied')
          }, 2000)
        } catch (e) {
          logError('复制失败:', e)
        }
        document.body.removeChild(textArea)
      }
    })
    
    preElement.appendChild(copyButton)
  })
}

// ==========================================
// Link Processing Logic
// ==========================================

export const processLinks = (container: HTMLElement, loadFileCallback?: (path: string) => void) => {
  const links = container.querySelectorAll('a')
  
  links.forEach((link) => {
    const href = link.getAttribute('href')
    if (!href) return
    
    if (href.startsWith('http://') || href.startsWith('https://')) {
      link.classList.add('external-link')
      if (!link.querySelector('.external-link-icon')) {
        const icon = document.createElement('span')
        icon.className = 'external-link-icon'
        icon.innerHTML = '↗'
        icon.title = '外部链接'
        link.appendChild(icon)
      }
      
      link.addEventListener('click', async (e) => {
        e.preventDefault()
        e.stopPropagation()
        try {
          await openUrlInBrowser(href)
        } catch (err) {
          logError('打开链接失败:', err)
          window.open(href, '_blank', 'noopener,noreferrer')
        }
      })
    }
    else if (link.classList.contains('wiki-internal-link') && loadFileCallback) {
      link.classList.add('internal-link')
      link.addEventListener('click', (e) => {
        e.preventDefault()
        const targetPath = (link as HTMLElement).dataset.wikiLink
        if (targetPath) {
          loadFileCallback(targetPath)
        }
      })
    }
  })
}

// ==========================================
// Collapsible Blocks Logic
// ==========================================

export const initCollapsibleBlocks = (container: HTMLElement) => {
  const collapsibles = container.querySelectorAll('.collapsible-block')
  collapsibles.forEach((block) => {
    const header = block.querySelector('.collapsible-header') as HTMLElement
    const content = block.querySelector('.collapsible-content') as HTMLElement
    const icon = block.querySelector('.collapsible-icon') as HTMLElement
    
    if (header && content && icon) {
      content.style.display = 'none'
      block.classList.add('collapsed')
      
      header.addEventListener('click', () => {
        const isCollapsed = block.classList.contains('collapsed')
        block.classList.toggle('collapsed')
        content.style.display = isCollapsed ? 'block' : 'none'
        icon.textContent = isCollapsed ? '▼' : '▶'
      })
    }
  })
}

// ==========================================
// KaTeX Logic
// ==========================================

const loadKaTeX = (): Promise<void> => {
  return new Promise((resolve, reject) => {
    // 检查是否已经加载
    if (typeof (window as any).katex !== 'undefined' && typeof (window as any).renderMathInElement !== 'undefined') {
      resolve()
      return
    }
    
    // 定义资源路径（本地优先，CDN 兜底）
    const resources = [
      // KaTeX 主库
      {
        type: 'script',
        local: ['/katex/katex.min.js', '/katex/dist/katex.min.js'],
        cdn: 'https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js',
        check: () => typeof (window as any).katex !== 'undefined'
      },
      // KaTeX CSS
      {
        type: 'style',
        local: ['/katex/katex.min.css', '/katex/dist/katex.min.css'],
        cdn: 'https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css',
        check: () => true // CSS 难以检测，假设成功
      },
      // Auto-render 扩展
      {
        type: 'script',
        local: ['/katex/contrib/auto-render.min.js', '/katex/dist/contrib/auto-render.min.js'],
        cdn: 'https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/contrib/auto-render.min.js',
        check: () => typeof (window as any).renderMathInElement !== 'undefined'
      }
    ]

    // 辅助函数：加载单个资源
    const loadResource = (res: any): Promise<void> => {
      return new Promise((resResolve, resReject) => {
        if (res.check && res.check()) {
          resResolve()
          return
        }

        // 尝试本地加载
        let localIndex = 0
        const tryLocal = () => {
          if (localIndex >= res.local.length) {
            // 本地失败，尝试 CDN
            tryCDN()
            return
          }

          const element = res.type === 'script' ? document.createElement('script') : document.createElement('link')
          if (res.type === 'script') {
            (element as HTMLScriptElement).src = res.local[localIndex]
          } else {
            (element as HTMLLinkElement).rel = 'stylesheet';
            (element as HTMLLinkElement).href = res.local[localIndex]
          }
          
          element.onload = () => resResolve()
          element.onerror = () => {
            localIndex++
            tryLocal()
          }
          document.head.appendChild(element)
        }

        // 尝试 CDN 加载
        const tryCDN = () => {
          if (!res.cdn) {
            resReject(new Error(`无法加载资源且无 CDN 备份`))
            return
          }
          
          const element = res.type === 'script' ? document.createElement('script') : document.createElement('link')
          if (res.type === 'script') {
            (element as HTMLScriptElement).src = res.cdn
            // 设置跨域属性
            ;(element as HTMLScriptElement).crossOrigin = "anonymous"
          } else {
            (element as HTMLLinkElement).rel = 'stylesheet';
            (element as HTMLLinkElement).href = res.cdn;
            (element as HTMLLinkElement).crossOrigin = "anonymous"
          }

          element.onload = () => resResolve()
          element.onerror = (e) => resReject(new Error(`CDN 加载失败: ${res.cdn}`))
          document.head.appendChild(element)
        }

        tryLocal()
      })
    }

    // 串行加载所有资源（确保依赖顺序：先主库，再扩展）
    // 使用 reduce 串行执行 Promise
    resources.reduce((promise, res) => {
      return promise.then(() => loadResource(res))
    }, Promise.resolve())
    .then(() => resolve())
    .catch(err => {
      logError('KaTeX 资源加载失败:', err)
      // 即使部分失败也尝试继续，可能主库加载成功了但扩展失败
      resolve() 
    })
  })
}

const cleanKaTeXFormula = (formula: string): string => {
  if (!formula) return ''
  formula = formula.replace(/[\u200B-\u200D\uFEFF]/g, '')
  formula = formula.replace(/[\u0000-\u001F\u007F-\u009F]/g, '')
  
  const unicodeToLatex: Record<string, string> = {
    '√': '\\sqrt',
    'π': '\\pi',
    '∞': '\\infty',
    '∫': '\\int',
    '∑': '\\sum',
    '∏': '\\prod',
    'α': '\\alpha',
    'β': '\\beta',
    'γ': '\\gamma',
    'δ': '\\delta',
    'ε': '\\epsilon',
    'θ': '\\theta',
    'λ': '\\lambda',
    'μ': '\\mu',
    'σ': '\\sigma',
    'φ': '\\phi',
    'ω': '\\omega',
    'Δ': '\\Delta',
    'Ω': '\\Omega',
    '≤': '\\leq',
    '≥': '\\geq',
    '≠': '\\neq',
    '≈': '\\approx',
    '±': '\\pm',
    '×': '\\times',
    '÷': '\\div',
    '→': '\\rightarrow',
    '←': '\\leftarrow',
    '⇒': '\\Rightarrow',
    '⇐': '\\Leftarrow',
    '∈': '\\in',
    '∉': '\\notin',
    '⊂': '\\subset',
    '⊃': '\\supset',
    '∪': '\\cup',
    '∩': '\\cap',
    '∅': '\\emptyset',
    '∀': '\\forall',
    '∃': '\\exists',
    '∂': '\\partial',
    '∇': '\\nabla',
    'ℵ': '\\aleph',
    'ℜ': '\\Re',
    'ℑ': '\\Im',
  }
  
  for (const [unicode, latex] of Object.entries(unicodeToLatex)) {
    if (unicode === '√') {
      formula = formula.replace(/√(\w+|\([^)]+\))/g, (_match, content) => {
        if (content.startsWith('(')) {
          return `\\sqrt${content}`
        } else {
          return `\\sqrt{${content}}`
        }
      })
      formula = formula.replace(/√/g, '\\sqrt{}')
    } else {
      formula = formula.replace(new RegExp(unicode, 'g'), latex)
    }
  }
  
  return formula.replace(/\s+/g, ' ').trim()
}

export const renderKaTeX = async (container: HTMLElement) => {
  try {
    if (typeof (window as any).katex === 'undefined') {
      await loadKaTeX()
    }
    
    // Process block formulas
    const displayMathElements = container.querySelectorAll('.katex-block, .katex-display')
    displayMathElements.forEach((el) => {
      const tex = el.textContent || ''
      try {
        (window as any).katex.render(cleanKaTeXFormula(tex), el as HTMLElement, {
          displayMode: true,
          throwOnError: false,
          output: 'html',
        })
      } catch (err) {
        logError('KaTeX render error:', err)
      }
    })
    
    // Process inline formulas
    const inlineMathElements = container.querySelectorAll('.katex-inline')
    inlineMathElements.forEach((el) => {
      const tex = el.textContent || ''
      try {
        (window as any).katex.render(cleanKaTeXFormula(tex), el as HTMLElement, {
          displayMode: false,
          throwOnError: false,
          output: 'html',
        })
      } catch (err) {
        logError('KaTeX render error:', err)
      }
    })
  } catch (err) {
    logError('无法加载 KaTeX:', err)
  }
}
