/**
 * Tauri API 诊断工具
 * 用于调试 Tauri API 访问问题
 */

export function diagnoseTauriAPI() {
  const diagnostics = {
    userAgent: navigator.userAgent,
    location: window.location.href,
    hasTAURI: !!(window as any).__TAURI__,
    hasInternals: !!(window as any).__TAURI_INTERNALS__,
    hasMetadata: !!(window as any).__TAURI_METADATA__,
    tauriObject: (window as any).__TAURI__ ? Object.keys((window as any).__TAURI__) : [],
  }
  
  // eslint-disable-next-line no-console
  console.log('Tauri API 诊断信息:', diagnostics)
  
  return diagnostics
}

