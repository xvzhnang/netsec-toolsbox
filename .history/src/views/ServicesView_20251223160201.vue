<template>
  <div class="services-view">
    <header class="view-header">
      <h1 class="view-title">服务管理</h1>
      <p class="view-description">统一管理所有服务（AI Gateway、Wiki、工具等）</p>
    </header>

    <main class="services-content">
      <div class="services-list">
        <ServiceStatusCard
          v-for="service in services"
          :key="service.id"
          :service="service"
          @updated="refreshServices"
        />
      </div>

      <div v-if="services.length === 0" class="empty-state">
        <p>暂无注册的服务</p>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import ServiceStatusCard from '../components/ServiceStatusCard.vue'
import { getAllServices, type ServiceStatusDTO } from '../utils/serviceManager'
import { onServiceEvent, startServiceEventPolling, stopServiceEventPolling } from '../utils/serviceEvents'
import { error as logError } from '../utils/logger'

const services = ref<ServiceStatusDTO[]>([])
let refreshInterval: ReturnType<typeof setInterval> | undefined = undefined

const refreshServices = async () => {
  try {
    const result = await getAllServices()
    services.value = result.services
  } catch (err) {
    logError('[ServicesView] 获取服务列表失败:', err)
  }
}

onMounted(() => {
  refreshServices()
  
  // 订阅服务事件（实时更新）
  const unsubscribe = onServiceEvent((_event) => {
    // 任何服务事件都触发刷新
    refreshServices()
  })
  
  // 启动事件轮询（临时方案，等待 WebSocket/SSE）
  startServiceEventPolling(2000)
  
  // 每 10 秒自动刷新服务状态（作为备份）
  refreshInterval = setInterval(() => {
    refreshServices()
  }, 10000)
  
  onUnmounted(() => {
    unsubscribe()
    stopServiceEventPolling()
    if (refreshInterval) {
      clearInterval(refreshInterval)
    }
  })
})
</script>

<style scoped>
.services-view {
  padding: 24px;
  max-width: 1200px;
  margin: 0 auto;
}

.view-header {
  margin-bottom: 32px;
}

.view-title {
  font-size: 28px;
  font-weight: 700;
  color: #f1f5f9;
  margin-bottom: 8px;
}

.view-description {
  color: #94a3b8;
  font-size: 14px;
}

.services-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.services-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
  gap: 16px;
}

.empty-state {
  text-align: center;
  padding: 48px;
  color: #64748b;
}
</style>

