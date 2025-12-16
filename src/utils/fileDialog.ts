// 文件对话框工具函数
interface TauriWindow {
  __TAURI__?: {
    invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>
    dialog?: {
      open: (options?: {
        filters?: Array<{ name: string; extensions: string[] }>
        multiple?: boolean
        defaultPath?: string
      }) => Promise<string | string[] | null>
    }
  }
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
    const tauriWindow = window as unknown as TauriWindow
    const invoke = tauriWindow.__TAURI__?.invoke

    // 尝试使用 Tauri invoke 调用后端命令（如果已实现）
    if (invoke) {
      try {
        const result = await invoke<string | null>('open_file_dialog', {
          filters,
          defaultPath,
        })
        if (result) {
          return result
        }
      } catch (err) {
        // 如果后端命令未实现，继续尝试其他方法
        // eslint-disable-next-line no-console
        if (import.meta.env.DEV) {
          // eslint-disable-next-line no-console
          console.log('open_file_dialog command not implemented, falling back to HTML5 file input')
        }
      }
    }

    // 尝试使用 Tauri dialog API（如果可用）
    const dialog = tauriWindow.__TAURI__?.dialog
    if (dialog?.open) {
      const result = await dialog.open({
        filters,
        multiple: false,
        defaultPath,
      })
      if (typeof result === 'string') {
        return result
      }
      if (Array.isArray(result) && result.length > 0) {
        return result[0]
      }
      return null
    }

    // 降级到 HTML5 文件选择（开发环境或非 Tauri 环境）
    return new Promise((resolve) => {
      const input = document.createElement('input')
      input.type = 'file'
      if (filters && filters.length > 0) {
        const extensions = filters.flatMap((f) => f.extensions)
        input.accept = extensions.map((ext) => `.${ext}`).join(',')
      }
      input.onchange = (e) => {
        const file = (e.target as HTMLInputElement).files?.[0]
        if (file) {
          // 在浏览器环境中，返回文件名（实际路径不可用）
          // 注意：在生产环境中，这需要后端支持
          resolve(file.name)
        } else {
          resolve(null)
        }
      }
      input.oncancel = () => resolve(null)
      input.click()
    })
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error('Failed to open file dialog:', error)
    return null
  }
}

