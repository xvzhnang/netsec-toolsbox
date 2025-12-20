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
import { startService, getServiceStatus } from './utils/serviceManager'
import { getTauriInvoke } from './utils/tauri'
import { debug, info } from './utils/logger'

// 应用启动时自动启动 AI Gateway 服务（使用统一服务管理）
onMounted(() => {
  // 使用 setTimeout 将启动任务放到下一个事件循环，不阻塞 UI 渲染
  setTimeout(async () => {
    try {
      // 等待 Tauri API 可用
      const invoker = getTauriInvoke()
      if (!invoker) {
        debug('Tauri API 不可用，跳过 AI Gateway 服务自动启动')
        return
      }

      // 检查服务状态（使用统一服务管理）
      try {
        const serviceStatus = await getServiceStatus('ai-gateway')
        if (serviceStatus && serviceStatus.is_available) {
          debug('AI Gateway 服务已在运行，跳过启动')
          return
        }
      } catch (error) {
        debug('检查 AI Gateway 服务状态失败，尝试启动:', error)
      }

      // 使用统一服务管理启动 AI Gateway
      info('正在后台启动 AI Gateway 服务（统一服务管理）...')
      startService('ai-gateway').then(() => {
        info('AI Gateway 服务已自动启动')
      }).catch(error => {
        debug('后台启动 AI Gateway 服务失败:', error)
        // 服务管理器会自动重试，这里只记录日志
      })
    } catch (error) {
      // 静默处理错误，不影响应用启动
      debug('自动启动 AI Gateway 服务失败（不影响应用使用）:', error)
    }
  }, 100) // 延迟 100ms 执行，让 UI 先渲染
})
</script>

<style>
/* 路由切换过渡动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), 
              transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.fade-enter-from {
  opacity: 0;
  transform: translateY(10px) scale(0.98);
}

.fade-leave-to {
  opacity: 0;
  transform: translateY(-10px) scale(0.98);
}

/* 滑动过渡 */
.slide-enter-active,
.slide-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  position: absolute;
  width: 100%;
  height: 100%;
  top: 0;
  left: 0;
  will-change: transform, opacity;
}

.slide-enter-from {
  opacity: 0;
  transform: translateX(40px) scale(0.96);
}

.slide-leave-to {
  opacity: 0;
  transform: translateX(-40px) scale(0.96);
}

/* 确保路由视图容器正确布局 */
.router-view-container {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--gradient-bg);
}
</style>
