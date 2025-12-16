<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'

interface Props {
  items: unknown[]
  itemHeight?: number
  containerHeight?: number
  overscan?: number
}

const props = withDefaults(defineProps<Props>(), {
  itemHeight: 200,
  containerHeight: 600,
  overscan: 3,
})

const emit = defineEmits<{
  render: [item: unknown, index: number]
}>()

const containerRef = ref<HTMLElement | null>(null)
const scrollTop = ref(0)

// 计算可见范围
const visibleRange = computed(() => {
  const start = Math.floor(scrollTop.value / props.itemHeight)
  const end = Math.ceil(
    (scrollTop.value + props.containerHeight) / props.itemHeight,
  )
  return {
    start: Math.max(0, start - props.overscan),
    end: Math.min(props.items.length, end + props.overscan),
  }
})

// 可见项目
const visibleItems = computed(() => {
  const range = visibleRange.value
  return props.items.slice(range.start, range.end).map((item, idx) => ({
    item,
    index: range.start + idx,
  }))
})

// 总高度
const totalHeight = computed(() => props.items.length * props.itemHeight)

// 偏移量
const offsetY = computed(() => visibleRange.value.start * props.itemHeight)

const handleScroll = (e: Event) => {
  const target = e.target as HTMLElement
  scrollTop.value = target.scrollTop
}

// 重置滚动位置
const resetScroll = () => {
  scrollTop.value = 0
  if (containerRef.value) {
    containerRef.value.scrollTop = 0
  }
}

onMounted(() => {
  if (containerRef.value) {
    containerRef.value.addEventListener('scroll', handleScroll, { passive: true })
  }
})

onUnmounted(() => {
  // 清理事件监听器
  if (containerRef.value) {
    containerRef.value.removeEventListener('scroll', handleScroll)
  }
  // 重置状态
  scrollTop.value = 0
})

// 监听 items 变化，重置滚动位置
watch(
  () => props.items.length,
  () => {
    resetScroll()
  },
)

// 监听路由变化，重置滚动位置
watch(
  () => props.items,
  () => {
    nextTick(() => {
      resetScroll()
    })
  },
  { deep: false },
)
</script>

<template>
  <div
    ref="containerRef"
    class="virtual-list-container"
    :style="{ height: `${containerHeight}px`, overflowY: 'auto' }"
    @wheel.stop
  >
    <div class="virtual-list-spacer" :style="{ height: `${totalHeight}px` }">
      <div
        class="virtual-list-content"
        :style="{ transform: `translateY(${offsetY}px)` }"
      >
        <div
          v-for="{ item, index } in visibleItems"
          :key="index"
          class="virtual-list-item"
          :style="{ height: `${itemHeight}px` }"
        >
          <slot :item="item" :index="index" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.virtual-list-container {
  position: relative;
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.4) rgba(15, 23, 42, 0.9);
}

.virtual-list-container::-webkit-scrollbar {
  width: 8px;
}

.virtual-list-container::-webkit-scrollbar-track {
  background: rgba(15, 23, 42, 0.9);
}

.virtual-list-container::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.4);
  border-radius: 4px;
}

.virtual-list-container::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.6);
}

.virtual-list-spacer {
  position: relative;
}

.virtual-list-content {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  will-change: transform;
}

.virtual-list-item {
  width: 100%;
  box-sizing: border-box;
}
</style>

