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
  backdrop-filter: blur(4px);
}

.modal-container {
  width: 90%;
  max-width: 420px;
  border-radius: 16px;
  border: 1px solid rgba(148, 163, 184, 0.4);
  background: linear-gradient(145deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.96));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 24px 60px rgba(0, 0, 0, 0.9);
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 18px 20px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.2);
  background: rgba(15, 23, 42, 0.95);
}

.modal-icon {
  font-size: 24px;
  flex-shrink: 0;
}

.modal-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #e5e7eb;
}

.modal-body {
  padding: 20px;
}

.modal-message {
  margin: 0;
  font-size: 14px;
  color: #9ca3af;
  line-height: 1.6;
}

.modal-footer {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  padding: 16px 20px;
  border-top: 1px solid rgba(148, 163, 184, 0.15);
  background: rgba(15, 23, 42, 0.95);
}

.btn {
  padding: 8px 16px;
  border-radius: 8px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background: rgba(15, 23, 42, 0.9);
  color: #e5e7eb;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.14s ease-out;
  min-width: 80px;
}

.btn:hover {
  border-color: rgba(148, 163, 184, 0.8);
  background: rgba(15, 23, 42, 0.98);
  transform: translateY(-1px);
}

.btn.cancel {
  background: transparent;
  border-color: rgba(148, 163, 184, 0.3);
}

.btn.cancel:hover {
  background: rgba(15, 23, 42, 0.6);
}

.btn.confirm {
  border-color: #4da3ff;
  background: linear-gradient(135deg, #4da3ff, #22d3ee);
  color: #0b1120;
}

.btn.confirm:hover {
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 10px 24px rgba(37, 99, 235, 0.9);
}

.btn.confirm.danger {
  border-color: #f87171;
  background: linear-gradient(135deg, #f87171, #fb7185);
  color: #ffffff;
}

.btn.confirm.danger:hover {
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 10px 24px rgba(248, 113, 113, 0.9);
}

.btn.confirm.warning {
  border-color: #fbbf24;
  background: linear-gradient(135deg, #fbbf24, #f59e0b);
  color: #0b1120;
}

.btn.confirm.warning:hover {
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 10px 24px rgba(251, 191, 36, 0.9);
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

