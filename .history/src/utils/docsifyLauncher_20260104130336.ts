import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'
import { error as logError, debug } from './logger'

/**
 * 获取 Docsify 服务的基础 URL
 */
export const getDocsifyBaseUrl = async (): Promise<string> => {
  const port = await invoke<number>('start_docs_server')
  return `http://localhost:${port}`
}

/**
 * 获取指定路径的 Wiki Markdown 内容
 * @param path - 文档路径 (例如 "tools/nmap")
 */
export const fetchWikiContent = async (path: string): Promise<string> => {
  try {
    const baseUrl = await getDocsifyBaseUrl()
    // 假设文件扩展名为 .md，如果 path 没有后缀，尝试添加
    let fileUrl = `${baseUrl}/${path}`
    if (!fileUrl.toLowerCase().endsWith('.md')) {
      fileUrl += '.md'
    }
    
    debug('Fetching Wiki content from:', fileUrl)
    const response = await fetch(fileUrl)
    if (!response.ok) {
      throw new Error(`Failed to fetch wiki content: ${response.statusText}`)
    }
    return await response.text()
  } catch (err) {
    logError('Failed to fetch wiki content:', err)
    throw err
  }
}

/**
 * 启动 Docsify 服务器并打开指定的文档路径
 * @param path - 文档路径 (可选)，例如 "tools/nmap" 或完整 URL
 */
export const openDocsify = async (path?: string) => {
  try {
    debug('Starting Docsify server...')
    const port = await invoke<number>('start_docs_server')
    let url = `http://localhost:${port}`

    if (path) {
      if (path.startsWith('http')) {
        url = path
      } else {
        // Docsify 使用 hash 路由
        // 处理相对路径，确保不以 / 开头
        const cleanPath = path.startsWith('/') ? path.slice(1) : path
        url = `${url}/#/${cleanPath}`
      }
    }

    debug('Opening Docsify URL:', url)
    await open(url)
  } catch (err) {
    logError('Failed to open Docsify:', err)
    throw err
  }
}
