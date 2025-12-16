/**
 * JSON文件存储工具
 * 通过Tauri命令读写JSON配置文件
 */

interface TauriWindow {
  __TAURI__?: {
    invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>
  }
}

export async function readConfigFile(): Promise<string> {
  try {
    const tauriWindow = window as unknown as TauriWindow
    const invoker = tauriWindow.__TAURI__?.invoke
    if (invoker) {
      return await invoker<string>('read_categories_config')
    }
    // 降级到localStorage（开发环境或非Tauri环境）
    return localStorage.getItem('netsec-toolbox_categoriesConfig') || '{}'
  } catch (error) {
    console.error('Failed to read config file:', error)
    // 降级到localStorage
    return localStorage.getItem('netsec-toolbox_categoriesConfig') || '{}'
  }
}

export async function writeConfigFile(content: string): Promise<void> {
  try {
    const tauriWindow = window as unknown as TauriWindow
    const invoker = tauriWindow.__TAURI__?.invoke
    if (invoker) {
      await invoker('write_categories_config', { content })
      return
    }
    // 降级到localStorage（开发环境或非Tauri环境）
    localStorage.setItem('netsec-toolbox_categoriesConfig', content)
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error('Failed to write config file:', error)
    // 降级到localStorage
    localStorage.setItem('netsec-toolbox_categoriesConfig', content)
  }
}

