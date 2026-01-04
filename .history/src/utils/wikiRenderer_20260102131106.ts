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
        // 本地加载失败，尝试 CDN
        const cdnScript = document.createElement('script')
        cdnScript.src = 'https://cdn.jsdelivr.net/npm/highlight.js@11.9.0/lib/core.min.js' // 使用核心库或完整库
        // 这里为了简单直接使用完整库
        cdnScript.src = 'https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11.9.0/build/highlight.min.js'
        
        cdnScript.onload = () => {
          // 加载 CDN CSS
          const link = document.createElement('link')
          link.rel = 'stylesheet'
          link.href = 'https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11.9.0/build/styles/github-dark.min.css'
          document.head.appendChild(link)
          resolve((window as any).hljs)
        }
        
        cdnScript.onerror = () => {
             reject(new Error('无法从本地或 CDN 加载 highlight.js'))
        }
        
        document.head.appendChild(cdnScript)
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
    // 检查是否已经加载了 KaTeX 核心库
    const isKatexLoaded = typeof (window as any).katex !== 'undefined'
    // 检查是否已经加载了 auto-render 扩展
    const isAutoRenderLoaded = typeof (window as any).renderMathInElement !== 'undefined'

    if (isKatexLoaded && isAutoRenderLoaded) {
      resolve()
      return
    }
    
    // 1. 加载 KaTeX 核心库
    const loadCore = () => {
      return new Promise<void>((resolveCore, rejectCore) => {
        if (isKatexLoaded) {
          resolveCore()
          return
        }

        const localPaths = [
          '/katex/katex.min.js',
          '/katex/dist/katex.min.js',
        ]
        
        let currentPathIndex = 0
        
        const tryLoadScript = () => {
          if (currentPathIndex >= localPaths.length) {
            // 本地加载失败，尝试 CDN
            const cdnScript = document.createElement('script')
            cdnScript.src = 'https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js'
            cdnScript.onload = () => {
              loadCSS()
              resolveCore()
            }
            cdnScript.onerror = () => rejectCore(new Error('无法加载 KaTeX 核心库 (CDN)'))
            document.head.appendChild(cdnScript)
            return
          }
          
          const script = document.createElement('script')
          script.src = localPaths[currentPathIndex] || ''
          script.onerror = () => {
            currentPathIndex++
            tryLoadScript()
          }
          script.onload = () => {
            loadCSS()
            resolveCore()
          }
          document.head.appendChild(script)
        }

        const loadCSS = () => {
          const cssPaths = [
            '/katex/katex.min.css',
            '/katex/dist/katex.min.css',
          ]
          // 同时也尝试加载 CDN CSS
          if (currentPathIndex >= localPaths.length) { // 如果是 CDN 加载的脚本
             const link = document.createElement('link')
             link.rel = 'stylesheet'
             link.href = 'https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css'
             document.head.appendChild(link)
             return
          }
          
          let cssPathIndex = 0
          const tryLoadCSS = () => {
            if (cssPathIndex >= cssPaths.length) return
            
            const link = document.createElement('link')
            link.rel = 'stylesheet'
            link.href = cssPaths[cssPathIndex] || ''
            link.onerror = () => {
              cssPathIndex++
              tryLoadCSS()
            }
            document.head.appendChild(link)
          }
          tryLoadCSS()
        }
        
        tryLoadScript()
      })
    }

    // 2. 加载 auto-render 扩展
    const loadAutoRender = () => {
      return new Promise<void>((resolveAR, rejectAR) => {
        if (typeof (window as any).renderMathInElement !== 'undefined') {
          resolveAR()
          return
        }

        const localPaths = [
          '/katex/contrib/auto-render.min.js',
          '/katex/dist/contrib/auto-render.min.js',
        ]

        let currentPathIndex = 0
        
        const tryLoadScript = () => {
          if (currentPathIndex >= localPaths.length) {
             // 本地加载失败，尝试 CDN
            const cdnScript = document.createElement('script')
            cdnScript.src = 'https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/contrib/auto-render.min.js'
            cdnScript.onload = () => resolveAR()
            cdnScript.onerror = () => {
                logError('无法加载 KaTeX auto-render (CDN)')
                // auto-render 加载失败不应阻断核心功能，resolve 即可
                resolveAR() 
            }
            document.head.appendChild(cdnScript)
            return
          }
          
          const script = document.createElement('script')
          script.src = localPaths[currentPathIndex] || ''
          script.onerror = () => {
            currentPathIndex++
            tryLoadScript()
          }
          script.onload = () => resolveAR()
          document.head.appendChild(script)
        }
        
        tryLoadScript()
      })
    }

    // 串行加载：先核心，后扩展
    loadCore()
      .then(() => loadAutoRender())
      .then(() => resolve())
      .catch(err => reject(err))
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
    await loadKaTeX()
    
    // 如果 renderMathInElement 可用（auto-render 扩展），优先使用它
    // 它可以自动处理文本中的公式定界符，无需 Markdown 预先解析
    if (typeof (window as any).renderMathInElement !== 'undefined') {
      try {
        (window as any).renderMathInElement(container, {
          delimiters: [
            { left: '$$', right: '$$', display: true },
            { left: '$', right: '$', display: false },
            { left: '\\(', right: '\\)', display: false },
            { left: '\\[', right: '\\]', display: true },
            { left: '\\begin{equation}', right: '\\end{equation}', display: true },
            { left: '\\begin{align}', right: '\\end{align}', display: true },
            { left: '\\begin{alignat}', right: '\\end{alignat}', display: true },
            { left: '\\begin{gather}', right: '\\end{gather}', display: true },
            { left: '\\begin{CD}', right: '\\end{CD}', display: true },
            { left: '\\[', right: '\\]', display: true }
          ],
          throwOnError: false,
          output: 'html', // 优先使用 HTML 输出，兼容性更好
          strict: false,
          trust: true,
          // 预处理函数（保留之前的清理逻辑）
          preProcess: (math: string) => cleanKaTeXFormula(math)
        })
        return // 如果 auto-render 成功，直接返回
      } catch (err) {
        logError('KaTeX auto-render error:', err)
        // 如果失败，回退到手动处理
      }
    }
    
    if (typeof (window as any).katex === 'undefined') {
      logError('KaTeX 未加载且无法加载')
      return
    }

    // Fallback: 手动处理已标记的元素（如果 Markdown 渲染器已经生成了特定的类名）
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
