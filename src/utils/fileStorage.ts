/**
 * JSON文件存储工具
 * 通过Tauri命令读写JSON配置文件
 */

import { getTauriInvoke } from './tauri'

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
    // eslint-disable-next-line no-console
    console.error('Failed to read config file:', error)
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
      // eslint-disable-next-line no-console
      console.log(`调用 Tauri write_config_file 命令: ${filename}`, {
        contentLength: content.length,
        contentPreview: content.substring(0, 200) + '...',
      })
      await invoker('write_config_file', { filename, content })
      // eslint-disable-next-line no-console
      console.log(`✅ Tauri write_config_file 调用成功: ${filename}`)
      return
    }
    // 降级到localStorage（开发环境或非Tauri环境）
    // eslint-disable-next-line no-console
    console.warn('⚠️ Tauri API 不可用，降级到 localStorage')
    localStorage.setItem(`netsec-toolbox_${filename}`, content)
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error('❌ Failed to write config file:', error)
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
      // eslint-disable-next-line no-console
      console.log(`检查配置文件存在性 (Tauri): ${filename}`, exists)
      return exists
    }
    // 降级检查 localStorage
    const exists = !!localStorage.getItem(`netsec-toolbox_${filename}`)
    // eslint-disable-next-line no-console
    console.log(`检查配置文件存在性 (localStorage): ${filename}`, exists)
    return exists
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error('Failed to check config file existence:', error)
    // 降级检查 localStorage
    return !!localStorage.getItem(`netsec-toolbox_${filename}`)
  }
}

