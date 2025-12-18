/**
 * 图片处理工具函数
 * 用于裁剪、压缩和转换图片
 */

/**
 * 裁剪并压缩图片
 * @param file 图片文件
 * @param targetSize 目标尺寸（正方形，默认160）
 * @param quality 压缩质量（0-1，默认0.9）
 * @returns Promise<string> base64 数据URL
 */
export async function processImage(
  file: File,
  targetSize: number = 160,
  quality: number = 0.9
): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    
    reader.onload = (e) => {
      const img = new Image()
      
      img.onload = () => {
        try {
          // 创建 Canvas 进行裁剪和压缩
          const canvas = document.createElement('canvas')
          const ctx = canvas.getContext('2d')
          
          if (!ctx) {
            reject(new Error('无法创建 Canvas 上下文'))
            return
          }
          
          // 计算裁剪区域（正方形，取中心）
          const size = Math.min(img.width, img.height)
          const x = (img.width - size) / 2
          const y = (img.height - size) / 2
          
          // 设置 Canvas 尺寸
          canvas.width = targetSize
          canvas.height = targetSize
          
          // 使用高质量缩放算法
          ctx.imageSmoothingEnabled = true
          ctx.imageSmoothingQuality = 'high'
          
          // 绘制裁剪后的图片
          ctx.drawImage(
            img,
            x, y, size, size, // 源图片裁剪区域
            0, 0, targetSize, targetSize // 目标 Canvas 区域
          )
          
          // 转换为 base64
          const dataUrl = canvas.toDataURL('image/png', quality)
          resolve(dataUrl)
        } catch (error) {
          reject(error)
        }
      }
      
      img.onerror = () => {
        reject(new Error('图片加载失败'))
      }
      
      img.src = e.target?.result as string
    }
    
    reader.onerror = () => {
      reject(new Error('文件读取失败'))
    }
    
    reader.readAsDataURL(file)
  })
}

/**
 * 选择图片文件
 * @returns Promise<File | null> 选择的图片文件
 */
export function selectImageFile(): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = 'image/*'
    input.onchange = (e) => {
      const file = (e.target as HTMLInputElement).files?.[0]
      if (file && file.type.startsWith('image/')) {
        resolve(file)
      } else {
        resolve(null)
      }
    }
    input.oncancel = () => resolve(null)
    input.click()
  })
}

import { getTauriInvoke } from './tauri'
import { debug, error as logError, warn } from './logger'

/**
 * 从可执行文件路径提取图标
 * @param execPath 可执行文件路径
 * @param toolType 工具类型（可选，用于确定提取方式）
 * @returns Promise<string | null> 图标数据URL（base64）或null
 */
export async function extractIconFromExecutable(execPath: string, toolType?: string): Promise<string | null> {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      warn('Tauri API 不可用，无法提取图标:', execPath)
      return null
    }
    
    const iconData = await invoker<string>('extract_icon_from_file', {
      params: {
        filePath: execPath,
        toolType: toolType,
      }
    })
    
    if (iconData && iconData.startsWith('data:image')) {
      return iconData
    }
    
    // 如果不是 data URL，添加前缀
    if (iconData && !iconData.startsWith('data:')) {
      return `data:image/png;base64,${iconData}`
    }
    
    return iconData || null
  } catch (error) {
    logError('提取图标失败:', error)
    return null
  }
}

/**
 * 从 URL 抓取 favicon
 * @param url 网页 URL
 * @returns Promise<string | null> 图标数据URL（base64）或null
 */
export async function fetchFaviconFromUrl(url: string): Promise<string | null> {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      warn('Tauri API 不可用，无法抓取 favicon:', url)
      return null
    }
    
    const iconData = await invoker<string>('fetch_favicon', {
      params: {
        urlStr: url
      }
    })
    
    if (iconData && iconData.startsWith('data:image')) {
      return iconData
    }
    
    // 如果不是 data URL，添加前缀
    if (iconData && !iconData.startsWith('data:')) {
      return `data:image/png;base64,${iconData}`
    }
    
    return iconData || null
  } catch (error) {
    logError('抓取 favicon 失败:', error)
    return null
  }
}

/**
 * 根据文件路径自动判断文件类型
 * @param filePath 文件路径或 URL
 * @returns 工具类型字符串
 */
export function detectFileTypeFromPath(filePath: string): string {
  if (!filePath) {
    return '其他'
  }
  
  // 检查是否是 URL
  if (filePath.startsWith('http://') || filePath.startsWith('https://')) {
    return '网页'
  }
  
  // 提取文件扩展名
  const pathParts = filePath.split(/[/\\]/)
  const fileName = pathParts[pathParts.length - 1]
  const lastDotIndex = fileName.lastIndexOf('.')
  
  if (lastDotIndex === -1 || lastDotIndex === fileName.length - 1) {
    // 没有扩展名
    return '其他'
  }
  
  const ext = fileName.substring(lastDotIndex + 1).toLowerCase()
  
  switch (ext) {
    case 'exe':
    case 'dll':
    case 'com':
    case 'bat':
    case 'cmd':
    case 'scr':
      return 'GUI'
    case 'lnk':
      return 'LNK'
    case 'html':
    case 'htm':
      return 'HTML'
    case 'py':
    case 'pyw':
      return 'Python'
    case 'jar':
      return 'JAR'
    case 'sh':
    case 'bash':
    case 'zsh':
    case 'fish':
    case 'ps1':
    case 'psm1':
    case 'psd1':
      return 'CLI'
    default:
      return '其他'
  }
}

/**
 * 根据工具类型自动获取图标
 * @param toolType 工具类型（可选，如果不提供则根据 execPath 自动判断）
 * @param execPath 执行路径（文件路径或 URL）
 * @returns Promise<string | null> 图标数据URL或null
 */
export async function autoFetchIcon(toolType: string | undefined, execPath: string): Promise<string | null> {
  if (!execPath) {
    return null
  }
  
  // 如果没有提供工具类型，根据文件路径自动判断
  const detectedType = toolType || detectFileTypeFromPath(execPath)
  
  try {
    switch (detectedType) {
      case 'GUI':
      case 'CLI':
      case 'LNK':
      case 'Python':
      case 'JAR':
      case '其他': {
        // 所有本地文件类型：尝试提取图标（后端会自动判断类型）
        // 不传递 toolType，让后端根据文件路径自动判断
        return await extractIconFromExecutable(execPath, undefined)
      }
      case 'HTML': {
        // HTML：提取 HTML 文件中的 favicon
        return await extractIconFromExecutable(execPath, 'HTML')
      }
      case '网页': {
        // URL：抓取网页 favicon
        return await fetchFaviconFromUrl(execPath)
      }
      default: {
        // 未知类型，尝试作为可执行文件提取
        return await extractIconFromExecutable(execPath, undefined)
      }
    }
  } catch (error) {
    logError('自动获取图标失败:', error)
    return null
  }
}

