/**
 * 服务事件推送工具（WebSocket/SSE）
 * 
 * 前端通过事件推送接收服务状态变化，无需轮询
 */

import { getTauriInvoke } from './tauri'
import type { ServiceEvent, ServiceStatusDTO } from './serviceManager'

export interface ServiceEventCallback {
  (event: ServiceEvent): void
}

class ServiceEventManager {
  private callbacks: Map<string, ServiceEventCallback[]> = new Map()
  private pollingInterval: ReturnType<typeof setInterval> | null = null
  private lastEventTimestamp: number = 0

  /**
   * 订阅服务事件
   */
  subscribe(eventType: string, callback: ServiceEventCallback): () => void {
    if (!this.callbacks.has(eventType)) {
      this.callbacks.set(eventType, [])
    }
    this.callbacks.get(eventType)!.push(callback)

    // 返回取消订阅函数
    return () => {
      const callbacks = this.callbacks.get(eventType)
      if (callbacks) {
        const index = callbacks.indexOf(callback)
        if (index > -1) {
          callbacks.splice(index, 1)
        }
      }
    }
  }

  /**
   * 订阅所有事件
   */
  subscribeAll(callback: ServiceEventCallback): () => void {
    return this.subscribe('*', callback)
  }

  /**
   * 触发事件回调
   */
  private emit(event: ServiceEvent) {
    // 触发特定类型的事件
    const typeCallbacks = this.callbacks.get(event.type || '*')
    if (typeCallbacks) {
      typeCallbacks.forEach(cb => {
        try {
          cb(event)
        } catch (error) {
          console.error('[ServiceEvents] 事件回调执行失败:', error)
        }
      })
    }

    // 触发所有事件的回调
    const allCallbacks = this.callbacks.get('*')
    if (allCallbacks) {
      allCallbacks.forEach(cb => {
        try {
          cb(event)
        } catch (error) {
          console.error('[ServiceEvents] 事件回调执行失败:', error)
        }
      })
    }
  }

  /**
   * 启动事件轮询（临时方案，等待 WebSocket/SSE 实现）
   */
  startPolling(intervalMs: number = 2000) {
    if (this.pollingInterval) {
      return // 已经在轮询
    }

    this.pollingInterval = setInterval(async () => {
      try {
        const invoke = getTauriInvoke()
        if (!invoke) {
          return // Tauri API 不可用，跳过
        }
        // 获取所有服务状态
        const result = await invoke<{ services: ServiceStatusDTO[] }>('get_all_services')
        
        // 检查状态变化（简化版，实际应该通过事件推送）
        result.services.forEach(service => {
          // 这里可以比较状态变化并触发事件
          // 实际实现中应该通过 WebSocket/SSE 接收事件
        })
      } catch (error) {
        console.error('[ServiceEvents] 轮询失败:', error)
      }
    }, intervalMs)
  }

  /**
   * 停止事件轮询
   */
  stopPolling() {
    if (this.pollingInterval) {
      clearInterval(this.pollingInterval)
      this.pollingInterval = null
    }
  }

  /**
   * 模拟接收事件（用于测试）
   */
  simulateEvent(event: ServiceEvent) {
    this.emit(event)
  }
}

// 单例
export const serviceEventManager = new ServiceEventManager()

/**
 * 订阅服务状态变化事件
 */
export function onServiceStateChanged(callback: (event: ServiceEvent & { type: 'StateChanged' }) => void): () => void {
  return serviceEventManager.subscribe('StateChanged', callback as ServiceEventCallback)
}

/**
 * 订阅服务健康检查事件
 */
export function onServiceHealthCheck(callback: (event: ServiceEvent & { type: 'HealthCheck' }) => void): () => void {
  return serviceEventManager.subscribe('HealthCheck', callback as ServiceEventCallback)
}

/**
 * 订阅服务错误事件
 */
export function onServiceError(callback: (event: ServiceEvent & { type: 'Error' }) => void): () => void {
  return serviceEventManager.subscribe('Error', callback as ServiceEventCallback)
}

/**
 * 订阅所有服务事件
 */
export function onServiceEvent(callback: ServiceEventCallback): () => void {
  return serviceEventManager.subscribeAll(callback)
}

/**
 * 启动事件监听（轮询模式，临时方案）
 */
export function startServiceEventPolling(intervalMs: number = 2000) {
  serviceEventManager.startPolling(intervalMs)
}

/**
 * 停止事件监听
 */
export function stopServiceEventPolling() {
  serviceEventManager.stopPolling()
}

