/**
 * AI 服务 API 工具函数
 * 与 Python AI 服务通信
 */

import { debug, error as logError, warn, info } from './logger'
import { getTauriInvoke } from './tauri'

const AI_SERVICE_URL = 'http://127.0.0.1:8765'

export interface AIMessage {
  role: 'user' | 'assistant'
  text: string
}

export interface AIChatRequest {
  provider: string
  messages: AIMessage[]
  wiki_context?: string  // Wiki 上下文
  timeout?: number       // 超时时间（秒）
}

export interface AIChatResponse {
  success: boolean
  response?: string
  error?: string
}

export interface AIProviderInfo {
  success: boolean
  providers?: string[]
  available?: {
    openai?: boolean
    deepseek?: boolean
    ollama?: boolean
    lmstudio?: boolean
    llamacpp?: boolean
  }
  error?: string
}

/**
 * 检查 AI 服务是否可用
 */
export async function checkAIServiceHealth(): Promise<boolean> {
  try {
    // 使用 AbortSignal.timeout 如果支持，否则使用 AbortController
    let signal: AbortSignal
    let timeoutId: ReturnType<typeof setTimeout> | null = null
    
    if (typeof AbortSignal !== 'undefined' && 'timeout' in AbortSignal) {
      // 使用 AbortSignal.timeout (较新的浏览器)
      signal = (AbortSignal as any).timeout(2000)
    } else {
      // 降级到 AbortController
      const controller = new AbortController()
      timeoutId = setTimeout(() => controller.abort(), 2000)
      signal = controller.signal
    }
    
    const response = await fetch(`${AI_SERVICE_URL}/health`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
      signal,
      // 添加这些选项以减少错误日志
      cache: 'no-cache',
      credentials: 'omit',
    }).catch(() => {
      // 静默捕获所有错误，包括网络错误
      return null
    })
    
    if (timeoutId) {
      clearTimeout(timeoutId)
    }
    
    if (!response) {
      return false
    }
    
    if (response.ok) {
      try {
        const data = await response.json()
        return data.success === true
      } catch {
        return false
      }
    }
    return false
  } catch (error) {
    // 静默处理所有错误，不输出到控制台
    // 连接被拒绝是正常情况（服务未启动），不需要记录错误
    return false
  }
}

/**
 * 获取可用的 AI 提供商列表
 */
export async function getAIProviders(): Promise<AIProviderInfo> {
  try {
    const response = await fetch(`${AI_SERVICE_URL}/providers`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    })
    
    if (response.ok) {
      return await response.json()
    }
    
    return {
      success: false,
      error: `HTTP ${response.status}`,
    }
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    logError('获取 AI 提供商列表失败:', errorMsg)
    return {
      success: false,
      error: errorMsg,
    }
  }
}

/**
 * 发送聊天消息
 */
export async function sendAIChat(
  provider: string,
  messages: AIMessage[],
  options?: {
    wikiContext?: string
    timeout?: number
  }
): Promise<AIChatResponse> {
  try {
    const request: AIChatRequest = {
      provider,
      messages,
      wiki_context: options?.wikiContext,
      timeout: options?.timeout,
    }
    
    const response = await fetch(`${AI_SERVICE_URL}/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    })
    
    if (response.ok) {
      return await response.json()
    }
    
    const errorData = await response.json().catch(() => ({}))
    return {
      success: false,
      error: errorData.error || `HTTP ${response.status}`,
    }
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    logError('AI 聊天请求失败:', errorMsg)
    return {
      success: false,
      error: errorMsg,
    }
  }
}

/**
 * 启动 AI 服务
 */
export async function startAIService(): Promise<void> {
  const invoker = getTauriInvoke()
  if (!invoker) {
    throw new Error('Tauri API 不可用，无法启动 AI 服务')
  }
  
  try {
    const result = await invoker('start_ai_service')
    if (import.meta.env.DEV) {
      debug('AI 服务启动结果:', result)
    }
    info('AI 服务启动命令已发送')
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    // 提供更详细的错误信息
    if (errorMsg.includes('Python') || errorMsg.includes('不存在')) {
      logError('启动 AI 服务失败: Python 路径或服务脚本路径不正确。请检查 python313 目录和 ai_service/main.py 是否存在')
    } else if (errorMsg.includes('已在运行')) {
      debug('AI 服务已在运行')
    } else {
      logError('启动 AI 服务失败:', errorMsg)
    }
    throw error
  }
}

/**
 * 等待 AI 服务启动（轮询检查）
 */
export async function waitForAIService(
  maxAttempts: number = 10,
  interval: number = 1000
): Promise<boolean> {
  for (let i = 0; i < maxAttempts; i++) {
    const isHealthy = await checkAIServiceHealth()
    if (isHealthy) {
      if (import.meta.env.DEV) {
        debug('AI 服务已就绪')
      }
      return true
    }
    
    if (i < maxAttempts - 1) {
      await new Promise(resolve => setTimeout(resolve, interval))
    }
  }
  
  // 只在开发模式下输出警告
  if (import.meta.env.DEV) {
    warn('AI 服务启动超时')
  }
  return false
}

