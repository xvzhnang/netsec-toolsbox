/**
 * AI 性能监控工具
 * 记录和统计 AI 模型的使用性能
 */
import { readConfigFile, writeConfigFile } from './fileStorage'
import { debug, error as logError, info } from './logger'

/**
 * 单次请求性能指标
 */
export interface RequestMetrics {
  model: string
  timestamp: number
  responseTime: number
  promptTokens: number
  completionTokens: number
  totalTokens: number
  success: boolean
  error?: string
}

/**
 * 模型性能统计
 */
export interface ModelPerformanceStats {
  model: string
  requestCount: number
  successCount: number
  failureCount: number
  totalTokens: number
  totalResponseTime: number
  averageResponseTime: number
  averageTokens: number
  lastUsed?: number
}

/**
 * 性能监控数据
 */
export interface PerformanceData {
  requests: RequestMetrics[]
  modelStats: Record<string, ModelPerformanceStats>
  totalRequests: number
  totalTokens: number
  lastUpdated: number
}

const PERFORMANCE_FILE = 'ai_performance.json'
const MAX_REQUESTS = 10000 // 最多保留10000条请求记录

/**
 * 加载性能数据
 */
export async function loadPerformanceData(): Promise<PerformanceData> {
  try {
    const content = await readConfigFile(PERFORMANCE_FILE)
    if (!content || content === '{}' || content.trim() === '') {
      return {
        requests: [],
        modelStats: {},
        totalRequests: 0,
        totalTokens: 0,
        lastUpdated: Date.now(),
      }
    }
    
    const data: PerformanceData = JSON.parse(content)
    
    // 验证数据结构
    if (!data.requests || !Array.isArray(data.requests)) {
      data.requests = []
    }
    if (!data.modelStats || typeof data.modelStats !== 'object') {
      data.modelStats = {}
    }
    
    return data
  } catch (error) {
    logError('加载性能数据失败:', error)
    return {
      requests: [],
      modelStats: {},
      totalRequests: 0,
      totalTokens: 0,
      lastUpdated: Date.now(),
    }
  }
}

/**
 * 保存性能数据
 */
export async function savePerformanceData(data: PerformanceData): Promise<void> {
  try {
    // 限制请求记录数量
    if (data.requests.length > MAX_REQUESTS) {
      // 按时间排序，保留最新的
      data.requests.sort((a, b) => b.timestamp - a.timestamp)
      data.requests = data.requests.slice(0, MAX_REQUESTS)
    }
    
    data.lastUpdated = Date.now()
    const content = JSON.stringify(data, null, 2)
    await writeConfigFile(PERFORMANCE_FILE, content)
    
    debug('已保存性能数据', { requestsCount: data.requests.length, modelsCount: Object.keys(data.modelStats).length })
  } catch (error) {
    logError('保存性能数据失败:', error)
  }
}

/**
 * 记录请求指标
 */
export async function recordRequest(metrics: RequestMetrics): Promise<void> {
  const data = await loadPerformanceData()
  
  // 添加到请求列表
  data.requests.push(metrics)
  
  // 更新模型统计
  if (!data.modelStats[metrics.model]) {
    data.modelStats[metrics.model] = {
      model: metrics.model,
      requestCount: 0,
      successCount: 0,
      failureCount: 0,
      totalTokens: 0,
      totalResponseTime: 0,
      averageResponseTime: 0,
      averageTokens: 0,
    }
  }
  
  const stats = data.modelStats[metrics.model]
  stats.requestCount++
  stats.lastUsed = metrics.timestamp
  
  if (metrics.success) {
    stats.successCount++
    stats.totalTokens += metrics.totalTokens
    stats.totalResponseTime += metrics.responseTime
    stats.averageResponseTime = stats.totalResponseTime / stats.successCount
    stats.averageTokens = stats.totalTokens / stats.successCount
  } else {
    stats.failureCount++
  }
  
  // 更新总体统计
  data.totalRequests++
  if (metrics.success) {
    data.totalTokens += metrics.totalTokens
  }
  
  await savePerformanceData(data)
}

/**
 * 获取模型性能统计
 */
export async function getModelPerformanceStats(model: string): Promise<ModelPerformanceStats | null> {
  const data = await loadPerformanceData()
  return data.modelStats[model] || null
}

/**
 * 获取所有模型性能统计
 */
export async function getAllModelPerformanceStats(): Promise<ModelPerformanceStats[]> {
  const data = await loadPerformanceData()
  return Object.values(data.modelStats)
}

/**
 * 清空性能数据
 */
export async function clearPerformanceData(): Promise<void> {
  await savePerformanceData({
    requests: [],
    modelStats: {},
    totalRequests: 0,
    totalTokens: 0,
    lastUpdated: Date.now(),
  })
  info('性能数据已清空')
}

/**
 * 获取性能摘要
 */
export async function getPerformanceSummary(): Promise<{
  totalRequests: number
  totalTokens: number
  modelsCount: number
  averageResponseTime: number
  successRate: number
}> {
  const data = await loadPerformanceData()
  
  const allStats = Object.values(data.modelStats)
  const totalSuccess = allStats.reduce((sum, s) => sum + s.successCount, 0)
  const totalFailures = allStats.reduce((sum, s) => sum + s.failureCount, 0)
  const totalResponseTime = allStats.reduce((sum, s) => sum + s.totalResponseTime, 0)
  const totalSuccessCount = totalSuccess
  
  return {
    totalRequests: data.totalRequests,
    totalTokens: data.totalTokens,
    modelsCount: allStats.length,
    averageResponseTime: totalSuccessCount > 0 ? totalResponseTime / totalSuccessCount : 0,
    successRate: data.totalRequests > 0 ? (totalSuccess / data.totalRequests) * 100 : 0,
  }
}

