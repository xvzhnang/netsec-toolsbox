<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import AiAssistantPanel from '../components/AiAssistantPanel.vue'
import ContextMenu, { type MenuItem } from '../components/ContextMenu.vue'
import ModalDialog from '../components/ModalDialog.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import {
  categoriesConfig,
  categoriesData,
  syncCategoryConfigToData,
  type CategoryConfig,
} from '../stores/categories'

interface SearchItem {
  id: string
  name: string
  type: 'category' | 'subcategory' | 'tool'
  categoryId: string
  subCategoryId?: string
  description?: string
}

const router = useRouter()

const categoriesRef = categoriesConfig

const query = ref('')
const isAiOpen = ref(true)

// AI 按钮拖拽位置
const aiButtonPosition = ref({ x: window.innerWidth - 80, y: window.innerHeight - 180 })
const isDragging = ref(false)
const dragStart = ref({ x: 0, y: 0 })

// 开发者信息（硬编码，留空供用户后续填写）
const developerInfo = {
  name: '',
  github: '',
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

const onResultClick = (item: SearchItem) => {
  if (item.type === 'category') {
    router.push({ name: 'category', params: { id: item.categoryId } })
  } else if (item.type === 'subcategory' && item.subCategoryId) {
    router.push({ 
      name: 'category', 
      params: { id: item.categoryId },
      query: { sub: item.subCategoryId }
    })
  } else if (item.type === 'tool' && item.subCategoryId) {
    router.push({ 
      name: 'category', 
      params: { id: item.categoryId },
      query: { sub: item.subCategoryId, tool: item.id.split('_').pop() }
    })
  }
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
  if (target.closest('.category-card') || target.closest('.search-box') || target.closest('.page-header')) {
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
  // 同步配置到分类数据，确保CategoryView能访问到新分类
  syncCategoryConfigToData(categoryForm.value.id)
  showCategoryModal.value = false
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div class="title-block">
        <h1 class="title">NetSec Toolbox</h1>
        <p class="subtitle">网络攻防工具箱 · 桌面版</p>
      </div>
      <div class="header-actions">
        <button type="button" class="icon-button" @click="goToSettings">
          <span class="icon">⚙</span>
          <span class="icon-label">设置</span>
        </button>
        <button
          type="button"
          class="icon-button"
          @click="showDeveloperModal = true"
        >
          <span class="icon">👤</span>
          <span class="icon-label">开发者信息</span>
        </button>
      </div>
    </header>

    <main class="page-main">
      <div class="search-row">
        <div class="search-box">
          <span class="search-icon">🔍</span>
          <input
            v-model="query"
            class="search-input"
            type="search"
            placeholder="搜索分类 / 二级分类 / 工具名称（规划中）..."
          />
          <div v-if="filteredResults.length" class="search-results">
            <button
              v-for="item in filteredResults"
              :key="item.id"
              type="button"
              class="search-result-item"
              @click="onResultClick(item)"
            >
              <span class="result-name">{{ item.name }}</span>
              <span class="result-type">
                {{ item.type === 'category' ? '分类' : item.type === 'subcategory' ? '子分类' : '工具' }}
              </span>
            </button>
          </div>
        </div>
      </div>

      <div v-if="isAiOpen" class="content-row ai-open" @contextmenu="showBlankMenu">
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

      <div v-else class="cards-row" @contextmenu="showBlankMenu">
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

      <button
        type="button"
        class="ai-toggle"
        :class="{ dragging: isDragging }"
        :style="{
          left: `${aiButtonPosition.x}px`,
          top: `${aiButtonPosition.y}px`,
          right: 'auto',
          bottom: 'auto',
        }"
        @mousedown="handleDragStart"
        @click="toggleAi"
        title="打开 / 收起 AI 助手（可拖拽）"
      >
        🤖
      </button>
    </main>

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

    <footer class="page-footer">
      <div class="footer-content">
        <span class="copyright">© 2025 By 序章</span>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.page {
  height: 100vh; /* 固定高度为视口高度 */
  display: flex;
  flex-direction: column;
  background: radial-gradient(circle at top, #020617 0, #020617 40%, #000000 100%);
  color: #e5e7eb;
  overflow: hidden; /* 固定整体页面 */
}

.page-header {
  flex: 0 0 auto; /* 固定头部，不伸缩 */
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 32px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.2);
  backdrop-filter: blur(14px);
  background: linear-gradient(to bottom, rgba(15, 23, 42, 0.9), rgba(15, 23, 42, 0.7));
  z-index: 10; /* 确保头部在最上层 */
}


.title-block {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.subtitle {
  margin: 0;
  font-size: 13px;
  color: #9ca3af;
}

.icon-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background: radial-gradient(circle at top left, rgba(148, 163, 184, 0.12), rgba(15, 23, 42, 0.95));
  color: #e5e7eb;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.18s ease-out;
}

.icon-button:hover {
  border-color: #e5e7eb;
  box-shadow: 0 0 0 1px rgba(148, 163, 184, 0.5), 0 12px 24px rgba(15, 23, 42, 0.9);
  transform: translateY(-1px);
}

.icon {
  font-size: 14px;
}

.icon-label {
  white-space: nowrap;
}

.page-main {
  flex: 1;
  padding: 24px 32px 32px; /* 保持原有 padding，确保内容不贴边 */
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 0;
  overflow-y: auto; /* 主内容区域可以滚动 */
  overflow-x: hidden;
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
  position: absolute;
  left: 12px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 16px;
  color: #94a3b8; /* 更亮的颜色，提高可见性 */
  pointer-events: none;
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.3)); /* 添加阴影增强可见性 */
  transition: color 0.16s ease-out;
  z-index: 1;
}

.search-box:focus-within .search-icon {
  color: #4da3ff; /* 聚焦时变为蓝色，更明显 */
}

.search-input {
  width: 100%;
  padding: 8px 12px 8px 36px; /* 增加左侧 padding，为更大的图标留出空间 */
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background: rgba(15, 23, 42, 0.95);
  color: #e5e7eb;
  font-size: 13px;
  outline: none;
  transition: border-color 0.16s ease-out, box-shadow 0.16s ease-out, background 0.16s ease-out;
}

.search-input::placeholder {
  color: #6b7280;
}

.search-input:focus {
  border-color: #4da3ff;
  box-shadow: 0 0 0 1px rgba(77, 163, 255, 0.5);
  background: rgba(15, 23, 42, 0.98);
}

.search-results {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  right: 0;
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.4);
  background: rgba(15, 23, 42, 0.98);
  box-shadow:
    0 12px 28px rgba(0, 0, 0, 0.85),
    0 0 0 1px rgba(15, 23, 42, 0.9);
  padding: 4px;
  z-index: 10;
}

.search-result-item {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 8px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: inherit;
  cursor: pointer;
  font-size: 13px;
  transition: background 0.14s ease-out;
}

.search-result-item:hover {
  background: rgba(30, 64, 175, 0.45);
}

.result-name {
  font-weight: 500;
}

.result-type {
  font-size: 11px;
  color: #9ca3af;
}

.card-grid {
  width: 100%;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 20px;
  padding: 4px 0; /* 上下留出空间，防止顶部和底部卡片被遮挡 */
}

.content-row {
  flex: 1;
  max-width: 1200px;
  margin: 0 auto;
  display: grid;
  gap: 20px;
  align-items: stretch;
  min-height: 0;
  overflow: visible;
}

.content-row.ai-open {
  grid-template-columns: minmax(0, 1.7fr) minmax(320px, 0.9fr);
}

.content-row:not(.ai-open) {
  grid-template-columns: minmax(0, 1fr);
}

.cards-column {
  min-width: 0;
  min-height: 0;
  padding: 8px 4px 8px 4px; /* 上下左右留出空间，防止卡片被遮挡 */
  overflow-y: auto; /* 分类卡片区域可以滚动 */
  overflow-x: hidden;
  /* 确保可以接收鼠标滚轮事件 */
  overscroll-behavior: contain;
  /* 优化滚动条样式 */
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.4) rgba(15, 23, 42, 0.9);
}

.cards-column::-webkit-scrollbar {
  width: 8px;
}

.cards-column::-webkit-scrollbar-track {
  background: rgba(15, 23, 42, 0.9);
}

.cards-column::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.4);
  border-radius: 4px;
}

.cards-column::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.6);
}

.ai-column {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden; /* AI 列容器不滚动，内部组件滚动 */
}

.cards-row {
  flex: 1;
  max-width: 1200px;
  margin: 0 auto;
  width: 100%;
  padding: 8px 4px; /* 上下左右留出空间，防止卡片被遮挡 */
}

.ai-toggle {
  position: fixed;
  width: 40px;
  height: 40px;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.7);
  background: radial-gradient(circle at top left, rgba(148, 163, 184, 0.3), rgba(15, 23, 42, 0.98));
  color: #e5e7eb;
  cursor: move;
  user-select: none;
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 14px 30px rgba(0, 0, 0, 0.9);
  transition: box-shadow 0.18s ease-out, transform 0.18s ease-out;
  font-size: 18px;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.ai-toggle:hover:not(.dragging) {
  transform: translateY(-2px) scale(1.03);
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 20px 40px rgba(0, 0, 0, 0.95);
}

.ai-toggle.dragging {
  cursor: grabbing;
  transform: scale(1.1);
  box-shadow:
    0 0 0 2px rgba(77, 163, 255, 0.6),
    0 20px 40px rgba(0, 0, 0, 0.95);
  transition: none;
}

.category-card {
  position: relative;
  display: flex;
  flex-direction: row;
  align-items: stretch;
  padding: 14px 16px;
  border-radius: 14px;
  border: 1px solid rgba(148, 163, 184, 0.35);
  background:
    radial-gradient(circle at top left, rgba(148, 163, 184, 0.16), transparent 55%),
    linear-gradient(135deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.94));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 18px 35px rgba(0, 0, 0, 0.75);
  cursor: pointer;
  text-align: left;
  color: inherit;
  transition: transform 0.2s cubic-bezier(0.22, 0.88, 0.25, 1.05),
    box-shadow 0.2s ease-out,
    border-color 0.2s ease-out,
    background 0.2s ease-out;
}

.category-card::before {
  content: '';
  position: absolute;
  inset: -1px;
  border-radius: inherit;
  background: radial-gradient(circle at top left, color-mix(in srgb, var(--card-color) 45%, transparent), transparent 60%);
  opacity: 0.25;
  pointer-events: none;
  z-index: -1;
}

.category-card.add-card {
  border-style: dashed;
  border-color: rgba(148, 163, 184, 0.5);
  background: rgba(15, 23, 42, 0.6);
}

.category-card.add-card:hover {
  border-color: rgba(77, 163, 255, 0.7);
  background: rgba(15, 23, 42, 0.8);
}

.add-icon {
  background: rgba(77, 163, 255, 0.15);
  border: 1px dashed rgba(77, 163, 255, 0.5);
  color: #4da3ff;
  font-size: 24px;
  font-weight: 300;
}

.category-card:hover {
  transform: translateY(-4px) scale(1.02);
  border-color: color-mix(in srgb, var(--card-color) 70%, #e5e7eb 30%);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--card-color) 40%, rgba(15, 23, 42, 1) 60%),
    0 22px 45px rgba(0, 0, 0, 0.9);
}

.card-icon {
  flex: 0 0 auto;
  width: 42px;
  height: 42px;
  border-radius: 999px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 12px;
  background: radial-gradient(circle at 30% 0, #ffffff30, transparent 55%);
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 0 0 1px color-mix(in srgb, var(--card-color) 40%, transparent);
}

.icon-fallback {
  font-weight: 600;
  font-size: 18px;
  color: color-mix(in srgb, var(--card-color) 80%, #e5e7eb 20%);
}

.card-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.card-title-row {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.card-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  letter-spacing: 0.16em;
}

.card-label {
  font-size: 12px;
  color: #9ca3af;
}

.card-description {
  margin: 0;
  font-size: 13px;
  color: #9ca3af;
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

  .card-grid {
    gap: 14px;
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
  padding: 16px;
  border-radius: 12px;
  background: rgba(15, 23, 42, 0.5);
  border: 1px solid rgba(148, 163, 184, 0.15);
}

.section-title {
  margin: 0 0 4px 0;
  font-size: 14px;
  font-weight: 600;
  color: #e5e7eb;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.2);
}

.modal-form-actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  margin-top: 4px;
  padding-top: 16px;
  border-top: 1px solid rgba(148, 163, 184, 0.15);
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
  color: #9ca3af;
  font-weight: 500;
}

.field-input,
.field-textarea {
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid rgba(148, 163, 184, 0.4);
  background: rgba(15, 23, 42, 0.9);
  color: #e5e7eb;
  font-size: 14px;
  transition: all 0.14s ease-out;
}

.field-input:focus,
.field-textarea:focus {
  outline: none;
  border-color: #4da3ff;
  box-shadow: 0 0 0 1px rgba(77, 163, 255, 0.5);
  background: rgba(15, 23, 42, 0.96);
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
  border-radius: 8px;
  background: rgba(15, 23, 42, 0.9);
  border: 1px solid rgba(148, 163, 184, 0.4);
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
  border-radius: 8px;
  border: 1px solid rgba(148, 163, 184, 0.4);
  background: rgba(15, 23, 42, 0.9);
  cursor: pointer;
  flex-shrink: 0;
}

.color-picker::-webkit-color-swatch-wrapper {
  padding: 0;
}

.color-picker::-webkit-color-swatch {
  border: none;
  border-radius: 6px;
}

.btn {
  padding: 8px 16px;
  border-radius: 8px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background: rgba(15, 23, 42, 0.9);
  color: #e5e7eb;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.14s ease-out;
}

.btn:hover {
  border-color: rgba(148, 163, 184, 0.8);
  background: rgba(15, 23, 42, 0.98);
}

.btn.primary {
  border-color: #4da3ff;
  background: linear-gradient(135deg, #4da3ff, #22d3ee);
  color: #0b1120;
}

.btn.primary:hover {
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 10px 24px rgba(37, 99, 235, 0.9);
  transform: translateY(-1px);
}

.btn.ghost {
  border-color: rgba(148, 163, 184, 0.3);
  background: transparent;
}

.btn.ghost:hover {
  background: rgba(15, 23, 42, 0.6);
}

.page-footer {
  flex: 0 0 auto; /* 固定底部，不伸缩 */
  padding: 16px 32px;
  border-top: 1px solid rgba(148, 163, 184, 0.1);
  background: rgba(15, 23, 42, 0.3);
  backdrop-filter: blur(8px);
  z-index: 10; /* 确保底部在最上层 */
}

.footer-content {
  display: flex;
  justify-content: center;
  align-items: center;
}

.copyright {
  font-size: 12px;
  color: #9ca3af;
  letter-spacing: 0.05em;
}
</style>


