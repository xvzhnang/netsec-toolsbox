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
    const response = await fetch(`${AI_SERVICE_URL}/health`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
      signal: AbortSignal.timeout(2000), // 2秒超时
    })
    
    if (response.ok) {
      const data = await response.json()
      return data.success === true
    }
    return false
  } catch (error) {
    // 静默处理连接错误，不输出到控制台
    // 只在调试模式下输出
    if (import.meta.env.DEV) {
      debug('AI 服务健康检查失败:', error)
    }
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
    await invoker('start_ai_service')
    info('AI 服务启动命令已发送')
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    logError('启动 AI 服务失败:', errorMsg)
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

