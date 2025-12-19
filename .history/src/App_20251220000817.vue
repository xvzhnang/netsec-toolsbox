<template>
  <ErrorBoundary>
    <router-view v-slot="{ Component, route }">
      <Transition
        :name="(route.meta?.transition as string) || 'fade'"
        mode="out-in"
        appear
      >
        <component :is="Component" :key="route.path" />
      </Transition>
    </router-view>
  </ErrorBoundary>
</template>

<script setup lang="ts">
import { Transition, onMounted } from 'vue'
import ErrorBoundary from './components/ErrorBoundary.vue'
import { startAIService, waitForAIService } from './utils/aiService'
import { getTauriInvoke } from './utils/tauri'
import { debug, info, warn } from './utils/logger'

// 应用启动时自动启动 AI Gateway 服务
onMounted(async () => {
  try {
    // 等待 Tauri API 可用
    const invoker = getTauriInvoke()
    if (!invoker) {
      debug('Tauri API 不可用，跳过 AI Gateway 服务自动启动')
      return
    }

    // 检查服务是否已经在运行
    try {
      const isRunning = await invoker('check_ai_service_status') as boolean
      if (isRunning) {
        debug('AI Gateway 服务已在运行，跳过启动')
        return
      }
    } catch (error) {
      debug('检查 AI Gateway 服务状态失败，尝试启动:', error)
    }

    // 启动 AI Gateway 服务
    info('正在自动启动 AI Gateway 服务...')
    await startAIService()
    
    // 等待服务就绪（最多等待 10 秒）
    const isReady = await waitForAIService(10, 1000)
    if (isReady) {
      info('AI Gateway 服务已自动启动并就绪')
    } else {
      warn('AI Gateway 服务启动超时，但将继续尝试连接')
    }
  } catch (error) {
    // 静默处理错误，不影响应用启动
    debug('自动启动 AI Gateway 服务失败（不影响应用使用）:', error)
  }
})
</script>

<style>
/* 路由切换过渡动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.fade-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

/* 滑动过渡 */
.slide-enter-active,
.slide-leave-active {
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  position: absolute;
  width: 100%;
  height: 100%;
  top: 0;
  left: 0;
}

.slide-enter-from {
  opacity: 0;
  transform: translateX(30px);
}

.slide-leave-to {
  opacity: 0;
  transform: translateX(-30px);
}

/* 确保路由视图容器正确布局 */
.router-view-container {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
}
</style>
