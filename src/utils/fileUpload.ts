/**
 * 文件上传工具函数
 * 将文件上传到后端并保存，返回保存后的文件路径
 */

import { getTauriInvoke } from './tauri'

/**
 * 选择文件并读取为 base64
 * @returns Promise<{ file: File, base64: string } | null>
 */
export function selectFile(): Promise<{ file: File, base64: string } | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.onchange = (e) => {
      const file = (e.target as HTMLInputElement).files?.[0]
      if (file) {
        const reader = new FileReader()
        reader.onload = () => {
          const base64 = reader.result as string
          resolve({ file, base64 })
        }
        reader.onerror = () => resolve(null)
        reader.readAsDataURL(file)
      } else {
        resolve(null)
      }
    }
    input.oncancel = () => resolve(null)
    input.click()
  })
}

/**
 * 上传文件到后端并保存
 * @param file 文件对象
 * @param base64Data base64 编码的文件数据
 * @param toolId 可选的工具ID，用于组织文件
 * @returns Promise<string | null> 保存后的文件路径，失败返回 null
 */
export async function uploadFile(
  file: File,
  base64Data: string,
  toolId?: string
): Promise<string | null> {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      if (import.meta.env.DEV) {
        // eslint-disable-next-line no-console
        console.warn('Tauri API 不可用，无法上传文件:', file.name)
      }
      return null
    }
    
    const filePath = await invoker<string>('upload_file', {
      params: {
        fileName: file.name,
        fileData: base64Data,
        toolId: toolId,
      }
    })
    
    return filePath || null
  } catch (error) {
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.error('上传文件失败:', error)
    }
    return null
  }
}

/**
 * 选择并上传文件（一步完成）
 * @param toolId 可选的工具ID，用于组织文件
 * @returns Promise<string | null> 保存后的文件路径，失败返回 null
 */
export async function selectAndUploadFile(toolId?: string): Promise<string | null> {
  const result = await selectFile()
  if (!result) {
    return null
  }
  
  const { file, base64 } = result
  return await uploadFile(file, base64, toolId)
}

