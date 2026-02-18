<template>
  <div class="loading-screen" :class="{ 'fade-out': !visible }">
    <div class="loading-content">
      <div class="logo-container">
        <div class="logo-icon">🛡️</div>
        <h1 class="app-title">NetSec Toolbox</h1>
      </div>
      
      <div class="progress-container">
        <div class="progress-bar-bg">
          <div 
            class="progress-bar-fill" 
            :style="{ width: `${progress}%` }"
          ></div>
        </div>
        <div class="status-text">{{ status }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  visible: boolean
  progress: number
  status: string
}>()
</script>

<style scoped>
.loading-screen {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: var(--bg-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: opacity 0.5s ease-out, visibility 0.5s ease-out;
}

.fade-out {
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
}

.loading-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 40px;
  width: 320px;
}

.logo-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.logo-icon {
  font-size: 64px;
}

.app-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.progress-container {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.progress-bar-bg {
  width: 100%;
  height: 4px;
  background: var(--bg-secondary);
  border-radius: 2px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  background: var(--accent-color);
  border-radius: 2px;
  transition: width 0.2s ease-out;
}

.status-text {
  font-size: 12px;
  color: var(--text-secondary);
  text-align: center;
  font-family: monospace;
  height: 1.5em; /* Prevent layout shift */
}
</style>
