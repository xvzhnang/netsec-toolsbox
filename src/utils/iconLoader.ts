/**
 * 图标加载工具函数
 * 用于将图标路径转换为可用的 URL
 */

import { getTauriInvoke } from './tauri'
import { debug, error as logError } from './logger'

/**
 * 转换图标路径为可用的 URL
 * 支持：
 * - base64 数据 URL（直接返回）
 * - 相对路径（如 "icons/xxx.png"）- 转换为 file:// 协议
 * - 绝对路径 - 转换为 file:// 协议
 * - HTTP/HTTPS URL（直接返回）
 * @param iconPath 图标路径
 * @returns 可用的图标 URL
 */
export function convertIconPath(iconPath: string | undefined | null): string | undefined {
  if (!iconPath) {
    return undefined
  }
  
  // base64 数据 URL
  if (iconPath.startsWith('data:image')) {
    return iconPath
  }
  
  // HTTP/HTTPS URL
  if (iconPath.startsWith('http://') || iconPath.startsWith('https://')) {
    return iconPath
  }
  
  // file:// 协议（已经是可用的格式）
  if (iconPath.startsWith('file://')) {
    return iconPath
  }
  
  // 相对路径（如 "icons/xxx.png"）- 需要转换为绝对路径
  if (iconPath.startsWith('icons/')) {
    // 在 Tauri 中，可以使用 convertFileSrc 或直接读取文件
    // 这里我们使用 Tauri 的 read_config_file 类似的逻辑
    // 但实际上，对于图标，我们可以直接使用相对路径，让 Tauri 处理
    // 或者转换为绝对路径
    
    // 尝试使用 Tauri API 获取配置目录路径
    try {
      const invoker = getTauriInvoke()
      if (invoker) {
        // 使用 Tauri 的 convertFileSrc（如果可用）
        // 或者直接返回相对路径，让前端通过 Tauri 命令读取
        // 为了简化，我们直接返回相对路径，前端会通过 Tauri 命令读取
        return iconPath
      }
    } catch (error) {
      debug('无法转换图标路径:', error)
    }
    
    // 降级：返回相对路径
    return iconPath
  }
  
  // 绝对路径 - 转换为 file:// 协议
  if (iconPath.includes(':') || iconPath.startsWith('/')) {
    // Windows 路径（如 C:\path\to\file.png）
    if (iconPath.includes(':')) {
      return `file:///${iconPath.replace(/\\/g, '/')}`
    }
    // Unix 路径（如 /path/to/file.png）
    return `file://${iconPath}`
  }
  
  // 其他情况，直接返回
  return iconPath
}


/**
 * 获取图标的实际 URL（用于 img 标签的 src 属性）
 * 对于相对路径，需要通过 Tauri 命令读取文件并转换为 base64
 * @param iconPath 图标路径
 * @returns Promise<string | undefined> 可用的图标 URL
 */
export async function getIconUrl(iconPath: string | undefined | null): Promise<string | undefined> {
  if (!iconPath) {
    return undefined
  }
  
  // base64 数据 URL、HTTP/HTTPS URL、file:// 协议 - 直接返回
  if (
    iconPath.startsWith('data:image') ||
    iconPath.startsWith('http://') ||
    iconPath.startsWith('https://') ||
    iconPath.startsWith('file://')
  ) {
    return iconPath
  }
  
  // 相对路径（如 "icons/xxx.png" 或 ".config/icons/xxx.png"）- 需要通过 Tauri 命令读取
  // 注意：不规范化 .config/icons/ 格式，保留用户手动修改的格式
  if (iconPath.startsWith('icons/') || iconPath.startsWith('.config/icons/')) {
    try {
      const invoker = getTauriInvoke()
      if (!invoker) {
        logError('Tauri invoker 不可用，无法读取图标文件', { icon_path: iconPath })
        return undefined
      }
      
      debug('调用 read_icon_file 读取图标:', { icon_path: iconPath })
      // 使用 read_icon_file 命令读取图标文件并转换为 base64
      // Tauri 会自动将 camelCase 转换为 snake_case，所以使用 iconPath
      const base64Url = await invoker<string>('read_icon_file', { iconPath })
      
      if (base64Url && base64Url.startsWith('data:image')) {
        debug('图标读取成功:', { icon_path: iconPath, base64Length: base64Url.length })
        return base64Url
      } else {
        logError('图标读取返回无效数据:', { 
          icon_path: iconPath,
          result: base64Url ? base64Url.substring(0, 100) : 'null',
          resultType: typeof base64Url
        })
        return undefined
      }
    } catch (error) {
      logError('无法读取图标文件:', { icon_path: iconPath, error })
      throw error // 重新抛出错误，让调用者知道失败
    }
  }
  
  // 对于其他路径（绝对路径等），尝试处理
  // 如果路径包含 icons 目录，尝试提取文件名并使用相对路径读取
  const iconsPattern = /[\\/](?:\.config[\\/]icons[\\/]|icons[\\/])([^\\/]+\.(png|jpg|jpeg|gif|svg))$/i
  const match = iconPath.match(iconsPattern)
  if (match) {
    // 如果是绝对路径但指向 icons 目录，尝试使用相对路径读取
    const fileName = match[1]
    const relativePath = `icons/${fileName}`
    debug('检测到绝对路径指向 icons 目录，尝试使用相对路径读取:', { original: iconPath, relative: relativePath })
    try {
      const invoker = getTauriInvoke()
      if (invoker) {
        const base64Url = await invoker<string>('read_icon_file', { iconPath: relativePath })
        if (base64Url && base64Url.startsWith('data:image')) {
          return base64Url
        }
      }
    } catch (error) {
      debug('使用相对路径读取失败，尝试绝对路径:', { error })
    }
  }
  
  // 其他情况，使用 convertIconPath 转换（可能返回 file:// URL）
  return convertIconPath(iconPath)
}

