<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted } from 'vue'
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
  type ToolType,
} from '../stores/categories'
import { openFileDialog } from '../utils/fileDialog'
import { selectImageFile, processImage, autoFetchIcon, detectFileTypeFromPath } from '../utils/imageProcessor'
import { getTauriInvoke } from '../utils/tauri'
import { launchTool } from '../utils/toolLauncher'
import { saveIconToCache } from '../utils/fileStorage'
import { getIconUrl } from '../utils/iconLoader'
import { debug, error as logError, info, warn } from '../utils/logger'
import WikiModal from '../components/WikiModal.vue'
import ToolWikiPanel from '../components/ToolWikiPanel.vue'

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
            tool.iconUrl = autoIcon
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

// 模糊搜索：支持多关键词匹配
const filteredTools = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return tools.value
  
  // 将查询字符串拆分为关键词
  const keywords = q.split(/\s+/).filter((k) => k.length > 0)
  
  return tools.value.filter((t) => {
    // 对每个关键词进行匹配
    return keywords.every((keyword) => {
      const nameMatch = t.name.toLowerCase().includes(keyword)
      const descMatch = t.description?.toLowerCase().includes(keyword) ?? false
      return nameMatch || descMatch
    })
  })
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

// Wiki 模态框状态（用于右上角 Wiki 按钮）
const wikiModalVisible = ref(false)
const wikiModalFilePath = ref<string | undefined>(undefined)
const wikiModalToolId = ref<string | undefined>(undefined)
const wikiModalToolName = ref<string | undefined>(undefined)

// 保存 Wiki 模态框状态到 sessionStorage
const saveWikiModalState = () => {
  try {
    const state = {
      visible: wikiModalVisible.value,
      filePath: wikiModalFilePath.value,
      toolId: wikiModalToolId.value,
      toolName: wikiModalToolName.value,
    }
    sessionStorage.setItem('wiki-modal-state', JSON.stringify(state))
  } catch (err) {
    warn('保存 Wiki 模态框状态失败:', err)
  }
}

// 从 sessionStorage 恢复 Wiki 模态框状态
const restoreWikiModalState = async () => {
  try {
    const saved = sessionStorage.getItem('wiki-modal-state')
    if (saved) {
      const state = JSON.parse(saved)
      if (state.visible) {
        wikiModalFilePath.value = state.filePath
        wikiModalToolId.value = state.toolId
        wikiModalToolName.value = state.toolName
        wikiModalVisible.value = true
        debug('已恢复 Wiki 模态框状态:', state)
      }
    }
  } catch (err) {
    warn('恢复 Wiki 模态框状态失败:', err)
  }
}

// 清除 Wiki 模态框状态
const clearWikiModalState = () => {
  try {
    sessionStorage.removeItem('wiki-modal-state')
  } catch (err) {
    warn('清除 Wiki 模态框状态失败:', err)
  }
}

// 监听模态框可见性变化，保存状态
watch(wikiModalVisible, (visible) => {
  if (visible) {
    saveWikiModalState()
  } else {
    clearWikiModalState()
  }
})

// 监听模态框参数变化，保存状态
watch([wikiModalFilePath, wikiModalToolId, wikiModalToolName], () => {
  if (wikiModalVisible.value) {
    saveWikiModalState()
  }
})

// 组件挂载时恢复 Wiki 模态框状态
onMounted(async () => {
  await restoreWikiModalState()
})

const openWiki = async (wikiUrl?: string, toolId?: string, toolName?: string) => {
  // 调试代码已注释
  // console.log('========== openWiki 开始 ==========')
  // console.log('openWiki 被调用:', { wikiUrl, toolId, toolName })
  debug('openWiki 被调用')
  const invoker = getTauriInvoke()
  // console.log('Tauri invoker 状态:', { hasInvoker: !!invoker })
  debug('Tauri invoker 状态:', { hasInvoker: !!invoker })
  
  try {
    // 解析文件路径（如果提供了 wikiUrl）
    let filePath: string | undefined = undefined
    if (wikiUrl) {
      debug('处理 wikiUrl:', wikiUrl)
      // 如果是 HTTP URL，提取路径（兼容旧配置）
      if (wikiUrl.startsWith('http://') || wikiUrl.startsWith('https://')) {
        const url = new URL(wikiUrl)
        if (url.pathname.startsWith('/file/')) {
          filePath = url.pathname.substring(6) // 移除 '/file/'
        } else {
          // 如果是普通 HTTP URL，尝试提取路径部分
          filePath = url.pathname.startsWith('/') ? url.pathname.substring(1) : url.pathname
        }
      } else {
        // 直接使用提供的相对路径（如 tools/tool-name.md）
        // 规范化路径：将反斜杠转换为正斜杠，移除 wiki\ 或 wiki/ 前缀
        let normalizedPath = wikiUrl.trim().replace(/\\/g, '/')
        // 移除开头的 wiki/ 或 wiki\ 前缀（如果存在）
        if (normalizedPath.toLowerCase().startsWith('wiki/')) {
          normalizedPath = normalizedPath.substring(5)
        }
        filePath = normalizedPath
        debug('规范化后的 Wiki 路径:', { original: wikiUrl, normalized: filePath })
      }
    }
    
    // 如果没有提供文件路径，尝试根据工具 ID 或名称自动查找
    if (!filePath && (toolId || toolName) && invoker) {
      debug('尝试自动查找 Wiki 文件:', { toolId, toolName })
      try {
        const found = await invoker('find_wiki_for_tool', {
          tool_id: toolId || '',
          tool_name: toolName || undefined,
        }) as { path: string } | null
        if (found && found.path) {
          filePath = found.path
          debug('自动查找到 Wiki 文件:', filePath)
        } else {
          debug('未找到匹配的 Wiki 文件')
        }
      } catch (err) {
        debug('查找 Wiki 文件失败:', err)
        // 如果查找失败，继续打开模态框（显示首页）
      }
    }
    
    // console.log('========== openWiki 准备打开模态框 ==========')
    // console.log('打开 Wiki 模态框:', { 
    //   filePath, 
    //   toolId, 
    //   toolName,
    //   wikiModalVisible: wikiModalVisible.value,
    //   wikiModalFilePath: wikiModalFilePath.value
    // })
    debug('准备打开 Wiki 模态框')
    
    // 打开 Wiki 模态框，并设置对应的文件路径和工具信息
    // 这样 WikiView 会自动加载对应的文章
    wikiModalFilePath.value = filePath
    wikiModalToolId.value = toolId
    wikiModalToolName.value = toolName
    // console.log('设置模态框参数后:', { 
    //   wikiModalFilePath: wikiModalFilePath.value,
    //   wikiModalToolId: wikiModalToolId.value,
    //   wikiModalToolName: wikiModalToolName.value
    // })
    debug('设置模态框参数')
    wikiModalVisible.value = true
    // console.log('模态框已打开:', { wikiModalVisible: wikiModalVisible.value })
    debug('模态框已打开')
    saveWikiModalState()
    // console.log('========== openWiki 完成 ==========')
    debug('openWiki 完成')
  } catch (err) {
    logError('========== openWiki 失败 ==========')
    logError('打开 Wiki 失败:', err)
    showConfirm('错误', `打开 Wiki 失败: ${err instanceof Error ? err.message : String(err)}`, () => {}, 'danger')
  }
}

const goSettings = () => {
  router.push({ name: 'settings' })
}

const openWikiHome = async () => {
  // 打开 Wiki 模态框（右上角按钮使用模态框）
  wikiModalFilePath.value = undefined
  wikiModalToolId.value = undefined
  wikiModalToolName.value = undefined
  wikiModalVisible.value = true
  saveWikiModalState()
}

// 侧边栏 Wiki 面板状态
const showWikiPanel = ref(false)
const currentWikiToolId = ref<string | undefined>(undefined)
const currentWikiToolName = ref<string | undefined>(undefined)
const currentWikiFilePath = ref<string | undefined>(undefined)
// Wiki 面板宽度（默认 450px）
const wikiPanelWidth = ref(450)
const isResizingWiki = ref(false)

const startResizeWiki = (e: MouseEvent) => {
  isResizingWiki.value = true
  document.addEventListener('mousemove', handleResizeWiki)
  document.addEventListener('mouseup', stopResizeWiki)
  document.body.style.userSelect = 'none' // 防止拖拽时选中文本
}

const handleResizeWiki = (e: MouseEvent) => {
  if (!isResizingWiki.value) return
  
  // 计算新宽度：窗口宽度 - 鼠标位置
  // 因为面板在右侧，所以宽度 = window.innerWidth - e.clientX
  const newWidth = window.innerWidth - e.clientX
  
  // 限制宽度范围
  if (newWidth >= 300 && newWidth <= 800) {
    wikiPanelWidth.value = newWidth
  }
}

const stopResizeWiki = () => {
  isResizingWiki.value = false
  document.removeEventListener('mousemove', handleResizeWiki)
  document.removeEventListener('mouseup', stopResizeWiki)
  document.body.style.userSelect = ''
}

const openToolWikiPanel = (toolId: string, toolName: string, filePath?: string) => {
  // 如果已经打开且是同一个工具，则关闭
  if (showWikiPanel.value && currentWikiToolId.value === toolId) {
    showWikiPanel.value = false
    return
  }
  
  currentWikiToolId.value = toolId
  currentWikiToolName.value = toolName
  currentWikiFilePath.value = filePath
  showWikiPanel.value = true
}

const closeWikiPanel = () => {
  showWikiPanel.value = false
  currentWikiToolId.value = undefined
  currentWikiToolName.value = undefined
  currentWikiFilePath.value = undefined
}

const onOpenToolWiki = (tool: ToolItem) => {
  let filePath: string | undefined = undefined
  if (tool.wikiUrl) {
    // 简单的路径规范化
    let normalized = tool.wikiUrl.trim().replace(/\\/g, '/')
    if (normalized.toLowerCase().startsWith('wiki/')) {
      normalized = normalized.substring(5)
    }
    filePath = normalized
  }
  openToolWikiPanel(tool.id, tool.name, filePath)
}

const onOverlayClick = (toolId: string) => {
  openTool(toolId)
  searchQuery.value = ''
  selectedSearchIndex.value = -1
}

// 处理搜索输入框的键盘事件
const handleSearchInputKeydown = (e: KeyboardEvent) => {
  if (!searchQuery.value || filteredTools.value.length === 0) return
  
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedSearchIndex.value = Math.min(selectedSearchIndex.value + 1, filteredTools.value.length - 1)
    // 滚动到选中项
    scrollToSelectedItem()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedSearchIndex.value = Math.max(selectedSearchIndex.value - 1, -1)
    // 滚动到选中项
    scrollToSelectedItem()
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (selectedSearchIndex.value >= 0 && selectedSearchIndex.value < filteredTools.value.length) {
      const tool = filteredTools.value[selectedSearchIndex.value]
      if (tool) {
        onOverlayClick(tool.id)
      }
    } else if (filteredTools.value.length > 0) {
      // 如果没有选中项，打开第一个
      const firstTool = filteredTools.value[0]
      if (firstTool) {
        onOverlayClick(firstTool.id)
      }
    }
  } else if (e.key === 'Escape') {
    searchQuery.value = ''
    selectedSearchIndex.value = -1
  }
}

// 处理搜索覆盖层的键盘事件
const handleSearchKeydown = (e: KeyboardEvent) => {
  if (!searchQuery.value || filteredTools.value.length === 0) return
  
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedSearchIndex.value = Math.min(selectedSearchIndex.value + 1, filteredTools.value.length - 1)
    scrollToSelectedItem()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedSearchIndex.value = Math.max(selectedSearchIndex.value - 1, -1)
    scrollToSelectedItem()
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (selectedSearchIndex.value >= 0 && selectedSearchIndex.value < filteredTools.value.length) {
      const tool = filteredTools.value[selectedSearchIndex.value]
      if (tool) {
        onOverlayClick(tool.id)
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
  const execPath = toolForm.value.toolType === 'JAR' 
    ? toolForm.value.jarPath.trim() 
    : toolForm.value.execPath.trim()
  
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
  
  // 验证路径格式（对于网页类型）
  if (toolForm.value.toolType === '网页') {
    try {
      new URL(execPath)
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
    debug('开始自动获取图标:', { toolType: toolForm.value.toolType, execPath })
    const autoIcon = await autoFetchIcon(toolForm.value.toolType, execPath)
    if (autoIcon) {
      toolForm.value.iconUrl = autoIcon
      autoFetchedIconPath.value = execPath
      isManualIcon.value = false
      debug('自动获取图标成功（输入时）:', { toolType: toolForm.value.toolType, execPath, iconLength: autoIcon.length })
    } else {
      warn('自动获取图标返回 null:', { toolType: toolForm.value.toolType, execPath })
    }
  } catch (error) {
    logError('自动获取图标失败（输入时）:', error, { toolType: toolForm.value.toolType, execPath })
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
            placeholder="搜索当前子分类的工具名称或描述（↑↓ 选择，Enter 打开，Esc 清除）"
            @keydown="handleSearchInputKeydown"
            @input="handleSearchInput"
          />
        </div>
      </div>
      <div class="content-row" @contextmenu="showBlankMenu">
        <div class="main-interface-wrapper" :class="{ 'with-wiki': showWikiPanel }">
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

          <div
            v-if="searchQuery && filteredTools.length"
            class="search-overlay"
            @keydown="handleSearchKeydown"
            tabindex="0"
          >
            <div class="overlay-title">搜索结果（{{ filteredTools.length }}）</div>
            <div class="overlay-list">
              <button
                v-for="(tool, index) in filteredTools"
                :key="tool.id"
                type="button"
                class="overlay-item"
                :class="{ 'selected': selectedSearchIndex === index }"
                @click="onOverlayClick(tool.id)"
                @dblclick="onOverlayClick(tool.id)"
                @mouseenter="selectedSearchIndex = index"
              >
                <span class="overlay-icon">🛠️</span>
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
        
        <div v-if="showWikiPanel" class="wiki-panel-wrapper" :style="{ flexBasis: `${wikiPanelWidth}px` }">
          <div class="wiki-resize-handle" @mousedown="startResizeWiki"></div>
          <div class="wiki-panel-header">
            <h3>{{ currentWikiToolName }}</h3>
            <div class="wiki-panel-actions">
              <button class="icon-button small" @click="closeWikiPanel" title="关闭">×</button>
            </div>
          </div>
          <div class="wiki-panel-body">
            <ToolWikiPanel 
              :tool-id="currentWikiToolId" 
              :tool-name="currentWikiToolName"
              :file-path="currentWikiFilePath"
            />
          </div>
        </div>
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

    <footer class="page-footer">
      <div class="footer-content">
        <span class="copyright">© 2025 By 序章</span>
      </div>
    </footer>
    
    <!-- Wiki 模态框（用于右上角 Wiki 按钮） -->
    <WikiModal
      v-model:visible="wikiModalVisible"
      :file-path="wikiModalFilePath"
      :tool-id="wikiModalToolId"
      :tool-name="wikiModalToolName"
      :title="wikiModalToolName ? `${wikiModalToolName} - Wiki` : 'Wiki 文档'"
      @close="clearWikiModalState"
    />
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

.icon-button.active {
  border-color: #4da3ff;
  background: radial-gradient(circle at top left, rgba(77, 163, 255, 0.2), rgba(15, 23, 42, 0.95));
  color: #4da3ff;
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
  padding: 12px 16px 16px; /* 增加顶部 padding，防止内容被遮挡 */
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
  left: 12px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 16px;
  color: #94a3b8; /* 更亮的颜色，提高可见性 */
  pointer-events: none;
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.3)); /* 添加阴影增强可见性 */
  transition: color 0.16s ease-out;
  z-index: 1;
  /* 确保图标位置固定，不会因为聚焦而移动 */
  will-change: color;
}

.search-box:focus-within .search-icon {
  color: #4da3ff; /* 聚焦时变为蓝色，更明显 */
  /* 保持位置绝对不变 */
  transform: translateY(-50%) !important;
  left: 12px !important;
  top: 50% !important;
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
  /* 确保聚焦时padding不变，防止图标移动 */
  padding: 8px 12px 8px 36px;
}

.content-row {
  display: flex;
  gap: 14px;
  width: 100%;
  align-items: flex-start;
  flex: 1; /* 允许内容行占据剩余空间 */
  min-height: 0; /* 关键：允许 flex 子元素缩小 */
  overflow: hidden; /* 限制内容行的高度，让内部滚动容器工作 */
}

.sub-list {
  flex: 0 0 260px;
  border-right: 1px solid rgba(148, 163, 184, 0.2);
  padding: 8px 14px 8px 8px; /* 上下左右留出空间，防止子分类被遮挡 */
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0; /* 关键：允许 flex 子元素缩小 */
  height: 100%; /* 确保子分类列表占据父容器高度 */
  overflow: hidden; /* 子分类列表容器不滚动，内部 .sub-cards 滚动 */
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
  min-height: 0; /* 关键：允许 flex 子元素缩小 */
  flex: 1; /* 占据剩余空间 */
  overflow-y: auto; /* 子分类卡片列表可以滚动 */
  overflow-x: hidden;
  padding: 4px 4px 8px 0; /* 上下左右留出空间，防止顶部和左侧被遮挡，右侧为滚动条留空间 */
  /* 确保可以接收鼠标滚轮事件 */
  overscroll-behavior: contain;
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
  min-height: 0; /* 关键：允许 flex 子元素缩小 */
  height: 100%; /* 确保工具区域占据父容器高度 */
  overflow: hidden; /* 工具区域容器不滚动，内部 .tools-wrapper 滚动 */
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
  flex: 0 0 auto; /* 固定工具头部，不伸缩 */
  flex-shrink: 0; /* 防止头部被压缩 */
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

.tools-wrapper {
  flex: 1;
  min-height: 0;
  padding: 8px 4px 8px 4px; /* 上下左右留出空间，防止工具卡片被遮挡 */
  overflow-y: auto; /* 工具网格区域可以滚动 */
  overflow-x: hidden;
  /* 确保可以接收鼠标滚轮事件 */
  overscroll-behavior: contain;
  /* 优化滚动条样式 */
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.4) rgba(15, 23, 42, 0.9);
  /* 优化渲染性能 */
  transform: translateZ(0);
  -webkit-transform: translateZ(0);
  will-change: contents;
  contain: layout style paint;
}

.tools-wrapper::-webkit-scrollbar {
  width: 8px;
}

.tools-wrapper::-webkit-scrollbar-track {
  background: rgba(15, 23, 42, 0.9);
}

.tools-wrapper::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.4);
  border-radius: 4px;
}

.tools-wrapper::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.6);
}

.tools-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
  padding: 4px 0 8px 0; /* 上下留出空间，防止顶部和底部工具卡片被遮挡 */
  min-height: min-content; /* 确保网格可以延展 */
}

.virtual-tools-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
  padding: 8px 0 0 0;
}

.tool-card {
  padding: 14px;
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.25);
  background:
    radial-gradient(circle at top left, rgba(148, 163, 184, 0.12), transparent 60%),
    linear-gradient(140deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.95));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 6px 18px rgba(0, 0, 0, 0.6);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  transition: transform 0.2s ease-out, box-shadow 0.2s ease-out, border-color 0.2s ease-out;
  cursor: pointer;
  position: relative;
  overflow: hidden;
}

.tool-card:hover {
  transform: translateY(-4px) scale(1.02);
  border-color: rgba(77, 163, 255, 0.6);
  box-shadow:
    0 0 0 1px rgba(77, 163, 255, 0.4),
    0 12px 32px rgba(0, 0, 0, 0.8);
}

.tool-icon-wrapper {
  width: 60px;
  height: 60px;
  border-radius: 14px;
  background: rgba(15, 23, 42, 0.6);
  border: 2px solid rgba(148, 163, 184, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  flex-shrink: 0;
  box-shadow: 0 3px 10px rgba(0, 0, 0, 0.4);
  transition: transform 0.2s ease-out, box-shadow 0.2s ease-out;
}

.tool-card:hover .tool-icon-wrapper {
  transform: scale(1.05);
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.5);
}

.tool-icon-img {
  width: 100%;
  height: 100%;
  object-fit: contain; /* 使用 contain 确保图标完整显示，不被裁剪 */
  display: block;
  image-rendering: -webkit-optimize-contrast; /* 优化图标渲染质量 */
  image-rendering: crisp-edges;
}

.tool-icon-default {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 36px;
  line-height: 1;
}

.tool-content {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  text-align: center;
}

.tool-name {
  font-size: 14px;
  font-weight: 600;
  color: #e5e7eb;
  line-height: 1.3;
  word-break: break-word;
  width: 100%;
}

.tool-desc {
  margin: 0;
  font-size: 11px;
  color: #9ca3af;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
  width: 100%;
  min-height: 30px;
}

.tool-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  margin-top: 2px;
  width: 100%;
}

.btn.small {
  padding: 6px 12px;
  font-size: 12px;
  min-width: auto;
}

/* 列表视图样式 */
.virtual-tools-list.list-view {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 4px 0 0 0;
}

.tools-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 4px 0 8px 0;
}

.tool-card-list {
  padding: 10px 14px;
  border-radius: 10px;
  border: 1px solid rgba(148, 163, 184, 0.25);
  background:
    radial-gradient(circle at top left, rgba(148, 163, 184, 0.12), transparent 60%),
    linear-gradient(140deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.95));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 4px 12px rgba(0, 0, 0, 0.6);
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 12px;
  transition: transform 0.2s ease-out, box-shadow 0.2s ease-out, border-color 0.2s ease-out;
  cursor: pointer;
  position: relative;
  overflow: hidden;
}

.tool-card-list:hover {
  transform: translateX(2px);
  border-color: rgba(77, 163, 255, 0.6);
  box-shadow:
    0 0 0 1px rgba(77, 163, 255, 0.4),
    0 6px 20px rgba(0, 0, 0, 0.8);
}

.tool-icon-wrapper-list {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: rgba(15, 23, 42, 0.6);
  border: 2px solid rgba(148, 163, 184, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  flex-shrink: 0;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  transition: transform 0.2s ease-out, box-shadow 0.2s ease-out;
}

.tool-card-list:hover .tool-icon-wrapper-list {
  transform: scale(1.05);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
}

.tool-icon-wrapper-list .tool-icon-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
  image-rendering: -webkit-optimize-contrast;
  image-rendering: crisp-edges;
}

.tool-icon-wrapper-list .tool-icon-default {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  line-height: 1;
}

.tool-content-list {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  text-align: left;
}

.tool-name-list {
  font-size: 14px;
  font-weight: 600;
  color: #e5e7eb;
  line-height: 1.3;
  word-break: break-word;
}

.tool-desc-list {
  margin: 0;
  font-size: 11px;
  color: #9ca3af;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tool-actions-list {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
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

.overlay-item:hover,
.overlay-item.selected {
  border-color: rgba(77, 163, 255, 0.6);
  box-shadow: 0 10px 22px rgba(0, 0, 0, 0.65);
  background: rgba(77, 163, 255, 0.1);
}

.overlay-item.selected {
  border-color: rgba(77, 163, 255, 0.8);
  background: rgba(77, 163, 255, 0.15);
}

.overlay-icon {
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  flex-shrink: 0;
  /* 去掉背景，去掉小方块样式 */
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

/* JAR 配置面板样式 */
.jar-config-panel {
  margin-top: 8px;
  padding: 16px;
  background: rgba(15, 23, 42, 0.4);
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 8px;
}

.jar-config-header {
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.15);
}

.jar-config-title {
  font-size: 14px;
  font-weight: 600;
  color: #4da3ff;
}

/* 网页配置面板样式 */
.web-config-panel {
  margin-top: 8px;
  padding: 16px;
  background: rgba(15, 23, 42, 0.4);
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 8px;
}

/* HTML 配置面板样式 */
.html-config-panel {
  margin-top: 8px;
  padding: 16px;
  background: rgba(15, 23, 42, 0.4);
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 8px;
}

/* LNK 配置面板样式 */
.lnk-config-panel {
  margin-top: 8px;
  padding: 16px;
  background: rgba(15, 23, 42, 0.4);
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 8px;
}

.field-with-button {
  display: flex;
  gap: 8px;
  align-items: stretch;
}

.field-with-button .field-input {
  flex: 1;
}

.file-select-btn {
  flex-shrink: 0;
  white-space: nowrap;
  padding: 8px 16px;
  font-size: 13px;
}

.field-hint {
  display: block;
  margin-top: 4px;
  font-size: 11px;
  color: #9ca3af;
  line-height: 1.4;
}

/* 图标预览样式 */
.icon-preview {
  margin-top: 12px;
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 12px;
  background: rgba(15, 23, 42, 0.4);
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 8px;
}

.icon-preview-img {
  width: 80px;
  height: 80px;
  object-fit: cover;
  border-radius: 12px;
  border: 2px solid rgba(148, 163, 184, 0.3);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
}

.icon-preview-error {
  padding: 8px 12px;
  font-size: 12px;
  color: #f87171;
  text-align: center;
  background: rgba(248, 113, 113, 0.1);
  border: 1px solid rgba(248, 113, 113, 0.3);
  border-radius: 6px;
}

.icon-preview-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 20px;
  text-align: center;
  color: #9ca3af;
}

.icon-preview-text {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 4px;
}

.icon-preview-hint {
  font-size: 12px;
  opacity: 0.7;
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

/* Wiki 模态框样式 */
:deep(.wiki-modal .modal-container) {
  width: 90vw;
  max-width: 1200px;
  height: 85vh;
  max-height: 900px;
  display: flex;
  flex-direction: column;
}

:deep(.wiki-modal .modal-body) {
  flex: 1;
  overflow: hidden;
  padding: 0;
  display: flex;
  flex-direction: column;
}

/* Wiki Panel Styles */
.main-interface-wrapper {
  display: flex;
  flex: 1;
  gap: 14px;
  min-width: 0;
  height: 100%;
  overflow: hidden;
  transition: all 0.3s ease;
}

.wiki-panel-wrapper {
  flex: 0 0 450px;
  max-width: 50%;
  display: flex;
  flex-direction: column;
  background: rgba(15, 23, 42, 0.95);
  border-left: 1px solid rgba(148, 163, 184, 0.2);
  height: 100%;
  overflow: hidden;
  transition: all 0.3s ease;
  border-radius: 12px 0 0 12px;
}

.wiki-panel-header {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.2);
  background: rgba(15, 23, 42, 0.98);
}

.wiki-panel-header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #e5e7eb;
}

.wiki-panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 0;
}

.wiki-panel-actions .icon-button {
  background: transparent;
  border: none;
  color: #9ca3af;
  font-size: 20px;
  cursor: pointer;
  padding: 4px;
  line-height: 1;
  border-radius: 4px;
  transition: all 0.2s;
}

.wiki-panel-actions .icon-button:hover {
  color: #e5e7eb;
  background: rgba(148, 163, 184, 0.2);
}

@media (max-width: 1200px) {
  .wiki-panel-wrapper {
    flex: 0 0 350px;
  }
}

</style>
