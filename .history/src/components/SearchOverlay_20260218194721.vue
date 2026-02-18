<script setup lang="ts">
import { nextTick, watch, onMounted, onUnmounted } from 'vue'
import type { SearchItem } from '../composables/useSearch'

const props = defineProps<{
  results: SearchItem[]
  selectedIndex: number
  title?: string
}>()

const emit = defineEmits<{
  (e: 'select', item: SearchItem): void
  (e: 'update:selectedIndex', index: number): void
  (e: 'close'): void
}>()

const handleIconError = (e: Event, item: SearchItem) => {
  const img = e.target as HTMLImageElement
  if (img) {
    img.style.display = 'none'
    // Show fallback icon
    const parent = img.parentElement
    if (parent) {
      // Create a span for the fallback if it doesn't exist
      let fallback = parent.querySelector('.fallback-icon')
      if (!fallback) {
        fallback = document.createElement('span')
        fallback.className = 'fallback-icon'
        parent.appendChild(fallback)
      }
      fallback.textContent = item.iconEmoji || '🛠️'
    }
  }
}

// Scroll to selected item
const scrollToSelectedItem = () => {
  if (props.selectedIndex < 0) return
  
  nextTick(() => {
    const overlayList = document.querySelector('.overlay-list')
    if (!overlayList) return
    
    const selectedItem = overlayList.children[props.selectedIndex] as HTMLElement
    if (selectedItem) {
      selectedItem.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest'
      })
    }
  })
}

watch(() => props.selectedIndex, scrollToSelectedItem)

// Close on click outside
const handleClickOutside = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  if (!target.closest('.search-overlay') && !target.closest('.search-box')) {
    emit('close')
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})

</script>

<template>
  <div class="search-overlay" tabindex="-1">
    <div class="overlay-header">
      <span class="overlay-title">{{ title || `搜索结果（${results.length}）` }}</span>
      <span class="overlay-hint">↑↓ 选择，Enter 打开，Esc 关闭</span>
    </div>
    <div class="overlay-list">
      <button
        v-for="(item, index) in results"
        :key="item.id"
        type="button"
        class="overlay-item"
        :class="{ 'selected': selectedIndex === index }"
        @click.stop="$emit('select', item)"
        @mouseenter="$emit('update:selectedIndex', index)"
      >
        <div class="overlay-icon">
          <!-- Image Icon -->
          <img
            v-if="item.iconUrl"
            :src="item.iconUrl"
            :alt="item.name"
            class="overlay-icon-img"
            @error="(e) => handleIconError(e, item)"
          />
          <!-- Emoji Icon or Fallback (only shown if no iconUrl or img error) -->
          <span v-else class="fallback-icon">{{ item.iconEmoji || '🛠️' }}</span>
        </div>
        
        <div class="overlay-content">
          <div class="overlay-row-top">
            <span class="overlay-name">{{ item.name }}</span>
            <span v-if="item.type === 'category'" class="type-badge category">分类</span>
            <span v-else-if="item.type === 'subcategory'" class="type-badge subcategory">子分类</span>
            <!-- No badge for tools to keep it clean, or maybe '工具' -->
          </div>
          <div class="overlay-desc">{{ item.description || '暂无描述' }}</div>
        </div>
        
        <div class="overlay-action">
          <span class="action-text">打开</span>
          <span class="action-key">↵</span>
        </div>
      </button>
    </div>
  </div>
</template>

<style scoped>
.search-overlay {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  margin-top: 8px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  z-index: 100;
  max-height: 450px;
  overflow: hidden;
  backdrop-filter: blur(10px);
}

.overlay-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  background: rgba(var(--bg-tertiary-rgb), 0.5);
}

.overlay-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
}

.overlay-hint {
  font-size: 11px;
  color: var(--text-tertiary);
}

.overlay-list {
  overflow-y: auto;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.overlay-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  cursor: pointer;
  text-align: left;
  transition: all 0.2s ease;
  width: 100%;
  position: relative;
}

.overlay-item:hover,
.overlay-item.selected {
  background: var(--bg-hover);
  border-color: var(--border-color);
}

.overlay-item.selected {
  background: var(--bg-active);
  border-color: var(--primary-color);
}

.overlay-icon {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  border-radius: 8px;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  overflow: hidden;
  border: 1px solid var(--border-color);
}

.overlay-icon-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.fallback-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
}

.overlay-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.overlay-row-top {
  display: flex;
  align-items: center;
  gap: 8px;
}

.overlay-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.type-badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  font-weight: 500;
  text-transform: uppercase;
}

.type-badge.category {
  background: rgba(77, 163, 255, 0.1);
  color: #4DA3FF;
  border: 1px solid rgba(77, 163, 255, 0.2);
}

.type-badge.subcategory {
  background: rgba(167, 139, 250, 0.1);
  color: #A78BFA;
  border: 1px solid rgba(167, 139, 250, 0.2);
}

.overlay-desc {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.overlay-action {
  display: flex;
  align-items: center;
  gap: 6px;
  opacity: 0;
  transform: translateX(10px);
  transition: all 0.2s ease;
}

.overlay-item:hover .overlay-action,
.overlay-item.selected .overlay-action {
  opacity: 1;
  transform: translateX(0);
}

.action-text {
  font-size: 12px;
  color: var(--primary-color);
  font-weight: 500;
}

.action-key {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  border: 1px solid var(--border-color);
}
</style>
