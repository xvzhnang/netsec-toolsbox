/**
 * Tauri API 工具函数
 * 兼容 Tauri 1.x 和 2.x
 */

interface TauriWindow extends Window {
  __TAURI__?: {
    invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
    core?: {
      invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
    }
    tauri?: {
      invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
    }
    [key: string]: any // 允许其他可能的属性
  }
  __TAURI_INTERNALS__?: {
    invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
    [key: string]: any
  }
  __TAURI_METADATA__?: any
}

/**
 * 获取 Tauri invoke 函数（兼容 Tauri 1.x 和 2.x）
 * @returns invoke 函数，如果不可用则返回 null
 */
export function getTauriInvoke() {
  try {
    const tauriWindow = window as unknown as TauriWindow
    const win = window as any
    
    // 调试信息（仅在开发环境且首次调用时输出）
    if (import.meta.env.DEV && !(win.__tauri_api_checked)) {
      win.__tauri_api_checked = true
      const internals = win.__TAURI_INTERNALS__
      // eslint-disable-next-line no-console
      console.log('检查 Tauri API:', {
        hasTAURI: !!tauriWindow.__TAURI__,
        hasCore: !!tauriWindow.__TAURI__?.core,
        hasInvoke: !!tauriWindow.__TAURI__?.invoke,
        hasCoreInvoke: !!tauriWindow.__TAURI__?.core?.invoke,
        hasTauriInvoke: !!tauriWindow.__TAURI__?.tauri?.invoke,
        hasInternals: !!internals,
        internalsType: typeof internals,
        internalsInvoke: !!internals?.invoke,
        internalsKeys: internals ? Object.keys(internals) : [],
        internalsIpc: !!internals?.ipc,
        internalsIpcInvoke: !!internals?.ipc?.invoke,
        tauriKeys: tauriWindow.__TAURI__ ? Object.keys(tauriWindow.__TAURI__) : [],
        allKeys: Object.keys(win).filter(k => k.includes('TAURI')),
        internalsFull: internals, // 完整对象用于调试
      })
    }
    
    // 尝试多种可能的访问方式
    // 1. Tauri 2.x core.invoke
    if (tauriWindow.__TAURI__?.core?.invoke) {
      const invoke = tauriWindow.__TAURI__.core.invoke
      return invoke.bind(tauriWindow.__TAURI__.core) as typeof invoke
    }
    
    // 2. Tauri 2.x tauri.invoke
    if (tauriWindow.__TAURI__?.tauri?.invoke) {
      const invoke = tauriWindow.__TAURI__.tauri.invoke
      return invoke.bind(tauriWindow.__TAURI__.tauri) as typeof invoke
    }
    
    // 3. Tauri 1.x invoke
    if (tauriWindow.__TAURI__?.invoke) {
      const invoke = tauriWindow.__TAURI__.invoke
      return invoke.bind(tauriWindow.__TAURI__) as typeof invoke
    }
    
    // 4. 尝试通过 __TAURI_INTERNALS__ 访问（Tauri 2.x 可能使用这种方式）
    if (tauriWindow.__TAURI_INTERNALS__?.invoke) {
      const invoke = tauriWindow.__TAURI_INTERNALS__.invoke
      return invoke.bind(tauriWindow.__TAURI_INTERNALS__) as typeof invoke
    }
    
    // 5. 尝试直接访问 window.__TAURI_INTERNALS__.invoke（如果存在）
    const internals = win.__TAURI_INTERNALS__
    if (internals && typeof internals.invoke === 'function') {
      return internals.invoke.bind(internals)
    }
    
    // 6. 尝试通过事件系统访问（Tauri 2.x 可能需要）
    if (typeof internals !== 'undefined' && internals.ipc) {
      // 某些情况下可能需要通过 ipc 访问
      if (typeof internals.ipc.invoke === 'function') {
        return internals.ipc.invoke.bind(internals.ipc)
      }
    }
    
    // 7. 深度搜索 internals 对象，查找任何可能的 invoke 函数
    if (internals && typeof internals === 'object') {
      const searchInvoke = (obj: any, depth = 0): any => {
        if (depth > 3) return null // 限制搜索深度
        if (!obj || typeof obj !== 'object') return null
        
        // 检查当前对象的 invoke
        if (typeof obj.invoke === 'function') {
          return obj.invoke.bind(obj)
        }
        
        // 递归搜索所有属性
        for (const key in obj) {
          if (Object.prototype.hasOwnProperty.call(obj, key)) {
            const result = searchInvoke(obj[key], depth + 1)
            if (result) return result
          }
        }
        
        return null
      }
      
      const found = searchInvoke(internals)
      if (found) return found
    }
    
    return null
  } catch (error) {
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.error('获取 Tauri API 时出错:', error)
    }
    return null
  }
}

/**
 * 检查是否在 Tauri 环境中
 */
export function isTauriEnvironment(): boolean {
  const tauriWindow = window as unknown as TauriWindow
  return !!(
    tauriWindow.__TAURI__ ||
    tauriWindow.__TAURI_INTERNALS__ ||
    (window as any).__TAURI_METADATA__
  )
}

/**
 * 等待 Tauri API 加载完成
 * @param timeout 超时时间（毫秒），默认 5000ms
 * @returns Promise<boolean> 是否成功加载
 */
export async function waitForTauriAPI(timeout = 5000): Promise<boolean> {
  const startTime = Date.now()
  
  // 如果已经可用，直接返回
  if (getTauriInvoke()) {
    return true
  }
  
  // 等待 DOM 加载完成（如果还没加载）
  if (document.readyState !== 'complete') {
    await new Promise<void>(resolve => {
      if (document.readyState === 'complete') {
        resolve()
      } else {
        window.addEventListener('load', () => resolve(), { once: true })
      }
    })
  }
  
  // 等待 API 加载
  while (Date.now() - startTime < timeout) {
    if (getTauriInvoke()) {
      return true
    }
    // 等待 50ms 后重试（更频繁的检查）
    await new Promise(resolve => setTimeout(resolve, 50))
  }
  
  return false
}

