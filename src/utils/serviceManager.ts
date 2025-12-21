/**
 * 统一服务管理工具（新架构）
 * 
 * 前端只认统一的状态格式，不再区分 AI / Wiki / Tool
 */

import { getTauriInvoke } from './tauri'

export interface ServiceState {
  stopped: 'stopped'
  starting: 'starting'
  warmup: 'warmup'
  idle: 'idle'
  busy: 'busy'
  degraded: 'degraded'
  unhealthy: 'unhealthy'
  restarting: 'restarting'
  stopping: 'stopping'
}

export type ServiceStateValue = ServiceState[keyof ServiceState]

export interface ServiceEvent {
  type?: string
  service_id?: string
  from?: ServiceStateValue
  to?: ServiceStateValue
  status?: 'Healthy' | 'Degraded' | 'Unhealthy'
  error?: string
  timestamp?: number
}

export interface ServiceStatusDTO {
  id: string
  name: string
  state: ServiceStateValue
  message?: string
  description?: string
  is_healthy: boolean
  is_available: boolean
  metadata: Record<string, any>
}

export interface ServiceStatusListDTO {
  services: ServiceStatusDTO[]
}

/**
 * 获取所有服务状态
 */
export async function getAllServices(): Promise<ServiceStatusListDTO> {
  try {
    const invoke = getTauriInvoke()
    if (!invoke) {
      throw new Error('Tauri API 不可用')
    }
    const result = await invoke<ServiceStatusListDTO>('get_all_services')
    return result
  } catch (error) {
    console.error('[ServiceManager] 获取服务列表失败:', error)
    throw error
  }
}

/**
 * 获取单个服务状态
 */
export async function getServiceStatus(id: string): Promise<ServiceStatusDTO | null> {
  try {
    const invoke = getTauriInvoke()
    if (!invoke) {
      throw new Error('Tauri API 不可用')
    }
    const result = await invoke<ServiceStatusDTO | null>('get_service_status', { id })
    return result
  } catch (error) {
    console.error(`[ServiceManager] 获取服务 ${id} 状态失败:`, error)
    throw error
  }
}

/**
 * 启动服务
 */
export async function startService(id: string): Promise<string> {
  try {
    const invoke = getTauriInvoke()
    if (!invoke) {
      throw new Error('Tauri API 不可用')
    }
    const result = await invoke<string>('start_service', { id })
    return result
  } catch (error) {
    console.error(`[ServiceManager] 启动服务 ${id} 失败:`, error)
    throw error
  }
}

/**
 * 停止服务
 */
export async function stopService(id: string): Promise<string> {
  try {
    const invoke = getTauriInvoke()
    if (!invoke) {
      throw new Error('Tauri API 不可用')
    }
    const result = await invoke<string>('stop_service', { id })
    return result
  } catch (error) {
    console.error(`[ServiceManager] 停止服务 ${id} 失败:`, error)
    throw error
  }
}

/**
 * 重启服务
 */
export async function restartService(id: string): Promise<string> {
  try {
    const invoke = getTauriInvoke()
    if (!invoke) {
      throw new Error('Tauri API 不可用')
    }
    const result = await invoke<string>('restart_service', { id })
    return result
  } catch (error) {
    console.error(`[ServiceManager] 重启服务 ${id} 失败:`, error)
    throw error
  }
}

/**
 * 获取服务状态显示文本
 */
export function getServiceStateText(state: ServiceStateValue): string {
  const stateMap: Record<ServiceStateValue, string> = {
    stopped: '已停止',
    starting: '启动中',
    warmup: '预热中',
    idle: '空闲',
    busy: '忙碌',
    degraded: '降级',
    unhealthy: '不健康',
    restarting: '重启中',
    stopping: '停止中',
  }
  return stateMap[state] || state
}

/**
 * 获取服务状态颜色（用于 UI）
 */
export function getServiceStateColor(state: ServiceStateValue): string {
  const colorMap: Record<ServiceStateValue, string> = {
    stopped: '#9ca3af',
    starting: '#3b82f6',
    warmup: '#f59e0b',
    idle: '#10b981',
    busy: '#3b82f6',
    degraded: '#f97316',
    unhealthy: '#ef4444',
    restarting: '#8b5cf6',
    stopping: '#9ca3af',
  }
  return colorMap[state] || '#9ca3af'
}

/**
 * 检查服务是否可用
 */
export function isServiceAvailable(service: ServiceStatusDTO): boolean {
  return service.is_available && service.is_healthy
}

