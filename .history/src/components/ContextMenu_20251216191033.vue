<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'

export interface MenuItem {
  label: string
  icon?: string
  action: () => void
  danger?: boolean
  disabled?: boolean
}

interface Props {
  items: MenuItem[]
}

const props = defineProps<Props>()

const emit = defineEmits<{
  close: []
}>()

const menuRef = ref<HTMLElement | null>(null)
const x = ref(-9999)
const y = ref(-9999)
const visible = ref(false)

const handleClickOutside = (e: MouseEvent) => {
  if (menuRef.value && !menuRef.value.contains(e.target as Node)) {
    close()
  }
}

const handleItemClick = (item: MenuItem) => {
  if (!item.disabled) {
    item.action()
    close()
  }
}

const close = () => {
  visible.value = false
  emit('close')
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
  document.addEventListener('contextmenu', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
  document.removeEventListener('contextmenu', handleClickOutside)
})

defineExpose({
  show: (clientX: number, clientY: number) => {
    // 先设置位置和可见性
    x.value = clientX
    y.value = clientY
    visible.value = true
    // 使用 nextTick 确保 DOM 更新后再调整位置
    nextTick(() => {
      if (menuRef.value) {
        const rect = menuRef.value.getBoundingClientRect()
        const viewportWidth = window.innerWidth
        const viewportHeight = window.innerHeight
        
        let newX = clientX
        let newY = clientY
        
        // 如果菜单超出右边界，调整到左侧
        if (rect.right > viewportWidth) {
          newX = clientX - rect.width
        }
        // 如果菜单超出下边界，调整到上方
        if (rect.bottom > viewportHeight) {
          newY = clientY - rect.height
        }
        // 确保不会超出左边界和上边界
        if (newX < 0) newX = 8
        if (newY < 0) newY = 8
        
        x.value = newX
        y.value = newY
      }
    })
  },
})
</script>

<template>
  <div
    v-if="visible"
    ref="menuRef"
    class="context-menu"
    :style="{ left: `${x}px`, top: `${y}px` }"
  >
    <button
      v-for="(item, idx) in items"
      :key="idx"
      type="button"
      class="menu-item"
      :class="{ danger: item.danger, disabled: item.disabled }"
      @click="handleItemClick(item)"
    >
      <span v-if="item.icon" class="menu-icon">{{ item.icon }}</span>
      <span class="menu-label">{{ item.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.context-menu {
  position: fixed;
  z-index: 1000;
  min-width: 160px;
  border-radius: 10px;
  border: 1px solid rgba(148, 163, 184, 0.4);
  background: rgba(15, 23, 42, 0.98);
  backdrop-filter: blur(12px);
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 12px 28px rgba(0, 0, 0, 0.85);
  padding: 4px;
  display: flex;
  flex-direction: column;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border: none;
  background: transparent;
  color: #e5e7eb;
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  border-radius: 6px;
  transition: background 0.14s ease-out;
}

.menu-item:hover:not(.disabled) {
  background: rgba(77, 163, 255, 0.2);
}

.menu-item.danger {
  color: #f87171;
}

.menu-item.danger:hover:not(.disabled) {
  background: rgba(248, 113, 113, 0.15);
}

.menu-item.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.menu-icon {
  font-size: 14px;
  width: 16px;
  text-align: center;
}

.menu-label {
  flex: 1;
}
</style>

