import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'
import { error as logError, debug } from './logger'

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
