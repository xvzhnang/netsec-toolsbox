import MarkdownIt from 'markdown-it'
import type { Options } from 'markdown-it/lib/index.mjs'
import type Token from 'markdown-it/lib/token.mjs'
import type Renderer from 'markdown-it/lib/renderer.mjs'
import type { RenderRule } from 'markdown-it/lib/renderer.mjs'
import hljs from 'highlight.js'
import 'highlight.js/styles/github-dark.css'

let markdown: MarkdownIt

const initMarkdown = () => {
  if (markdown) return

  markdown = new MarkdownIt({
    html: false,
    linkify: true,
    breaks: true,
    highlight: (code: string, language: string, attrs: string): string => {
      void attrs
      const lang = (language || '').trim().toLowerCase()
      if (lang && hljs.getLanguage(lang)) {
        const highlighted = hljs.highlight(code, { language: lang, ignoreIllegals: true }).value
        return `<pre class="hljs"><code class="hljs language-${markdown.utils.escapeHtml(lang)}">${highlighted}</code></pre>`
      }
      return `<pre class="hljs"><code class="hljs">${markdown.utils.escapeHtml(code)}</code></pre>`
    },
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
