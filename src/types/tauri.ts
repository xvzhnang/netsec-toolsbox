/**
 * Tauri API 类型定义
 * 统一管理 Tauri 相关的类型，避免重复定义
 */

/**
 * Tauri Window 接口（兼容 Tauri 1.x 和 2.x）
 */
export interface TauriWindow extends Window {
  __TAURI__?: {
    invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
    dialog?: {
      open: (options?: {
        filters?: Array<{ name: string; extensions: string[] }>
        multiple?: boolean
        defaultPath?: string
      }) => Promise<string | string[] | null>
    }
    core?: {
      invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
      dialog?: {
        open: (options?: {
          filters?: Array<{ name: string; extensions: string[] }>
          multiple?: boolean
          defaultPath?: string
        }) => Promise<string | string[] | null>
      }
    }
    tauri?: {
      invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
      dialog?: {
        open: (options?: {
          filters?: Array<{ name: string; extensions: string[] }>
          multiple?: boolean
          defaultPath?: string
        }) => Promise<string | string[] | null>
      }
    }
    [key: string]: any // 允许其他可能的属性
  }
  __TAURI_INTERNALS__?: {
    invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
    dialog?: {
      open: (options?: {
        filters?: Array<{ name: string; extensions: string[] }>
        multiple?: boolean
        defaultPath?: string
      }) => Promise<string | string[] | null>
    }
    ipc?: {
      invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
    }
    [key: string]: any
  }
  __TAURI_METADATA__?: any
}

