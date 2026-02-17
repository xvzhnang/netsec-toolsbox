<script setup lang="ts">
import { computed, nextTick, watch } from 'vue'
import type { SearchItem } from '../composables/useSearch'
import { DEFAULT_TOOL_ICON, SUBCATEGORY_ICON } from '../utils/constants'

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

// Icon logic
const getIcon = (item: SearchItem) => {
  if (item.type === 'category') {
    return item.iconEmoji || '📂'
  } else if (item.type === 'subcategory') {
    return SUBCATEGORY_ICON || '📁'
  }
  return DEFAULT_TOOL_ICON || '🛠️'
}

const handleIconError = (e: Event) => {
  const img = e.target as HTMLImageElement
  if (img) {
    img.style.display = 'none'
    // Show fallback icon
    const parent = img.parentElement
    if (parent) {
      const fallback = document.createElement('span')
      fallback.textContent = getIcon(props.results[Number(parent.dataset.index)])
      fallback.className = 'fallback-icon'
      parent.appendChild(fallback)
    }
  }
}

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

</script>

<template>
  <div class="search-overlay" tabindex="0">
    <div class="overlay-title">{{ title || `搜索结果（${results.length}）` }}</div>
    <div class="overlay-list">
      <button
        v-for="(item, index) in results"
        :key="item.id"
        type="button"
        class="overlay-item"
        :class="{ 'selected': selectedIndex === index }"
        @click="$emit('select', item)"
        @dblclick="$emit('select', item)"
        @mouseenter="$emit('update:selectedIndex', index)"
        :data-index="index"
      >
        <span class="overlay-icon">
          <!-- Image Icon -->
          <img
            v-if="item.iconUrl"
            :src="item.iconUrl"
            :alt="item.name"
            class="overlay-icon-img"
            @error="handleIconError"
          />
          <!-- Emoji Icon or Fallback -->
          <span v-else>{{ getIcon(item) }}</span>
        </span>
        <span class="overlay-text">
          <span class="overlay-name">
            {{ item.name }}
            <span v-if="item.type === 'category'" class="type-badge">分类</span>
            <span v-if="item.type === 'subcategory'" class="type-badge">子分类</span>
          </span>
          <span class="overlay-desc">{{ item.description || '' }}</span>
        </span>
        <span class="overlay-action">打开</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.search-overlay {
  position: absolute;
  top: 100%; /* Position directly below the search box */
  left: 0;
  right: 0;
  margin-top: 8px; /* Slight gap */
  background: var(--bg-primary);
  /* Glassmorphism effect */
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  background-color: rgba(var(--bg-primary-rgb), 0.85); /* Need to ensure variable exists or use hardcoded rgba */
  border-radius: 8px;
  border: 1px solid var(--border-color);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 100; /* Ensure it's on top */
  max-height: 400px; /* Limit height */
  overflow-y: auto;
}

/* Fallback for bg color if RGB variable not available */
@supports not (backdrop-filter: blur(12px)) {
  .search-overlay {
    background-color: var(--bg-primary);
  }
}

.overlay-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-tertiary);
  margin-bottom: 4px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.overlay-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow-y: auto;
}

.overlay-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
  transition: all 0.2s ease;
  width: 100%;
}

.overlay-item:hover,
.overlay-item.selected {
  background: var(--bg-hover);
  border-color: var(--border-color);
}

.overlay-item.selected {
  border-color: var(--primary-color);
  background: var(--bg-active);
}

.overlay-icon {
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
  background: var(--bg-tertiary);
  border-radius: 6px;
  overflow: hidden;
}

.overlay-icon-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.overlay-text {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.overlay-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  display: flex;
  align-items: center;
  gap: 8px;
}

.type-badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  border: 1px solid var(--border-color);
}

.overlay-desc {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.overlay-action {
  font-size: 12px;
  color: var(--primary-color);
  flex-shrink: 0;
  opacity: 0;
  transform: translateX(-5px);
  transition: all 0.2s ease;
}

.overlay-item:hover .overlay-action,
.overlay-item.selected .overlay-action {
  opacity: 1;
  transform: translateX(0);
}
</style>
