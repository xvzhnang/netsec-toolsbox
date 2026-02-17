<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'

const props = defineProps<{
  title?: string
  subtitle?: string
  showBack?: boolean
  noPadding?: boolean
}>()

const router = useRouter()

const goBack = () => {
  router.back()
}
</script>

<template>
  <div class="app-layout">
    <header class="app-header">
      <div class="header-left">
        <button
          v-if="showBack"
          type="button"
          class="back-button"
          title="返回上层"
          @click="goBack"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="19" y1="12" x2="5" y2="12"></line>
            <polyline points="12 19 5 12 12 5"></polyline>
          </svg>
        </button>
        <div class="title-block">
          <h1 class="title">{{ title || 'NetSec Toolbox' }}</h1>
          <p v-if="subtitle" class="subtitle">{{ subtitle }}</p>
        </div>
      </div>
      
      <div class="header-actions">
        <slot name="header-actions"></slot>
      </div>
    </header>

    <div class="app-body">
      <aside v-if="$slots.sidebar" class="app-sidebar">
        <slot name="sidebar"></slot>
      </aside>

      <main class="app-content" :class="{ 'no-padding': noPadding }">
        <slot></slot>
      </main>
    </div>

    <footer class="app-footer">
      <div class="footer-content">
        <span class="copyright">© 2025 By 序章</span>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.app-layout {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: var(--bg-primary);
  color: var(--text-primary);
  overflow: hidden;
}

.app-header {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 24px;
  border-bottom: 1px solid var(--border-color);
  background-color: var(--bg-secondary);
  z-index: 10;
  box-shadow: var(--shadow-sm);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.back-button {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.back-button:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
  border-color: var(--text-primary);
}

.title-block {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.3px;
}

.subtitle {
  margin: 0;
  font-size: 12px;
  color: var(--text-muted);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.app-body {
  flex: 1;
  display: flex;
  min-height: 0;
  overflow: hidden;
  position: relative;
}

.app-sidebar {
  flex: 0 0 240px;
  border-right: 1px solid var(--border-color);
  background: var(--bg-secondary);
  overflow-y: auto;
  z-index: 5;
}

.app-content {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.app-content.no-padding {
  padding: 0;
}

.app-footer {
  flex: 0 0 auto;
  padding: 12px 24px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-secondary);
  z-index: 10;
}

.footer-content {
  display: flex;
  justify-content: center;
  align-items: center;
}

.copyright {
  font-size: 12px;
  color: var(--text-muted);
}

/* Responsive adjustments */
@media (max-width: 768px) {
  .app-header {
    padding: 12px 16px;
  }
  
  .app-content {
    padding: 16px;
  }
  
  .app-sidebar {
    display: none; /* Consider a toggle or overlay for mobile sidebar later */
  }
}
</style>
