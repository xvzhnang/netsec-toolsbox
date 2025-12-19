/**
 * AI 配置管理工具
 * 管理 AI 提供商的配置（API 密钥等）
 */

import { readConfigFile, writeConfigFile } from './fileStorage'
import { debug, error as logError } from './logger'

export interface AIProviderConfig {
  enabled?: boolean
  api_key?: string
  model?: string
  base_url?: string
  api_url?: string
  temperature?: number
  max_tokens?: number
  timeout?: number
  // 自定义提供商配置
  mode?: 'openai' | 'custom'
  endpoint?: string
  headers?: Record<string, string>
  request_format?: 'openai' | 'custom'
  response_path?: string
  auth_type?: 'bearer' | 'header' | 'api_key'
  auth_header?: string
  request_body?: Record<string, any>
  message_format?: 'openai' | 'list' | 'custom'
  name?: string
}

export interface CustomProviderConfig extends AIProviderConfig {
  name: string
}

export interface AIConfig {
  openai?: AIProviderConfig
  deepseek?: AIProviderConfig
  claude?: AIProviderConfig
  gemini?: AIProviderConfig
  zhipu?: AIProviderConfig
  qwen?: AIProviderConfig
  mistral?: AIProviderConfig
  ollama?: AIProviderConfig
  lmstudio?: AIProviderConfig
  llamacpp?: AIProviderConfig
  custom_providers?: Record<string, CustomProviderConfig>
}

const CONFIG_FILE = 'ai.json'

/**
 * 读取 AI 配置
 */
export async function readAIConfig(): Promise<AIConfig> {
  try {
    const configStr = await readConfigFile(CONFIG_FILE)
    
    // readConfigFile 返回的是字符串，需要解析 JSON
    if (!configStr || configStr.trim() === '') {
      return {}
    }
    
    try {
      const parsed = JSON.parse(configStr)
      
      // 确保解析后是对象
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        return parsed as AIConfig
      }
      
      return {}
    } catch (parseError) {
      debug('解析 AI 配置 JSON 失败:', parseError)
      return {}
    }
  } catch (error) {
    debug('读取 AI 配置失败，返回空配置:', error)
    return {}
  }
}

/**
 * 保存 AI 配置
 */
export async function writeAIConfig(config: AIConfig): Promise<void> {
  try {
    // writeConfigFile 需要字符串，所以需要序列化 JSON
    const configStr = JSON.stringify(config, null, 2)
    await writeConfigFile(CONFIG_FILE, configStr)
    debug('AI 配置已保存')
  } catch (error) {
    logError('保存 AI 配置失败:', error)
    throw error
  }
}

/**
 * 更新单个提供商的配置
 */
export async function updateProviderConfig(
  provider: 'openai' | 'deepseek' | 'claude' | 'gemini' | 'zhipu' | 'qwen' | 'mistral' | 'ollama' | 'lmstudio' | 'llamacpp',
  config: Partial<AIProviderConfig>
): Promise<void> {
  const aiConfig = await readAIConfig()
  
  if (!aiConfig[provider]) {
    aiConfig[provider] = {}
  }
  
  aiConfig[provider] = {
    ...aiConfig[provider],
    ...config,
  }
  
  await writeAIConfig(aiConfig)
}

