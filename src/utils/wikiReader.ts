/**
 * Wiki 内容读取工具
 * 用于 AI 助手读取 Wiki 文档内容作为上下文
 */

import { getTauriInvoke } from './tauri'
import { debug, error as logError } from './logger'

/**
 * 读取 Wiki 文件内容
 */
export async function readWikiFile(filePath: string): Promise<string> {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      throw new Error('Tauri API 不可用')
    }
    
    const content = await invoker<string>('read_wiki_file', { filePath })
    return content
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    logError('读取 Wiki 文件失败:', errorMsg)
    throw error
  }
}

/**
 * 搜索 Wiki 内容
 */
export async function searchWiki(query: string): Promise<string> {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      throw new Error('Tauri API 不可用')
    }
    
    const results = await invoker<any[]>('search_wiki', { query })
    
    // 将搜索结果转换为文本
    if (!results || results.length === 0) {
      return ''
    }
    
    // 读取前 3 个结果的内容
    const contents: string[] = []
    for (let i = 0; i < Math.min(3, results.length); i++) {
      const result = results[i]
      if (result.file_path) {
        try {
          const content = await readWikiFile(result.file_path)
          contents.push(`## ${result.title || result.file_path}\n\n${content.substring(0, 2000)}...`)
        } catch (e) {
          debug('读取搜索结果文件失败:', result.file_path, e)
        }
      }
    }
    
    return contents.join('\n\n---\n\n')
  } catch (error) {
    logError('搜索 Wiki 失败:', error)
    return ''
  }
}

/**
 * 根据工具 ID 查找相关的 Wiki 内容
 */
export async function getWikiForTool(toolId: string): Promise<string> {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      return ''
    }
    
    const wikiPath = await invoker<string>('find_wiki_for_tool', { toolId })
    if (!wikiPath) {
      return ''
    }
    
    return await readWikiFile(wikiPath)
  } catch (error) {
    debug('获取工具 Wiki 失败:', error)
    return ''
  }
}

