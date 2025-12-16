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
import { Transition } from 'vue'
import ErrorBoundary from './components/ErrorBoundary.vue'
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
