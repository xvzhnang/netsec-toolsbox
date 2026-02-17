<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, defineAsyncComponent, watch } from 'vue'
import { useRouter } from 'vue-router'
import AppLayout from '../layouts/AppLayout.vue'
import ContextMenu, { type MenuItem } from '../components/ContextMenu.vue'
import ModalDialog from '../components/ModalDialog.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import {
  categoriesConfig,
  categoriesData,
  syncCategoryConfigToData,
  type CategoryConfig,
  type ToolItem,
} from '../stores/categories'
import { launchTool } from '../utils/toolLauncher'
import { openMkDocs } from '../utils/mkdocsLauncher'
import { CATEGORY_ICON_MAP, DEFAULT_CATEGORY_ICON, SUBCATEGORY_ICON, DEFAULT_TOOL_ICON } from '../utils/constants'
import { useModuleStatusStore } from '../stores/moduleStatus'

const moduleStatusStore = useModuleStatusStore()

// 打开 MkDocs 文档
const handleOpenMkDocs = async () => {
  try {
    await openMkDocs()
  } catch (error) {
    // 错误已在工具类中记录
  }
}

interface SearchItem {
  id: string
  name: string
  type: 'category' | 'subcategory' | 'tool'
  categoryId: string
  subCategoryId?: string
  description?: string
  iconUrl?: string // 工具的图标 URL（仅工具类型有）
  tool?: ToolItem // 完整的工具对象（仅工具类型有）
  categoryIcon?: string // 分类的图标（仅分类类型有）
  categoryColor?: string // 分类的颜色（仅分类类型有）
}

const router = useRouter()

const categoriesRef = categoriesConfig

const AiAssistantPanel = defineAsyncComponent(() => import('../components/AiAssistantPanel.vue'))

const query = ref('')

// AI 状态持久化的 key
const AI_OPEN_STATE_KEY = 'netsec-toolbox_ai_open_state'

// 关键优化：AI功能默认关闭，实现真正的懒加载（只有用户点击时才加载组件）
// 从 localStorage 读取保存的状态
const loadAiOpenState = (): boolean => {
  try {
    const saved = localStorage.getItem(AI_OPEN_STATE_KEY)
    return saved === 'true'
  } catch (error) {
    console.warn('Failed to load AI open state:', error)
    return false
  }
}

// 保存 AI 状态到 localStorage
const saveAiOpenState = (state: boolean) => {
  try {
    localStorage.setItem(AI_OPEN_STATE_KEY, String(state))
  } catch (error) {
    console.warn('Failed to save AI open state:', error)
  }
}

const isAiOpen = ref(loadAiOpenState())

// 监听 isAiOpen 变化，自动保存状态
watch(isAiOpen, (newValue) => {
  saveAiOpenState(newValue)
}, { immediate: false })
// 搜索结果的选中索引（用于键盘导航）
const selectedSearchIndex = ref(-1)

// AI 按钮拖拽位置
const aiButtonPosition = ref({ x: window.innerWidth - 60, y: window.innerHeight - 200 })
const isDragging = ref(false)
const dragStart = ref({ x: 0, y: 0 })

// 开发者信息（硬编码，留空供用户后续填写）
const developerInfo = {
  name: '序章',
  github: 'https://github.com/xvzhnang',
  contact: '',
}

const showDeveloperModal = ref(false)

// 构建完整的搜索项列表（包括分类、子分类和工具）
const searchItems = computed<SearchItem[]>(() => {
  const items: SearchItem[] = []
  
      // 添加分类
      categoriesRef.value
        .filter((c) => c.enabled)
        .forEach((c) => {
          items.push({
            id: c.id,
            name: c.label || c.name,
            type: 'category' as const,
            categoryId: c.id,
            description: c.description,
            categoryIcon: c.icon,
            categoryColor: c.color,
          })
          
          // 查找对应的分类数据
          const categoryData = categoriesData.value.find((d) => d.id === c.id)
          if (categoryData) {
            // 添加子分类
            categoryData.subCategories.forEach((sub) => {
              items.push({
                id: `${c.id}_${sub.id}`,
                name: sub.name,
                type: 'subcategory' as const,
                categoryId: c.id,
                subCategoryId: sub.id,
                description: sub.description,
              })
              
              // 添加工具
              sub.tools.forEach((tool) => {
                items.push({
                  id: `${c.id}_${sub.id}_${tool.id}`,
                  name: tool.name,
                  type: 'tool' as const,
                  categoryId: c.id,
                  subCategoryId: sub.id,
                  description: tool.description,
                  iconUrl: tool.iconUrl,
                  tool: tool, // 保存完整的工具对象
                })
              })
            })
          }
        })
  
  return items
})

// 模糊搜索：支持名称和描述的模糊匹配
const filteredResults = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return []
  
  // 将查询字符串拆分为关键词
  const keywords = q.split(/\s+/).filter((k) => k.length > 0)
  
  return searchItems.value
    .filter((item) => {
      // 对每个关键词进行匹配
      return keywords.every((keyword) => {
        const nameMatch = item.name.toLowerCase().includes(keyword)
        const descMatch = item.description?.toLowerCase().includes(keyword) ?? false
        return nameMatch || descMatch
      })
    })
    .slice(0, 12) // 增加显示数量
})

const goToSettings = () => {
  router.push({ name: 'settings' })
}

// 获取分类图标的 emoji
const getCategoryIcon = (iconName?: string): string => {
  return CATEGORY_ICON_MAP[iconName || ''] || DEFAULT_CATEGORY_ICON
}

// 获取搜索结果的图标
const getSearchItemIcon = (item: SearchItem): string => {
  if (item.type === 'category') {
    return getCategoryIcon(item.categoryIcon)
  } else if (item.type === 'subcategory') {
    return SUBCATEGORY_ICON
  } else if (item.type === 'tool') {
    // 工具类型返回空字符串，使用 img 标签显示
    return ''
  }
  return DEFAULT_TOOL_ICON
}

const onResultClick = async (item: SearchItem) => {
  if (item.type === 'category') {
    router.push({ name: 'category', params: { id: item.categoryId } })
  } else if (item.type === 'subcategory' && item.subCategoryId) {
    router.push({ 
      name: 'category', 
      params: { id: item.categoryId },
      query: { sub: item.subCategoryId }
    })
  } else if (item.type === 'tool' && item.tool) {
    // 工具类型直接打开，不跳转
    await openTool(item.tool)
  }
  query.value = ''
  selectedSearchIndex.value = -1
}

// 处理图标加载错误
const handleIconError = (e: Event) => {
  const img = e.target as HTMLImageElement
  if (img) {
    img.style.display = 'none'
    // 显示默认图标
    const parent = img.parentElement
    if (parent) {
      const fallback = document.createElement('span')
      fallback.textContent = DEFAULT_TOOL_ICON
      parent.appendChild(fallback)
    }
  }
}

// 打开工具（使用公共工具函数）
const openTool = async (tool: ToolItem) => {
  await launchTool(tool, showConfirm)
}

// 处理搜索输入框的键盘事件
const handleSearchInputKeydown = (e: KeyboardEvent) => {
  if (!query.value || filteredResults.value.length === 0) return
  
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedSearchIndex.value = Math.min(selectedSearchIndex.value + 1, filteredResults.value.length - 1)
    scrollToSelectedItem()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedSearchIndex.value = Math.max(selectedSearchIndex.value - 1, -1)
    scrollToSelectedItem()
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (selectedSearchIndex.value >= 0 && selectedSearchIndex.value < filteredResults.value.length) {
      const item = filteredResults.value[selectedSearchIndex.value]
      if (item) {
        onResultClick(item)
      }
    } else if (filteredResults.value.length > 0) {
      // 如果没有选中项，打开第一个
      const firstItem = filteredResults.value[0]
      if (firstItem) {
        onResultClick(firstItem)
      }
    }
  } else if (e.key === 'Escape') {
    query.value = ''
    selectedSearchIndex.value = -1
  }
}

// 处理搜索覆盖层的键盘事件
const handleSearchKeydown = (e: KeyboardEvent) => {
  if (!query.value || filteredResults.value.length === 0) return
  
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedSearchIndex.value = Math.min(selectedSearchIndex.value + 1, filteredResults.value.length - 1)
    scrollToSelectedItem()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedSearchIndex.value = Math.max(selectedSearchIndex.value - 1, -1)
    scrollToSelectedItem()
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (selectedSearchIndex.value >= 0 && selectedSearchIndex.value < filteredResults.value.length) {
      const item = filteredResults.value[selectedSearchIndex.value]
      if (item) {
        onResultClick(item)
      }
    }
  } else if (e.key === 'Escape') {
    query.value = ''
    selectedSearchIndex.value = -1
  }
}

// 处理搜索输入变化
const handleSearchInput = () => {
  // 搜索内容改变时重置选中索引
  selectedSearchIndex.value = -1
}

// 滚动到选中的搜索结果项
const scrollToSelectedItem = () => {
  if (selectedSearchIndex.value < 0) return
  
  nextTick(() => {
    const overlayList = document.querySelector('.overlay-list')
    if (!overlayList) return
    
    const selectedItem = overlayList.children[selectedSearchIndex.value] as HTMLElement
    if (selectedItem) {
      selectedItem.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest'
      })
    }
  })
}

const toggleAi = () => {
  // 如果正在拖拽，不触发切换
  if (isDragging.value) return
  isAiOpen.value = !isAiOpen.value
}

// AI 按钮拖拽处理
const handleDragStart = (e: MouseEvent) => {
  isDragging.value = false
  // 记录鼠标相对于按钮的偏移量
  dragStart.value = {
    x: e.clientX - aiButtonPosition.value.x,
    y: e.clientY - aiButtonPosition.value.y,
  }
  document.addEventListener('mousemove', handleDragMove)
  document.addEventListener('mouseup', handleDragEnd)
  e.preventDefault()
  e.stopPropagation()
}

const handleDragMove = (e: MouseEvent) => {
  if (!isDragging.value) {
    // 检测是否真的在拖拽（移动超过5px）
    const deltaX = Math.abs(e.clientX - (dragStart.value.x + aiButtonPosition.value.x))
    const deltaY = Math.abs(e.clientY - (dragStart.value.y + aiButtonPosition.value.y))
    if (deltaX > 5 || deltaY > 5) {
      isDragging.value = true
    } else {
      return
    }
  }
  
  const newX = e.clientX - dragStart.value.x
  const newY = e.clientY - dragStart.value.y
  
  // 限制在窗口内
  const maxX = window.innerWidth - 40
  const maxY = window.innerHeight - 40
  aiButtonPosition.value = {
    x: Math.max(0, Math.min(newX, maxX)),
    y: Math.max(0, Math.min(newY, maxY)),
  }
}

const handleDragEnd = () => {
  document.removeEventListener('mousemove', handleDragMove)
  document.removeEventListener('mouseup', handleDragEnd)
  // 延迟重置拖拽状态，避免触发点击事件
  setTimeout(() => {
    isDragging.value = false
  }, 100)
}

// 窗口大小变化时，确保按钮在可视区域内
const handleResize = () => {
  const buttonSize = 40
  const maxX = Math.max(0, window.innerWidth - buttonSize)
  const maxY = Math.max(0, window.innerHeight - buttonSize)
  aiButtonPosition.value = {
    x: Math.min(aiButtonPosition.value.x, maxX),
    y: Math.min(aiButtonPosition.value.y, maxY),
  }
}

onMounted(() => {
  window.addEventListener('resize', handleResize)
  // 初始化位置
  handleResize()
  
  // 注意：AI 面板状态已经在组件初始化时从 localStorage 恢复
  // 由于组件是懒加载的，每次路由切换都会重新创建，状态会自动恢复
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  document.removeEventListener('mousemove', handleDragMove)
  document.removeEventListener('mouseup', handleDragEnd)
})

const contextMenuRef = ref<InstanceType<typeof ContextMenu> | null>(null)
const contextMenuVisible = ref(false)
const contextMenuTarget = ref<CategoryConfig | null>(null)

// 确认对话框
const confirmDialogVisible = ref(false)
const confirmDialogTitle = ref('')
const confirmDialogMessage = ref('')
const confirmDialogType = ref<'danger' | 'warning' | 'info'>('info')
const confirmDialogCallback = ref<(() => void) | null>(null)

const showConfirm = (
  title: string,
  message: string,
  callback: () => void,
  type: 'danger' | 'warning' | 'info' = 'info',
) => {
  confirmDialogTitle.value = title
  confirmDialogMessage.value = message
  confirmDialogType.value = type
  confirmDialogCallback.value = callback
  confirmDialogVisible.value = true
}

const onConfirm = () => {
  if (confirmDialogCallback.value) {
    confirmDialogCallback.value()
    confirmDialogCallback.value = null
  }
}

const showCategoryMenu = (e: MouseEvent, category: CategoryConfig) => {
  e.preventDefault()
  e.stopPropagation()
  contextMenuTarget.value = category
  if (contextMenuRef.value) {
    contextMenuRef.value.show(e.clientX, e.clientY)
  }
  contextMenuVisible.value = true
}

const categoryMenuItems = computed<MenuItem[]>(() => {
  if (!contextMenuTarget.value) return []
  return [
    {
      label: '编辑分类',
      icon: '✏️',
      action: () => {
        editCategory(contextMenuTarget.value!)
      },
    },
    {
      label: '删除分类',
      icon: '🗑️',
      action: () => {
        const target = contextMenuTarget.value
        if (!target) return
        const categoryName = target.label || target.name
        const categoryId = target.id
        showConfirm(
          '确认删除分类',
          `确定删除分类「${categoryName}」？`,
          () => {
            const idx = categoriesRef.value.findIndex((c) => c.id === categoryId)
            if (idx !== -1) {
              categoriesRef.value.splice(idx, 1)
              // 触发响应式更新
              categoriesRef.value = [...categoriesRef.value]
            }
          },
          'danger',
        )
      },
      danger: true,
    },
  ]
})

const closeContextMenu = () => {
  contextMenuVisible.value = false
  contextMenuTarget.value = null
  if (contextMenuRef.value) {
    // 重置菜单位置到屏幕外
    contextMenuRef.value.show(-9999, -9999)
  }
}

const showBlankMenu = (e: MouseEvent) => {
  // 只在空白区域显示
  const target = e.target as HTMLElement
  if (target.closest('.category-card') || target.closest('.search-box') || target.closest('.app-header')) {
    return
  }
  e.preventDefault()
  contextMenuTarget.value = null
  if (contextMenuRef.value) {
    contextMenuRef.value.show(e.clientX, e.clientY)
  }
  contextMenuVisible.value = true
}

const blankMenuItems = computed<MenuItem[]>(() => {
  if (contextMenuTarget.value) return []
  return [
    {
      label: '添加分类',
      icon: '➕',
      action: () => {
        startNewCategory()
      },
    },
  ]
})

const finalMenuItems = computed(() => {
  if (contextMenuTarget.value) return categoryMenuItems.value
  return blankMenuItems.value
})

// 分类配置弹窗
const showCategoryModal = ref(false)
const isNewCategory = ref(false)
const categoryForm = ref<{
  id: string
  name: string
  label: string
  description: string
  icon: string
  color: string
}>({
  id: '',
  name: '',
  label: '',
  description: '',
  icon: 'apps',
  color: '#4DA3FF',
})

const startNewCategory = () => {
  categoryForm.value = {
    id: `category_${Date.now()}`,
    name: 'NEW',
    label: '新分类',
    description: '请编辑此分类信息。',
    icon: 'apps',
    color: '#4DA3FF',
  }
  isNewCategory.value = true
  showCategoryModal.value = true
}

const editCategory = (category: CategoryConfig) => {
  categoryForm.value = {
    id: category.id,
    name: category.name,
    label: category.label || '',
    description: category.description || '',
    icon: category.icon,
    color: category.color,
  }
  isNewCategory.value = false
  showCategoryModal.value = true
}

const saveCategory = () => {
  if (!categoryForm.value.name.trim()) {
    showConfirm('提示', '请输入分类名称', () => {}, 'warning')
    return
  }
  if (!categoryForm.value.id) {
    showConfirm('提示', '分类ID不能为空', () => {}, 'warning')
    return
  }
  const idx = categoriesRef.value.findIndex((c) => c.id === categoryForm.value.id)
  if (idx >= 0) {
    const existing = categoriesRef.value[idx]
    if (existing) {
      categoriesRef.value[idx] = {
        id: existing.id,
        name: categoryForm.value.name.trim(),
        label: categoryForm.value.label.trim() || undefined,
        description: categoryForm.value.description.trim() || undefined,
        icon: categoryForm.value.icon,
        color: categoryForm.value.color,
        order: existing.order,
        enabled: existing.enabled,
      }
    }
  } else {
    const nextOrder = categoriesRef.value.reduce((max, c) => Math.max(max, c.order), 0) + 1
    categoriesRef.value.push({
      id: categoryForm.value.id,
      name: categoryForm.value.name.trim(),
      label: categoryForm.value.label.trim() || undefined,
      description: categoryForm.value.description.trim() || undefined,
      icon: categoryForm.value.icon,
      color: categoryForm.value.color,
      order: nextOrder,
      enabled: true,
    })
  }
  // 触发响应式更新
  categoriesRef.value = [...categoriesRef.value]
  // 同步配置到分类数据，确保CategoryView能访问到新分类
  syncCategoryConfigToData(categoryForm.value.id)
  showCategoryModal.value = false
}
</script>

<template>
  <AppLayout title="NetSec Toolbox" subtitle="网络攻防工具箱 · 桌面版">
    <template #header-actions>
      <button type="button" class="icon-button" @click="goToSettings">
        <span class="icon">⚙</span>
        <span class="icon-label">设置</span>
      </button>
      <button 
        type="button" 
        class="icon-button" 
        @click="handleOpenMkDocs"
        :class="{ 'opacity-50 cursor-wait': !moduleStatusStore.wikiReady }"
        :title="moduleStatusStore.wikiReady ? '打开文档' : 'Wiki 服务正在启动...'"
      >
        <span class="icon">
          <span v-if="!moduleStatusStore.wikiReady" class="animate-spin">⏳</span>
          <span v-else>📚</span>
        </span>
        <span class="icon-label">Wiki / 文档</span>
      </button>
      <button
        type="button"
        class="icon-button"
        @click="showDeveloperModal = true"
      >
        <span class="icon">👤</span>
        <span class="icon-label">开发者信息</span>
      </button>
    </template>

    <div class="dashboard-content">
      <div class="search-row">
        <div class="search-box">
          <span class="search-icon">🔍</span>
          <input
            v-model="query"
            class="search-input"
            type="search"
            placeholder="搜索分类 / 二级分类 / 工具名称（↑↓ 选择，Enter 打开，Esc 清除）..."
            @keydown="handleSearchInputKeydown"
            @input="handleSearchInput"
          />
        </div>
      </div>

      <div v-if="isAiOpen" class="content-row ai-open" @contextmenu="showBlankMenu" :class="{ 'search-overlay-active': filteredResults.length }">
        <div 
          v-if="filteredResults.length" 
          class="search-overlay"
          @keydown="handleSearchKeydown"
          tabindex="0"
        >
          <div class="overlay-title">搜索结果（{{ filteredResults.length }}）</div>
          <div class="overlay-list">
            <button
              v-for="(item, index) in filteredResults"
              :key="item.id"
              type="button"
              class="overlay-item"
              :class="{ 'selected': selectedSearchIndex === index }"
              @click="onResultClick(item)"
              @dblclick="onResultClick(item)"
              @mouseenter="selectedSearchIndex = index"
            >
              <span class="overlay-icon">
                <img
                  v-if="item.type === 'tool' && item.iconUrl"
                  :src="item.iconUrl"
                  :alt="item.name"
                  class="overlay-icon-img"
                  @error="handleIconError"
                />
                <span v-else>{{ getSearchItemIcon(item) }}</span>
              </span>
              <span class="overlay-text">
                <span class="overlay-name">{{ item.name }}</span>
                <span class="overlay-desc">{{ item.description || '' }}</span>
              </span>
              <span class="overlay-action">打开</span>
            </button>
          </div>
        </div>
        <div class="cards-column">
          <div class="card-grid">
            <button
              v-for="category in categoriesRef.filter((c) => c.enabled)"
              :key="category.id"
              type="button"
              class="category-card"
              :data-category-id="category.id"
              :style="{ '--card-color': category.color }"
              @click="router.push({ name: 'category', params: { id: category.id } })"
              @contextmenu="showCategoryMenu($event, category)"
            >
              <div class="card-icon">
                <span class="icon-fallback">{{ category.name.charAt(0) }}</span>
              </div>
              <div class="card-content">
                <div class="card-title-row">
                  <h2 class="card-title">{{ category.name }}</h2>
                  <span v-if="category.label" class="card-label">{{ category.label }}</span>
                </div>
                <p class="card-description">
                  {{ category.description }}
                </p>
              </div>
            </button>
            <button
              type="button"
              class="category-card add-card"
              @click="startNewCategory"
            >
              <div class="card-icon add-icon">
                <span class="icon-fallback">+</span>
              </div>
              <div class="card-content">
                <div class="card-title-row">
                  <h2 class="card-title">添加分类</h2>
                </div>
                <p class="card-description">点击添加新的分类</p>
              </div>
            </button>
          </div>
        </div>

        <aside class="ai-column">
          <AiAssistantPanel />
        </aside>
      </div>

      <div v-else class="cards-row" @contextmenu="showBlankMenu" :class="{ 'search-overlay-active': filteredResults.length }">
        <div 
          v-if="filteredResults.length" 
          class="search-overlay"
          @keydown="handleSearchKeydown"
          tabindex="0"
        >
          <div class="overlay-title">搜索结果（{{ filteredResults.length }}）</div>
          <div class="overlay-list">
            <button
              v-for="(item, index) in filteredResults"
              :key="item.id"
              type="button"
              class="overlay-item"
              :class="{ 'selected': selectedSearchIndex === index }"
              @click="onResultClick(item)"
              @dblclick="onResultClick(item)"
              @mouseenter="selectedSearchIndex = index"
            >
              <span class="overlay-icon">
                <img
                  v-if="item.type === 'tool' && item.iconUrl"
                  :src="item.iconUrl"
                  :alt="item.name"
                  class="overlay-icon-img"
                  @error="handleIconError"
                />
                <span v-else>{{ getSearchItemIcon(item) }}</span>
              </span>
              <span class="overlay-text">
                <span class="overlay-name">{{ item.name }}</span>
                <span class="overlay-desc">{{ item.description || '' }}</span>
              </span>
              <span class="overlay-action">打开</span>
            </button>
          </div>
        </div>
        <div class="cards-row-inner">
          <div class="card-grid">
          <button
            v-for="category in categoriesRef.filter((c) => c.enabled)"
            :key="category.id"
            type="button"
            class="category-card"
            :data-category-id="category.id"
            :style="{ '--card-color': category.color }"
            @click="router.push({ name: 'category', params: { id: category.id } })"
            @contextmenu="showCategoryMenu($event, category)"
          >
            <div class="card-icon">
              <span class="icon-fallback">{{ category.name.charAt(0) }}</span>
            </div>
            <div class="card-content">
              <div class="card-title-row">
                <h2 class="card-title">{{ category.name }}</h2>
                <span v-if="category.label" class="card-label">{{ category.label }}</span>
              </div>
              <p class="card-description">
                {{ category.description }}
              </p>
            </div>
          </button>
          <button
            type="button"
            class="category-card add-card"
            @click="startNewCategory"
          >
            <div class="card-icon add-icon">
              <span class="icon-fallback">+</span>
            </div>
            <div class="card-content">
              <div class="card-title-row">
                <h2 class="card-title">添加分类</h2>
              </div>
              <p class="card-description">点击添加新的分类</p>
            </div>
          </button>
          </div>
        </div>
      </div>

      <button
        type="button"
        class="ai-toggle"
        :class="{ dragging: isDragging, 'opacity-50': !moduleStatusStore.aiReady }"
        :style="{
          left: `${aiButtonPosition.x}px`,
          top: `${aiButtonPosition.y}px`,
          right: 'auto',
          bottom: 'auto',
        }"
        @mousedown="handleDragStart"
        @click="toggleAi"
        :title="moduleStatusStore.aiReady ? '打开 / 收起 AI 助手（可拖拽）' : 'AI 服务正在后台初始化...'"
      >
        <span v-if="!moduleStatusStore.aiReady" class="animate-spin text-sm">⏳</span>
        <span v-else>🤖</span>
      </button>
    </div>

    <ContextMenu
      v-if="contextMenuVisible"
      ref="contextMenuRef"
      :items="finalMenuItems"
      @close="closeContextMenu"
    />

    <ModalDialog
      v-model:visible="showCategoryModal"
      :title="isNewCategory ? '新增分类' : '编辑分类'"
      :collapsible="true"
    >
      <div class="modal-form">
        <div class="form-section">
          <h3 class="section-title">基础信息</h3>
          <div class="form-grid">
            <label class="field">
              <span class="field-label">名称（代号）</span>
              <input v-model="categoryForm.name" class="field-input" placeholder="例如: WEB" />
            </label>
            <label class="field">
              <span class="field-label">显示名称</span>
              <input v-model="categoryForm.label" class="field-input" placeholder="例如: Web 攻击与防御" />
            </label>
          </div>
          <label class="field">
            <span class="field-label">简要说明</span>
            <textarea
              v-model="categoryForm.description"
              class="field-textarea"
              rows="3"
              placeholder="分类的简要说明"
            />
          </label>
        </div>

        <div class="form-section">
          <h3 class="section-title">视觉风格</h3>
          <div class="form-grid">
            <label class="field">
              <span class="field-label">图标</span>
              <div class="icon-row">
                <span class="icon-preview">
                  <span v-if="categoryForm.icon === 'globe'">🌐</span>
                  <span v-else-if="categoryForm.icon === 'apps'">🔧</span>
                  <span v-else-if="categoryForm.icon === 'bug'">🐞</span>
                  <span v-else-if="categoryForm.icon === 'lock'">🔒</span>
                  <span v-else-if="categoryForm.icon === 'search'">🔍</span>
                  <span v-else-if="categoryForm.icon === 'fingerprint'">🆔</span>
                  <span v-else-if="categoryForm.icon === 'link'">🔗</span>
                  <span v-else-if="categoryForm.icon === 'command'">⌘</span>
                  <span v-else>★</span>
                </span>
                <select v-model="categoryForm.icon" class="field-input">
                  <option value="globe">🌐 globe</option>
                  <option value="apps">🔧 apps</option>
                  <option value="bug">🐞 bug</option>
                  <option value="lock">🔒 lock</option>
                  <option value="search">🔍 search</option>
                  <option value="fingerprint">🆔 fingerprint</option>
                  <option value="link">🔗 link</option>
                  <option value="command">⌘ command</option>
                </select>
              </div>
            </label>
            <label class="field">
              <span class="field-label">颜色</span>
              <div class="color-row">
                <input v-model="categoryForm.color" type="color" class="color-picker" />
                <input v-model="categoryForm.color" class="field-input" placeholder="#4DA3FF" />
              </div>
            </label>
          </div>
        </div>

        <div class="modal-form-actions">
          <button type="button" class="btn ghost" @click="showCategoryModal = false">取消</button>
          <button type="button" class="btn primary" @click="saveCategory">保存</button>
        </div>
      </div>
    </ModalDialog>

    <ConfirmDialog
      v-model:visible="confirmDialogVisible"
      :title="confirmDialogTitle"
      :message="confirmDialogMessage"
      :type="confirmDialogType"
      confirm-text="确认"
      @confirm="onConfirm"
    />

    <ModalDialog
      v-model:visible="showDeveloperModal"
      title="开发者信息"
      :collapsible="true"
    >
      <div class="modal-form">
        <div v-if="!developerInfo.name && !developerInfo.github && !developerInfo.contact" class="developer-empty">
          <p>开发者信息暂未配置</p>
        </div>
        <div v-else class="developer-display">
          <div v-if="developerInfo.name" class="info-item">
            <span class="info-label">开发者：</span>
            <span class="info-value">{{ developerInfo.name }}</span>
          </div>
          <div v-if="developerInfo.github" class="info-item">
            <span class="info-label">GitHub：</span>
            <a
              :href="developerInfo.github"
              target="_blank"
              rel="noopener noreferrer"
              class="info-link"
            >
              {{ developerInfo.github }}
            </a>
          </div>
          <div v-if="developerInfo.contact" class="info-item">
            <span class="info-label">联系方式：</span>
            <a
              v-if="developerInfo.contact.startsWith('http') || developerInfo.contact.includes('@')"
              :href="developerInfo.contact.startsWith('http') ? developerInfo.contact : `mailto:${developerInfo.contact}`"
              target="_blank"
              rel="noopener noreferrer"
              class="info-link"
            >
              {{ developerInfo.contact }}
            </a>
            <span v-else class="info-value">{{ developerInfo.contact }}</span>
          </div>
        </div>
        <div class="modal-form-actions">
          <button type="button" class="btn ghost" @click="showDeveloperModal = false">关闭</button>
        </div>
      </div>
    </ModalDialog>
  </AppLayout>
</template>

<style scoped>
.dashboard-content {
  display: flex;
  flex-direction: column;
  gap: 20px;
  min-height: 0;
  height: 100%;
}

.icon-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-primary);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 13px;
  transition: all 0.1s ease-out;
}

.icon-button:hover {
  background-color: var(--bg-tertiary);
  border-color: var(--text-primary);
  color: var(--text-primary);
}

.icon {
  font-size: 14px;
}

.search-row {
  width: 100%;
  max-width: 1200px;
  margin: 0 auto;
}

.search-box {
  position: relative;
  width: 100%;
}

.search-icon {
  position: absolute !important;
  left: 12px !important;
  top: 50% !important;
  transform: translateY(-50%) !important;
  font-size: 16px;
  color: var(--text-muted);
  pointer-events: none;
  z-index: 1;
}

.search-box:focus-within .search-icon {
  color: var(--accent-primary);
}

.search-input {
  width: 100%;
  padding: 8px 12px 8px 36px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 14px;
  outline: none;
  transition: border-color 0.15s ease;
}

.search-input::placeholder {
  color: var(--text-muted);
}

.search-input:focus {
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 2px rgba(31, 111, 235, 0.2);
}

/* 搜索结果覆盖层样式 */
.content-row.search-overlay-active,
.cards-row.search-overlay-active {
  position: relative;
}

.search-overlay {
  position: absolute;
  inset: 0;
  background: var(--bg-secondary);
  border-radius: 6px;
  border: 1px solid var(--border-color);
  box-shadow: var(--shadow-lg);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 20;
  overflow-y: auto;
}

.overlay-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-muted);
  margin-bottom: 4px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.overlay-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.overlay-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
  transition: background 0.1s ease;
  width: 100%;
}

.overlay-item:hover,
.overlay-item.selected {
  background: var(--bg-tertiary);
}

.overlay-item.selected {
  border-color: var(--accent-primary);
}

.overlay-icon {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  flex-shrink: 0;
}

.overlay-icon-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  border-radius: 4px;
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
}

.overlay-desc {
  font-size: 12px;
  color: var(--text-muted);
}

.overlay-action {
  font-size: 12px;
  color: var(--accent-primary);
  flex-shrink: 0;
}

.card-grid {
  width: 100%;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 16px;
  padding: 4px 0;
}

.content-row {
  flex: 1;
  max-width: 1200px;
  margin: 0 auto;
  width: 100%;
  display: grid;
  gap: 20px;
  align-items: stretch;
  min-height: 0;
  overflow: visible;
  position: relative;
}

.content-row.ai-open {
  grid-template-columns: minmax(0, 1.5fr) minmax(350px, 1.2fr);
  gap: 20px;
}

.content-row:not(.ai-open) {
  grid-template-columns: minmax(0, 1fr);
}

.cards-column {
  min-width: 0;
  min-height: 0;
  padding: 4px;
  overflow-y: auto;
  overflow-x: hidden;
}

.ai-column {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 4px 0;
}

.cards-row {
  flex: 1;
  max-width: 1200px;
  margin: 0 auto;
  width: 100%;
  padding: 4px;
  position: relative;
}

.cards-row-inner {
  position: relative;
  width: 100%;
  height: 100%;
}

.ai-toggle {
  position: fixed;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  border: 1px solid var(--border-color);
  background: var(--bg-secondary);
  color: var(--text-primary);
  cursor: pointer;
  box-shadow: var(--shadow-lg);
  font-size: 20px;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.1s ease, background 0.1s ease;
}

.ai-toggle:hover:not(.dragging) {
  background: var(--bg-tertiary);
  transform: scale(1.05);
}

.ai-toggle.dragging {
  cursor: grabbing;
  opacity: 0.8;
}

.category-card {
  position: relative;
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  padding: 16px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background: var(--bg-secondary);
  cursor: pointer;
  text-align: left;
  color: inherit;
  transition: all 0.1s ease-out;
}

.category-card:hover {
  border-color: var(--accent-primary);
  background: var(--bg-tertiary);
}

/* 使用左边框作为颜色标识 */
.category-card::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background-color: var(--card-color);
  border-top-left-radius: 6px;
  border-bottom-left-radius: 6px;
  opacity: 0.8;
}

.category-card.add-card {
  border-style: dashed;
  background: transparent;
}

.category-card.add-card:hover {
  border-color: var(--accent-primary);
  background: var(--bg-secondary);
}

.add-icon {
  background: transparent;
  color: var(--text-secondary);
  font-size: 24px;
  font-weight: 300;
  border: 1px dashed var(--border-color);
}

.card-icon {
  flex: 0 0 auto;
  width: 40px;
  height: 40px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 12px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
}

.icon-fallback {
  font-weight: 600;
  font-size: 18px;
  color: var(--text-primary);
}

.card-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.card-title-row {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.card-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.card-label {
  font-size: 12px;
  color: var(--text-muted);
}

.card-description {
  margin: 0;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.4;
}

@media (max-width: 768px) {
  .page-header {
    padding: 12px 16px;
  }

  .page-main {
    padding: 16px;
  }

  .content-row {
    grid-template-columns: minmax(0, 1fr);
  }
}

.modal-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.form-section {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 0;
  border-radius: 0;
  background: transparent;
  border: none;
}

.section-title {
  margin: 0 0 8px 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.modal-form-actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  margin-top: 8px;
  padding-top: 16px;
  border-top: 1px solid var(--border-color);
}

.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 13px;
  color: var(--text-secondary);
  font-weight: 500;
}

.field-input,
.field-textarea {
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 14px;
  transition: border-color 0.15s ease;
}

.field-input:focus,
.field-textarea:focus {
  outline: none;
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 2px rgba(31, 111, 235, 0.2);
}

.field-textarea {
  resize: vertical;
  font-family: inherit;
}

.icon-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.icon-preview {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}

.color-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.color-picker {
  width: 40px;
  height: 32px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background: var(--bg-secondary);
  cursor: pointer;
  flex-shrink: 0;
  padding: 2px;
}
</style>


