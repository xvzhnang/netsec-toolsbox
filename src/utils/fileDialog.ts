// 文件对话框工具函数
import { getTauriInvoke, waitForTauriAPI } from './tauri'

interface TauriWindow extends Window {
  __TAURI__?: {
    dialog?: {
      open: (options?: {
        filters?: Array<{ name: string; extensions: string[] }>
        multiple?: boolean
        defaultPath?: string
      }) => Promise<string | string[] | null>
    }
    core?: {
      dialog?: {
        open: (options?: {
          filters?: Array<{ name: string; extensions: string[] }>
          multiple?: boolean
          defaultPath?: string
        }) => Promise<string | string[] | null>
      }
    }
    tauri?: {
      dialog?: {
        open: (options?: {
          filters?: Array<{ name: string; extensions: string[] }>
          multiple?: boolean
          defaultPath?: string
        }) => Promise<string | string[] | null>
      }
    }
    [key: string]: any
  }
  __TAURI_INTERNALS__?: {
    dialog?: {
      open: (options?: {
        filters?: Array<{ name: string; extensions: string[] }>
        multiple?: boolean
        defaultPath?: string
      }) => Promise<string | string[] | null>
    }
    [key: string]: any
  }
}

/**
 * 获取 Tauri dialog API（兼容 Tauri 1.x 和 2.x）
 */
function getTauriDialog(): { open: (options?: any) => Promise<string | string[] | null> } | null {
  const tauriWindow = window as unknown as TauriWindow
  const win = window as any
  
  // 调试信息（仅在开发环境且首次调用时输出）
  if (import.meta.env.DEV && !(win.__tauri_dialog_checked)) {
    win.__tauri_dialog_checked = true
    // eslint-disable-next-line no-console
    console.log('检查 Tauri Dialog API:', {
      hasTAURI: !!tauriWindow.__TAURI__,
      hasDialog: !!tauriWindow.__TAURI__?.dialog,
      hasCoreDialog: !!tauriWindow.__TAURI__?.core?.dialog,
      hasTauriDialog: !!tauriWindow.__TAURI__?.tauri?.dialog,
      hasInternals: !!tauriWindow.__TAURI_INTERNALS__,
      hasInternalsDialog: !!tauriWindow.__TAURI_INTERNALS__?.dialog,
      tauriKeys: tauriWindow.__TAURI__ ? Object.keys(tauriWindow.__TAURI__) : [],
      internalsKeys: tauriWindow.__TAURI_INTERNALS__ ? Object.keys(tauriWindow.__TAURI_INTERNALS__) : [],
    })
  }
  
  // 尝试多种可能的访问方式
  // 1. Tauri 2.x core.dialog
  if (tauriWindow.__TAURI__?.core?.dialog?.open) {
    return tauriWindow.__TAURI__.core.dialog
  }
  // 2. Tauri 2.x tauri.dialog
  if (tauriWindow.__TAURI__?.tauri?.dialog?.open) {
    return tauriWindow.__TAURI__.tauri.dialog
  }
  // 3. Tauri 1.x dialog
  if (tauriWindow.__TAURI__?.dialog?.open) {
    return tauriWindow.__TAURI__.dialog
  }
  // 4. Tauri 2.x internals.dialog
  if (tauriWindow.__TAURI_INTERNALS__?.dialog?.open) {
    return tauriWindow.__TAURI_INTERNALS__.dialog
  }
  
  return null
}

/**
 * 打开文件选择对话框
 * @param filters 文件过滤器，例如 [{ name: 'JAR Files', extensions: ['jar'] }]
 * @param defaultPath 默认路径
 * @returns 选择的文件路径，如果取消则返回 null
 */
export async function openFileDialog(
  filters?: Array<{ name: string; extensions: string[] }>,
  defaultPath?: string
): Promise<string | null> {
  try {
    // 等待 Tauri API 加载（最多等待 2 秒）
    await waitForTauriAPI(2000)
    
    // 优先使用 Tauri dialog API（兼容 Tauri 1.x 和 2.x）
    const dialog = getTauriDialog()
    
    if (dialog?.open) {
      try {
        const result = await dialog.open({
          filters,
          multiple: false,
          defaultPath,
        })
        let filePath: string | null = null
        if (typeof result === 'string') {
          filePath = result
        } else if (Array.isArray(result) && result.length > 0) {
          filePath = result[0]
        }
        
        if (filePath) {
          // 确保返回绝对路径
          // Tauri dialog.open 应该返回绝对路径，但在某些情况下可能返回相对路径
          // 为了确保一致性，总是通过后端解析路径
          const invoke = getTauriInvoke()
          if (invoke) {
            try {
              // 总是通过后端解析路径，确保返回绝对路径
              const absPath = await invoke<string>('resolve_file_path', {
                params: {
                  filePath: filePath,
                }
              })
              if (absPath) {
                if (import.meta.env.DEV) {
                  // eslint-disable-next-line no-console
                  console.log('文件对话框：路径已解析为绝对路径:', absPath)
                }
                return absPath
              }
            } catch (err) {
              if (import.meta.env.DEV) {
                // eslint-disable-next-line no-console
                console.warn('解析文件路径失败，使用原始路径:', err)
              }
              // 如果解析失败，检查原始路径是否是绝对路径
              // Windows: 包含 ':' 或以 '\\' 开头
              // Unix: 以 '/' 开头
              if (filePath.includes(':') || filePath.startsWith('\\') || filePath.startsWith('/')) {
                return filePath
              }
              // 如果是相对路径且解析失败，返回 null
              return null
            }
          }
          // 如果没有 invoke，检查是否是绝对路径
          if (filePath.includes(':') || filePath.startsWith('\\') || filePath.startsWith('/')) {
            return filePath
          }
          // 相对路径且无法解析，返回 null
          return null
        }
        return null
      } catch (err) {
        if (import.meta.env.DEV) {
          // eslint-disable-next-line no-console
          console.error('Tauri dialog API 调用失败:', err)
        }
        // 继续尝试其他方法
      }
    }

    // 如果前端 dialog API 不可用，尝试使用后端命令（降级方案）
    const invoke = getTauriInvoke()
    if (invoke) {
      try {
        const result = await invoke<string | null>('open_file_dialog', {
          params: {
            filters,
            defaultPath,
          }
        })
        if (result) {
          // 后端已经返回绝对路径
          if (import.meta.env.DEV) {
            // eslint-disable-next-line no-console
            console.log('文件对话框：通过后端命令获取路径:', result)
          }
          return result
        }
        return null // 用户取消
      } catch (err) {
        if (import.meta.env.DEV) {
          // eslint-disable-next-line no-console
          console.warn('后端文件对话框命令失败:', err)
        }
      }
    }
    
    // 在非 Tauri 环境中，无法获取绝对路径
    // 返回 null 而不是文件名，因为文件名无法用于文件操作
    if (import.meta.env.DEV) {
      const tauriWindow = window as unknown as TauriWindow
      // eslint-disable-next-line no-console
      console.warn('文件对话框：Tauri dialog API 不可用，且后端命令也失败', {
        hasTAURI: !!tauriWindow.__TAURI__,
        hasDialog: !!tauriWindow.__TAURI__?.dialog,
        hasCoreDialog: !!tauriWindow.__TAURI__?.core?.dialog,
        hasTauriDialog: !!tauriWindow.__TAURI__?.tauri?.dialog,
        hasInternals: !!tauriWindow.__TAURI_INTERNALS__,
        hasInvoke: !!invoke,
        tauriKeys: tauriWindow.__TAURI__ ? Object.keys(tauriWindow.__TAURI__) : [],
        isTauriEnv: !!(tauriWindow.__TAURI__ || tauriWindow.__TAURI_INTERNALS__),
      })
    }
    return null
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error('Failed to open file dialog:', error)
    return null
  }
}

