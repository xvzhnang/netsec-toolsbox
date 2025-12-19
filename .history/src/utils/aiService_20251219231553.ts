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

// OpenAI-compatible 接口
export interface OpenAIMessage {
  role: 'system' | 'user' | 'assistant'
  content: string
}

export interface OpenAIChatRequest {
  model: string
  messages: OpenAIMessage[]
  temperature?: number
  max_tokens?: number
  stream?: boolean
  timeout?: number
}

export interface OpenAIChatResponse {
  id: string
  object: string
  created: number
  model: string
  choices: Array<{
    index: number
    message: {
      role: string
      content: string
    }
    finish_reason: string
  }>
  usage?: {
    prompt_tokens: number
    completion_tokens: number
    total_tokens: number
  }
}

export interface OpenAIErrorResponse {
  error: {
    message: string
    type: string
    code: string
    param?: string
  }
}

// 向后兼容的接口（保留用于 Wiki 上下文）
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
        // Gateway 返回 { status: 'ok' } 或 { success: true }
        return data.status === 'ok' || data.success === true
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
 * 获取可用的 AI 提供商列表（使用 Gateway API）
 */
export async function getAIProviders(): Promise<AIProviderInfo> {
  try {
    // 使用 /v1/models 获取模型列表
    const models = await getAvailableModels()
    
    return {
      success: true,
      providers: models,
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
 * 获取指定提供商的可用模型列表
 */
export async function getProviderModels(provider: string): Promise<string[]> {
  try {
    const response = await fetch(`${AI_SERVICE_URL}/models/${provider}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
      cache: 'no-cache',
      credentials: 'omit',
    })
    
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    
    const data = await response.json()
    return data.models || []
  } catch (error) {
    debug(`获取 ${provider} 模型列表失败:`, error)
    return []
  }
}

/**
 * 发送聊天消息（使用 OpenAI-compatible API）
 */
export async function sendAIChat(
  model: string,
  messages: AIMessage[],
  options?: {
    wikiContext?: string
    timeout?: number
    temperature?: number
    maxTokens?: number
  }
): Promise<AIChatResponse> {
  try {
    // 转换消息格式为 OpenAI 格式
    const openaiMessages: OpenAIMessage[] = []
    
    // 如果有 Wiki 上下文，添加为系统消息
    if (options?.wikiContext) {
      openaiMessages.push({
        role: 'system',
        content: `以下是相关的 Wiki 文档内容，请参考这些信息回答问题：\n\n${options.wikiContext}`
      })
    }
    
    // 转换用户消息
    for (const msg of messages) {
      openaiMessages.push({
        role: msg.role === 'assistant' ? 'assistant' : 'user',
        content: msg.text
      })
    }
    
    // 构建 OpenAI-compatible 请求
    const request: OpenAIChatRequest = {
      model,
      messages: openaiMessages,
      temperature: options?.temperature,
      max_tokens: options?.maxTokens,
      timeout: options?.timeout,
    }
    
    const response = await fetch(`${AI_SERVICE_URL}/v1/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    })
    
    if (response.ok) {
      const data: OpenAIChatResponse = await response.json()
      return {
        success: true,
        response: data.choices[0]?.message?.content || '',
      }
    }
    
    // 处理错误响应
    const errorData: OpenAIErrorResponse = await response.json().catch(() => ({
      error: {
        message: `HTTP ${response.status}`,
        type: 'server_error',
        code: 'http_error'
      }
    }))
    
    return {
      success: false,
      error: errorData.error?.message || `HTTP ${response.status}`,
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
 * 获取可用模型列表（使用 OpenAI-compatible API）
 */
export async function getAvailableModels(): Promise<string[]> {
  try {
    const response = await fetch(`${AI_SERVICE_URL}/v1/models`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
      cache: 'no-cache',
      credentials: 'omit',
    })
    
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    
    const data = await response.json()
    // OpenAI Models API 格式: { object: "list", data: [{ id: "...", ... }, ...] }
    if (data.object === 'list' && Array.isArray(data.data)) {
      return data.data.map((model: any) => model.id)
    }
    
    return []
  } catch (error) {
    debug('获取模型列表失败:', error)
    return []
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

