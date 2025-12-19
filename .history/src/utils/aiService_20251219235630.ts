/**
 * AI Gateway 服务工具函数
 * 对应 One API 的 OpenAI-compatible API
 */
import { getTauriInvoke } from './tauri'
import { debug, warn } from './logger'

const GATEWAY_BASE_URL = 'http://127.0.0.1:8765'

/**
 * OpenAI 消息格式
 */
export interface OpenAIMessage {
  role: 'user' | 'assistant' | 'system'
  content: string
  name?: string
}

/**
 * OpenAI Chat Completions 请求格式
 */
export interface OpenAIChatRequest {
  model: string
  messages: OpenAIMessage[]
  temperature?: number
  max_tokens?: number
  stream?: boolean
  top_p?: number
  frequency_penalty?: number
  presence_penalty?: number
  stop?: string[]
  user?: string
}

/**
 * OpenAI Chat Completions 响应格式
 */
export interface OpenAIChatResponse {
  id: string
  object: string
  created: number
  model: string
  choices: Array<{
    index: number
    message: OpenAIMessage
    finish_reason: string | null
  }>
  usage?: {
    prompt_tokens: number
    completion_tokens: number
    total_tokens: number
  }
}

/**
 * OpenAI 错误响应格式
 */
export interface OpenAIErrorResponse {
  error: {
    message: string
    type: string
    code: string
  }
}

/**
 * 检查 AI Gateway 服务健康状态
 */
export async function checkAIServiceHealth(): Promise<boolean> {
  try {
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), 2000)
    
    const response = await fetch(`${GATEWAY_BASE_URL}/health`, {
      method: 'GET',
      signal: controller.signal,
      cache: 'no-cache',
      credentials: 'omit',
    }).catch(() => null)
    
    clearTimeout(timeoutId)
    
    if (!response || !response.ok) {
      return false
    }
    
    return true
  } catch (error) {
    // 静默处理连接错误
    return false
  }
}

/**
 * 获取可用模型列表
 */
export async function getAvailableModels(): Promise<string[]> {
  try {
    const response = await fetch(`${GATEWAY_BASE_URL}/v1/models`, {
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
 * 发送 AI 聊天请求
 */
export async function sendAIChat(
  model: string,
  messages: OpenAIMessage[],
  options?: {
    temperature?: number
    max_tokens?: number
    stream?: boolean
  }
): Promise<OpenAIChatResponse> {
  const request: OpenAIChatRequest = {
    model,
    messages,
    ...options,
  }
  
  const response = await fetch(`${GATEWAY_BASE_URL}/v1/chat/completions`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(request),
    cache: 'no-cache',
    credentials: 'omit',
  })
  
  if (!response.ok) {
    const errorData: OpenAIErrorResponse = await response.json()
    throw new Error(errorData.error?.message || `HTTP error! status: ${response.status}`)
  }
  
  return response.json()
}

/**
 * 启动 AI Gateway 服务
 */
export async function startAIService(): Promise<void> {
  const invoker = getTauriInvoke()
  if (!invoker) {
    throw new Error('Tauri API 不可用，无法启动 AI Gateway 服务')
  }
  
  try {
    await invoker('start_ai_service')
  } catch (error) {
    throw new Error(`启动 AI Gateway 服务失败: ${error instanceof Error ? error.message : String(error)}`)
  }
}

/**
 * 停止 AI Gateway 服务
 */
export async function stopAIService(): Promise<void> {
  const invoker = getTauriInvoke()
  if (!invoker) {
    throw new Error('Tauri API 不可用，无法停止 AI Gateway 服务')
  }
  
  try {
    await invoker('stop_ai_service')
  } catch (error) {
    throw new Error(`停止 AI Gateway 服务失败: ${error instanceof Error ? error.message : String(error)}`)
  }
}

/**
 * 检查 AI Gateway 服务状态
 */
export async function checkAIServiceStatus(): Promise<boolean> {
  const invoker = getTauriInvoke()
  if (!invoker) {
    return false
  }
  
  try {
    const status = await invoker('check_ai_service_status') as boolean
    return status
  } catch (error) {
    debug('检查 AI Gateway 服务状态失败:', error)
    return false
  }
}

/**
 * 等待 AI Gateway 服务就绪
 */
export async function waitForAIService(
  maxWaitSeconds: number = 10,
  checkIntervalMs: number = 1000
): Promise<boolean> {
  const maxAttempts = Math.floor((maxWaitSeconds * 1000) / checkIntervalMs)
  
  for (let i = 0; i < maxAttempts; i++) {
    const isHealthy = await checkAIServiceHealth()
    if (isHealthy) {
      return true
    }
    
    if (i < maxAttempts - 1) {
      await new Promise(resolve => setTimeout(resolve, checkIntervalMs))
    }
  }
  
  return false
}

