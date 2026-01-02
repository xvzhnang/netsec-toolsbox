/**
 * AI 聊天历史管理
 * 负责保存和加载聊天历史记录
 */
import { readConfigFile, writeConfigFile } from './fileStorage'
import { getTauriInvoke } from './tauri'
import { debug, error as logError, info } from './logger'

/**
 * 聊天消息接口
 */
export interface ChatMessage {
  id: number
  role: 'user' | 'assistant' | 'system'
  text: string
  timestamp?: number
  model?: string
  usage?: {
    prompt_tokens?: number
    completion_tokens?: number
    total_tokens?: number
  }
}

/**
 * 聊天会话接口
 */
export interface ChatSession {
  id: string
  title: string
  model: string
  messages: ChatMessage[]
  createdAt: number
  updatedAt: number
}

/**
 * 会话摘要接口
 */
export interface ChatSessionSummary {
  id: string
  title: string
  model: string
  createdAt: number
  updatedAt: number
  messageCount: number
}

/**
 * 聊天历史摘要接口
 */
export interface ChatHistory {
  sessions: ChatSessionSummary[]
  currentSessionId?: string
}

const HISTORY_FILE = 'ai_history.json'
const MAX_SESSIONS = 100 // 最多保留100个会话
const MAX_MESSAGES_PER_SESSION = 1000 // 每个会话最多1000条消息

/**
 * 读取聊天历史
 */
export async function loadChatHistory(): Promise<ChatHistory> {
  try {
    const invoker = getTauriInvoke()
    if (invoker) {
      const result = await invoker<{
        sessions: Array<{
          id: string
          title: string
          model: string
          created_at: number
          updated_at: number
          message_count: number
        }>
        current_session_id?: string | null
      }>('ai_history_load')

      return {
        sessions: (result.sessions || []).map(s => ({
          id: s.id,
          title: s.title,
          model: s.model,
          createdAt: s.created_at,
          updatedAt: s.updated_at,
          messageCount: s.message_count,
        })),
        currentSessionId: result.current_session_id || undefined,
      }
    }

    const content = await readConfigFile(HISTORY_FILE)
    if (!content || content === '{}' || content.trim() === '') {
      return { sessions: [] }
    }

    const historyFile: { sessions: ChatSession[]; currentSessionId?: string } = JSON.parse(content)
    if (!historyFile.sessions || !Array.isArray(historyFile.sessions)) {
      return { sessions: [] }
    }

    const sessions = historyFile.sessions.map(s => ({
      id: s.id,
      title: s.title,
      model: s.model,
      createdAt: s.createdAt,
      updatedAt: s.updatedAt,
      messageCount: s.messages?.length || 0,
    }))

    debug('已加载聊天历史', { sessionsCount: sessions.length })
    return { sessions, currentSessionId: historyFile.currentSessionId }
  } catch (error) {
    logError('加载聊天历史失败:', error)
    return { sessions: [] }
  }
}

/**
 * 加载完整会话（含消息）
 */
export async function loadSession(sessionId: string): Promise<ChatSession | null> {
  try {
    const invoker = getTauriInvoke()
    if (invoker) {
      const session = await invoker<{
        id: string
        title: string
        model: string
        messages: Array<{
          id: number
          role: 'user' | 'assistant' | 'system'
          text: string
          timestamp?: number | null
          model?: string | null
          usage?: ChatMessage['usage'] | null
        }>
        created_at: number
        updated_at: number
      } | null>('ai_history_get_session', { sessionId })

      if (!session) return null

      return {
        id: session.id,
        title: session.title,
        model: session.model,
        messages: (session.messages || []).map(m => ({
          id: m.id,
          role: m.role,
          text: m.text,
          timestamp: m.timestamp ?? undefined,
          model: m.model ?? undefined,
          usage: m.usage ?? undefined,
        })),
        createdAt: session.created_at,
        updatedAt: session.updated_at,
      }
    }

    const content = await readConfigFile(HISTORY_FILE)
    if (!content || content === '{}' || content.trim() === '') {
      return null
    }
    const historyFile: { sessions: ChatSession[]; currentSessionId?: string } = JSON.parse(content)
    const session = historyFile.sessions.find(s => s.id === sessionId)
    return session || null
  } catch (error) {
    logError('加载会话失败:', error)
    return null
  }
}

async function saveChatHistoryFile(historyFile: { sessions: ChatSession[]; currentSessionId?: string }): Promise<void> {
  try {
    if (historyFile.sessions.length > MAX_SESSIONS) {
      historyFile.sessions.sort((a, b) => b.updatedAt - a.updatedAt)
      historyFile.sessions = historyFile.sessions.slice(0, MAX_SESSIONS)
    }

    for (const session of historyFile.sessions) {
      if (session.messages.length > MAX_MESSAGES_PER_SESSION) {
        session.messages = session.messages.slice(-MAX_MESSAGES_PER_SESSION)
      }
    }

    const content = JSON.stringify(historyFile, null, 2)
    await writeConfigFile(HISTORY_FILE, content)
    debug('已保存聊天历史', { sessionsCount: historyFile.sessions.length })
  } catch (error) {
    logError('保存聊天历史失败:', error)
  }
}

/**
 * 创建新会话
 */
export function createSession(model: string, title?: string): ChatSession {
  const id = `session_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
  return {
    id,
    title: title || `新对话 (${new Date().toLocaleString()})`,
    model,
    messages: [],
    createdAt: Date.now(),
    updatedAt: Date.now(),
  }
}

/**
 * 添加消息到会话
 */
export async function addMessageToSession(
  sessionId: string,
  message: ChatMessage
): Promise<void> {
  const invoker = getTauriInvoke()
  if (invoker) {
    await invoker('ai_history_add_message', {
      sessionId,
      message: {
        id: message.id,
        role: message.role,
        text: message.text,
        timestamp: message.timestamp ?? null,
        model: message.model ?? null,
        usage: message.usage ?? null,
      },
    })
    return
  }

  const content = await readConfigFile(HISTORY_FILE)
  const historyFile: { sessions: ChatSession[]; currentSessionId?: string } = content && content.trim() ? JSON.parse(content) : { sessions: [] }
  const session = historyFile.sessions.find(s => s.id === sessionId)
  if (!session) throw new Error(`会话 ${sessionId} 不存在`)
  session.messages.push(message)
  session.updatedAt = Date.now()
  historyFile.currentSessionId = sessionId
  await saveChatHistoryFile(historyFile)
}

/**
 * 更新会话消息
 */
export async function updateSessionMessage(
  sessionId: string,
  messageId: number,
  updates: Partial<ChatMessage>
): Promise<void> {
  const invoker = getTauriInvoke()
  if (invoker) {
    await invoker('ai_history_update_message', {
      sessionId,
      messageId,
      patch: {
        text: updates.text ?? null,
        timestamp: updates.timestamp ?? null,
        model: updates.model ?? null,
        usage: updates.usage ?? null,
      },
    })
    return
  }

  const content = await readConfigFile(HISTORY_FILE)
  const historyFile: { sessions: ChatSession[]; currentSessionId?: string } = content && content.trim() ? JSON.parse(content) : { sessions: [] }
  const session = historyFile.sessions.find(s => s.id === sessionId)
  if (!session) throw new Error(`会话 ${sessionId} 不存在`)
  const messageIndex = session.messages.findIndex(m => m.id === messageId)
  if (messageIndex === -1) {
    const newMessage: ChatMessage = { id: messageId, role: 'assistant', text: '', timestamp: Date.now(), ...updates }
    session.messages.push(newMessage)
  } else {
    const existingMessage = session.messages[messageIndex]
    if (existingMessage) Object.assign(existingMessage, updates)
    else session.messages.push({ id: messageId, role: 'assistant', text: '', timestamp: Date.now(), ...updates })
  }
  session.updatedAt = Date.now()
  historyFile.currentSessionId = sessionId
  await saveChatHistoryFile(historyFile)
}

/**
 * 保存会话
 */
export async function saveSession(session: ChatSession): Promise<void> {
  session.updatedAt = Date.now()

  const invoker = getTauriInvoke()
  if (invoker) {
    await invoker('ai_history_upsert_session', {
      session: {
        id: session.id,
        title: session.title,
        model: session.model,
        messages: session.messages.map(m => ({
          id: m.id,
          role: m.role,
          text: m.text,
          timestamp: m.timestamp ?? null,
          model: m.model ?? null,
          usage: m.usage ?? null,
        })),
        created_at: session.createdAt,
        updated_at: session.updatedAt,
      },
    })
    return
  }

  const content = await readConfigFile(HISTORY_FILE)
  const historyFile: { sessions: ChatSession[]; currentSessionId?: string } = content && content.trim() ? JSON.parse(content) : { sessions: [] }
  const index = historyFile.sessions.findIndex(s => s.id === session.id)
  if (index >= 0) historyFile.sessions[index] = session
  else historyFile.sessions.push(session)
  historyFile.currentSessionId = session.id
  await saveChatHistoryFile(historyFile)
}

/**
 * 删除会话
 */
export async function deleteSession(sessionId: string): Promise<void> {
  const invoker = getTauriInvoke()
  if (invoker) {
    await invoker('ai_history_delete_session', { sessionId })
    return
  }

  const content = await readConfigFile(HISTORY_FILE)
  const historyFile: { sessions: ChatSession[]; currentSessionId?: string } = content && content.trim() ? JSON.parse(content) : { sessions: [] }
  historyFile.sessions = historyFile.sessions.filter(s => s.id !== sessionId)
  if (historyFile.currentSessionId === sessionId) {
    historyFile.currentSessionId = historyFile.sessions[0]?.id
  }
  await saveChatHistoryFile(historyFile)
}

/**
 * 清空所有历史
 */
export async function clearChatHistory(): Promise<void> {
  const invoker = getTauriInvoke()
  if (invoker) {
    await invoker('ai_history_clear')
  } else {
    await saveChatHistoryFile({ sessions: [] })
  }
  info('聊天历史已清空')
}

