/**
 * AI 聊天历史管理
 * 负责保存和加载聊天历史记录
 */
import { readConfigFile, writeConfigFile } from './fileStorage'
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
 * 聊天历史接口
 */
export interface ChatHistory {
  sessions: ChatSession[]
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
    const content = await readConfigFile(HISTORY_FILE)
    if (!content || content === '{}' || content.trim() === '') {
      return { sessions: [] }
    }
    
    const history: ChatHistory = JSON.parse(content)
    
    // 验证数据结构
    if (!history.sessions || !Array.isArray(history.sessions)) {
      return { sessions: [] }
    }
    
    debug('已加载聊天历史', { sessionsCount: history.sessions.length })
    return history
  } catch (error) {
    logError('加载聊天历史失败:', error)
    return { sessions: [] }
  }
}

/**
 * 保存聊天历史
 */
export async function saveChatHistory(history: ChatHistory): Promise<void> {
  try {
    // 限制会话数量
    if (history.sessions.length > MAX_SESSIONS) {
      // 按更新时间排序，保留最新的
      history.sessions.sort((a, b) => b.updatedAt - a.updatedAt)
      history.sessions = history.sessions.slice(0, MAX_SESSIONS)
    }
    
    // 限制每个会话的消息数量
    for (const session of history.sessions) {
      if (session.messages.length > MAX_MESSAGES_PER_SESSION) {
        session.messages = session.messages.slice(-MAX_MESSAGES_PER_SESSION)
      }
    }
    
    const content = JSON.stringify(history, null, 2)
    await writeConfigFile(HISTORY_FILE, content)
    
    debug('已保存聊天历史', { sessionsCount: history.sessions.length })
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
  const history = await loadChatHistory()
  const session = history.sessions.find(s => s.id === sessionId)
  
  if (!session) {
    throw new Error(`会话 ${sessionId} 不存在`)
  }
  
  session.messages.push(message)
  session.updatedAt = Date.now()
  
  await saveChatHistory(history)
}

/**
 * 更新会话消息
 */
export async function updateSessionMessage(
  sessionId: string,
  messageId: number,
  updates: Partial<ChatMessage>
): Promise<void> {
  const history = await loadChatHistory()
  const session = history.sessions.find(s => s.id === sessionId)
  
  if (!session) {
    throw new Error(`会话 ${sessionId} 不存在`)
  }
  
  const messageIndex = session.messages.findIndex(m => m.id === messageId)
  if (messageIndex === -1) {
    // 如果消息不存在，创建新消息
    const newMessage: ChatMessage = {
      id: messageId,
      role: 'assistant',
      text: '',
      timestamp: Date.now(),
      ...updates,
    }
    session.messages.push(newMessage)
  } else {
    // 更新现有消息
    Object.assign(session.messages[messageIndex], updates)
  }
  
  session.updatedAt = Date.now()
  
  await saveChatHistory(history)
}

/**
 * 保存会话
 */
export async function saveSession(session: ChatSession): Promise<void> {
  const history = await loadChatHistory()
  
  const index = history.sessions.findIndex(s => s.id === session.id)
  if (index >= 0) {
    history.sessions[index] = session
  } else {
    history.sessions.push(session)
  }
  
  history.currentSessionId = session.id
  session.updatedAt = Date.now()
  
  await saveChatHistory(history)
}

/**
 * 删除会话
 */
export async function deleteSession(sessionId: string): Promise<void> {
  const history = await loadChatHistory()
  history.sessions = history.sessions.filter(s => s.id !== sessionId)
  
  if (history.currentSessionId === sessionId) {
    history.currentSessionId = history.sessions.length > 0 ? history.sessions[0].id : undefined
  }
  
  await saveChatHistory(history)
}

/**
 * 清空所有历史
 */
export async function clearChatHistory(): Promise<void> {
  await saveChatHistory({ sessions: [] })
  info('聊天历史已清空')
}

