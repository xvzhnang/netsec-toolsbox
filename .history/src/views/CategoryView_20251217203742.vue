<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
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
import { selectImageFile, processImage, extractIconFromExecutable, autoFetchIcon, detectFileTypeFromPath } from '../utils/imageProcessor'
import { getTauriInvoke, waitForTauriAPI, isTauriEnvironment } from '../utils/tauri'

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

// 自动获取工具图标（在工具加载时，如果还没有图标）
const autoFetchToolIcons = async () => {
  if (!currentSub.value) return
  
  for (const tool of currentSub.value.tools) {
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
            if (import.meta.env.DEV) {
              // eslint-disable-next-line no-console
              console.log('自动获取工具图标成功:', { toolId: tool.id, toolName: tool.name, toolType: tool.toolType })
            }
          }
        } catch (error) {
          if (import.meta.env.DEV) {
            // eslint-disable-next-line no-console
            console.warn('自动获取工具图标失败:', { toolId: tool.id, error })
          }
        }
      }
    }
  }
  
  // 触发响应式更新
  if (category.value) {
    const categoryIndex = categoriesData.value.findIndex(c => c.id === category.value?.id)
    if (categoryIndex >= 0) {
      categoriesData.value[categoryIndex] = { ...categoriesData.value[categoryIndex] }
    }
  }
}

// 监听当前子分类变化，自动获取图标
watch(
  currentSub,
  async (newSub) => {
    if (newSub) {
      // 延迟执行，避免阻塞 UI
      setTimeout(() => {
        autoFetchToolIcons()
      }, 500)
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

// Tauri API 类型声明（保留用于其他用途）
interface TauriWindow extends Window {
  __TAURI__?: {
    invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
    core?: {
      invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>
    }
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
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.error('工具未找到:', toolId, '可用工具:', tools.value.map(t => t.id))
    }
    return
  }
  
  // 从 JSON 配置文件中读取的工具类型（如果未设置则默认为 GUI）
  const toolType = tool.toolType || 'GUI'
  
  // 调试信息：显示从 JSON 配置文件中读取的完整工具信息
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.log('打开工具（从 JSON 配置文件读取）:', {
      toolId,
      toolName: tool.name,
      toolType,
      toolTypeRaw: tool.toolType,
      execPath: tool.execPath, // 从 JSON 读取的执行路径
      args: tool.args, // 从 JSON 读取的参数
      workingDir: tool.workingDir, // 从 JSON 读取的工作目录
      jarConfig: tool.jarConfig, // 从 JSON 读取的 JAR 配置
      fullTool: JSON.parse(JSON.stringify(tool)), // 完整工具对象（来自 JSON）
    })
  }
  
  // 所有类型都通过后端处理，包括网页类型（避免浏览器弹窗拦截）
  try {
    // 先尝试获取，如果失败则等待 API 加载
    let invoker = getTauriInvoke()
    
    if (!invoker) {
      // 等待 Tauri API 加载（最多等待 5 秒，因为 Tauri 2.x 可能需要更长时间）
      const apiLoaded = await waitForTauriAPI(5000)
      if (apiLoaded) {
        invoker = getTauriInvoke()
      }
    }
    
    if (!invoker) {
      // 检查是否在 Tauri 环境中
      const isTauri = isTauriEnvironment()
      
      if (import.meta.env.DEV) {
        // eslint-disable-next-line no-console
        console.error('Tauri API 不可用', {
          isTauri,
          userAgent: navigator.userAgent,
          location: window.location.href,
        })
      }
      
      // 如果不在 Tauri 环境中，对于网页工具可以降级到 window.open
      if (toolType === '网页' && !isTauri) {
        const url = tool.execPath
        if (url) {
          try {
            new URL(url)
            const opened = window.open(url, '_blank', 'noopener,noreferrer')
            if (!opened) {
              showConfirm('提示', '浏览器阻止了弹窗，请允许弹窗后重试', () => {}, 'warning')
            }
            // 启动成功，不显示提示，简化操作
            return
          } catch {
            showConfirm('提示', 'URL 地址格式无效', () => {}, 'warning')
            return
          }
        }
      }
      
      if (isTauri) {
        showConfirm('错误', 'Tauri API 加载失败，请刷新页面重试。如果问题持续，请检查 Tauri 配置。', () => {}, 'warning')
      } else {
        showConfirm('错误', '无法连接到后端服务。请确保在 Tauri 桌面应用中运行（使用 `npm run tauri dev` 启动），而不是直接在浏览器中打开。', () => {}, 'warning')
      }
      return
    }
    
    // 调试：检查工具数据
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.log('工具数据检查:', {
        toolId: tool.id,
        toolName: tool.name,
        toolType: tool.toolType,
        execPath: tool.execPath,
        execPathType: typeof tool.execPath,
        hasExecPath: !!tool.execPath,
        workingDir: tool.workingDir,
        jarConfig: tool.jarConfig,
      })
    }
    
    // 根据工具类型准备参数
    let execPath: string | undefined
    let workingDir: string | undefined
    let jarConfig: ToolItem['jarConfig'] | undefined
    
    if (toolType === 'JAR' && tool.jarConfig) {
      // JAR 类型使用 jarConfig
      jarConfig = tool.jarConfig
    } else if (toolType === 'Python' || toolType === 'CLI') {
      // Python 和 CLI 工具使用 execPath
      execPath = tool.execPath
      if (!execPath) {
        showConfirm('提示', '工具路径未配置', () => {}, 'warning')
        return
      }
    } else if (toolType === 'HTML' || toolType === 'LNK') {
      // HTML 和 LNK 工具只需要 execPath
      execPath = tool.execPath
      if (!execPath) {
        showConfirm('提示', '文件路径未配置', () => {}, 'warning')
        return
      }
    } else if (toolType === '网页') {
      // 网页类型使用 execPath（URL 地址）
      execPath = tool.execPath
      if (!execPath) {
        showConfirm('提示', 'URL 地址未配置', () => {}, 'warning')
        return
      }
      // 验证 URL 格式
      try {
        new URL(execPath)
      } catch {
        showConfirm('提示', 'URL 地址格式无效（必须以 http:// 或 https:// 开头）', () => {}, 'warning')
        return
      }
    } else {
      // GUI 等其他类型直接使用 execPath 和 workingDir
      execPath = tool.execPath
      workingDir = tool.workingDir
      if (!execPath) {
        showConfirm('提示', '工具路径未配置', () => {}, 'warning')
        return
      }
    }
    
    // 调用后端启动工具
    // 调试信息
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.log('调用后端启动工具:', {
        tool_type: toolType,
        tool_type_type: typeof toolType,
        exec_path: execPath,
        exec_path_type: typeof execPath,
        has_args: !!tool.args,
        working_dir: workingDir,
        has_jar_config: !!jarConfig,
      })
    }
    
    // 确保 tool_type 是字符串类型，并确保值正确
    // 注意：使用 tool.toolType 而不是 toolType，因为 toolType 可能是默认值 'GUI'
    // 如果 tool.toolType 是 null 或 undefined，使用默认值 'GUI'
    const actualToolType = (tool.toolType && tool.toolType !== 'null' && tool.toolType !== 'undefined') 
      ? tool.toolType 
      : (toolType || 'GUI')
    const toolTypeStr = String(actualToolType).trim()
    
    // 验证 tool_type 值
    if (!toolTypeStr || toolTypeStr === 'undefined' || toolTypeStr === 'null') {
      showConfirm('错误', `工具类型无效: ${toolType} (实际: ${actualToolType})`, () => {}, 'warning')
      return
    }
    
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.log('工具类型处理:', {
        originalToolType: toolType,
        toolToolType: tool.toolType,
        actualToolType,
        toolTypeStr,
      })
    }
    
    // 构建调用参数（所有信息都来自 JSON 配置文件）
    // 注意：后端使用结构体接收参数，支持 camelCase 和 snake_case 两种命名方式
    // 只传递非 undefined 的值，避免后端接收到 None
    const invokeParams: Record<string, unknown> = {
      tool_type: toolTypeStr, // 使用 snake_case（也支持 toolType）
    }
    
    // 只添加非 undefined 的参数，使用 snake_case（后端会自动匹配）
    if (execPath !== undefined && execPath !== null && execPath !== '') {
      invokeParams.exec_path = execPath // 也支持 execPath
    }
    if (tool.args !== undefined && tool.args !== null && tool.args.length > 0) {
      invokeParams.args = tool.args
    }
    if (workingDir !== undefined && workingDir !== null && workingDir !== '') {
      invokeParams.working_dir = workingDir // 也支持 workingDir
    }
    
    // JAR 配置（从 JSON 读取的 jar_config），使用 snake_case（后端会自动匹配）
    if (jarConfig) {
      invokeParams.jar_config = {
        jar_path: jarConfig.jarPath, // 也支持 jarPath
        java_path: jarConfig.javaPath || null, // 也支持 javaPath，确保 null 而不是 undefined
        jvm_args: jarConfig.jvmArgs || null, // 也支持 jvmArgs
        program_args: jarConfig.programArgs || null, // 也支持 programArgs
      }
    }
    
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.log('调用后端启动工具（参数来自 JSON 配置文件）:', invokeParams)
    }
    
    // 调用后端启动工具，传递从 JSON 配置文件中读取的所有信息
    // 注意：Tauri 2.x 使用结构体参数时，需要将参数对象包装在结构体字段名中
    await invoker('launch_tool', { params: invokeParams })
    
    // 启动成功，不显示提示，简化操作
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    showConfirm('错误', `启动工具失败：${errorMessage}`, () => {}, 'warning')
  }
}

const openWiki = (wikiUrl?: string) => {
  if (!wikiUrl) return
  window.open(wikiUrl, '_blank')
}

const goSettings = () => {
  router.push({ name: 'settings' })
}

const openWikiHome = async () => {
  try {
    const invoker = getTauriInvoke()
    if (invoker) {
      // 尝试启动 Wiki 服务器（如果后端支持）
      try {
        await invoker('start_wiki_server')
      } catch {
        // 静默处理错误，允许继续打开浏览器
      }
      // 通过后端打开浏览器，避免弹窗拦截
      try {
        await invoker('launch_tool', {
          tool_type: '网页',
          exec_path: 'http://127.0.0.1:8777',
        })
      } catch (error) {
        // 如果后端打开失败，降级到 window.open
        const errorMessage = error instanceof Error ? error.message : String(error)
        if (import.meta.env.DEV) {
          // eslint-disable-next-line no-console
          console.warn('通过后端打开浏览器失败，降级到 window.open:', errorMessage)
        }
        const opened = window.open('http://127.0.0.1:8777', '_blank')
        if (!opened) {
          showConfirm('提示', '浏览器阻止了弹窗，请允许弹窗后重试', () => {}, 'warning')
        }
      }
    } else {
      // 如果没有 Tauri API，降级到 window.open
      const opened = window.open('http://127.0.0.1:8777', '_blank')
      if (!opened) {
        showConfirm('提示', '浏览器阻止了弹窗，请允许弹窗后重试', () => {}, 'warning')
      }
    }
  } catch (err) {
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.error('open wiki failed', err)
    }
    // 即使启动服务失败，也尝试打开浏览器
    const opened = window.open('http://127.0.0.1:8777', '_blank')
    if (!opened) {
      showConfirm('提示', '浏览器阻止了弹窗，请允许弹窗后重试', () => {}, 'warning')
    }
  }
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
      onOverlayClick(filteredTools.value[selectedSearchIndex.value].id)
    } else if (filteredTools.value.length > 0) {
      // 如果没有选中项，打开第一个
      onOverlayClick(filteredTools.value[0].id)
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
      onOverlayClick(filteredTools.value[selectedSearchIndex.value].id)
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
  
  // 确保修改被 Vue 响应式系统检测到
  if (category.value) {
    const categoryIndex = categoriesData.value.findIndex(c => c.id === category.value?.id)
    if (categoryIndex >= 0) {
      // 创建一个新对象来触发响应式更新
      categoriesData.value[categoryIndex] = { ...categoriesData.value[categoryIndex] }
    }
  }
  
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.log('子分类已保存，等待自动同步到配置文件...')
  }
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
    if (categoryIndex >= 0) {
      // 创建一个新对象来触发响应式更新
      categoriesData.value[categoryIndex] = { ...categoriesData.value[categoryIndex] }
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
  const list = currentSub.value.tools
  const idx = list.findIndex((t) => t.id === toolForm.value.id)
  
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
          if (import.meta.env.DEV) {
            // eslint-disable-next-line no-console
            console.log('自动获取图标成功:', { toolType: toolForm.value.toolType, execPath })
          }
        }
      } catch (error) {
        if (import.meta.env.DEV) {
          // eslint-disable-next-line no-console
          console.warn('自动获取图标失败:', error)
        }
      }
    }
  }
  
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
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.log('保存工具:', {
      id: base.id,
      name: base.name,
      toolType: base.toolType,
      execPath: base.execPath,
      hasIcon: !!base.iconUrl,
    })
  }
  if (idx >= 0) {
    list[idx] = { ...list[idx], ...base }
  } else {
    list.push(base)
  }
  
  // 确保修改被 Vue 响应式系统检测到
  // 通过触发数组更新来确保 watch 能够捕获变化
  if (category.value) {
    // 强制触发响应式更新
    const categoryIndex = categoriesData.value.findIndex(c => c.id === category.value?.id)
    if (categoryIndex >= 0) {
      // 创建一个新对象来触发响应式更新
      categoriesData.value[categoryIndex] = { ...categoriesData.value[categoryIndex] }
    }
  }
  
  editingToolId.value = null
  showToolModal.value = false
  
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.log('工具已保存，等待自动同步到配置文件...')
  }
}

const deleteTool = (id: string) => {
  if (!currentSub.value) return
  const idx = currentSub.value.tools.findIndex((t) => t.id === id)
  if (idx >= 0) {
    currentSub.value.tools.splice(idx, 1)
    // 确保修改被 Vue 响应式系统检测到
    if (category.value) {
      const categoryIndex = categoriesData.value.findIndex(c => c.id === category.value?.id)
      if (categoryIndex >= 0) {
        // 创建一个新对象来触发响应式更新
        categoriesData.value[categoryIndex] = { ...categoriesData.value[categoryIndex] }
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
        if (import.meta.env.DEV) {
          // eslint-disable-next-line no-console
          console.warn('解析文件路径失败，使用原始路径:', err)
        }
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
        if (import.meta.env.DEV) {
          // eslint-disable-next-line no-console
          console.warn('解析文件路径失败，使用原始路径:', err)
        }
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
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.log('开始自动获取图标:', { toolType: toolForm.value.toolType, execPath })
    }
    const autoIcon = await autoFetchIcon(toolForm.value.toolType, execPath)
    if (autoIcon) {
      toolForm.value.iconUrl = autoIcon
      autoFetchedIconPath.value = execPath
      isManualIcon.value = false
      if (import.meta.env.DEV) {
        // eslint-disable-next-line no-console
        console.log('自动获取图标成功（输入时）:', { toolType: toolForm.value.toolType, execPath, iconLength: autoIcon.length })
      }
    } else {
      if (import.meta.env.DEV) {
        // eslint-disable-next-line no-console
        console.warn('自动获取图标返回 null:', { toolType: toolForm.value.toolType, execPath })
      }
    }
  } catch (error) {
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.error('自动获取图标失败（输入时）:', error, { toolType: toolForm.value.toolType, execPath })
    }
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
    
    // 更新表单
    toolForm.value.iconUrl = processedImage
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
        if (import.meta.env.DEV) {
          // eslint-disable-next-line no-console
          console.warn('解析文件路径失败，使用原始路径:', err)
        }
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
            placeholder="搜索当前子分类的工具名称或描述（↑↓ 选择，Enter 打开，Esc 清除）"
            @keydown="handleSearchInputKeydown"
            @input="handleSearchInput"
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
                      v-if="(tool as ToolItem).iconUrl && !(tool as ToolItem).iconUrl?.startsWith('file://')"
                      :src="(tool as ToolItem).iconUrl"
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
                        <button type="button" class="btn ghost small" @click="openWiki((tool as ToolItem).wikiUrl)">📚 Wiki</button>
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
                      v-if="tool.iconUrl"
                      :src="tool.iconUrl"
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
                      <button type="button" class="btn ghost small" @click="openWiki(tool.wikiUrl)">📚 Wiki</button>
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
                        v-if="(tool as ToolItem).iconUrl && !(tool as ToolItem).iconUrl?.startsWith('file://')"
                        :src="(tool as ToolItem).iconUrl"
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
                      <button type="button" class="btn ghost small" @click="openWiki((tool as ToolItem).wikiUrl)">📚 Wiki</button>
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
                      v-if="tool.iconUrl"
                      :src="tool.iconUrl"
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
                    <button type="button" class="btn ghost small" @click="openWiki(tool.wikiUrl)">📚 Wiki</button>
                    <button type="button" class="btn primary small" @click="openTool(tool.id)">打开</button>
                  </div>
                </div>
              </div>
            </template>
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
                        v-if="(tool as ToolItem).iconUrl && !(tool as ToolItem).iconUrl?.startsWith('file://')"
                        :src="(tool as ToolItem).iconUrl"
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
                      <button type="button" class="btn ghost small" @click="openWiki((tool as ToolItem).wikiUrl)">📚 Wiki</button>
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
                      v-if="tool.iconUrl"
                      :src="tool.iconUrl"
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
                    <button type="button" class="btn ghost small" @click="openWiki(tool.wikiUrl)">📚 Wiki</button>
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
</style>


