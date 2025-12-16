<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ContextMenu, { type MenuItem } from '../components/ContextMenu.vue'
import ModalDialog from '../components/ModalDialog.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import VirtualList from '../components/VirtualList.vue'
import {
  categoriesData,
  getOrCreateCategoryData,
  syncCategoryConfigToData,
  type SubCategory,
  type ToolItem,
} from '../stores/categories'

const route = useRoute()
const router = useRouter()

const categoryId = computed(() => (route.params.id as string) || 'web')

// 监听分类ID变化，确保数据存在
watch(
  categoryId,
  (id) => {
    syncCategoryConfigToData(id)
  },
  { immediate: true },
)

// 获取当前分类数据，如果不存在则自动创建
const category = computed(() => {
  const cat = getOrCreateCategoryData(categoryId.value)
  // 确保数据已同步到categoriesData（getOrCreateCategoryData 已经处理了）
  // 直接返回 categoriesData 中的引用，以便直接修改
  return categoriesData.value.find((c) => c.id === cat.id) ?? cat
})

const selectedSubId = ref<string | null>(null)
const searchQuery = ref('')

// 组件挂载时重置状态
watch(
  categoryId,
  () => {
    // 切换分类时重置选中状态和搜索
    selectedSubId.value = null
    searchQuery.value = ''
  },
  { immediate: false },
)

const subCategories = computed(() => category.value?.subCategories ?? [])

const currentSub = computed(() => {
  if (selectedSubId.value) {
    return subCategories.value.find((s) => s.id === selectedSubId.value) ?? null
  }
  return subCategories.value[0] ?? null
})

const tools = computed(() => currentSub.value?.tools ?? [])

// 虚拟滚动阈值：当工具数量超过此值时启用虚拟滚动
const VIRTUAL_SCROLL_THRESHOLD = 50
const shouldUseVirtualScroll = computed(() => filteredTools.value.length > VIRTUAL_SCROLL_THRESHOLD)

const filteredTools = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return tools.value
  return tools.value.filter(
    (t) =>
      t.name.toLowerCase().includes(q) ||
      (t.description && t.description.toLowerCase().includes(q)),
  )
})

const selectSub = (id: string) => {
  selectedSubId.value = id
  const target = subCategories.value.find((s) => s.id === id)
  if (target) {
    subForm.value = { id: target.id, name: target.name, description: target.description ?? '' }
    toolForm.value = emptyToolForm()
    editingToolId.value = null
  }
}

const goBack = () => {
  router.back()
}

// Tauri API 类型声明
interface TauriWindow {
  __TAURI__?: {
    invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>
  }
}

const openTool = async (toolId: string) => {
  const tool = tools.value.find((t) => t.id === toolId)
  if (!tool) return
  // 占位：调用后端命令启动外部程序；若后端未实现，则降级为日志
  try {
    const tauriWindow = window as unknown as TauriWindow
    const invoker = tauriWindow.__TAURI__?.invoke
    if (invoker && tool.execPath) {
      await invoker('launch_tool', {
        execPath: tool.execPath,
        args: tool.args ?? [],
        workingDir: tool.workingDir ?? null,
      })
    } else if (import.meta.env.DEV) {
      // 仅在开发环境输出日志
      console.log('launch tool (placeholder):', tool.execPath || tool.name, tool.args)
    }
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : '未知错误'
    if (import.meta.env.DEV) {
      console.error('launch tool failed', err)
    }
    showConfirm('提示', `启动工具失败：${errorMessage}。请检查配置或后端命令实现。`, () => {}, 'warning')
  }
}

const openWiki = (wikiUrl?: string) => {
  if (!wikiUrl) return
  window.open(wikiUrl, '_blank')
}

const goSettings = () => {
  router.push({ name: 'settings' })
}

const openWikiHome = () => {
  try {
    const tauriWindow = window as unknown as TauriWindow
    const invoker = tauriWindow.__TAURI__?.invoke
    if (invoker) {
      invoker('start_wiki_server').catch(() => {
        // 静默处理错误，允许继续打开浏览器
      })
    }
    window.open('http://127.0.0.1:8777', '_blank')
  } catch (err) {
    if (import.meta.env.DEV) {
      console.error('open wiki failed', err)
    }
    // 即使启动服务失败，也尝试打开浏览器
    window.open('http://127.0.0.1:8777', '_blank')
  }
}

const onOverlayClick = (toolId: string) => {
  openTool(toolId)
  searchQuery.value = ''
}

const subForm = ref<{ id: string; name: string; description: string }>({
  id: '',
  name: '',
  description: '',
})
const isNewSub = ref(false)

const startNewSub = () => {
  subForm.value = {
    id: `sub_${Date.now()}`,
    name: '',
    description: '',
  }
  isNewSub.value = true
  showSubModal.value = true
}

const editSub = (sub: SubCategory) => {
  subForm.value = { id: sub.id, name: sub.name, description: sub.description ?? '' }
  isNewSub.value = false
  showSubModal.value = true
}

const saveSub = () => {
  if (!subForm.value.name.trim()) {
    showConfirm('提示', '请输入子分类名称', () => {}, 'warning')
    return
  }
  if (!category.value) return
  // 确保subCategories数组存在
  if (!category.value.subCategories) {
    category.value.subCategories = []
  }
  const list = category.value.subCategories
  // 确保 id 存在
  if (!subForm.value.id) {
    subForm.value.id = `sub_${Date.now()}`
  }
  const idx = list.findIndex((s) => s.id === subForm.value.id)
  if (idx >= 0 && list[idx]) {
    // 更新现有子分类
    const existing = list[idx]
    existing.name = subForm.value.name.trim()
    existing.description = subForm.value.description.trim() || undefined
  } else {
    // 创建新子分类
    list.push({
      id: subForm.value.id,
      name: subForm.value.name.trim(),
      description: subForm.value.description.trim() || undefined,
      tools: [],
    })
  }
  selectedSubId.value = subForm.value.id
  isNewSub.value = false
  showSubModal.value = false
}

const deleteSub = (id: string) => {
  if (!category.value || !category.value.subCategories) return
  const list = category.value.subCategories
  const idx = list.findIndex((s) => s.id === id)
  if (idx >= 0) {
    list.splice(idx, 1)
    if (selectedSubId.value === id) {
      selectedSubId.value = list[0]?.id ?? null
    }
  }
}

const emptyToolForm = () => ({
  id: '',
  name: '',
  description: '',
  iconEmoji: '🛠️',
  execPath: '',
  argsText: '',
  wikiUrl: '',
})

const toolForm = ref<{
  id: string
  name: string
  description: string
  iconEmoji: string
  execPath: string
  argsText: string
  wikiUrl: string
}>({
  ...emptyToolForm(),
})
const editingToolId = ref<string | null>(null)

const startNewTool = () => {
  toolForm.value = { ...emptyToolForm(), id: `tool_${Date.now()}` }
  editingToolId.value = null
  showToolModal.value = true
}

const editTool = (tool: ToolItem) => {
  toolForm.value = {
    id: tool.id,
    name: tool.name,
    description: tool.description ?? '',
    iconEmoji: tool.iconEmoji || '🛠️',
    execPath: tool.execPath || '',
    argsText: tool.args?.join(' ') || '',
    wikiUrl: tool.wikiUrl || '',
  }
  editingToolId.value = tool.id
  showToolModal.value = true
}

const saveTool = () => {
  if (!currentSub.value) {
    showConfirm('提示', '请先选择一个子分类', () => {}, 'warning')
    return
  }
  if (!toolForm.value.name.trim()) {
    showConfirm('提示', '请输入工具名称', () => {}, 'warning')
    return
  }
  const args = toolForm.value.argsText
    .split(' ')
    .map((s) => s.trim())
    .filter(Boolean)
  const list = currentSub.value.tools
  const idx = list.findIndex((t) => t.id === toolForm.value.id)
  const base: ToolItem = {
    id: toolForm.value.id,
    name: toolForm.value.name.trim(),
    description: toolForm.value.description.trim(),
    iconEmoji: toolForm.value.iconEmoji || '🛠️',
    execPath: toolForm.value.execPath || undefined,
    args: args.length ? args : undefined,
    wikiUrl: toolForm.value.wikiUrl.trim() || undefined,
  }
  if (idx >= 0) {
    list[idx] = { ...list[idx], ...base }
  } else {
    list.push(base)
  }
  editingToolId.value = null
  showToolModal.value = false
}

const deleteTool = (id: string) => {
  if (!currentSub.value) return
  const idx = currentSub.value.tools.findIndex((t) => t.id === id)
  if (idx >= 0) currentSub.value.tools.splice(idx, 1)
}

const contextMenuRef = ref<InstanceType<typeof ContextMenu> | null>(null)
const contextMenuVisible = ref(false)
const contextMenuType = ref<'sub' | 'tool' | null>(null)
const contextMenuTarget = ref<SubCategory | ToolItem | null>(null)

const showSubMenu = (e: MouseEvent, sub: SubCategory) => {
  e.preventDefault()
  e.stopPropagation()
  contextMenuType.value = 'sub'
  contextMenuTarget.value = sub
  if (contextMenuRef.value) {
    contextMenuRef.value.show(e.clientX, e.clientY)
  }
  contextMenuVisible.value = true
}

const showToolMenu = (e: MouseEvent, tool: ToolItem) => {
  e.preventDefault()
  e.stopPropagation()
  contextMenuType.value = 'tool'
  contextMenuTarget.value = tool
  if (contextMenuRef.value) {
    contextMenuRef.value.show(e.clientX, e.clientY)
  }
  contextMenuVisible.value = true
}

const subMenuItems = computed<MenuItem[]>(() => {
  if (contextMenuType.value !== 'sub' || !contextMenuTarget.value) return []
  const sub = contextMenuTarget.value as SubCategory
  return [
    {
      label: '编辑子分类',
      icon: '✏️',
      action: () => editSub(sub),
    },
    {
      label: '删除子分类',
      icon: '🗑️',
      action: () => {
        const subId = sub.id
        const subName = sub.name
        showConfirm(
          '确认删除子分类',
          `确定删除子分类「${subName}」？`,
          () => deleteSub(subId),
          'danger',
        )
      },
      danger: true,
    },
  ]
})

const toolMenuItems = computed<MenuItem[]>(() => {
  if (contextMenuType.value !== 'tool' || !contextMenuTarget.value) return []
  const tool = contextMenuTarget.value as ToolItem
  return [
    {
      label: '编辑工具',
      icon: '✏️',
      action: () => editTool(tool),
    },
    {
      label: '打开工具',
      icon: '▶️',
      action: () => openTool(tool.id),
    },
    ...(tool.wikiUrl
      ? [
          {
            label: '在 Wiki 中查看',
            icon: '📚',
            action: () => openWiki(tool.wikiUrl),
          },
        ]
      : []),
    {
      label: '删除工具',
      icon: '🗑️',
      action: () => {
        const toolId = tool.id
        const toolName = tool.name
        showConfirm(
          '确认删除工具',
          `确定删除工具「${toolName}」？`,
          () => deleteTool(toolId),
          'danger',
        )
      },
      danger: true,
    },
  ]
})

const contextMenuItems = computed<MenuItem[]>(() => {
  if (contextMenuType.value === 'sub') return subMenuItems.value
  if (contextMenuType.value === 'tool') return toolMenuItems.value
  return []
})

const closeContextMenu = () => {
  contextMenuVisible.value = false
  contextMenuType.value = null
  contextMenuTarget.value = null
  if (contextMenuRef.value) {
    // 重置菜单位置到屏幕外
    contextMenuRef.value.show(-9999, -9999)
  }
}

const showBlankMenu = (e: MouseEvent) => {
  // 只在空白区域显示
  const target = e.target as HTMLElement
  if (
    target.closest('.sub-card') ||
    target.closest('.tool-card') ||
    target.closest('.search-box') ||
    target.closest('.page-header') ||
    target.closest('.sub-form') ||
    target.closest('.tool-form')
  ) {
    return
  }
  e.preventDefault()
  contextMenuType.value = null
  contextMenuTarget.value = null
  if (contextMenuRef.value) {
    contextMenuRef.value.show(e.clientX, e.clientY)
  }
  contextMenuVisible.value = true
}

const blankMenuItems = computed<MenuItem[]>(() => {
  if (contextMenuType.value || contextMenuTarget.value) return []
  return [
    {
      label: '添加子分类',
      icon: '➕',
      action: () => {
        startNewSub()
        showSubModal.value = true
      },
    },
    {
      label: '添加工具',
      icon: '🛠️',
      action: () => {
        if (!currentSub.value) {
          showConfirm('提示', '请先选择一个子分类', () => {}, 'warning')
          return
        }
        startNewTool()
        showToolModal.value = true
      },
    },
  ]
})

const finalMenuItems = computed(() => {
  if (contextMenuType.value || contextMenuTarget.value) return contextMenuItems.value
  return blankMenuItems.value
})

const showSubModal = ref(false)
const showToolModal = ref(false)

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
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div class="header-left">
        <button type="button" class="back-button" title="返回上层" @click="goBack">
          ←
        </button>
        <div class="title-block">
          <h1 class="title">{{ category?.name }}</h1>
          <p class="subtitle">
            {{ category?.label }} · {{ category?.description }}
          </p>
        </div>
      </div>
      <div class="header-actions">
        <button type="button" class="icon-button" @click="openWikiHome">
          <span class="icon">📚</span>
          <span class="icon-label">Wiki</span>
        </button>
        <button type="button" class="icon-button" @click="goSettings">
          <span class="icon">⚙</span>
          <span class="icon-label">设置</span>
        </button>
      </div>
    </header>

    <main class="page-main">
      <div class="search-row">
        <div class="search-box">
          <span class="search-icon">🔍</span>
          <input
            v-model="searchQuery"
            class="search-input"
            type="search"
            placeholder="搜索当前子分类的工具名称或描述"
          />
        </div>
      </div>
      <div class="content-row" @contextmenu="showBlankMenu">
        <aside class="sub-list">
          <div class="sub-title">子分类</div>
          <div class="sub-cards">
            <button
              v-for="sub in subCategories"
              :key="sub.id"
              type="button"
              class="sub-card"
              :class="{ active: sub.id === selectedSubId }"
              @click="selectSub(sub.id)"
              @contextmenu="showSubMenu($event, sub)"
            >
              <div class="sub-name">{{ sub.name }}</div>
              <div class="sub-desc">{{ sub.description }}</div>
            </button>
            <button
              type="button"
              class="sub-card add-sub-card"
              @click="startNewSub"
            >
              <div class="sub-name add-sub-name">+ 添加子分类</div>
            </button>
          </div>
        </aside>

        <section class="tools-area">
          <div class="tools-header">
            <div>
              <h2>{{ currentSub?.name || category?.label || category?.name || '分类工具' }}</h2>
              <p>{{ currentSub?.description || category?.description || '选择一个子分类以查看工具，或直接添加工具' }}</p>
            </div>
            <div class="tools-header-actions">
              <button
                v-if="!currentSub && subCategories.length === 0"
                type="button"
                class="icon-button"
                @click="startNewSub"
              >
                <span class="icon">＋</span>
                <span class="icon-label">添加子分类</span>
              </button>
              <button
                v-else-if="currentSub"
                type="button"
                class="icon-button"
                @click="startNewTool"
              >
                <span class="icon">＋</span>
                <span class="icon-label">新增工具</span>
              </button>
              <button
                v-else
                type="button"
                class="icon-button"
                @click="startNewTool"
                :disabled="true"
                title="请先选择一个子分类"
              >
                <span class="icon">＋</span>
                <span class="icon-label">新增工具</span>
              </button>
            </div>
          </div>

          <div
            v-if="searchQuery && filteredTools.length"
            class="search-overlay"
          >
            <div class="overlay-title">搜索结果</div>
            <div class="overlay-list">
              <button
                v-for="tool in filteredTools"
                :key="tool.id"
                type="button"
                class="overlay-item"
                @click="onOverlayClick(tool.id)"
              >
                <span class="overlay-icon">{{ tool.iconEmoji || '🛠️' }}</span>
                <span class="overlay-text">
                  <span class="overlay-name">{{ tool.name }}</span>
                  <span class="overlay-desc">{{ tool.description }}</span>
                </span>
                <span class="overlay-action">打开</span>
              </button>
            </div>
          </div>

          <div v-if="!currentSub && subCategories.length === 0" class="empty-state">
            <div class="empty-icon">📁</div>
            <h3>暂无子分类</h3>
            <p>点击上方"添加子分类"按钮或左侧"添加子分类"按钮开始创建子分类</p>
            <button type="button" class="btn primary" @click="startNewSub" style="margin-top: 16px">
              ＋ 添加子分类
            </button>
          </div>
          <div v-else-if="currentSub && filteredTools.length === 0 && !searchQuery" class="empty-state">
            <div class="empty-icon">🛠️</div>
            <h3>暂无工具</h3>
            <p>点击"新增工具"按钮添加工具到此子分类</p>
            <button type="button" class="btn primary" @click="startNewTool" style="margin-top: 16px">
              ＋ 新增工具
            </button>
          </div>
          <div v-else-if="currentSub" class="tools-grid-wrapper">
            <VirtualList
              v-if="shouldUseVirtualScroll"
              :items="filteredTools"
              :item-height="180"
              :container-height="600"
              class="virtual-tools-list"
            >
              <template #default="{ item: tool }">
                <div
                  class="tool-card"
                  @contextmenu="showToolMenu($event, tool as ToolItem)"
                >
                  <div class="tool-header">
                    <div class="tool-icon">{{ (tool as ToolItem).iconEmoji || '🛠️' }}</div>
                    <div class="tool-name">{{ (tool as ToolItem).name }}</div>
                  </div>
                  <p class="tool-desc">{{ (tool as ToolItem).description }}</p>
                  <div class="tool-actions">
                    <div class="tool-meta">工具ID：{{ (tool as ToolItem).id }}</div>
                    <div class="tool-buttons">
                      <button type="button" class="btn ghost" @click="openWiki((tool as ToolItem).wikiUrl)">📚</button>
                      <button type="button" class="btn primary" @click="openTool((tool as ToolItem).id)">打开工具</button>
                    </div>
                  </div>
                </div>
              </template>
            </VirtualList>
            <div v-else class="tools-grid">
              <div
                v-for="tool in filteredTools"
                :key="tool.id"
                class="tool-card"
                @contextmenu="showToolMenu($event, tool)"
                @wheel.stop
              >
                <div class="tool-header">
                  <div class="tool-icon">{{ tool.iconEmoji || '🛠️' }}</div>
                  <div class="tool-name">{{ tool.name }}</div>
                </div>
                <p class="tool-desc">{{ tool.description }}</p>
                <div class="tool-actions">
                  <div class="tool-meta">工具ID：{{ tool.id }}</div>
                  <div class="tool-buttons">
                    <button type="button" class="btn ghost" @click="openWiki(tool.wikiUrl)">📚</button>
                    <button type="button" class="btn primary" @click="openTool(tool.id)">打开工具</button>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div v-else-if="!currentSub && subCategories.length > 0" class="empty-state">
            <div class="empty-icon">👈</div>
            <h3>请选择子分类</h3>
            <p>从左侧选择一个子分类以查看工具</p>
          </div>
        </section>
      </div>
    </main>

    <ContextMenu
      v-if="contextMenuVisible"
      ref="contextMenuRef"
      :items="finalMenuItems"
      @close="closeContextMenu"
    />

    <ModalDialog
      v-model:visible="showSubModal"
      :title="isNewSub ? '新增子分类' : '编辑子分类'"
      :collapsible="true"
    >
      <div class="modal-form">
        <label class="field">
          <span class="field-label">名称</span>
          <input v-model="subForm.name" class="field-input" placeholder="请输入子分类名称" />
        </label>
        <label class="field">
          <span class="field-label">描述</span>
          <textarea
            v-model="subForm.description"
            class="field-textarea"
            rows="3"
            placeholder="简单说明"
          />
        </label>
        <div class="modal-form-actions">
          <button type="button" class="btn primary" @click="saveSub">保存</button>
          <button type="button" class="btn ghost" @click="showSubModal = false">取消</button>
        </div>
      </div>
    </ModalDialog>

    <ModalDialog
      v-model:visible="showToolModal"
      :title="editingToolId ? '编辑工具' : '新增工具'"
      :collapsible="true"
    >
      <div class="modal-form">
        <div class="tool-form-grid">
          <label class="field">
            <span class="field-label">名称</span>
            <input v-model="toolForm.name" class="field-input" placeholder="工具名称" />
          </label>
          <label class="field">
            <span class="field-label">图标(emoji)</span>
            <input v-model="toolForm.iconEmoji" class="field-input" placeholder="例如 🛠️" />
          </label>
        </div>
        <label class="field">
          <span class="field-label">描述</span>
          <textarea
            v-model="toolForm.description"
            class="field-textarea"
            rows="3"
            placeholder="工具用途简介"
          />
        </label>
        <div class="tool-form-grid">
          <label class="field">
            <span class="field-label">可执行路径</span>
            <input v-model="toolForm.execPath" class="field-input" placeholder="C:\\Tools\\tool.exe" />
          </label>
          <label class="field">
            <span class="field-label">参数(空格分隔)</span>
            <input v-model="toolForm.argsText" class="field-input" placeholder="-d example.com -v" />
          </label>
        </div>
        <label class="field">
          <span class="field-label">Wiki URL（可选）</span>
          <input
            v-model="toolForm.wikiUrl"
            class="field-input"
            placeholder="https://wiki.example.com/tool-name 或留空"
          />
        </label>
        <div class="modal-form-actions">
          <button type="button" class="btn primary" @click="saveTool">保存</button>
          <button type="button" class="btn ghost" @click="showToolModal = false">取消</button>
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
  background: radial-gradient(circle at top, #020617 0, #000000 80%);
  color: #e5e7eb;
  overflow: hidden; /* 固定整体页面 */
}

.page-header {
  flex: 0 0 auto; /* 固定头部，不伸缩 */
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 24px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.2);
  background: linear-gradient(to bottom, rgba(15, 23, 42, 0.96), rgba(15, 23, 42, 0.9));
  z-index: 10; /* 确保头部在最上层 */
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.back-button {
  width: 30px;
  height: 30px;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.7);
  background: rgba(15, 23, 42, 0.98);
  color: #e5e7eb;
  cursor: pointer;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.16s ease-out;
}

.back-button:hover {
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 8px 18px rgba(15, 23, 42, 0.9);
  transform: translateY(-1px);
}

.title-block h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.title-block p {
  margin: 2px 0 0;
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

.icon-button:hover:not(:disabled) {
  border-color: #e5e7eb;
  box-shadow: 0 0 0 1px rgba(148, 163, 184, 0.5), 0 12px 24px rgba(15, 23, 42, 0.9);
  transform: translateY(-1px);
}

.icon-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.icon {
  font-size: 14px;
}

.icon-label {
  white-space: nowrap;
}

.page-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 16px 16px;
  min-height: 0;
  overflow: hidden; /* 主内容区域不滚动，内部子区域滚动 */
}

.search-row {
  display: flex;
  justify-content: center;
}

.search-box {
  position: relative;
  width: 100%;
  max-width: 760px;
  margin: 0 auto;
}

.search-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 14px;
  color: #6b7280;
  pointer-events: none;
}

.search-input {
  width: 100%;
  padding: 8px 12px 8px 30px;
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

.content-row {
  display: flex;
  gap: 14px;
  width: 100%;
  align-items: flex-start;
}

.sub-list {
  flex: 0 0 260px;
  border-right: 1px solid rgba(148, 163, 184, 0.2);
  padding-right: 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  max-height: 100%;
  overflow: hidden; /* 子分类列表容器不滚动 */
}

.sub-title {
  font-size: 13px;
  color: #9ca3af;
  margin-bottom: 10px;
  flex: 0 0 auto; /* 固定标题，不伸缩 */
}

.sub-cards {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
  flex: 1;
  overflow-y: auto; /* 子分类卡片列表可以滚动 */
  overflow-x: hidden;
  padding-right: 4px; /* 为滚动条留出空间 */
  /* 优化滚动条样式 */
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.4) rgba(15, 23, 42, 0.9);
}

.sub-cards::-webkit-scrollbar {
  width: 6px;
}

.sub-cards::-webkit-scrollbar-track {
  background: rgba(15, 23, 42, 0.9);
}

.sub-cards::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.4);
  border-radius: 3px;
}

.sub-cards::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.6);
}

.sub-card {
  text-align: left;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.3);
  background: rgba(15, 23, 42, 0.92);
  color: #e5e7eb;
  cursor: pointer;
  transition: all 0.16s ease-out;
}

.sub-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 10px 20px rgba(0, 0, 0, 0.45);
}

.sub-card.active {
  border-color: #4da3ff;
  background: linear-gradient(135deg, rgba(77, 163, 255, 0.12), rgba(15, 23, 42, 0.96));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 14px 30px rgba(0, 0, 0, 0.6);
}

.sub-name {
  font-size: 14px;
  font-weight: 600;
}

.sub-desc {
  font-size: 12px;
  color: #9ca3af;
  margin-top: 2px;
}

.sub-card.add-sub-card {
  border-style: dashed;
  border-color: rgba(148, 163, 184, 0.5);
  background: rgba(15, 23, 42, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
}

.sub-card.add-sub-card:hover {
  border-color: rgba(77, 163, 255, 0.7);
  background: rgba(15, 23, 42, 0.8);
}

.add-sub-name {
  color: #4da3ff;
  font-size: 13px;
}

.sub-actions {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}

.sub-form {
  border: 1px dashed rgba(148, 163, 184, 0.4);
  border-radius: 12px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: rgba(15, 23, 42, 0.85);
}

.sub-form-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.sub-form-title {
  font-size: 13px;
  color: #e5e7eb;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-label {
  font-size: 12px;
  color: #9ca3af;
}

.field-input,
.field-textarea {
  border-radius: 8px;
  border: 1px solid rgba(148, 163, 184, 0.4);
  background: rgba(15, 23, 42, 0.9);
  color: #e5e7eb;
  padding: 6px 8px;
  font-size: 13px;
  outline: none;
  transition: border-color 0.16s ease-out, box-shadow 0.16s ease-out, background 0.16s ease-out;
}

.field-input:focus,
.field-textarea:focus {
  border-color: #4da3ff;
  box-shadow: 0 0 0 1px rgba(77, 163, 255, 0.5);
  background: rgba(15, 23, 42, 0.96);
}

.tools-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  position: relative;
  min-height: 0;
  overflow: hidden; /* 工具区域容器不滚动 */
  /* 优化渲染性能，防止残影 */
  transform: translateZ(0);
  -webkit-transform: translateZ(0);
  will-change: contents;
  contain: layout style paint;
}

.tools-header h2 {
  margin: 0;
  font-size: 16px;
}

.tools-header p {
  margin: 4px 0 0;
  color: #9ca3af;
  font-size: 13px;
}

.tools-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 8px;
  padding-bottom: 4px;
}

.tools-header-actions {
  display: flex;
  align-items: center;
}

.tool-form {
  border: 1px dashed rgba(148, 163, 184, 0.4);
  border-radius: 12px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: rgba(15, 23, 42, 0.85);
}

.tool-form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 8px;
}

.tool-form-actions {
  display: flex;
  gap: 6px;
}

.tools-grid-wrapper {
  flex: 1;
  min-height: 0;
  padding-top: 8px;
  overflow-y: auto; /* 工具网格区域可以滚动 */
  overflow-x: hidden;
  /* 优化滚动条样式 */
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.4) rgba(15, 23, 42, 0.9);
  /* 优化渲染性能 */
  transform: translateZ(0);
  -webkit-transform: translateZ(0);
  will-change: contents;
  contain: layout style paint;
}

.tools-grid-wrapper::-webkit-scrollbar {
  width: 8px;
}

.tools-grid-wrapper::-webkit-scrollbar-track {
  background: rgba(15, 23, 42, 0.9);
}

.tools-grid-wrapper::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.4);
  border-radius: 4px;
}

.tools-grid-wrapper::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.6);
}

.tools-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 14px;
  padding-top: 8px;
}

.virtual-tools-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 14px;
  padding: 8px 0 0 0;
}

.tool-card {
  padding: 12px;
  border-radius: 14px;
  border: 1px solid rgba(148, 163, 184, 0.3);
  background:
    radial-gradient(circle at top left, rgba(148, 163, 184, 0.14), transparent 55%),
    linear-gradient(140deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.94));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 16px 32px rgba(0, 0, 0, 0.75);
  display: flex;
  flex-direction: column;
  gap: 8px;
  transition: transform 0.18s ease-out, box-shadow 0.18s ease-out, border-color 0.18s ease-out;
}

.tool-card:hover {
  transform: translateY(-3px);
  border-color: rgba(77, 163, 255, 0.7);
  box-shadow:
    0 0 0 1px rgba(77, 163, 255, 0.3),
    0 22px 40px rgba(0, 0, 0, 0.85);
}

.tool-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.tool-icon {
  width: 38px;
  height: 38px;
  border-radius: 12px;
  background: rgba(77, 163, 255, 0.08);
  border: 1px solid rgba(148, 163, 184, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
}

.tool-name {
  font-size: 15px;
  font-weight: 600;
}

.tool-desc {
  margin: 0;
  font-size: 13px;
  color: #9ca3af;
  line-height: 1.5;
}

.tool-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-top: auto;
}

.tool-meta {
  font-size: 12px;
  color: #9ca3af;
}

.tool-buttons {
  display: flex;
  gap: 6px;
}

.search-overlay {
  position: absolute;
  inset: 0;
  background: rgba(2, 6, 23, 0.78);
  backdrop-filter: blur(6px);
  border-radius: 14px;
  border: 1px solid rgba(148, 163, 184, 0.35);
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 18px 40px rgba(0, 0, 0, 0.75);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 5;
}

.overlay-title {
  font-size: 13px;
  color: #9ca3af;
}

.overlay-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  overflow-y: auto;
}

.overlay-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid rgba(148, 163, 184, 0.25);
  background: rgba(15, 23, 42, 0.9);
  color: #e5e7eb;
  cursor: pointer;
  text-align: left;
  transition: all 0.16s ease-out;
}

.overlay-item:hover {
  border-color: rgba(77, 163, 255, 0.6);
  box-shadow: 0 10px 22px rgba(0, 0, 0, 0.65);
}

.overlay-icon {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: rgba(77, 163, 255, 0.1);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
}

.overlay-text {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.overlay-name {
  font-size: 14px;
  font-weight: 600;
}

.overlay-desc {
  font-size: 12px;
  color: #9ca3af;
  line-height: 1.4;
}

.overlay-action {
  font-size: 12px;
  color: #4da3ff;
}

.btn {
  border-radius: 999px;
  border: 1px solid transparent;
  padding: 6px 12px;
  font-size: 12px;
  cursor: pointer;
  color: #e5e7eb;
  background: transparent;
  transition: all 0.16s ease-out;
}

.btn.ghost {
  border-color: rgba(148, 163, 184, 0.6);
}

.btn.ghost:hover {
  background: rgba(15, 23, 42, 0.96);
}

.btn.danger {
  border-color: #f87171;
  color: #fca5a5;
}

.btn.danger:hover {
  background: rgba(248, 113, 113, 0.12);
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 10px 20px rgba(248, 113, 113, 0.35);
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

.chip {
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.6);
  background: transparent;
  color: #e5e7eb;
  padding: 4px 8px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.14s ease-out;
}

.chip:hover {
  background: rgba(15, 23, 42, 0.96);
}

.chip.primary {
  border-color: #4da3ff;
  color: #4da3ff;
}

.chip.danger {
  border-color: #f87171;
  color: #fca5a5;
}

.btn.full {
  width: 100%;
}

.modal-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.modal-form-actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  margin-top: 8px;
}

.modal-form .tool-form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}

@media (max-width: 960px) {
  .page-main {
    padding-top: 12px;
    gap: 6px;
  }

  .content-row {
    flex-direction: column;
  }

  .sub-list {
    width: 100%;
    border-right: none;
    border-bottom: 1px solid rgba(148, 163, 184, 0.2);
    padding-bottom: 14px;
    margin-bottom: 8px;
  }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 24px;
  text-align: center;
  gap: 16px;
  min-height: 300px;
}

.empty-icon {
  font-size: 64px;
  opacity: 0.5;
}

.empty-state h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: #e5e7eb;
}

.empty-state p {
  margin: 0;
  font-size: 14px;
  color: #9ca3af;
  max-width: 400px;
  line-height: 1.6;
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


