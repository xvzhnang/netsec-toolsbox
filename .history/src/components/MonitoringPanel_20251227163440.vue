<template>
  <div class="monitoring-panel">
    <h2 class="panel-title">线程监控面板</h2>
    <div class="thread-list">
      <div
        v-for="(status, name) in threadStatuses"
        :key="name"
        class="thread-item"
        :class="{ 'thread-dead': !status.is_alive }"
      >
        <div class="thread-header">
          <span class="thread-name">{{ status.name }}</span>
          <span
            class="thread-status"
            :class="{
              'status-alive': status.is_alive,
              'status-dead': !status.is_alive,
            }"
          >
            {{ status.is_alive ? '🟢 正常' : '🔴 超时' }}
          </span>
        </div>
        <div class="thread-details">
          <div class="detail-item">
            <span class="detail-label">最后心跳:</span>
            <span class="detail-value">{{ formatLastPing(status.last_ping_ms) }}</span>
          </div>
          <div class="detail-item">
            <span class="detail-label">重启次数:</span>
            <span class="detail-value">{{ status.restart_count }}</span>
          </div>
          <div v-if="status.last_restart" class="detail-item">
            <span class="detail-label">最后重启:</span>
            <span class="detail-value">{{ formatTime(status.last_restart) }}</span>
          </div>
        </div>
      </div>
      <div v-if="Object.keys(threadStatuses).length === 0" class="empty-state">
        暂无线程监控数据
      </div>
    </div>
    <div class="panel-footer">
      <button @click="refreshStatus" class="refresh-btn" :disabled="loading">
        {{ loading ? '刷新中...' : '刷新状态' }}
      </button>
      <span class="last-update">最后更新: {{ lastUpdateTime }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { getTauriInvoke } from '../utils/tauri'
import { info } from '../utils/logger'

interface ThreadStatus {
  name: string
  is_alive: boolean
  last_ping_ms: number
  restart_count: number
  last_restart: number | null
}

const threadStatuses = ref<Record<string, ThreadStatus>>({})
const loading = ref(false)
const lastUpdateTime = ref<string>('')
let refreshInterval: number | null = null

const invoke = getTauriInvoke()

const refreshStatus = async () => {
  if (!invoke) {
    info('[MonitoringPanel] Tauri invoke 不可用')
    return
  }

  loading.value = true
  try {
    const statuses = await invoke<Record<string, ThreadStatus>>(
      'get_thread_heartbeat_status'
    )
    threadStatuses.value = statuses
    lastUpdateTime.value = new Date().toLocaleTimeString()
    info('[MonitoringPanel] 线程状态已更新', statuses)
  } catch (error) {
    console.error('[MonitoringPanel] 获取线程状态失败:', error)
  } finally {
    loading.value = false
  }
}

const formatLastPing = (ms: number): string => {
  if (ms === 0) return '从未'
  const now = Date.now()
  const elapsed = now - ms
  if (elapsed < 1000) return '刚刚'
  if (elapsed < 60000) return `${Math.floor(elapsed / 1000)}秒前`
  if (elapsed < 3600000) return `${Math.floor(elapsed / 60000)}分钟前`
  return `${Math.floor(elapsed / 3600000)}小时前`
}

const formatTime = (timestamp: number): string => {
  return new Date(timestamp).toLocaleString()
}

onMounted(() => {
  refreshStatus()
  // 每 5 秒自动刷新
  refreshInterval = window.setInterval(refreshStatus, 5000)
})

onUnmounted(() => {
  if (refreshInterval !== null) {
    clearInterval(refreshInterval)
  }
})
</script>

<style scoped>
.monitoring-panel {
  padding: 20px;
  background: var(--bg-secondary, #f5f5f5);
  border-radius: 8px;
}

.panel-title {
  font-size: 1.5rem;
  font-weight: 600;
  margin-bottom: 20px;
  color: var(--text-primary, #333);
}

.thread-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.thread-item {
  background: var(--bg-primary, #fff);
  border: 1px solid var(--border-color, #e0e0e0);
  border-radius: 6px;
  padding: 16px;
  transition: all 0.2s;
}

.thread-item:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.thread-item.thread-dead {
  border-color: #f44336;
  background: #ffebee;
}

.thread-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.thread-name {
  font-weight: 600;
  font-size: 1.1rem;
  color: var(--text-primary, #333);
}

.thread-status {
  padding: 4px 12px;
  border-radius: 12px;
  font-size: 0.9rem;
  font-weight: 500;
}

.status-alive {
  background: #e8f5e9;
  color: #2e7d32;
}

.status-dead {
  background: #ffebee;
  color: #c62828;
}

.thread-details {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.detail-item {
  display: flex;
  justify-content: space-between;
  font-size: 0.9rem;
}

.detail-label {
  color: var(--text-secondary, #666);
}

.detail-value {
  color: var(--text-primary, #333);
  font-weight: 500;
}

.empty-state {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary, #999);
}

.panel-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 20px;
  padding-top: 20px;
  border-top: 1px solid var(--border-color, #e0e0e0);
}

.refresh-btn {
  padding: 8px 16px;
  background: var(--primary-color, #1976d2);
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9rem;
  transition: background 0.2s;
}

.refresh-btn:hover:not(:disabled) {
  background: var(--primary-hover, #1565c0);
}

.refresh-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.last-update {
  font-size: 0.85rem;
  color: var(--text-secondary, #666);
}
</style>

