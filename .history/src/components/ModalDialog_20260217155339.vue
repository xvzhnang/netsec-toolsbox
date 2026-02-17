<script setup lang="ts">
import { ref } from 'vue'

interface Props {
  title: string
  visible: boolean
  collapsible?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  collapsible: true,
})

const emit = defineEmits<{
  close: []
  'update:visible': [value: boolean]
}>()

const isCollapsed = ref(false)

const close = () => {
  emit('update:visible', false)
  emit('close')
}

const toggleCollapse = () => {
  isCollapsed.value = !isCollapsed.value
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="visible" class="modal-overlay" @click.self="close">
        <div class="modal-container" :class="{ collapsed: isCollapsed && collapsible }">
          <header class="modal-header">
            <h3 class="modal-title">{{ title }}</h3>
            <div class="modal-actions">
              <button
                v-if="collapsible"
                type="button"
                class="modal-btn-icon"
                @click="toggleCollapse"
                :title="isCollapsed ? '展开' : '收起'"
              >
                {{ isCollapsed ? '▼' : '▲' }}
              </button>
              <button type="button" class="modal-btn-icon" @click="close" title="关闭">✕</button>
            </div>
          </header>
          <div v-if="!isCollapsed || !collapsible" class="modal-body">
            <slot />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 2000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(2px);
}

.modal-container {
  width: 90%;
  max-width: 600px;
  max-height: 85vh;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transition: max-height 0.3s ease-out;
}

.modal-container.collapsed {
  max-height: 60px;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-secondary);
}

.modal-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.modal-actions {
  display: flex;
  gap: 6px;
}

.modal-btn-icon {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.14s ease-out;
}

.modal-btn-icon:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.modal-body {
  flex: 1;
  padding: 18px;
  overflow-y: auto;
  color: var(--text-primary);
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease-out;
}

.modal-enter-active .modal-container,
.modal-leave-active .modal-container {
  transition: opacity 0.2s ease-out, transform 0.2s ease-out;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal-container,
.modal-leave-to .modal-container {
  opacity: 0;
  transform: scale(0.95) translateY(-10px);
}
</style>

