<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  visible: boolean
  title: string
  message: string
  confirmText?: string
  cancelText?: string
  type?: 'danger' | 'warning' | 'info'
}

const props = withDefaults(defineProps<Props>(), {
  confirmText: '确认',
  cancelText: '取消',
  type: 'info',
})

const emit = defineEmits<{
  confirm: []
  cancel: []
  'update:visible': [value: boolean]
}>()

const close = () => {
  emit('update:visible', false)
  emit('cancel')
}

const confirm = () => {
  emit('update:visible', false)
  emit('confirm')
}

const icon = computed(() => {
  switch (props.type) {
    case 'danger':
      return '⚠️'
    case 'warning':
      return '⚠️'
    case 'info':
    default:
      return 'ℹ️'
  }
})
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="visible" class="modal-overlay" @click.self="close">
        <div class="modal-container">
          <div class="modal-header">
            <div class="modal-icon">{{ icon }}</div>
            <h3 class="modal-title">{{ title }}</h3>
          </div>
          <div class="modal-body">
            <p class="modal-message">{{ message }}</p>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn cancel" @click="close">
              {{ cancelText }}
            </button>
            <button
              type="button"
              class="btn confirm"
              :class="type"
              @click="confirm"
            >
              {{ confirmText }}
            </button>
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
  z-index: 3000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(2px);
}

.modal-container {
  width: 90%;
  max-width: 420px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 18px 20px;
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-secondary);
}

.modal-icon {
  font-size: 24px;
  flex-shrink: 0;
}

.modal-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.modal-body {
  padding: 20px;
  color: var(--text-primary);
}

.modal-message {
  margin: 0;
  font-size: 14px;
  color: var(--text-secondary);
  line-height: 1.6;
}

.modal-footer {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  padding: 16px 20px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-secondary);
}

.btn {
  padding: 8px 16px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-primary);
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.14s ease-out;
  min-width: 80px;
}

.btn:hover {
  background: var(--bg-hover);
  border-color: var(--text-primary);
}

.btn.cancel {
  background: transparent;
  border-color: var(--border-color);
}

.btn.cancel:hover {
  background: var(--bg-hover);
}

.btn.confirm {
  border-color: var(--accent-color);
  background: var(--accent-color);
  color: white;
}

.btn.confirm:hover {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
}

.btn.confirm.danger {
  border-color: #ef4444;
  background: #ef4444;
  color: #ffffff;
}

.btn.confirm.danger:hover {
  background: #dc2626;
  border-color: #dc2626;
}

.btn.confirm.warning {
  border-color: #f59e0b;
  background: #f59e0b;
  color: #ffffff;
}

.btn.confirm.warning:hover {
  background: #d97706;
  border-color: #d97706;
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

