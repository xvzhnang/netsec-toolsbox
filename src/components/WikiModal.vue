<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="visible" class="wiki-modal-overlay" @click.self="handleClose">
        <div 
          ref="modalContainer"
          class="wiki-modal-container"
          :class="{ maximized: isMaximized }"
        >
          <!-- 调整大小的手柄 -->
          <div class="resize-handle resize-handle-right" @mousedown="startResize($event, 'right')"></div>
          <div class="resize-handle resize-handle-bottom" @mousedown="startResize($event, 'bottom')"></div>
          <div class="resize-handle resize-handle-bottom-right" @mousedown="startResize($event, 'bottom-right')"></div>
          <div class="resize-handle resize-handle-left" @mousedown="startResize($event, 'left')"></div>
          <div class="resize-handle resize-handle-top" @mousedown="startResize($event, 'top')"></div>
          <div class="resize-handle resize-handle-top-left" @mousedown="startResize($event, 'top-left')"></div>
          <div class="resize-handle resize-handle-top-right" @mousedown="startResize($event, 'top-right')"></div>
          <div class="resize-handle resize-handle-bottom-left" @mousedown="startResize($event, 'bottom-left')"></div>
          
          <header class="wiki-modal-header" @mousedown="startDrag">
            <h3 class="wiki-modal-title">{{ title }}</h3>
            <div class="wiki-modal-actions">
              <button
                type="button"
                class="wiki-modal-btn-icon"
                @click.stop="toggleMaximize"
                :title="isMaximized ? '还原' : '最大化'"
              >
                {{ isMaximized ? '🗗' : '🗖' }}
              </button>
              <button type="button" class="wiki-modal-btn-icon" @click.stop="handleClose" title="关闭">✕</button>
            </div>
          </header>
          <div class="wiki-modal-body">
            <div class="wiki-modal-content">
              <!-- 使用 v-if 确保组件在模态框关闭时被销毁，重新打开时重新创建 -->
              <WikiView
                v-if="visible"
                :key="`wiki-${filePath || 'home'}-${toolId || 'none'}`"
                :file-path="filePath"
                :tool-id="toolId"
                :tool-name="toolName"
                :is-modal="true"
              />
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import WikiView from '../views/WikiView.vue'

interface Props {
  visible: boolean
  filePath?: string
  toolId?: string
  toolName?: string
  title?: string
}

const props = withDefaults(defineProps<Props>(), {
  title: 'Wiki 文档',
})

const emit = defineEmits<{
  'update:visible': [value: boolean]
  close: []
}>()

// 窗口状态
const isMaximized = ref(false)
const modalContainer = ref<HTMLElement | null>(null)
const originalSize = ref({ width: 0, height: 0, left: 0, top: 0 })

// 保存原始大小和位置
const saveOriginalSize = () => {
  if (modalContainer.value) {
    const rect = modalContainer.value.getBoundingClientRect()
    originalSize.value = {
      width: rect.width,
      height: rect.height,
      left: rect.left,
      top: rect.top,
    }
  }
}

// 最大化/还原
const toggleMaximize = () => {
  if (!modalContainer.value) return
  
  if (isMaximized.value) {
    // 还原
    modalContainer.value.style.width = `${originalSize.value.width}px`
    modalContainer.value.style.height = `${originalSize.value.height}px`
    modalContainer.value.style.left = `${originalSize.value.left}px`
    modalContainer.value.style.top = `${originalSize.value.top}px`
    modalContainer.value.style.maxWidth = '1400px'
    modalContainer.value.style.maxHeight = '90vh'
    modalContainer.value.style.position = 'fixed'
    isMaximized.value = false
  } else {
    // 最大化
    saveOriginalSize()
    modalContainer.value.style.width = '100vw'
    modalContainer.value.style.height = '100vh'
    modalContainer.value.style.left = '0'
    modalContainer.value.style.top = '0'
    modalContainer.value.style.maxWidth = 'none'
    modalContainer.value.style.maxHeight = 'none'
    modalContainer.value.style.position = 'fixed'
    isMaximized.value = true
  }
}

// 拖拽移动相关
const isDragging = ref(false)
let dragStartX = 0
let dragStartY = 0
let dragStartLeft = 0
let dragStartTop = 0

const startDrag = (e: MouseEvent) => {
  if (isMaximized.value) return
  e.preventDefault()
  if (!modalContainer.value) return
  
  isDragging.value = true
  dragStartX = e.clientX
  dragStartY = e.clientY
  
  const rect = modalContainer.value.getBoundingClientRect()
  dragStartLeft = rect.left
  dragStartTop = rect.top
  
  document.addEventListener('mousemove', handleDrag)
  document.addEventListener('mouseup', stopDrag)
}

const handleDrag = (e: MouseEvent) => {
  if (!isDragging.value || !modalContainer.value || isMaximized.value) return
  
  const deltaX = e.clientX - dragStartX
  const deltaY = e.clientY - dragStartY
  
  const newLeft = dragStartLeft + deltaX
  const newTop = dragStartTop + deltaY
  
  // 限制在屏幕范围内
  const rect = modalContainer.value.getBoundingClientRect()
  const maxLeft = window.innerWidth - rect.width
  const maxTop = window.innerHeight - rect.height
  
  modalContainer.value.style.left = `${Math.max(0, Math.min(newLeft, maxLeft))}px`
  modalContainer.value.style.top = `${Math.max(0, Math.min(newTop, maxTop))}px`
}

const stopDrag = () => {
  isDragging.value = false
  document.removeEventListener('mousemove', handleDrag)
  document.removeEventListener('mouseup', stopDrag)
}

// 调整大小相关
const isResizing = ref(false)
const resizeDirection = ref('')
let startX = 0
let startY = 0
let startWidth = 0
let startHeight = 0
let startLeft = 0
let startTop = 0

const startResize = (e: MouseEvent, direction: string) => {
  if (isMaximized.value) return
  e.preventDefault()
  e.stopPropagation()
  if (!modalContainer.value) return
  
  isResizing.value = true
  resizeDirection.value = direction
  startX = e.clientX
  startY = e.clientY
  
  const rect = modalContainer.value.getBoundingClientRect()
  startWidth = rect.width
  startHeight = rect.height
  startLeft = rect.left
  startTop = rect.top
  
  document.addEventListener('mousemove', handleResize)
  document.addEventListener('mouseup', stopResize)
}

const handleResize = (e: MouseEvent) => {
  if (!isResizing.value || !modalContainer.value) return
  
  const deltaX = e.clientX - startX
  const deltaY = e.clientY - startY
  
  // 处理宽度调整
  if (resizeDirection.value.includes('right')) {
    const newWidth = startWidth + deltaX
    const maxWidth = window.innerWidth - startLeft
    if (newWidth > 400 && newWidth < maxWidth) {
      modalContainer.value.style.width = `${newWidth}px`
    }
  }
  if (resizeDirection.value.includes('left')) {
    const newWidth = startWidth - deltaX
    const newLeft = startLeft + deltaX
    if (newWidth > 400 && newLeft >= 0) {
      modalContainer.value.style.width = `${newWidth}px`
      modalContainer.value.style.left = `${newLeft}px`
    }
  }
  
  // 处理高度调整
  if (resizeDirection.value.includes('bottom')) {
    const newHeight = startHeight + deltaY
    const maxHeight = window.innerHeight - startTop
    if (newHeight > 300 && newHeight < maxHeight) {
      modalContainer.value.style.height = `${newHeight}px`
    }
  }
  if (resizeDirection.value.includes('top')) {
    const newHeight = startHeight - deltaY
    const newTop = startTop + deltaY
    if (newHeight > 300 && newTop >= 0) {
      modalContainer.value.style.height = `${newHeight}px`
      modalContainer.value.style.top = `${newTop}px`
    }
  }
}

const stopResize = () => {
  isResizing.value = false
  resizeDirection.value = ''
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
}

const handleClose = () => {
  emit('update:visible', false)
  emit('close')
}

// 初始化时设置默认大小和居中位置
onMounted(() => {
  nextTick(() => {
    if (modalContainer.value) {
      const width = Math.min(window.innerWidth * 0.95, 1400)
      const height = Math.min(window.innerHeight * 0.9, window.innerHeight * 0.9)
      const left = (window.innerWidth - width) / 2
      const top = (window.innerHeight - height) / 2
      
      modalContainer.value.style.width = `${width}px`
      modalContainer.value.style.maxWidth = '1400px'
      modalContainer.value.style.height = `${height}px`
      modalContainer.value.style.maxHeight = '90vh'
      modalContainer.value.style.left = `${left}px`
      modalContainer.value.style.top = `${top}px`
      modalContainer.value.style.position = 'fixed'
      
      // 保存初始大小
      originalSize.value = { width, height, left, top }
    }
  })
})

onUnmounted(() => {
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
  document.removeEventListener('mousemove', handleDrag)
  document.removeEventListener('mouseup', stopDrag)
})
</script>

<style scoped>
.wiki-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 2000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(4px);
}

.wiki-modal-container {
  position: fixed;
  width: 95%;
  max-width: 1400px;
  height: 90vh;
  max-height: 90vh;
  border-radius: 16px;
  border: 1px solid rgba(148, 163, 184, 0.4);
  background: linear-gradient(145deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.96));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 24px 60px rgba(0, 0, 0, 0.9);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transition: border-radius 0.2s ease-out;
}

.wiki-modal-container.maximized {
  border-radius: 0;
}

.wiki-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.3);
  background: rgba(15, 23, 42, 0.95);
  flex-shrink: 0;
  cursor: move;
  user-select: none;
}

.wiki-modal-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #e5e7eb;
}

.wiki-modal-actions {
  display: flex;
  gap: 6px;
}

.wiki-modal-btn-icon {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: #9ca3af;
  cursor: pointer;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.14s ease-out;
}

.wiki-modal-btn-icon:hover {
  background: rgba(148, 163, 184, 0.2);
  color: #e5e7eb;
}

.wiki-modal-body {
  flex: 1;
  padding: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.wiki-modal-content {
  width: 100%;
  height: 100%;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* 调整大小的手柄 */
.resize-handle {
  position: absolute;
  z-index: 10;
  background: transparent;
}

.wiki-modal-container.maximized .resize-handle {
  display: none;
}

.resize-handle-right,
.resize-handle-left {
  top: 0;
  bottom: 0;
  width: 4px;
  cursor: ew-resize;
}

.resize-handle-right {
  right: 0;
}

.resize-handle-left {
  left: 0;
}

.resize-handle-top,
.resize-handle-bottom {
  left: 0;
  right: 0;
  height: 4px;
  cursor: ns-resize;
}

.resize-handle-top {
  top: 0;
}

.resize-handle-bottom {
  bottom: 0;
}

.resize-handle-top-left,
.resize-handle-top-right,
.resize-handle-bottom-left,
.resize-handle-bottom-right {
  width: 8px;
  height: 8px;
  cursor: nwse-resize;
}

.resize-handle-top-left {
  top: 0;
  left: 0;
  cursor: nwse-resize;
}

.resize-handle-top-right {
  top: 0;
  right: 0;
  cursor: nesw-resize;
}

.resize-handle-bottom-left {
  bottom: 0;
  left: 0;
  cursor: nesw-resize;
}

.resize-handle-bottom-right {
  bottom: 0;
  right: 0;
  cursor: nwse-resize;
}

/* 调整大小时的样式 */
.wiki-modal-container:has(.resize-handle:hover) {
  user-select: none;
}

/* 过渡动画 */
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease-out;
}

.modal-enter-active .wiki-modal-container,
.modal-leave-active .wiki-modal-container {
  transition: opacity 0.2s ease-out, transform 0.2s ease-out;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .wiki-modal-container,
.modal-leave-to .wiki-modal-container {
  opacity: 0;
  transform: scale(0.95) translateY(-10px);
}
</style>

