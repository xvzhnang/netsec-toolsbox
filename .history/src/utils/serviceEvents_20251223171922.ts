/**
 * 服务事件推送工具（WebSocket/SSE）
 * 
 * 前端通过事件推送接收服务状态变化，无需轮询
 */

import { getTauriInvoke } from './tauri'
import type { ServiceEvent, ServiceStateValue, ServiceStatusDTO } from './serviceManager'
import { debug, warn } from './logger'

export interface ServiceEventCallback {
  (event: ServiceEvent): void
}

class ServiceEventManager {
  private callbacks: Map<string, ServiceEventCallback[]> = new Map()
  private pollingTimer: ReturnType<typeof setTimeout> | null = null
  private pollingBaseIntervalMs: number = 5000
  private pollingCurrentIntervalMs: number = 5000
  private visibilityHandler: (() => void) | null = null
  private lastServicesSnapshot: Map<string, { state: ServiceStateValue; isHealthy: boolean; isAvailable: boolean }> = new Map()

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
          warn('[ServiceEvents] 事件回调执行失败:', error)
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
          warn('[ServiceEvents] 事件回调执行失败:', error)
        }
      })
    }
  }

  /**
   * 启动事件轮询（临时方案，等待 WebSocket/SSE 实现）
   */
  startPolling(intervalMs: number = 5000) {
    if (this.pollingTimer) {
      return // 已经在轮询
    }

    this.pollingBaseIntervalMs = intervalMs
    this.pollingCurrentIntervalMs = intervalMs

    const scheduleNext = (delayMs: number) => {
      if (this.pollingTimer) {
        clearTimeout(this.pollingTimer)
      }
      this.pollingTimer = setTimeout(() => {
        void pollOnce()
      }, delayMs)
    }

    const pollOnce = async () => {
      try {
        if (typeof document !== 'undefined' && document.hidden) {
          const delay = Math.max(this.pollingCurrentIntervalMs, 15000)
          scheduleNext(delay)
          return
        }
        const invoke = getTauriInvoke()
        if (!invoke) {
          this.pollingCurrentIntervalMs = Math.min(this.pollingCurrentIntervalMs * 2, 60000)
          scheduleNext(this.pollingCurrentIntervalMs)
          return
        }
        // 获取所有服务状态
        const result = await invoke<{ services: ServiceStatusDTO[] }>('get_all_services')
        
        // 检查状态变化（简化版，实际应该通过事件推送）
        result.services.forEach((service) => {
          const prev = this.lastServicesSnapshot.get(service.id)
          if (prev) {
            const changed =
              prev.state !== service.state ||
              prev.isHealthy !== service.is_healthy ||
              prev.isAvailable !== service.is_available
            if (changed) {
              this.emit({
                type: 'StateChanged',
                service_id: service.id,
                from: prev.state,
                to: service.state,
                status: service.is_healthy ? 'Healthy' : 'Unhealthy',
                timestamp: Date.now(),
              })
            }
          }
          this.lastServicesSnapshot.set(service.id, {
            state: service.state,
            isHealthy: service.is_healthy,
            isAvailable: service.is_available,
          })
        })
        this.pollingCurrentIntervalMs = this.pollingBaseIntervalMs
        scheduleNext(this.pollingCurrentIntervalMs)
      } catch (error) {
        debug('[ServiceEvents] 轮询失败:', error)
        this.pollingCurrentIntervalMs = Math.min(this.pollingCurrentIntervalMs * 2, 60000)
        scheduleNext(this.pollingCurrentIntervalMs)
      }
    }

    if (typeof document !== 'undefined') {
      this.visibilityHandler = () => {
        if (!document.hidden) {
          this.pollingCurrentIntervalMs = this.pollingBaseIntervalMs
          scheduleNext(0)
        }
      }
      document.addEventListener('visibilitychange', this.visibilityHandler)
    }

    scheduleNext(0)
  }

  /**
   * 停止事件轮询
   */
  stopPolling() {
    if (this.pollingTimer) {
      clearTimeout(this.pollingTimer)
      this.pollingTimer = null
    }
    if (this.visibilityHandler && typeof document !== 'undefined') {
      document.removeEventListener('visibilitychange', this.visibilityHandler)
      this.visibilityHandler = null
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
export function startServiceEventPolling(intervalMs: number = 5000) {
  serviceEventManager.startPolling(intervalMs)
}

/**
 * 停止事件监听
 */
export function stopServiceEventPolling() {
  serviceEventManager.stopPolling()
}

