<template>
  <LoadingScreen :visible="isLoading" :progress="loadProgress" :status="loadStatus" />
  <div class="app-container">
    <ErrorBoundary>
      <router-view v-slot="{ Component, route }">
        <Transition :name="(route.meta?.transition as string) || 'fade'" mode="out-in">
          <component :is="Component" :key="route.path" />
        </Transition>
      </router-view>
    </ErrorBoundary>
  </div>
</template>

<script setup lang="ts">
import { Transition, nextTick, onMounted, ref } from 'vue'
import ErrorBoundary from './components/ErrorBoundary.vue'
import LoadingScreen from './components/LoadingScreen.vue'
import { useModuleStatusStore } from './stores/moduleStatus'

const moduleStatusStore = useModuleStatusStore()
const isLoading = ref(true)
const loadProgress = ref(0)
const loadStatus = ref('Starting...')

onMounted(() => {
  loadProgress.value = 20
  loadStatus.value = 'Rendering UI...'

  void nextTick().then(() => {
    requestAnimationFrame(() => {
      loadProgress.value = 100
      loadStatus.value = 'Ready'
      isLoading.value = false
      window.dispatchEvent(new Event('app-ready'))
    })
  })

  void moduleStatusStore.startListening().catch(() => {})

  const preloadAi = async () => {
    try {
      await import('./components/AiAssistantPanel.vue')
    } catch {
      return
    }
  }

  const ric = (window as unknown as { requestIdleCallback?: (cb: () => void, opts?: { timeout?: number }) => void })
    .requestIdleCallback

  if (ric) {
    ric(() => {
      void preloadAi()
    }, { timeout: 2000 })
  } else {
    setTimeout(() => {
      void preloadAi()
    }, 2000)
  }
})
</script>

<style>
/* 确保应用容器占满全屏 */
.app-container {
  width: 100%;
  height: 100%;
}
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
