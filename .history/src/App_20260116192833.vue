<template>
  <LoadingScreen 
    :visible="isLoading" 
    :progress="loadProgress" 
    :status="loadStatus"
  />
  <ErrorBoundary v-show="!isLoading">
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
import { Transition, onMounted, ref } from 'vue'
import ErrorBoundary from './components/ErrorBoundary.vue'
import LoadingScreen from './components/LoadingScreen.vue'
import { useModuleStatusStore } from './stores/moduleStatus'
import { categoriesConfig } from './stores/categories'

const moduleStatusStore = useModuleStatusStore()
const isLoading = ref(true)
const loadProgress = ref(0)
const loadStatus = ref('Initializing...')

// 模拟加载步骤的辅助函数
const runLoadStep = async (
  name: string, 
  weight: number, 
  task: () => Promise<void>
) => {
  loadStatus.value = name
  try {
    await task()
  } catch (e) {
    console.warn(`Step ${name} failed:`, e)
  }
  loadProgress.value += weight
}

onMounted(async () => {
  // 步骤 1: 加载字体和基础样式 (10%)
  await runLoadStep('Loading Fonts & Styles...', 10, async () => {
    await document.fonts.ready
  })

  // 步骤 2: 加载配置数据 (20%)
  await runLoadStep('Loading Configuration...', 20, async () => {
    // 确保 categoriesConfig 已从缓存或默认值加载（同步过程，此处仅做检查）
    if (!categoriesConfig.value.length) {
      // 理论上 store 初始化时已经同步读取了，这里只是为了进度条效果
      await new Promise(resolve => setTimeout(resolve, 100))
    }
  })

  // 步骤 3: 预加载 AI 模块 JS (40%)
  // 关键：在这里强制加载 AI 组件的 JS 资源，确保显示 UI 后点击即开
  await runLoadStep('Initializing AI Module...', 40, async () => {
    try {
      await import('./components/AiAssistantPanel.vue')
    } catch (e) {
      console.error('Failed to preload AI module:', e)
    }
  })

  // 步骤 4: 检查后端服务状态 (20%)
  await runLoadStep('Connecting to Backend Services...', 20, async () => {
    // 启动状态监听
    await moduleStatusStore.startListening()
    // 等待一个短暂时间以获取最新状态，但不要阻塞太久以免超时
    // 如果后端太慢，我们只保证连接建立，不保证 ready
    await new Promise(resolve => setTimeout(resolve, 500))
  })

  // 步骤 5: 完成 (10%)
  loadProgress.value = 100
  loadStatus.value = 'Ready!'
  
  // 给用户一点时间看到 100%
  setTimeout(() => {
    isLoading.value = false
  }, 400)
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
