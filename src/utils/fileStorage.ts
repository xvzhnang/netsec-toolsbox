/**
 * JSON文件存储工具
 * 通过Tauri命令读写JSON配置文件
 */

import { getTauriInvoke } from './tauri'
import { debug, error as logError, info, warn } from './logger'

/**
 * 读取配置文件
 * @param filename 文件名（如 'categories.json', 'tools.json', 'ai.json'）
 */
export async function readConfigFile(filename: string = 'categories.json'): Promise<string> {
  try {
    const invoker = getTauriInvoke()
    if (invoker) {
      return await invoker<string>('read_config_file', { filename })
    }
    // 降级到localStorage（开发环境或非Tauri环境）
    return localStorage.getItem(`netsec-toolbox_${filename}`) || '{}'
  } catch (error) {
    logError('Failed to read config file:', error)
    // 降级到localStorage
    return localStorage.getItem(`netsec-toolbox_${filename}`) || '{}'
  }
}

/**
 * 写入配置文件
 * @param filename 文件名（如 'categories.json', 'tools.json', 'ai.json'）
 * @param content 文件内容
 */
export async function writeConfigFile(filename: string, content: string): Promise<void> {
  try {
    const invoker = getTauriInvoke()
    if (invoker) {
      // 获取实际配置文件路径（用于调试）
      try {
        const configPath = await invoker<string>('get_config_file_path', { filename })
        debug(`配置文件路径: ${configPath}`)
      } catch (err) {
        debug('无法获取配置文件路径:', err)
      }
      
      debug(`调用 Tauri write_config_file 命令: ${filename}`, {
        contentLength: content.length,
        contentPreview: content.substring(0, 200) + '...',
      })
      await invoker('write_config_file', { filename, content })
      info(`✅ Tauri write_config_file 调用成功: ${filename}`)
      return
    }
    // 降级到localStorage（开发环境或非Tauri环境）
    warn('⚠️ Tauri API 不可用，降级到 localStorage')
    localStorage.setItem(`netsec-toolbox_${filename}`, content)
  } catch (error) {
    logError('❌ Failed to write config file:', error)
    // 降级到localStorage
    localStorage.setItem(`netsec-toolbox_${filename}`, content)
  }
}

/**
 * 检查配置文件是否存在
 * @param filename 文件名
 * @returns true 如果文件存在，false 如果不存在
 */
export async function configFileExists(filename: string = 'categories.json'): Promise<boolean> {
  try {
    const invoker = getTauriInvoke()
    if (invoker) {
      const exists = await invoker<boolean>('config_file_exists', { filename })
      debug(`检查配置文件存在性 (Tauri): ${filename}`, exists)
      return exists
    }
    // 降级检查 localStorage
    const exists = !!localStorage.getItem(`netsec-toolbox_${filename}`)
    debug(`检查配置文件存在性 (localStorage): ${filename}`, exists)
    return exists
  } catch (error) {
    logError('Failed to check config file existence:', error)
    // 降级检查 localStorage
    return !!localStorage.getItem(`netsec-toolbox_${filename}`)
  }
}

/**
 * 保存 base64 图标到 icons 目录并返回相对路径
 * @param iconData base64 数据 URL 或纯 base64 字符串
 * @returns Promise<string> 相对路径（如 ".config/icons/xxx.png"）
 */
export async function saveIconToCache(iconData: string): Promise<string> {
  try {
    const invoker = getTauriInvoke()
    if (invoker) {
      const iconPath = await invoker<string>('save_icon_to_cache', { iconData })
      debug(`图标已保存到缓存: ${iconPath}`)
      return iconPath
    }
    // 降级：返回原始 base64（非 Tauri 环境）
    warn('⚠️ Tauri API 不可用，无法保存图标到缓存')
    return iconData
  } catch (error) {
    logError('❌ Failed to save icon to cache:', error)
    // 降级：返回原始 base64
    return iconData
  }
}

