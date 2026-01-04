import MarkdownIt from 'markdown-it'
import type { Options } from 'markdown-it/lib/index.mjs'
import type Token from 'markdown-it/lib/token.mjs'
import type Renderer from 'markdown-it/lib/renderer.mjs'
import type { RenderRule } from 'markdown-it/lib/renderer.mjs'
import hljs from 'highlight.js'
import 'highlight.js/styles/github-dark.css'
import markdownItTexmath from 'markdown-it-texmath'
import katex from 'katex'
import 'katex/dist/katex.min.css'
import markdownItTaskLists from 'markdown-it-task-lists'
import markdownItContainer from 'markdown-it-container'
import markdownItAttrs from 'markdown-it-attrs'
import { full as markdownItEmoji } from 'markdown-it-emoji'
import markdownItAnchor from 'markdown-it-anchor'
// @ts-ignore - no types available
import markdownItTocDoneRight from 'markdown-it-toc-done-right'

let markdown: MarkdownIt

const initMarkdown = () => {
  if (markdown) return

  markdown = new MarkdownIt({
    html: true, // Enable HTML for custom rendering if needed
    linkify: true,
    breaks: true,
    highlight: (code: string, language: string, attrs: string): string => {
      void attrs
      const lang = (language || '').trim().toLowerCase()
      
      // Mermaid handling
      if (lang === 'mermaid') {
        return `<div class="mermaid">${code}</div>`
      }

      if (lang && hljs.getLanguage(lang)) {
        const highlighted = hljs.highlight(code, { language: lang, ignoreIllegals: true }).value
        return `<pre class="hljs"><code class="hljs language-${markdown.utils.escapeHtml(lang)}">${highlighted}</code></pre>`
      }
      return `<pre class="hljs"><code class="hljs">${markdown.utils.escapeHtml(code)}</code></pre>`
    },
  })

  // Plugins
  markdown.use(markdownItTexmath, {
    engine: katex,
    delimiters: 'dollars',
    katexOptions: { macros: { "\\RR": "\\mathbb{R}" } }
  })
  
  markdown.use(markdownItTaskLists, {
    enabled: true,
    label: true,
    labelAfter: true,
  })

  markdown.use(markdownItAttrs)
  markdown.use(markdownItEmoji)
  markdown.use(markdownItAnchor, {
    permalink: markdownItAnchor.permalink.headerLink()
  })
  markdown.use(markdownItTocDoneRight, {
    listType: 'ul',
    level: [1, 2, 3],
  })

  // Container support
  const containers = ['info', 'warning', 'danger', 'tip', 'success', 'details']
  containers.forEach(name => {
    (markdown as any).use(markdownItContainer as any, name, {
      validate: (params: string) => params.trim().match(new RegExp(`^${name}\\s*(.*)$`)),
      render: (tokens: Token[], idx: number) => {
        const t = tokens[idx]!
        const m = t.info.trim().match(new RegExp(`^${name}\\s*(.*)$`))
        if (t.nesting === 1) {
          return `<div class="${name} custom-block">\n` +
                 (m && m[1] ? `<p class="custom-block-title">${markdown.utils.escapeHtml(m[1])}</p>\n` : '')
        } else {
          return '</div>\n'
        }
      }
    })
  })

  markdown.validateLink = (url: string): boolean => {
    const normalized = url.trim().toLowerCase()
    if (normalized.startsWith('#') || normalized.startsWith('/')) return true
    return (
      normalized.startsWith('http://') ||
      normalized.startsWith('https://') ||
      normalized.startsWith('mailto:')
    )
  }

  const defaultLinkOpen: RenderRule =
    markdown.renderer.rules.link_open ??
    ((tokens: Token[], idx: number, options: Options, env: any, self: Renderer): string => {
      void env
      return self.renderToken(tokens, idx, options)
    })

  markdown.renderer.rules.link_open = (
    tokens: Token[],
    idx: number,
    options: Options,
    env: any,
    self: Renderer
  ): string => {
    void env
    const token = tokens[idx]
    if (token) {
      token.attrSet('rel', 'noopener noreferrer')
      token.attrSet('target', '_blank')
    }
    return defaultLinkOpen(tokens, idx, options, env, self)
  }
}

export const renderMarkdown = (text: string): string => {
  if (!markdown) {
    initMarkdown()
  }
  return markdown.render(text || '')
}
