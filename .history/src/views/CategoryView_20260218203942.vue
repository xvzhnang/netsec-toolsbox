<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ContextMenu, { type MenuItem } from '../components/ContextMenu.vue'
import ModalDialog from '../components/ModalDialog.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import VirtualList from '../components/VirtualList.vue'
import SearchOverlay from '../components/SearchOverlay.vue'
import { useSearch, type SearchItem as GlobalSearchItem } from '../composables/useSearch'
import {
  categoriesData,
  getOrCreateCategoryData,
  syncCategoryConfigToData,
  type SubCategory,
  type ToolItem,
  type ToolType,
} from '../stores/categories'
import { openFileDialog } from '../utils/fileDialog'
import { selectImageFile, processImage, autoFetchIcon, detectFileTypeFromPath } from '../utils/imageProcessor'
import { getTauriInvoke } from '../utils/tauri'
import { launchTool } from '../utils/toolLauncher'
import { saveIconToCache } from '../utils/fileStorage'
import { getIconUrl } from '../utils/iconLoader'
import { debug, error as logError, info, warn } from '../utils/logger'
import { fetchWikiContent } from '../utils/docsifyLauncher'
import { openMkDocs } from '../utils/mkdocsLauncher'
import { DEFAULT_TOOL_ICON } from '../utils/constants'
import ToolWikiPanel from '../components/ToolWikiPanel.vue'
import AppLayout from '../layouts/AppLayout.vue'

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
// 视图模式：'grid' 网格视图，'list' 列表视图
const viewMode = ref<'grid' | 'list'>('grid')
// 搜索结果的选中索引（用于键盘导航）
const selectedSearchIndex = ref(-1)

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

// 获取工具图标的显示 URL（优先使用 base64，否则使用原始路径）
const getToolIconUrl = (tool: ToolItem): string | undefined => {
  // 如果有 base64，优先使用（用于显示）
  if (tool._iconBase64) {
    return tool._iconBase64
  }
  // 如果有 base64 格式的 iconUrl（向后兼容）
  if (tool.iconUrl && tool.iconUrl.startsWith('data:image')) {
    return tool.iconUrl
  }
  // 否则使用原始路径
  return tool.iconUrl
}

// 自动获取工具图标（在工具加载时，如果还没有图标）
const autoFetchToolIcons = async () => {
  if (!currentSub.value) {
    debug('autoFetchToolIcons: currentSub 为空，跳过')
    return
  }
  
  debug('autoFetchToolIcons: 开始处理', { toolsCount: currentSub.value.tools.length })
  
  for (const tool of currentSub.value.tools) {
    // 如果图标路径需要转换为 base64（用于显示）
    // 支持多种格式：icons/, .config/icons/, 绝对路径等
    if (tool.iconUrl && !tool.iconUrl.startsWith('data:image') && !tool.iconUrl.startsWith('http://') && !tool.iconUrl.startsWith('https://')) {
      const originalPath = tool.iconUrl // 保存原始路径用于日志
      
      debug('发现图标路径需要转换为 base64:', { 
        toolId: tool.id, 
        toolName: tool.name, 
        iconPath: originalPath
      })
      
      try {
        // 直接使用原始路径读取，不进行规范化
        const base64Url = await getIconUrl(originalPath)
        if (base64Url && base64Url.startsWith('data:image')) {
          // 将 base64 存储到 _iconBase64 字段，保留原始路径在 iconUrl 中
          // 这样保存时可以使用原始路径，显示时使用 base64
          tool._iconBase64 = base64Url
          info('✅ 图标已加载为 base64（保留原始路径）:', { 
            toolId: tool.id, 
            toolName: tool.name, 
            originalPath,
            convertedLength: base64Url.length 
          })
        } else {
          warn('⚠️ 图标路径读取返回无效数据:', { 
            toolId: tool.id, 
            toolName: tool.name, 
            originalPath,
            result: base64Url ? base64Url.substring(0, 50) : 'null',
            resultType: typeof base64Url
          })
          // 如果读取失败，清除 base64，保留原始路径
          tool._iconBase64 = undefined
        }
      } catch (error) {
        logError('❌ 读取图标文件失败:', { 
          toolId: tool.id, 
          toolName: tool.name, 
          iconPath: originalPath, 
          error,
          errorMessage: error instanceof Error ? error.message : String(error)
        })
        // 读取失败时，清除 base64，保留原始路径
        tool._iconBase64 = undefined
      }
      continue
    }
    
    // 如果工具已经有图标，跳过
    if (tool.iconUrl) continue
    
    // 如果工具类型支持自动获取，且执行路径存在
    if (tool.toolType && (tool.execPath || (tool.toolType === 'JAR' && tool.jarConfig?.jarPath))) {
      const execPath = tool.toolType === 'JAR' 
        ? tool.jarConfig?.jarPath 
        : tool.execPath
      
      if (execPath) {
        try {
          const autoIcon = await autoFetchIcon(tool.toolType, execPath)
          if (autoIcon) {
            const iconPath = await saveIconToCache(autoIcon)
            tool._iconBase64 = autoIcon
            tool.iconUrl = iconPath
            debug('自动获取工具图标成功:', { toolId: tool.id, toolName: tool.name, toolType: tool.toolType })
          }
        } catch (error) {
            warn('自动获取工具图标失败:', { toolId: tool.id, error })
          }
      }
    }
  }
  
  // 触发响应式更新
  if (category.value) {
    const categoryIndex = categoriesData.value.findIndex(c => c.id === category.value?.id)
    if (categoryIndex >= 0 && categoriesData.value[categoryIndex]) {
      const existing = categoriesData.value[categoryIndex]
      categoriesData.value[categoryIndex] = {
        id: existing.id,
        name: existing.name,
        label: existing.label,
        description: existing.description,
        subCategories: existing.subCategories,
      }
    }
  }
}

// 监听当前子分类变化，自动获取图标
watch(
  currentSub,
  async (newSub) => {
    if (newSub) {
      debug('currentSub 变化，开始转换图标', { subId: newSub.id, toolsCount: newSub.tools.length })
      try {
        // 立即执行图标转换，确保相对路径图标被转换
        await autoFetchToolIcons()
        debug('图标转换完成')
      } catch (error) {
        logError('图标转换过程中出错:', error)
      }
    }
  },
  { immediate: true }
)

// 虚拟滚动阈值：当工具数量超过此值时启用虚拟滚动
const VIRTUAL_SCROLL_THRESHOLD = 50
const shouldUseVirtualScroll = computed(() => filteredTools.value.length > VIRTUAL_SCROLL_THRESHOLD)

// Flatten all tools in the category for the overlay search
const categorySearchItems = computed<GlobalSearchItem[]>(() => {
  if (!category.value) return []
  const items: GlobalSearchItem[] = []
  category.value.subCategories.forEach(sub => {
    sub.tools.forEach(tool => {
      items.push({
        id: tool.id,
        name: tool.name,
        description: tool.description,
        iconUrl: tool.iconUrl,
        iconEmoji: tool.iconEmoji || DEFAULT_TOOL_ICON,
        type: 'tool',
        originalData: tool,
        categoryId: category.value.id,
        subCategoryId: sub.id
      })
    })
  })
  return items
})

const { filteredResults: overlayResults } = useSearch(categorySearchItems, searchQuery)

// For the grid view, we only show tools in the current subcategory that match
const currentSubSearchItems = computed<GlobalSearchItem[]>(() => {
  return tools.value.map(tool => ({
    id: tool.id,
    name: tool.name,
    description: tool.description,
    iconUrl: tool.iconUrl,
    iconEmoji: tool.iconEmoji || DEFAULT_TOOL_ICON,
    type: 'tool',
    originalData: tool,
    categoryId: category.value.id,
    subCategoryId: currentSub.value?.id
  }))
})

const { filteredResults: filteredGridItems } = useSearch(currentSubSearchItems, searchQuery)

// 模糊搜索：支持多关键词匹配 (Grid View)
const filteredTools = computed(() => {
  if (!searchQuery.value.trim()) {
    return tools.value
  }
  return filteredGridItems.value.map(item => item.originalData as ToolItem)
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



/**
 * 打开工具
 * 从 JSON 配置文件中读取的工具信息（通过 categoriesData → category → currentSub → tools）
 * 工具的所有调用信息（execPath、args、toolType、jarConfig等）都来自 JSON 配置文件
 */
const openTool = async (toolId: string) => {
  // 从响应式数据中查找工具（这些数据来自 JSON 配置文件）
  // tools.value 来自: categoriesData (JSON) → category → currentSub → tools
  const tool = tools.value.find((t) => t.id === toolId)
  if (!tool) {
    logError('工具未找到:', toolId, '可用工具:', tools.value.map(t => t.id))
    return
  }
  
  debug('打开工具（从 JSON 配置文件读取）:', {
    toolId,
    toolName: tool.name,
    toolType: tool.toolType,
    execPath: tool.execPath,
    args: tool.args,
    workingDir: tool.workingDir,
    jarConfig: tool.jarConfig,
  })
  
  // 使用公共的工具启动函数
  await launchTool(tool, showConfirm)
}

const goSettings = () => {
  router.push({ name: 'settings' })
}

// Wiki 状态管理
const showWikiPanel = ref(false)
const currentWikiTool = ref<ToolItem | null>(null)
const wikiContent = ref('')
const wikiLoading = ref(false)
const wikiError = ref('')

const openWikiHome = async () => {
  try {
    await openMkDocs()
  } catch (error) {
    // 错误已记录
  }
}

const onOpenToolWiki = async (tool: ToolItem) => {
  debug('Opening Wiki for tool:', tool.name)
  
  // 重置状态
  currentWikiTool.value = tool
  wikiContent.value = ''
  wikiError.value = ''
  showWikiPanel.value = true
  wikiLoading.value = true
  
  try {
    // 如果 wikiUrl 存在，尝试获取内容
    if (tool.wikiUrl) {
      wikiContent.value = await fetchWikiContent(tool.wikiUrl)
    } else {
      // 如果没有配置 wikiUrl，显示默认提示
      wikiContent.value = '# ' + tool.name + '\n\n暂无文档内容。'
    }
  } catch (error) {
    logError('Failed to load wiki content:', error)
    wikiError.value = '加载文档失败，请检查 Docsify 服务或文档路径。'
  } finally {
    wikiLoading.value = false
  }
}

const onResultClick = (item: GlobalSearchItem) => {
  if (item.originalData) {
    openTool((item.originalData as ToolItem).id)
  }
  searchQuery.value = ''
  selectedSearchIndex.value = -1
}

// 处理搜索输入框的键盘事件
const handleSearchInputKeydown = (e: KeyboardEvent) => {
  if (!searchQuery.value || overlayResults.value.length === 0) return
  
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedSearchIndex.value = Math.min(selectedSearchIndex.value + 1, overlayResults.value.length - 1)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedSearchIndex.value = Math.max(selectedSearchIndex.value - 1, -1)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (selectedSearchIndex.value >= 0 && selectedSearchIndex.value < overlayResults.value.length) {
      const item = overlayResults.value[selectedSearchIndex.value]
      if (item) {
        onResultClick(item)
      }
    } else if (overlayResults.value.length > 0) {
      // 如果没有选中项，打开第一个
      const firstItem = overlayResults.value[0]
      if (firstItem) {
        onResultClick(firstItem)
      }
    }
  } else if (e.key === 'Escape') {
    searchQuery.value = ''
    selectedSearchIndex.value = -1
  }
}

// 处理搜索输入变化
const handleSearchInput = () => {
  // 搜索内容改变时重置选中索引
  selectedSearchIndex.value = -1
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
  
  // 确保修改被 Vue 响应式系统检测到
  if (category.value) {
    const categoryIndex = categoriesData.value.findIndex(c => c.id === category.value?.id)
    if (categoryIndex >= 0 && categoriesData.value[categoryIndex]) {
      const existing = categoriesData.value[categoryIndex]
      // 创建新的 subCategories 数组，确保引用改变
      let newSubCategories: SubCategory[]
      if (idx >= 0) {
        // 更新现有子分类
        newSubCategories = existing.subCategories.map(sub => {
          if (sub.id === subForm.value.id) {
            return {
              ...sub,
              name: subForm.value.name.trim(),
              description: subForm.value.description.trim() || undefined,
            }
          }
          return { ...sub }
        })
      } else {
        // 创建新子分类
        newSubCategories = [
          ...existing.subCategories,
          {
            id: subForm.value.id,
            name: subForm.value.name.trim(),
            description: subForm.value.description.trim() || undefined,
            tools: [],
          }
        ]
      }
      
      // 创建新的分类对象，确保所有引用都改变
      categoriesData.value[categoryIndex] = {
        id: existing.id,
        name: existing.name,
        label: existing.label,
        description: existing.description,
        subCategories: newSubCategories,
      }
      
      // 同步更新本地引用（用于 UI 显示）
      if (idx >= 0 && list[idx]) {
        list[idx] = {
          ...list[idx],
          name: subForm.value.name.trim(),
          description: subForm.value.description.trim() || undefined,
        }
      } else {
        list.push({
          id: subForm.value.id,
          name: subForm.value.name.trim(),
          description: subForm.value.description.trim() || undefined,
          tools: [],
        })
      }
      
      debug('子分类已保存，已触发响应式更新，等待自动同步到配置文件...', {
        subId: subForm.value.id,
        subName: subForm.value.name,
        categoryId: category.value.id,
      })
    }
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
    // 确保修改被 Vue 响应式系统检测到
    const categoryIndex = categoriesData.value.findIndex(c => c.id === category.value?.id)
    if (categoryIndex >= 0 && categoriesData.value[categoryIndex]) {
      const existing = categoriesData.value[categoryIndex]
      // 创建新的 subCategories 数组，排除被删除的子分类
      const newSubCategories = existing.subCategories.filter(sub => sub.id !== id)
      
      // 创建新的分类对象，确保所有引用都改变
      categoriesData.value[categoryIndex] = {
        id: existing.id,
        name: existing.name,
        label: existing.label,
        description: existing.description,
        subCategories: newSubCategories,
      }
      
      debug('子分类已删除，已触发响应式更新，等待自动保存到配置文件...', {
        subId: id,
        categoryId: category.value.id,
      })
    }
  }
}

const emptyToolForm = () => ({
  id: '',
  name: '',
  description: '',
  iconUrl: '',
  toolType: 'GUI' as ToolType,
  execPath: '',
  argsText: '',
  wikiUrl: '',
  // JAR 配置
  jarPath: '',
  javaPath: '',
  jvmArgsText: '',
  programArgsText: '',
  // Python 配置
  pythonEnv: '',
})

const toolForm = ref<{
  id: string
  name: string
  description: string
  iconUrl: string
  toolType: ToolType
  execPath: string
  argsText: string
  wikiUrl: string
  // JAR 配置
  jarPath: string
  javaPath: string
  jvmArgsText: string
  programArgsText: string
  // Python 配置
  pythonEnv: string
}>({
  ...emptyToolForm(),
})
const editingToolId = ref<string | null>(null)

const startNewTool = () => {
  toolForm.value = { ...emptyToolForm(), id: `tool_${Date.now()}` }
  editingToolId.value = null
  // 重置图标跟踪状态
  isManualIcon.value = false
  autoFetchedIconPath.value = null
  showToolModal.value = true
}

const editTool = async (tool: ToolItem) => {
  toolForm.value = {
    id: tool.id,
    name: tool.name,
    description: tool.description ?? '',
    iconUrl: tool.iconUrl || '',
    toolType: tool.toolType || 'GUI',
    execPath: tool.execPath || '',
    argsText: tool.args?.join(' ') || '',
    wikiUrl: tool.wikiUrl || '',
    // JAR 配置
    jarPath: tool.jarConfig?.jarPath || '',
    javaPath: tool.jarConfig?.javaPath || '',
    jvmArgsText: tool.jarConfig?.jvmArgs?.join(' ') || '',
    programArgsText: tool.jarConfig?.programArgs?.join(' ') || '',
    // Python 配置
    pythonEnv: tool.pythonEnv || '',
  }
  
  // 重置图标跟踪状态
  if (tool.iconUrl) {
    // 如果工具已有图标，认为是手动设置的（或之前自动获取的，但已保存）
    isManualIcon.value = true
    autoFetchedIconPath.value = null
  } else {
    // 如果没有图标，重置状态，等待自动获取
    isManualIcon.value = false
    autoFetchedIconPath.value = null
  }
  
  editingToolId.value = tool.id
  showToolModal.value = true
}

const saveTool = async () => {
  if (!currentSub.value) {
    showConfirm('提示', '请先选择一个子分类', () => {}, 'warning')
    return
  }
  if (!toolForm.value.name.trim()) {
    showConfirm('提示', '请输入工具名称', () => {}, 'warning')
    return
  }
  // JAR 类型需要验证 JAR 路径
  if (toolForm.value.toolType === 'JAR' && !toolForm.value.jarPath.trim()) {
    showConfirm('提示', '请选择 JAR 文件路径', () => {}, 'warning')
    return
  }
  // 网页类型需要验证 URL
  if (toolForm.value.toolType === '网页' && !toolForm.value.execPath.trim()) {
    showConfirm('提示', '请输入 URL 地址', () => {}, 'warning')
    return
  }
  // 网页类型验证 URL 格式
  if (toolForm.value.toolType === '网页') {
    const url = toolForm.value.execPath.trim()
    try {
      new URL(url)
    } catch {
      showConfirm('提示', '请输入有效的 URL 地址（例如：https://example.com）', () => {}, 'warning')
      return
    }
  }
  // HTML 类型需要验证文件路径
  if (toolForm.value.toolType === 'HTML' && !toolForm.value.execPath.trim()) {
    showConfirm('提示', '请选择 HTML 文件路径', () => {}, 'warning')
    return
  }
  // LNK 类型需要验证文件路径
  if (toolForm.value.toolType === 'LNK' && !toolForm.value.execPath.trim()) {
    showConfirm('提示', '请选择 LNK 快捷方式文件路径', () => {}, 'warning')
    return
  }
  const args = toolForm.value.argsText
    .split(' ')
    .map((s) => s.trim())
    .filter(Boolean)
  const targetSubId = currentSub.value.id
  
  // 处理 JAR 配置
  let jarConfig: ToolItem['jarConfig'] = undefined
  if (toolForm.value.toolType === 'JAR') {
    const jvmArgs = toolForm.value.jvmArgsText
      .split(' ')
      .map((s) => s.trim())
      .filter(Boolean)
    const programArgs = toolForm.value.programArgsText
      .split(' ')
      .map((s) => s.trim())
      .filter(Boolean)
    jarConfig = {
      jarPath: toolForm.value.jarPath.trim(),
      javaPath: toolForm.value.javaPath.trim() || undefined,
      jvmArgs: jvmArgs.length ? jvmArgs : undefined,
      programArgs: programArgs.length ? programArgs : undefined,
    }
  }
  
  // 优先使用用户自定义图标，如果没有则自动获取应用本身图标
  let finalIconUrl = toolForm.value.iconUrl.trim() || undefined
  
  // 如果用户没有手动设置图标，且工具类型支持自动获取，则尝试自动获取
  if (!finalIconUrl && toolForm.value.toolType) {
    const execPath = toolForm.value.toolType === 'JAR' 
      ? toolForm.value.jarPath.trim() 
      : toolForm.value.execPath.trim()
    
    if (execPath) {
      try {
        const autoIcon = await autoFetchIcon(toolForm.value.toolType, execPath)
        if (autoIcon) {
          finalIconUrl = autoIcon
          debug('自动获取图标成功:', { toolType: toolForm.value.toolType, execPath })
        }
      } catch (error) {
        warn('自动获取图标失败:', error)
      }
    }
  }
  
  // 如果图标是 base64 数据 URL，保存到 .config/icons/ 目录并转换为相对路径
  if (finalIconUrl && finalIconUrl.startsWith('data:image')) {
    try {
      const iconPath = await saveIconToCache(finalIconUrl)
      finalIconUrl = iconPath
      debug('图标已保存到缓存（.config/icons/ 目录）:', iconPath)
    } catch (error) {
      warn('保存图标到缓存失败，使用原始 base64:', error)
      // 如果保存失败，继续使用 base64（向后兼容）
    }
  }
  
  // 不再自动转换路径格式，保留用户设置的原始路径
  // 只有 base64 图标会被保存到缓存并转换为相对路径
  
  const base: ToolItem = {
    id: toolForm.value.id,
    name: toolForm.value.name.trim(),
    description: toolForm.value.description.trim(),
    iconUrl: finalIconUrl,
    toolType: toolForm.value.toolType || 'GUI', // 确保总是有值，默认使用 GUI
    execPath: toolForm.value.execPath || undefined,
    args: args.length ? args : undefined,
    wikiUrl: toolForm.value.wikiUrl.trim() || undefined,
    jarConfig,
  }
  
  // 调试信息
  debug('保存工具:', {
    id: base.id,
    name: base.name,
    toolType: base.toolType,
    execPath: base.execPath,
    hasIcon: !!base.iconUrl,
  })
  
  // 确保修改被 Vue 响应式系统检测到
  // 通过创建新的数组和对象引用来触发深层 watch
  if (category.value) {
    const upsertTool = (tools: ToolItem[], tool: ToolItem): ToolItem[] => {
      const seen = new Set<string>()
      const uniqueTools: ToolItem[] = []
      for (const t of tools) {
        if (seen.has(t.id)) continue
        seen.add(t.id)
        uniqueTools.push({ ...t })
      }
      const existingIndex = uniqueTools.findIndex(t => t.id === tool.id)
      if (existingIndex >= 0) {
        uniqueTools[existingIndex] = tool
        return uniqueTools
      }
      return [...uniqueTools, tool]
    }

    const categoryIndex = categoriesData.value.findIndex(c => c.id === category.value?.id)
    if (categoryIndex >= 0 && categoriesData.value[categoryIndex]) {
      const existing = categoriesData.value[categoryIndex]
      // 创建新的 subCategories 数组，确保工具数组的引用也改变
      const newSubCategories = existing.subCategories.map(sub => {
        if (sub.id === targetSubId) {
          const newTools = upsertTool(sub.tools, base)
          return {
            ...sub,
            tools: newTools,
          }
        }
        return { ...sub }
      })
      
      // 创建新的分类对象，确保所有引用都改变
      categoriesData.value[categoryIndex] = {
        id: existing.id,
        name: existing.name,
        label: existing.label,
        description: existing.description,
        subCategories: newSubCategories,
      }
      
      debug('工具已保存，已触发响应式更新，等待自动同步到配置文件...', {
        toolId: base.id,
        toolName: base.name,
        categoryId: category.value.id,
      })
      
      // 直接触发保存，确保数据立即持久化（watch 作为备用）
      try {
        const { flushSaveToolsData } = await import('../stores/categories')
        await flushSaveToolsData()
        info('✅ 工具数据已立即保存到配置文件')
      } catch (error) {
        warn('立即保存失败，将依赖 watch 自动保存:', error)
      }
    }
  }
  
  editingToolId.value = null
  showToolModal.value = false
}

const deleteTool = (id: string) => {
  if (!currentSub.value) return
  const idx = currentSub.value.tools.findIndex((t) => t.id === id)
  if (idx >= 0) {
    currentSub.value.tools.splice(idx, 1)
    // 确保修改被 Vue 响应式系统检测到
    if (category.value) {
      const categoryIndex = categoriesData.value.findIndex(c => c.id === category.value?.id)
      if (categoryIndex >= 0 && categoriesData.value[categoryIndex]) {
        const existing = categoriesData.value[categoryIndex]
        // 创建新的 subCategories 数组，确保工具数组的引用也改变
        const newSubCategories = existing.subCategories.map(sub => {
          if (sub.id === currentSub.value?.id) {
            // 创建新的 tools 数组，排除被删除的工具
            const newTools = sub.tools.filter(tool => tool.id !== id)
            return {
              ...sub,
              tools: newTools,
            }
          }
          return { ...sub }
        })
        
        // 创建新的分类对象，确保所有引用都改变
        categoriesData.value[categoryIndex] = {
          id: existing.id,
          name: existing.name,
          label: existing.label,
          description: existing.description,
          subCategories: newSubCategories,
        }
        
        debug('工具已删除，已触发响应式更新，等待自动保存到配置文件...', { 
          toolId: id, 
          categoryId: category.value.id 
        })
      }
    }
  }
}

// 选择 JAR 文件
const selectJarFile = async () => {
  const filePath = await openFileDialog(
    [{ name: 'JAR Files', extensions: ['jar'] }],
    toolForm.value.jarPath || undefined
  )
  if (filePath) {
    // 确保路径是绝对路径
    let absPath = filePath
    if (!filePath.includes(':') && !filePath.startsWith('/')) {
      // 可能是相对路径，尝试解析为绝对路径
      try {
        const invoker = getTauriInvoke()
        if (invoker) {
          const resolved = await invoker<string>('resolve_file_path', {
            params: {
              filePath: filePath,
            }
          })
          if (resolved) {
            absPath = resolved
          }
        }
      } catch (err) {
        warn('解析文件路径失败，使用原始路径:', err)
      }
    }
    
    // 清除之前的自动图标路径跟踪，强制重新获取
    autoFetchedIconPath.value = null
    toolForm.value.jarPath = absPath
    // 自动提取图标
    await autoFetchIconOnInput()
  }
}

// 选择 HTML 文件
const selectHtmlFile = async () => {
  const filePath = await openFileDialog(
    [{ name: 'HTML Files', extensions: ['html', 'htm'] }],
    toolForm.value.execPath || undefined
  )
  if (filePath) {
    // 确保路径是绝对路径
    let absPath = filePath
    if (!filePath.includes(':') && !filePath.startsWith('/')) {
      // 可能是相对路径，尝试解析为绝对路径
      try {
        const invoker = getTauriInvoke()
        if (invoker) {
          const resolved = await invoker<string>('resolve_file_path', {
            params: {
              filePath: filePath,
            }
          })
          if (resolved) {
            absPath = resolved
          }
        }
      } catch (err) {
        warn('解析文件路径失败，使用原始路径:', err)
      }
    }
    
    // 清除之前的自动图标路径跟踪，强制重新获取
    autoFetchedIconPath.value = null
    toolForm.value.execPath = absPath
    // 自动提取图标
    await autoFetchIconOnInput()
  }
}


// 选择 LNK 文件
const selectLnkFile = async () => {
  const filePath = await openFileDialog(
    [{ name: 'Shortcut Files', extensions: ['lnk'] }],
    toolForm.value.execPath || undefined
  )
  if (filePath) {
    // 清除之前的自动图标路径跟踪，强制重新获取
    autoFetchedIconPath.value = null
    toolForm.value.execPath = filePath
    // 自动提取图标
    await autoFetchIconOnInput()
  }
}


// 自动提取图标（在输入时触发）
const isFetchingIcon = ref(false)
// 跟踪自动获取图标对应的路径，用于检测路径变更
const autoFetchedIconPath = ref<string | null>(null)
// 跟踪用户是否手动设置了图标
const isManualIcon = ref(false)

// 监听工具表单的 execPath 和 jarPath 变化，自动清除图标并重新获取
// 注意：这个 watch 必须在 toolForm、autoFetchedIconPath 和 isManualIcon 定义之后
watch(
  () => [toolForm.value.execPath, toolForm.value.jarPath, toolForm.value.toolType],
  ([newExecPath, newJarPath, newToolType], [oldExecPath, oldJarPath, oldToolType]) => {
    // 如果路径改变了，清除之前的自动图标
    const currentPath = newToolType === 'JAR' ? newJarPath : newExecPath
    const oldPath = oldToolType === 'JAR' ? oldJarPath : oldExecPath
    
    // 如果路径或工具类型改变了，清除图标并重新获取
    if (currentPath !== oldPath || newToolType !== oldToolType) {
      // 只有在自动获取的图标时才清除
      if (!isManualIcon.value) {
        toolForm.value.iconUrl = ''
        autoFetchedIconPath.value = null
        
        // 如果新路径存在且不为空，延迟重新获取图标（避免频繁调用）
        if (currentPath && currentPath.trim() && newToolType) {
          // 使用 nextTick 确保路径已更新
          setTimeout(() => {
            autoFetchIconOnInput()
          }, 300) // 300ms 防抖，避免频繁调用
        }
      }
    }
  },
  { deep: true }
)

const autoFetchIconOnInput = async () => {
  // 如果用户已经手动设置了图标，不自动提取
  if (isManualIcon.value || toolForm.value.iconUrl.trim()) {
    return
  }
  
  // 如果正在提取，跳过
  if (isFetchingIcon.value) {
    return
  }
  
  // 确定执行路径
  const rawPath = toolForm.value.toolType === 'JAR' 
    ? toolForm.value.jarPath.trim() 
    : toolForm.value.execPath.trim()
  let execPath = rawPath.trim()
  if ((execPath.startsWith('`') && execPath.endsWith('`')) || (execPath.startsWith('"') && execPath.endsWith('"')) || (execPath.startsWith("'") && execPath.endsWith("'"))) {
    execPath = execPath.slice(1, -1).trim()
  }
  if (execPath.endsWith(':') && !/:\d+$/.test(execPath)) {
    execPath = execPath.slice(0, -1).trim()
  }
  
  // 如果没有路径或工具类型，清除之前的自动图标
  if (!execPath || !toolForm.value.toolType) {
    if (autoFetchedIconPath.value !== null) {
      toolForm.value.iconUrl = ''
      autoFetchedIconPath.value = null
    }
    return
  }
  
  // 如果路径改变了，清除之前的自动图标（所有文件类型都支持）
  if (autoFetchedIconPath.value !== null && autoFetchedIconPath.value !== execPath) {
    toolForm.value.iconUrl = ''
    autoFetchedIconPath.value = null
  }
  
  // 如果工具类型改变了，也清除之前的自动图标
  const currentToolType = toolForm.value.toolType
  if (autoFetchedIconPath.value !== null && currentToolType) {
    // 根据路径自动检测应该使用的工具类型
    const detectedType = detectFileTypeFromPath(execPath)
    if (detectedType !== currentToolType && currentToolType !== '其他') {
      // 如果检测到的类型与当前类型不匹配，清除图标
      toolForm.value.iconUrl = ''
      autoFetchedIconPath.value = null
    }
  }
  
  const isHttpUrl = execPath.startsWith('http://') || execPath.startsWith('https://')

  // 验证路径格式（对于网页类型，或 URL 误填到其他类型的情况）
  if (toolForm.value.toolType === '网页' || isHttpUrl) {
    try {
      const u = new URL(execPath)
      if (!u.hostname || u.hostname.endsWith('.') || !u.hostname.includes('.')) {
        return
      }
    } catch {
      // URL 格式无效，清除之前的自动图标
      if (autoFetchedIconPath.value !== null) {
        toolForm.value.iconUrl = ''
        autoFetchedIconPath.value = null
      }
      return
    }
  }
  
  isFetchingIcon.value = true
  try {
    const effectiveToolType = isHttpUrl ? '网页' : toolForm.value.toolType
    debug('开始自动获取图标:', { toolType: effectiveToolType, execPath })
    const autoIcon = await autoFetchIcon(effectiveToolType, execPath)
    if (autoIcon) {
      toolForm.value.iconUrl = autoIcon
      autoFetchedIconPath.value = execPath
      isManualIcon.value = false
      debug('自动获取图标成功（输入时）:', { toolType: effectiveToolType, execPath, iconLength: autoIcon.length })
    } else {
      warn('自动获取图标返回 null:', { toolType: effectiveToolType, execPath })
    }
  } catch (error) {
    const effectiveToolType = isHttpUrl ? '网页' : toolForm.value.toolType
    logError('自动获取图标失败（输入时）:', error, { toolType: effectiveToolType, execPath })
  } finally {
    isFetchingIcon.value = false
  }
}

// 防抖函数，路径改变时立即清除图标，然后延迟重新获取
let debounceTimer: ReturnType<typeof setTimeout> | null = null
const debouncedAutoFetchIcon = () => {
  // 确定当前路径
  const currentPath = toolForm.value.toolType === 'JAR' 
    ? toolForm.value.jarPath.trim() 
    : toolForm.value.execPath.trim()
  
  // 如果路径改变了，立即清除图标（显示无预览状态）
  if (autoFetchedIconPath.value !== null && autoFetchedIconPath.value !== currentPath) {
    if (!isManualIcon.value) {
      toolForm.value.iconUrl = ''
      autoFetchedIconPath.value = null
    }
  }
  
  // 清除之前的定时器
  if (debounceTimer) {
    clearTimeout(debounceTimer)
  }
  
  // 延迟重新获取图标（如果路径存在）
  debounceTimer = setTimeout(() => {
    autoFetchIconOnInput()
  }, 500) // 500ms 防抖
}

// 处理图标 URL 手动输入
const handleIconUrlInput = () => {
  // 如果用户手动输入了图标 URL，标记为手动设置
  if (toolForm.value.iconUrl.trim()) {
    isManualIcon.value = true
    autoFetchedIconPath.value = null
  } else {
    // 如果清空了图标 URL，重置状态，允许自动获取
    isManualIcon.value = false
    autoFetchedIconPath.value = null
    // 触发自动获取（如果路径存在）
    debouncedAutoFetchIcon()
  }
}

// 选择本地图片并处理
const selectLocalImage = async () => {
  try {
    const file = await selectImageFile()
    if (!file) return
    
    // 处理图片（裁剪、压缩）
    const processedImage = await processImage(file, 160, 0.9)
    
    // 更新表单（保存时会自动保存到 .config/icons/ 目录）
    toolForm.value.iconUrl = processedImage
    debug('用户上传自定义图标，已处理为 base64，保存时将保存到 .config/icons/ 目录')
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : '图片处理失败'
    showConfirm('错误', `图片处理失败：${errorMessage}`, () => {}, 'warning')
  }
}

// 选择可执行文件
const selectExecutableFile = async () => {
  const filePath = await openFileDialog(
    [{ name: 'Executable Files', extensions: ['exe', 'bat', 'cmd', 'ps1', 'sh', 'py', 'rb', 'pl'] }],
    toolForm.value.execPath || undefined
  )
  if (filePath) {
    // 确保路径是绝对路径
    let absPath = filePath
    if (!filePath.includes(':') && !filePath.startsWith('/')) {
      // 可能是相对路径，尝试解析为绝对路径
      try {
        const invoker = getTauriInvoke()
        if (invoker) {
          const resolved = await invoker<string>('resolve_file_path', {
            params: {
              filePath: filePath,
            }
          })
          if (resolved) {
            absPath = resolved
          }
        }
      } catch (err) {
        warn('解析文件路径失败，使用原始路径:', err)
      }
    }
    
    // 清除之前的自动图标路径跟踪，强制重新获取
    autoFetchedIconPath.value = null
    toolForm.value.execPath = absPath
    // 自动提取图标
    await autoFetchIconOnInput()
  }
}

// 从可执行文件提取图标


// 处理图标图片加载失败
const handleIconError = (event: Event) => {
  const img = event.target as HTMLImageElement
  if (img) {
    img.style.display = 'none'
    // 显示默认图标作为后备
    const card = img.closest('.tool-card')
    if (card) {
      const defaultDiv = card.querySelector('.tool-icon-default') as HTMLElement
      if (defaultDiv) {
        defaultDiv.style.display = 'flex'
      }
    }
  }
}

// 处理预览图片加载失败
const handlePreviewError = (event: Event) => {
  const img = event.target as HTMLImageElement
  if (img) {
    img.style.display = 'none'
    // 显示错误提示
    const preview = img.closest('.icon-preview')
    if (preview && !preview.querySelector('.icon-preview-error')) {
      const errorDiv = document.createElement('div')
      errorDiv.className = 'icon-preview-error'
      errorDiv.textContent = '⚠️ 图片加载失败，请检查 URL 是否正确'
      preview.appendChild(errorDiv)
    }
  }
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
    {
      label: '在 Wiki 中查看',
      icon: '📚',
      action: () => onOpenToolWiki(tool),
    },
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
    target.closest('.app-header') ||
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
  <AppLayout 
    :title="category?.name" 
    :subtitle="category?.label ? `${category.label} · ${category.description || ''}` : category?.description"
    :show-back="true"
  >
    <template #header-actions>
      <button type="button" class="icon-button" @click="openWikiHome">
        <span class="icon">📚</span>
        <span class="icon-label">Wiki</span>
      </button>
      <button type="button" class="icon-button" @click="goSettings">
        <span class="icon">⚙</span>
        <span class="icon-label">设置</span>
      </button>
    </template>

    <template #sidebar>
      <div class="sub-list" @contextmenu="showBlankMenu">
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
      </div>
    </template>

    <div class="category-content" @contextmenu="showBlankMenu">
      <div class="search-row">
        <div class="search-box">
          <span class="search-icon">🔍</span>
          <input
            v-model="searchQuery"
            class="search-input"
            type="search"
            placeholder="搜索当前子分类的工具名称或描述（↑↓ 选择，Enter 打开，Esc 清除）"
            @keydown="handleSearchInputKeydown"
            @input="handleSearchInput"
          />
        </div>
      </div>

      <section class="tools-area">
        <div class="tools-header">
            <div>
              <h2>{{ currentSub?.name || category?.label || category?.name || '分类工具' }}</h2>
              <p>{{ currentSub?.description || category?.description || '选择一个子分类以查看工具，或直接添加工具' }}</p>
            </div>
            <div class="tools-header-actions">
              <!-- 视图切换按钮（仅在选中子分类时显示） -->
              <template v-if="currentSub">
                <button
                  type="button"
                  class="icon-button"
                  :class="{ active: viewMode === 'grid' }"
                  @click="viewMode = 'grid'"
                  title="网格视图"
                >
                  <span class="icon">⊞</span>
                  <span class="icon-label">网格</span>
                </button>
                <button
                  type="button"
                  class="icon-button"
                  :class="{ active: viewMode === 'list' }"
                  @click="viewMode = 'list'"
                  title="列表视图"
                >
                  <span class="icon">☰</span>
                  <span class="icon-label">列表</span>
                </button>
              </template>
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

          <SearchOverlay
            v-if="searchQuery && overlayResults.length"
            :results="overlayResults"
            v-model:selected-index="selectedSearchIndex"
            @select="onResultClick"
            @close="searchQuery = ''; selectedSearchIndex = -1"
          />

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
          <div v-else-if="currentSub" class="tools-wrapper" :class="viewMode">
            <!-- 网格视图 -->
            <template v-if="viewMode === 'grid'">
              <VirtualList
                v-if="shouldUseVirtualScroll"
                :items="filteredTools"
                :item-height="150"
                :container-height="600"
                class="virtual-tools-list"
              >
                <template #default="{ item: tool }">
                  <div
                    class="tool-card"
                    @contextmenu="showToolMenu($event, tool as ToolItem)"
                  >
                  <div class="tool-icon-wrapper">
                    <img
                      v-if="getToolIconUrl(tool as ToolItem) && !getToolIconUrl(tool as ToolItem)?.startsWith('file://') && !getToolIconUrl(tool as ToolItem)?.startsWith('icons/') && !getToolIconUrl(tool as ToolItem)?.startsWith('.config/icons/')"
                      :src="getToolIconUrl(tool as ToolItem)"
                      :alt="(tool as ToolItem).name"
                      class="tool-icon-img"
                      @error="handleIconError($event)"
                    />
                    <div v-else class="tool-icon-default">🛠️</div>
                  </div>
                    <div class="tool-content">
                      <div class="tool-name">{{ (tool as ToolItem).name }}</div>
                      <p v-if="(tool as ToolItem).description" class="tool-desc">{{ (tool as ToolItem).description }}</p>
                      <div class="tool-actions">
                        <button type="button" class="btn ghost small" @click="onOpenToolWiki(tool as ToolItem)">📚 Wiki</button>
                        <button type="button" class="btn primary small" @click="openTool((tool as ToolItem).id)">打开</button>
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
                >
                  <div class="tool-icon-wrapper">
                    <img
                      v-if="getToolIconUrl(tool) && !getToolIconUrl(tool)?.startsWith('file://') && !getToolIconUrl(tool)?.startsWith('icons/') && !getToolIconUrl(tool)?.startsWith('.config/icons/')"
                      :src="getToolIconUrl(tool)"
                      :alt="tool.name"
                      class="tool-icon-img"
                      @error="handleIconError($event)"
                    />
                    <div v-else class="tool-icon-default">🛠️</div>
                  </div>
                  <div class="tool-content">
                    <div class="tool-name">{{ tool.name }}</div>
                    <p v-if="tool.description" class="tool-desc">{{ tool.description }}</p>
                    <div class="tool-actions">
                      <button type="button" class="btn ghost small" @click="onOpenToolWiki(tool)">📚 Wiki</button>
                      <button type="button" class="btn primary small" @click="openTool(tool.id)">打开</button>
                    </div>
                  </div>
                </div>
              </div>
            </template>
            
            <!-- 列表视图 -->
            <template v-else>
              <VirtualList
                v-if="shouldUseVirtualScroll"
                :items="filteredTools"
                :item-height="60"
                :container-height="600"
                class="virtual-tools-list list-view"
              >
                <template #default="{ item: tool }">
                  <div
                    class="tool-card-list"
                    @contextmenu="showToolMenu($event, tool as ToolItem)"
                  >
                    <div class="tool-icon-wrapper-list">
                      <img
                        v-if="getToolIconUrl(tool as ToolItem) && !getToolIconUrl(tool as ToolItem)?.startsWith('file://') && !getToolIconUrl(tool as ToolItem)?.startsWith('icons/') && !getToolIconUrl(tool as ToolItem)?.startsWith('.config/icons/')"
                        :src="getToolIconUrl(tool as ToolItem)"
                        :alt="(tool as ToolItem).name"
                        class="tool-icon-img"
                        @error="handleIconError($event)"
                      />
                      <div v-else class="tool-icon-default">🛠️</div>
                    </div>
                    <div class="tool-content-list">
                      <div class="tool-name-list">{{ (tool as ToolItem).name }}</div>
                      <p v-if="(tool as ToolItem).description" class="tool-desc-list">{{ (tool as ToolItem).description }}</p>
                    </div>
                    <div class="tool-actions-list">
                      <button type="button" class="btn ghost small" @click="onOpenToolWiki(tool as ToolItem)">📚 Wiki</button>
                      <button type="button" class="btn primary small" @click="openTool((tool as ToolItem).id)">打开</button>
                    </div>
                  </div>
                </template>
              </VirtualList>
              <div v-else class="tools-list">
                <div
                  v-for="tool in filteredTools"
                  :key="tool.id"
                  class="tool-card-list"
                  @contextmenu="showToolMenu($event, tool)"
                >
                  <div class="tool-icon-wrapper-list">
                    <img
                      v-if="getToolIconUrl(tool) && !getToolIconUrl(tool)?.startsWith('file://') && !getToolIconUrl(tool)?.startsWith('icons/') && !getToolIconUrl(tool)?.startsWith('.config/icons/')"
                      :src="getToolIconUrl(tool)"
                      :alt="tool.name"
                      class="tool-icon-img"
                      @error="handleIconError($event)"
                    />
                    <div v-else class="tool-icon-default">🛠️</div>
                  </div>
                  <div class="tool-content-list">
                    <div class="tool-name-list">{{ tool.name }}</div>
                    <p v-if="tool.description" class="tool-desc-list">{{ tool.description }}</p>
                  </div>
                  <div class="tool-actions-list">
                    <button type="button" class="btn ghost small" @click="onOpenToolWiki(tool)">📚 Wiki</button>
                    <button type="button" class="btn primary small" @click="openTool(tool.id)">打开</button>
                  </div>
                </div>
              </div>
            </template>
          </div>
          <div v-else-if="!currentSub && subCategories.length > 0" class="empty-state">
            <div class="empty-icon">👈</div>
            <h3>请选择子分类</h3>
            <p>从左侧选择一个子分类以查看工具</p>
          </div>
        </section>
    </div>
    
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
            <span class="field-label">自定义图标（可选）</span>
            <div class="field-with-button">
              <input
                v-model="toolForm.iconUrl"
                class="field-input"
                placeholder="图标将自动从应用本身获取，或在此设置自定义图标 URL"
                type="url"
                @input="handleIconUrlInput"
              />
              <button
                type="button"
                class="btn secondary file-select-btn"
                @click="selectLocalImage"
              >
                选择本地图片
              </button>
            </div>
            <span class="field-hint">留空则自动从应用本身获取图标。支持 URL 或本地图片（将自动裁剪为 160x160）</span>
          </label>
          <div v-if="toolForm.iconUrl" class="icon-preview">
            <img :src="toolForm.iconUrl" alt="图标预览" class="icon-preview-img" @error="handlePreviewError" />
          </div>
          <div v-else class="icon-preview">
            <div class="icon-preview-placeholder">
              <span class="icon-preview-text">图标预览</span>
              <span class="icon-preview-hint" v-if="isFetchingIcon">正在获取图标...</span>
              <span class="icon-preview-hint" v-else>输入路径后将自动从应用本身获取图标</span>
            </div>
          </div>
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
          <label class="field">
            <span class="field-label">工具类型</span>
            <select v-model="toolForm.toolType" class="field-input" @change="debouncedAutoFetchIcon">
              <option value="GUI">GUI（图形界面）</option>
              <option value="CLI">CLI（命令行）</option>
              <option value="JAR">JAR（Java应用）</option>
              <option value="Python">Python（Python脚本）</option>
              <option value="网页">网页（在线工具）</option>
              <option value="HTML">HTML（本地网页）</option>
              <option value="LNK">LNK（Windows快捷方式）</option>
              <option value="其他">其他</option>
            </select>
          </label>
        
        <!-- JAR 类型工具的专门配置面板 -->
        <div v-if="toolForm.toolType === 'JAR'" class="jar-config-panel">
          <div class="jar-config-header">
            <span class="jar-config-title">Java JAR 配置</span>
          </div>
          <label class="field">
            <span class="field-label">JAR 路径</span>
            <div class="field-with-button">
              <input
                v-model="toolForm.jarPath"
                class="field-input"
                placeholder="输入路径或选择文件"
                @input="debouncedAutoFetchIcon"
              />
              <button
                type="button"
                class="btn secondary file-select-btn"
                @click="selectJarFile"
              >
                选择文件
              </button>
            </div>
            <span class="field-hint">输入文件路径或点击"选择文件"按钮选择本地文件</span>
          </label>
          <label class="field">
            <span class="field-label">Java 路径（可选，留空使用 PATH）</span>
            <input
              v-model="toolForm.javaPath"
              class="field-input"
              placeholder="例如：C:\\Program Files\\Java\\jdk-17\\bin\\java.exe"
            />
          </label>
          <label class="field">
            <span class="field-label">JVM 参数（空格分隔）</span>
            <input
              v-model="toolForm.jvmArgsText"
              class="field-input"
              placeholder="例如：-Xmx512m -Dfile.encoding=UTF-8"
            />
            <span class="field-hint">JVM 参数，如 -Xmx、-Dxxx 等</span>
          </label>
          <label class="field">
            <span class="field-label">程序参数（空格分隔）</span>
            <input
              v-model="toolForm.programArgsText"
              class="field-input"
              placeholder="例如：--host 127.0.0.1 --port 8080"
            />
            <span class="field-hint">传递给 Java 程序的普通参数</span>
          </label>
        </div>
        
        <!-- 网页类型工具的配置 -->
        <div v-else-if="toolForm.toolType === '网页'" class="web-config-panel">
          <label class="field">
            <span class="field-label">URL 地址</span>
            <input
              v-model="toolForm.execPath"
              class="field-input"
              placeholder="https://example.com/tool"
              type="url"
              @input="debouncedAutoFetchIcon"
            />
            <span class="field-hint">在线工具的完整 URL 地址</span>
          </label>
        </div>
        
        <!-- HTML 类型工具的配置 -->
        <div v-else-if="toolForm.toolType === 'HTML'" class="html-config-panel">
          <label class="field">
            <span class="field-label">HTML 文件路径</span>
            <div class="field-with-button">
              <input
                v-model="toolForm.execPath"
                class="field-input"
                placeholder="输入路径或选择文件"
                @input="debouncedAutoFetchIcon"
              />
              <button
                type="button"
                class="btn secondary file-select-btn"
                @click="selectHtmlFile"
              >
                选择文件
              </button>
            </div>
            <span class="field-hint">输入文件路径或点击"选择文件"按钮选择本地文件</span>
          </label>
        </div>
        
        <!-- LNK 类型工具的配置 -->
        <div v-else-if="toolForm.toolType === 'LNK'" class="lnk-config-panel">
          <label class="field">
            <span class="field-label">LNK 快捷方式路径</span>
            <div class="field-with-button">
              <input
                v-model="toolForm.execPath"
                class="field-input"
                placeholder="输入路径或选择文件"
                @input="debouncedAutoFetchIcon"
              />
              <button
                type="button"
                class="btn secondary file-select-btn"
                @click="selectLnkFile"
              >
                选择文件
              </button>
            </div>
            <span class="field-hint">输入文件路径或点击"选择文件"按钮选择本地文件</span>
          </label>
        </div>
        
        <!-- 其他类型的通用配置 -->
        <div v-else class="tool-form-grid">
          <label class="field">
            <span class="field-label">可执行路径</span>
            <div class="field-with-button">
              <input 
                v-model="toolForm.execPath" 
                class="field-input" 
                placeholder="输入路径或选择文件"
                @input="debouncedAutoFetchIcon"
              />
              <button
                type="button"
                class="btn secondary file-select-btn"
                @click="selectExecutableFile"
                title="选择文件并自动设置路径"
              >
                选择文件
              </button>
            </div>
            <span class="field-hint">输入文件路径或点击"选择文件"按钮选择本地文件</span>
          </label>
          <label class="field">
            <span class="field-label">参数(空格分隔)</span>
            <input v-model="toolForm.argsText" class="field-input" placeholder="-d example.com -v" />
          </label>
        </div>
        <label class="field">
          <span class="field-label">Wiki 文件路径（可选）</span>
          <input
            v-model="toolForm.wikiUrl"
            class="field-input"
            placeholder="例如：tools/test-case-1.md 或留空自动查找"
          />
          <span class="field-hint">输入 Wiki 文件的相对路径（相对于 wiki 目录，如 tools/test-case-1.md），或留空让系统根据工具名称自动查找。注意：不要包含 wiki\ 或 wiki/ 前缀</span>
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

    <!-- 工具 Wiki 侧边栏 -->
    <ToolWikiPanel
      v-if="showWikiPanel && currentWikiTool"
      v-model:visible="showWikiPanel"
      :title="currentWikiTool.name"
      :content="wikiContent"
      :loading="wikiLoading"
      :error="wikiError"
    />
  </AppLayout>
</template>

<style scoped>
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
  width: 32px;
  height: 32px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-tertiary);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.back-button:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--text-secondary);
}

.title-block h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.title-block p {
  margin: 2px 0 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.icon-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-tertiary);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s ease;
}

.icon-button:hover:not(:disabled) {
  background-color: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--text-secondary);
}

.icon-button.active {
  background-color: var(--bg-active);
  border-color: var(--primary-color);
  color: var(--primary-color);
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

/* .page-main removed */

.category-content {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.search-row {
  display: flex;
  justify-content: center;
  padding-bottom: 16px;
  flex: 0 0 auto;
}

.search-box {
  position: relative;
  width: 100%;
  max-width: 600px;
  margin: 0 auto;
}

.search-icon {
  position: absolute;
  left: 12px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 16px;
  color: var(--text-tertiary);
  pointer-events: none;
  transition: color 0.2s ease;
  z-index: 1;
}

.search-box:focus-within .search-icon {
  color: var(--primary-color);
}

.search-input {
  width: 100%;
  padding: 10px 12px 10px 40px;
  border-radius: 8px;
  border: 1px solid var(--input-border);
  background-color: var(--input-bg);
  color: var(--text-primary);
  font-size: 14px;
  outline: none;
  transition: all 0.2s ease;
}

.search-input::placeholder {
  color: var(--text-tertiary);
}

.search-input:focus {
  border-color: var(--primary-color);
  box-shadow: 0 0 0 2px var(--primary-color-alpha);
  background-color: var(--bg-primary);
}

/* .content-row removed */

.sub-list {
  /* flex: 0 0 240px; removed - handled by AppLayout sidebar */
  /* border-right: ... removed */
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  height: 100%;
  overflow: hidden;
}

.sub-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  flex: 0 0 auto;
}

.sub-cards {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding-right: 4px;
}

.sub-card {
  text-align: left;
  padding: 10px 12px;
  border-radius: 6px;
  border: 1px solid transparent;
  background-color: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s ease;
}

.sub-card:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.sub-card.active {
  background-color: var(--bg-active);
  color: var(--text-primary);
  border-color: var(--border-color);
  font-weight: 500;
}

.sub-name {
  font-size: 14px;
}

.sub-desc {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 2px;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.sub-card.add-sub-card {
  border: 1px dashed var(--border-color);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-tertiary);
}

.sub-card.add-sub-card:hover {
  border-color: var(--primary-color);
  color: var(--primary-color);
  background-color: var(--bg-hover);
}

.add-sub-name {
  font-size: 13px;
}

.tools-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 16px;
  position: relative;
  min-height: 0;
  height: 100%;
  overflow: hidden;
}

.tools-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-color);
  flex: 0 0 auto;
}

.tools-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary);
}

.tools-header p {
  margin: 4px 0 0;
  color: var(--text-secondary);
  font-size: 13px;
}

.tools-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tools-wrapper {
  flex: 1;
  min-height: 0;
  padding-right: 4px;
  overflow-y: auto;
  overflow-x: hidden;
}

.tools-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 16px;
  padding-bottom: 16px;
}

.virtual-tools-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 16px;
  padding-bottom: 16px;
}

.tool-card {
  padding: 16px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-secondary);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  transition: all 0.2s ease;
  cursor: pointer;
  position: relative;
}

.tool-card:hover {
  transform: translateY(-2px);
  border-color: var(--text-secondary);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.tool-icon-wrapper {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  background-color: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  flex-shrink: 0;
  border: 1px solid var(--border-color);
}

.tool-icon-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.tool-icon-default {
  font-size: 28px;
}

.tool-content {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  text-align: center;
}

.tool-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}

.tool-desc {
  margin: 0;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  height: 36px;
}

.tool-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  margin-top: 4px;
}

.btn.small {
  padding: 4px 10px;
  font-size: 12px;
  min-width: auto;
}

/* List View Styles */
.virtual-tools-list.list-view,
.tools-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-bottom: 16px;
}

.tool-card-list {
  padding: 12px 16px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-secondary);
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 16px;
  transition: all 0.2s ease;
  cursor: pointer;
}

.tool-card-list:hover {
  background-color: var(--bg-hover);
  border-color: var(--text-secondary);
}

.tool-icon-wrapper-list {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  background-color: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  flex-shrink: 0;
  border: 1px solid var(--border-color);
}

.tool-icon-wrapper-list .tool-icon-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.tool-icon-wrapper-list .tool-icon-default {
  font-size: 20px;
}

.tool-content-list {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  text-align: left;
}

.tool-name-list {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.tool-desc-list {
  margin: 0;
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tool-actions-list {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.search-overlay {
  position: absolute;
  top: 60px;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  z-index: 20;
}

.overlay-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-tertiary);
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
  background-color: transparent;
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
  transition: all 0.2s ease;
}

.overlay-item:hover,
.overlay-item.selected {
  background-color: var(--bg-hover);
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

.overlay-text {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.overlay-name {
  font-size: 14px;
  font-weight: 500;
}

.overlay-desc {
  font-size: 12px;
  color: var(--text-secondary);
}

.overlay-action {
  font-size: 12px;
  color: var(--primary-color);
  opacity: 0;
  transition: opacity 0.2s ease;
}

.overlay-item:hover .overlay-action,
.overlay-item.selected .overlay-action {
  opacity: 1;
}

.btn {
  border-radius: 6px;
  border: 1px solid transparent;
  padding: 6px 12px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn.ghost {
  background-color: transparent;
  border-color: var(--border-color);
  color: var(--text-primary);
}

.btn.ghost:hover {
  background-color: var(--bg-hover);
  border-color: var(--text-secondary);
}

.btn.danger {
  border-color: var(--danger-color);
  color: var(--danger-color);
  background-color: transparent;
}

.btn.danger:hover {
  background-color: var(--danger-color);
  color: #ffffff;
}

.btn.primary {
  background-color: var(--primary-color);
  color: #ffffff;
}

.btn.primary:hover {
  background-color: var(--primary-hover);
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
  gap: 16px;
}

.jar-config-panel,
.web-config-panel,
.html-config-panel,
.lnk-config-panel {
  margin-top: 8px;
  padding: 16px;
  background-color: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
}

.jar-config-header {
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.jar-config-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
}

.field-input,
.field-textarea {
  border-radius: 6px;
  border: 1px solid var(--input-border);
  background-color: var(--input-bg);
  color: var(--text-primary);
  padding: 8px 10px;
  font-size: 13px;
  outline: none;
  transition: all 0.2s ease;
}

.field-input:focus,
.field-textarea:focus {
  border-color: var(--primary-color);
  box-shadow: 0 0 0 2px var(--primary-color-alpha);
}

.field-with-button {
  display: flex;
  gap: 8px;
}

.field-with-button .field-input {
  flex: 1;
}

.file-select-btn {
  white-space: nowrap;
}

.field-hint {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 4px;
}

.icon-preview {
  margin-top: 8px;
  padding: 12px;
  background-color: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  display: flex;
  justify-content: center;
}

.icon-preview-img {
  width: 64px;
  height: 64px;
  object-fit: contain;
  border-radius: 8px;
}

.icon-preview-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  color: var(--text-tertiary);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 24px;
  text-align: center;
  gap: 16px;
  height: 100%;
}

.empty-icon {
  font-size: 48px;
  opacity: 0.5;
  margin-bottom: 8px;
}

.empty-state h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.empty-state p {
  margin: 0;
  font-size: 14px;
  color: var(--text-secondary);
  max-width: 400px;
}

/* Media query removed - handled by AppLayout */
</style>
