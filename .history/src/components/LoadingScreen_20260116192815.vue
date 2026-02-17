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
  background: radial-gradient(circle at top, #0f172a 0%, #020617 100%);
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
  animation: float 3s ease-in-out infinite;
}

.logo-icon {
  font-size: 64px;
  filter: drop-shadow(0 0 20px rgba(77, 163, 255, 0.4));
}

.app-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  color: #e5e7eb;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  background: linear-gradient(135deg, #e5e7eb 0%, #94a3b8 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
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
  background: rgba(148, 163, 184, 0.2);
  border-radius: 2px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, #4da3ff, #22d3ee);
  border-radius: 2px;
  transition: width 0.2s ease-out;
  box-shadow: 0 0 10px rgba(77, 163, 255, 0.5);
}

.status-text {
  font-size: 12px;
  color: #94a3b8;
  text-align: center;
  font-family: monospace;
  height: 1.5em; /* Prevent layout shift */
}

@keyframes float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-10px); }
}
</style>
