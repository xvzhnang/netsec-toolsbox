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

/**
 * 从可执行文件路径提取图标（占位函数，实际需要后端支持）
 * @param execPath 可执行文件路径
 * @returns Promise<string | null> 图标数据URL或null
 */
export async function extractIconFromExecutable(execPath: string): Promise<string | null> {
  // 当前仅前端UI开发，此功能需要后端支持
  // 后端可以读取可执行文件的图标资源并返回base64
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.log('提取可执行文件图标需要后端支持:', execPath)
  }
  return null
}

