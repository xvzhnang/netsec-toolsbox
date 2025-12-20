/**
 * AI Gateway 服务工具函数
 * 对应 One API 的 OpenAI-compatible API
 */
import { getTauriInvoke } from './tauri'
import { debug, warn, info } from './logger'

// 使用连接池后，不再直接访问固定端口
// 所有请求通过 Rust 后端的连接池转发
const USE_POOL = true // 是否使用连接池模式

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

// 健康检查请求计数器（用于追踪）
let healthCheckCounter = 0

/**
 * 通过连接池转发 HTTP 请求
 */
async function forwardRequest(
  method: string,
  path: string,
  body?: string,
  headers?: Record<string, string>
): Promise<Response> {
  const invoker = getTauriInvoke()
  if (!invoker) {
    throw new Error('Tauri API 不可用')
  }

  try {
    // 转换 headers 为数组格式
    const headersArray = headers ? Object.entries(headers).map(([k, v]) => [k, v]) : undefined
    const bodyBytes = body ? new TextEncoder().encode(body) : undefined

    const result = await invoker('forward_ai_request', {
      method,
      path,
      body: bodyBytes,
      headers: headersArray,
    }) as [number, number[]] // [status_code, body_bytes]

    const [status, bodyBytesArray] = result
    const responseBody = new Uint8Array(bodyBytesArray)

    // 构建 Response 对象
    const response = new Response(responseBody, {
      status,
      headers: {
        'Content-Type': 'application/json',
      },
    })

    return response
  } catch (error) {
    debug('连接池转发请求失败，降级到直接访问:', error)
    // 如果连接池不可用，降级到直接访问（向后兼容）
    const GATEWAY_BASE_URL = 'http://127.0.0.1:8765'
    return fetch(`${GATEWAY_BASE_URL}${path}`, {
      method,
      headers,
      body,
    })
  }
}

/**
 * 检查 AI Gateway 服务健康状态
 * @param delayMs 延迟检测时间（毫秒），用于避免在请求完成后立即检测导致误判
 */

export async function checkAIServiceHealth(delayMs: number = 100): Promise<boolean> {
  const checkId = ++healthCheckCounter
  const checkStartTime = Date.now()
  
  try {
    info(`[健康检查-${checkId}] 开始健康检查 (延迟: ${delayMs}ms)`)
    
    // 如果指定了延迟，先等待
    if (delayMs > 0) {
      debug(`[健康检查-${checkId}] 延迟 ${delayMs}ms 后开始检测...`)
      await new Promise(resolve => setTimeout(resolve, delayMs))
      debug(`[健康检查-${checkId}] 延迟完成，开始发送请求`)
    }
    
    const requestStartTime = Date.now()
    const GATEWAY_BASE_URL = 'http://127.0.0.1:8765'
    info(`[健康检查-${checkId}] 发送健康检查请求${USE_POOL ? '（通过连接池）' : `到 ${GATEWAY_BASE_URL}/health`}...`)
    
    let fetchError: Error | null = null
    let response: Response | null = null
    
    if (USE_POOL) {
      // 使用连接池转发
      try {
        response = await forwardRequest('GET', '/health')
      } catch (error) {
        fetchError = error instanceof Error ? error : new Error(String(error))
        response = null
      }
    } else {
      // 直接访问（旧模式）
      const controller = new AbortController()
      const timeoutId = setTimeout(() => {
        warn(`[健康检查-${checkId}] 请求超时（5秒），触发 AbortController`)
        controller.abort()
      }, 5000)
      
      response = await fetch(`${GATEWAY_BASE_URL}/health`, {
        method: 'GET',
        signal: controller.signal,
        cache: 'no-cache',
        credentials: 'omit',
      }).catch((error) => {
        fetchError = error instanceof Error ? error : new Error(String(error))
        const elapsed = Date.now() - requestStartTime
        const totalElapsed = Date.now() - checkStartTime
        warn(`[健康检查-${checkId}] 请求失败 (请求耗时: ${elapsed}ms, 总耗时: ${totalElapsed}ms): ${fetchError.message}`)
        warn(`[健康检查-${checkId}] 错误类型: ${fetchError.name}, 错误消息: ${fetchError.message}`)
        if (fetchError.name === 'AbortError') {
          warn(`[健康检查-${checkId}] 请求超时（5秒），可能是服务响应慢或网络问题`)
        } else if (fetchError.message.includes('Failed to fetch') || fetchError.message.includes('ERR_CONNECTION_REFUSED')) {
          warn(`[健康检查-${checkId}] 连接被拒绝，服务可能未启动或端口未监听`)
        } else if (fetchError.message.includes('NetworkError')) {
          warn(`[健康检查-${checkId}] 网络错误，可能是服务暂时不可用`)
        }
        return null
      })
      
      clearTimeout(timeoutId)
    }
    
    const requestElapsed = Date.now() - requestStartTime
    const totalElapsed = Date.now() - checkStartTime
    
    if (!response) {
      if (fetchError) {
        warn(`[健康检查-${checkId}] ❌ 服务不可用 - 请求失败 (请求耗时: ${requestElapsed}ms, 总耗时: ${totalElapsed}ms, 错误: ${fetchError.message})`)
      } else {
        warn(`[健康检查-${checkId}] ❌ 服务不可用 - 无响应 (请求耗时: ${requestElapsed}ms, 总耗时: ${totalElapsed}ms)`)
      }
      return false
    }
    
    if (!response.ok) {
      warn(`[健康检查-${checkId}] ❌ 服务不可用 (状态码: ${response.status}, 请求耗时: ${requestElapsed}ms, 总耗时: ${totalElapsed}ms)`)
      try {
        const errorText = await response.text()
        debug(`[健康检查-${checkId}] 错误响应内容: ${errorText}`)
      } catch {
        // 忽略读取错误响应的异常
      }
      return false
    }
    
    // 尝试读取响应内容
    try {
      const responseData = await response.json()
      debug(`[健康检查-${checkId}] 响应内容:`, responseData)
    } catch {
      // 忽略 JSON 解析错误
    }
    
    info(`[健康检查-${checkId}] ✅ 服务健康检查通过 (请求耗时: ${requestElapsed}ms, 总耗时: ${totalElapsed}ms)`)
    return true
  } catch (error) {
    const totalElapsed = Date.now() - checkStartTime
    warn(`[健康检查-${checkId}] ❌ 健康检查异常 (总耗时: ${totalElapsed}ms): ${error instanceof Error ? error.message : String(error)}`)
    if (error instanceof Error && error.stack) {
      debug(`[健康检查-${checkId}] 异常堆栈: ${error.stack}`)
    }
    return false
  }
}

/**
 * 获取可用模型列表
 */
/**
 * 重新加载 AI Gateway 配置
 */
export async function reloadAIConfig(): Promise<void> {
  try {
    const GATEWAY_BASE_URL = 'http://127.0.0.1:8765'
    const response = USE_POOL
      ? await forwardRequest('GET', '/reload', undefined, {
          'Content-Type': 'application/json',
        })
      : await fetch(`${GATEWAY_BASE_URL}/reload`, {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
          },
          cache: 'no-cache',
          credentials: 'omit',
        })

    if (!response.ok) {
      // 404 错误说明服务可能还在运行旧版本，需要重启
      if (response.status === 404) {
        throw new Error('Not Found')
      }
      const errorData = await response.json().catch(() => ({}))
      throw new Error(errorData.error?.message || `HTTP error! status: ${response.status}`)
    }

    debug('AI Gateway 配置已重新加载')
  } catch (error) {
    // 只记录非 404 错误，404 错误由调用方静默处理
    const errorMsg = error instanceof Error ? error.message : String(error)
    if (!errorMsg.includes('404') && !errorMsg.includes('Not Found')) {
      warn('重新加载 AI Gateway 配置失败:', error)
    }
    throw error
  }
}

export async function getAvailableModels(): Promise<string[]> {
  try {
    const response = USE_POOL
      ? await forwardRequest('GET', '/v1/models', undefined, {
          'Content-Type': 'application/json',
        })
      : await fetch('http://127.0.0.1:8765/v1/models', {
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
 * OpenAI 流式响应数据块
 */
export interface OpenAIStreamChunk {
  id: string
  object: string
  created: number
  model: string
  choices: Array<{
    index: number
    delta?: {
      role?: string
      content?: string
    }
    message?: OpenAIMessage
    finish_reason?: string | null
  }>
  usage?: {
    prompt_tokens: number
    completion_tokens: number
    total_tokens: number
  }
}

/**
 * 发送 AI 聊天请求（非流式）
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
    stream: false,
    ...options,
  }
  
  const response = USE_POOL
    ? await forwardRequest('POST', '/v1/chat/completions', JSON.stringify(request), {
        'Content-Type': 'application/json',
      })
    : await fetch('http://127.0.0.1:8765/v1/chat/completions', {
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
 * 发送 AI 聊天请求（流式 SSE）
 * @param model 模型 ID
 * @param messages 消息列表
 * @param options 选项
 * @param onChunk 接收到数据块时的回调
 * @param onComplete 完成时的回调
 * @param onError 错误时的回调
 */
export async function sendAIChatStream(
  model: string,
  messages: OpenAIMessage[],
  options: {
    temperature?: number
    max_tokens?: number
    onChunk: (chunk: OpenAIStreamChunk) => void
    onComplete?: (usage?: OpenAIStreamChunk['usage']) => void
    onError?: (error: Error) => void
  }
): Promise<void> {
  const request: OpenAIChatRequest = {
    model,
    messages,
    stream: true,
    temperature: options.temperature,
    max_tokens: options.max_tokens,
  }
  
  try {
    // 流式请求暂时不支持连接池转发（需要 SSE 支持）
    // 先使用直接访问，后续可以改进
    const GATEWAY_BASE_URL = 'http://127.0.0.1:8765'
    const response = await fetch(`${GATEWAY_BASE_URL}/v1/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
      cache: 'no-cache',
      credentials: 'omit',
    }).catch((error) => {
      // 网络错误（连接被拒绝等）
      if (error instanceof TypeError && error.message.includes('fetch')) {
        throw new Error('无法连接到 AI Gateway 服务，请确保服务正在运行')
      }
      throw error
    })
    
    if (!response.ok) {
      const errorData: OpenAIErrorResponse = await response.json().catch(() => ({}))
      throw new Error(errorData.error?.message || `HTTP error! status: ${response.status}`)
    }
    
    // 读取 SSE 流
    const reader = response.body?.getReader()
    if (!reader) {
      throw new Error('无法读取响应流')
    }
    
    const decoder = new TextDecoder('utf-8')
    let buffer = ''
    let lastUsage: OpenAIStreamChunk['usage'] | undefined
    
    while (true) {
      const { done, value } = await reader.read()
      
      if (done) {
        break
      }
      
      // 解码数据
      buffer += decoder.decode(value, { stream: true })
      
      // 处理完整的 SSE 消息
      const lines = buffer.split('\n\n')
      buffer = lines.pop() || '' // 保留最后一个不完整的消息
      
      for (const line of lines) {
        if (line.trim() === '' || !line.startsWith('data: ')) {
          continue
        }
        
        const data = line.substring(6) // 移除 "data: " 前缀
        
        // 检查是否完成
        if (data === '[DONE]') {
          if (options.onComplete) {
            options.onComplete(lastUsage)
          }
          return
        }
        
        try {
          const chunk: OpenAIStreamChunk = JSON.parse(data)
          
          // 保存 usage（最后一个 chunk 可能包含）
          if (chunk.usage) {
            lastUsage = chunk.usage
          }
          
          // 调用回调
          options.onChunk(chunk)
          
          // 检查是否完成
          if (chunk.choices && chunk.choices.length > 0 && chunk.choices[0]) {
            const finishReason = chunk.choices[0].finish_reason
            if (finishReason) {
              if (options.onComplete) {
                options.onComplete(lastUsage)
              }
              return
            }
          }
        } catch (error) {
          // 忽略 JSON 解析错误（可能是其他 SSE 消息）
          debug('SSE 数据解析失败:', data, error)
        }
      }
    }
    
    // 流结束时调用完成回调
    if (options.onComplete) {
      options.onComplete(lastUsage)
    }
  } catch (error) {
    if (options.onError) {
      options.onError(error instanceof Error ? error : new Error(String(error)))
    } else {
      throw error
    }
  }
}

/**
 * 初始化 Gateway 连接池
 */
export async function initGatewayPool(): Promise<void> {
  const invoker = getTauriInvoke()
  if (!invoker) {
    throw new Error('Tauri API 不可用，无法初始化连接池')
  }
  
  try {
    await invoker('init_gateway_pool')
    info('[连接池] 连接池已初始化')
  } catch (error) {
    throw new Error(`初始化连接池失败: ${error instanceof Error ? error.message : String(error)}`)
  }
}

/**
 * 启动 AI Gateway 服务（兼容旧版，实际使用连接池）
 */
export async function startAIService(): Promise<void> {
  if (USE_POOL) {
    // 使用连接池模式
    return initGatewayPool()
  } else {
    // 旧版单进程模式
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
 * 重启 AI Gateway 服务
 */
export async function restartAIService(): Promise<void> {
  const invoker = getTauriInvoke()
  if (!invoker) {
    throw new Error('Tauri API 不可用，无法重启 AI Gateway 服务')
  }
  
  try {
    info('[服务恢复] 开始重启 AI Gateway 服务...')
    // 先停止服务
    try {
      await invoker('stop_ai_service')
      debug('[服务恢复] 服务已停止')
      // 等待一小段时间确保进程完全退出
      await new Promise(resolve => setTimeout(resolve, 500))
    } catch (error) {
      warn(`[服务恢复] 停止服务失败（可能服务未运行）: ${error instanceof Error ? error.message : String(error)}`)
    }
    
    // 然后启动服务
    await invoker('start_ai_service')
    info('[服务恢复] 服务已启动，等待服务就绪...')
    
    // 等待服务就绪（最多等待 10 秒）
    const maxWaitTime = 10000
    const checkInterval = 500
    const startTime = Date.now()
    
    while (Date.now() - startTime < maxWaitTime) {
      const isHealthy = await checkAIServiceHealth(0).catch(() => false)
      if (isHealthy) {
        info('[服务恢复] ✅ 服务已恢复并正常运行')
        return
      }
      await new Promise(resolve => setTimeout(resolve, checkInterval))
    }
    
    warn('[服务恢复] ⚠️ 服务重启后仍未就绪，可能需要更多时间')
  } catch (error) {
    throw new Error(`重启 AI Gateway 服务失败: ${error instanceof Error ? error.message : String(error)}`)
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
  maxWaitSeconds: number = 8,
  checkIntervalMs: number = 500
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

