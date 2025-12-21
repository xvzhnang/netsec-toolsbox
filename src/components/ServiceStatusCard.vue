<template>
  <div class="service-card" :class="cardClass">
    <div class="service-header">
      <div class="service-title">
        <span class="status-dot" :class="statusDotClass"></span>
        <span class="service-name">{{ service.name }}</span>
      </div>
      <div class="service-actions">
        <button
          v-if="service.state === 'stopped'"
          type="button"
          class="action-btn start"
          @click="handleStart"
          :disabled="actionInProgress"
          title="启动服务"
        >
          ▶️
        </button>
        <button
          v-else-if="service.state === 'idle' || service.state === 'busy'"
          type="button"
          class="action-btn stop"
          @click="handleStop"
          :disabled="actionInProgress"
          title="停止服务"
        >
          ⏹️
        </button>
        <button
          v-else-if="service.state === 'unhealthy'"
          type="button"
          class="action-btn restart"
          @click="handleRestart"
          :disabled="actionInProgress"
          title="重启服务"
        >
          🔄
        </button>
      </div>
    </div>
    
    <div class="service-body">
      <div class="service-state">
        <span class="state-label">状态:</span>
        <span class="state-value" :class="stateValueClass">
          {{ getStateText(service.state) }}
        </span>
      </div>
      
      <div v-if="service.message" class="service-message">
        {{ service.message }}
      </div>
      
      <div v-if="service.description" class="service-description">
        {{ service.description }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { 
  type ServiceStatusDTO, 
  startService,
  stopService,
  restartService
} from '../utils/serviceManager'
import { onServiceStateChanged, onServiceError } from '../utils/serviceEvents'
import { info, error as logError } from '../utils/logger'

interface Props {
  service: ServiceStatusDTO
}

const props = defineProps<Props>()
const emit = defineEmits<{
  updated: []
}>()

const actionInProgress = ref(false)

const cardClass = computed(() => {
  return {
    'healthy': props.service.is_healthy && props.service.is_available,
    'degraded': props.service.state === 'degraded',
    'unhealthy': !props.service.is_healthy,
    'stopped': props.service.state === 'stopped',
  }
})

const statusDotClass = computed(() => {
  return {
    'active': props.service.is_available && props.service.is_healthy,
    'degraded': props.service.state === 'degraded',
    'error': !props.service.is_healthy,
  }
})

const stateValueClass = computed(() => {
  return {
    'state-idle': props.service.state === 'idle',
    'state-busy': props.service.state === 'busy',
    'state-degraded': props.service.state === 'degraded',
    'state-unhealthy': props.service.state === 'unhealthy',
    'state-stopped': props.service.state === 'stopped',
  }
})

const getStateText = (state: string) => {
  const stateMap: Record<string, string> = {
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

const handleStart = async () => {
  actionInProgress.value = true
  try {
    await startService(props.service.id)
    info(`[ServiceCard] 服务 ${props.service.name} 已启动`)
    emit('updated')
  } catch (err) {
    logError(`[ServiceCard] 启动服务失败:`, err)
  } finally {
    actionInProgress.value = false
  }
}

const handleStop = async () => {
  actionInProgress.value = true
  try {
    await stopService(props.service.id)
    info(`[ServiceCard] 服务 ${props.service.name} 已停止`)
    emit('updated')
  } catch (err) {
    logError(`[ServiceCard] 停止服务失败:`, err)
  } finally {
    actionInProgress.value = false
  }
}

const handleRestart = async () => {
  actionInProgress.value = true
  try {
    await restartService(props.service.id)
    info(`[ServiceCard] 服务 ${props.service.name} 已重启`)
    emit('updated')
  } catch (err) {
    logError(`[ServiceCard] 重启服务失败:`, err)
  } finally {
    actionInProgress.value = false
  }
}

// 订阅服务事件（实时更新状态）
onMounted(() => {
  const unsubscribeState = onServiceStateChanged((event) => {
    if (event.service_id === props.service.id) {
      // 状态变化，触发更新
      emit('updated')
    }
  })

  const unsubscribeError = onServiceError((event) => {
    if (event.service_id === props.service.id) {
      // 错误事件，触发更新
      emit('updated')
    }
  })

  onUnmounted(() => {
    unsubscribeState()
    unsubscribeError()
  })
})
</script>

<style scoped>
.service-card {
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid rgba(148, 163, 184, 0.1);
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 12px;
  transition: all 0.2s;
}

.service-card.healthy {
  border-color: rgba(34, 197, 94, 0.3);
}

.service-card.degraded {
  border-color: rgba(251, 146, 60, 0.3);
}

.service-card.unhealthy {
  border-color: rgba(239, 68, 68, 0.3);
}

.service-card.stopped {
  border-color: rgba(148, 163, 184, 0.2);
  opacity: 0.7;
}

.service-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.service-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #64748b;
  transition: background 0.2s;
}

.status-dot.active {
  background: #22c55e;
  box-shadow: 0 0 8px rgba(34, 197, 94, 0.5);
}

.status-dot.degraded {
  background: #fb923c;
}

.status-dot.error {
  background: #ef4444;
}

.service-name {
  font-weight: 600;
  color: #f1f5f9;
}

.service-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  background: transparent;
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 4px;
  padding: 4px 8px;
  cursor: pointer;
  transition: all 0.2s;
  font-size: 14px;
}

.action-btn:hover:not(:disabled) {
  background: rgba(148, 163, 184, 0.1);
  border-color: rgba(148, 163, 184, 0.4);
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.service-body {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.service-state {
  display: flex;
  align-items: center;
  gap: 8px;
}

.state-label {
  color: #94a3b8;
  font-size: 13px;
}

.state-value {
  font-weight: 500;
  font-size: 13px;
}

.state-value.state-idle {
  color: #22c55e;
}

.state-value.state-busy {
  color: #3b82f6;
}

.state-value.state-degraded {
  color: #fb923c;
}

.state-value.state-unhealthy {
  color: #ef4444;
}

.state-value.state-stopped {
  color: #64748b;
}

.service-message {
  color: #cbd5e1;
  font-size: 13px;
  padding: 8px;
  background: rgba(15, 23, 42, 0.5);
  border-radius: 4px;
}

.service-description {
  color: #94a3b8;
  font-size: 12px;
}
</style>

