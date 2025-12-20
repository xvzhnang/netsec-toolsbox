/**
 * AI 模型配置工具函数
 * 用于读取和写入 models.json 配置文件
 */
import { getTauriInvoke } from './tauri'
import { debug, error as logError, info } from './logger'

/**
 * 模型配置接口
 */
export interface ModelConfig {
  id: string
  adapter: string
  base_url: string
  api_key: string
  enabled: boolean
  model: string
  temperature?: number
  max_tokens?: number
  timeout?: number
  _comment?: string
  _note?: string
  [key: string]: any // 允许其他字段
}

/**
 * Models 配置文件接口
 */
export interface ModelsConfig {
  _comment?: string
  _note?: string
  _usage?: string
  models: (ModelConfig | { _comment?: string; _note?: string })[]
}

/**
 * 读取 models.json 配置文件
 */
export async function readModelsConfig(): Promise<ModelsConfig> {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      throw new Error('Tauri API 不可用')
    }
    
    const content = await invoker<string>('read_models_config')
    
    // 解析 JSON
    const config: ModelsConfig = JSON.parse(content)
    
    debug('已读取 models.json 配置文件')
    return config
  } catch (error) {
    logError('读取 models.json 失败:', error)
    // 返回空配置
    return {
      models: []
    }
  }
}

/**
 * 写入 models.json 配置文件
 */
export async function writeModelsConfig(config: ModelsConfig): Promise<void> {
  try {
    const invoker = getTauriInvoke()
    if (!invoker) {
      throw new Error('Tauri API 不可用')
    }
    
    // 格式化为 JSON（保持格式，带缩进）
    const content = JSON.stringify(config, null, 2)
    
    await invoker('write_models_config', { content })
    
    info('✅ models.json 配置文件已保存')
  } catch (error) {
    logError('写入 models.json 失败:', error)
    throw error
  }
}

/**
 * 更新模型配置
 */
export async function updateModelConfig(
  modelId: string,
  updates: Partial<ModelConfig>
): Promise<void> {
  const config = await readModelsConfig()
  
  // 找到模型并更新
  const modelIndex = config.models.findIndex(
    (m) => !('_comment' in m) && !('_note' in m) && (m as ModelConfig).id === modelId
  )
  
  if (modelIndex === -1) {
    throw new Error(`模型 ${modelId} 不存在`)
  }
  
  const model = config.models[modelIndex] as ModelConfig
  Object.assign(model, updates)
  
  await writeModelsConfig(config)
}

/**
 * 切换模型启用状态
 */
export async function toggleModelEnabled(modelId: string, enabled: boolean): Promise<void> {
  await updateModelConfig(modelId, { enabled })
}

/**
 * 更新模型 API Key
 */
export async function updateModelApiKey(modelId: string, apiKey: string): Promise<void> {
  await updateModelConfig(modelId, { api_key: apiKey })
}

/**
 * 过滤出有效的模型配置（排除注释项）
 */
export function getValidModels(config: ModelsConfig): ModelConfig[] {
  return config.models.filter(
    (m): m is ModelConfig => {
      // 排除纯注释项（只有 _comment 或 _note 的项）
      const keys = Object.keys(m)
      if (keys.length <= 2 && keys.every(k => k.startsWith('_'))) {
        return false
      }
      
      // 必须有 id 字段
      if (!('id' in m) || typeof (m as any).id !== 'string' || (m as any).id.length === 0) {
        return false
      }
      
      // 确保基本字段存在
      const model = m as any
      return !!(model.id && model.adapter)
    }
  ).map(m => {
    // 确保所有必需字段都有默认值
    const model = m as any
    return {
      ...model,
      enabled: model.enabled !== undefined ? model.enabled : false,
      api_key: model.api_key !== undefined ? model.api_key : '',
      adapter: model.adapter || 'unknown',
      base_url: model.base_url || '',
      model: model.model || model.id
    } as ModelConfig
  })
}

/**
 * 按适配器类型分组模型
 */
export function groupModelsByAdapter(models: ModelConfig[]): Record<string, ModelConfig[]> {
  const grouped: Record<string, ModelConfig[]> = {}
  
  for (const model of models) {
    const adapter = model.adapter || 'unknown'
    if (!grouped[adapter]) {
      grouped[adapter] = []
    }
    grouped[adapter].push(model)
  }
  
  return grouped
}

/**
 * 检查 API Key 是否使用环境变量
 */
export function isEnvApiKey(apiKey: string): boolean {
  return apiKey.startsWith('ENV:')
}

/**
 * 获取环境变量名称
 */
export function getEnvName(apiKey: string): string | null {
  if (isEnvApiKey(apiKey)) {
    return apiKey.replace(/^ENV:/, '')
  }
  return null
}

