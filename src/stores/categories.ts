import { ref, watch } from 'vue'
import { readConfigFile, writeConfigFile, configFileExists } from '../utils/fileStorage'
import { getTauriInvoke } from '../utils/tauri'
import { debug, error as logError, info } from '../utils/logger'

/**
 * 转换工具项数据（从 JSON 格式到 TypeScript 类型）
 */
function transformToolItem(toolItem: Record<string, unknown>): ToolItem {
  const jarConfig = toolItem.jar_config || toolItem.jarConfig
  return {
    id: String(toolItem.id || ''),
    name: String(toolItem.name || ''),
    description: toolItem.description ? String(toolItem.description) : undefined,
    iconUrl: (() => {
      const iconUrlValue = toolItem.icon_url || toolItem.iconUrl
      if (!iconUrlValue) return undefined
      // 直接返回原始路径，不进行任何转换（保留用户手动修改的格式）
      return String(iconUrlValue)
    })(),
    wikiUrl: (toolItem.wiki_url || toolItem.wikiUrl) ? String(toolItem.wiki_url || toolItem.wikiUrl) : undefined,
    toolType: (() => {
      const toolTypeValue = toolItem.tool_type || toolItem.toolType
      if (toolTypeValue && String(toolTypeValue).trim()) {
        return String(toolTypeValue).trim() as ToolType
      }
      return undefined
    })(),
    execPath: (() => {
      const execPathValue = toolItem.exec_path || toolItem.execPath
      if (execPathValue && String(execPathValue).trim()) {
        return String(execPathValue).trim()
      }
      return undefined
    })(),
    args: Array.isArray(toolItem.args) ? (toolItem.args as unknown[]).map(a => String(a)) : undefined,
    workingDir: toolItem.working_dir || toolItem.workingDir ? String(toolItem.working_dir || toolItem.workingDir) : undefined,
    jarConfig: jarConfig ? (() => {
      const config = jarConfig as Record<string, unknown>
      const jvmArgsValue = config.jvm_args || config.jvmArgs
      const programArgsValue = config.program_args || config.programArgs
      return {
        jarPath: String(config.jar_path || config.jarPath || ''),
        javaPath: config.java_path || config.javaPath ? String(config.java_path || config.javaPath) : undefined,
        jvmArgs: Array.isArray(jvmArgsValue) ? (jvmArgsValue as unknown[]).map((a: unknown) => String(a)) : undefined,
        programArgs: Array.isArray(programArgsValue) ? (programArgsValue as unknown[]).map((a: unknown) => String(a)) : undefined,
      }
    })() : undefined,
  }
}

/**
 * 转换子分类数据（从 JSON 格式到 TypeScript 类型）
 */
function transformSubCategory(subCategory: Record<string, unknown>): SubCategory {
  return {
    id: String(subCategory.id || ''),
    name: String(subCategory.name || ''),
    description: subCategory.description ? String(subCategory.description) : undefined,
    tools: ((subCategory.tools || []) as unknown[]).map((tool: unknown) => {
      return transformToolItem(tool as Record<string, unknown>)
    }),
  }
}

/**
 * 转换分类页面数据（从 JSON 格式到 TypeScript 类型）
 */
function transformCategoryPageData(category: Record<string, unknown>): CategoryPageData {
  return {
    id: String(category.id || ''),
    name: String(category.name || ''),
    label: category.label ? String(category.label) : undefined,
    description: category.description ? String(category.description) : undefined,
    subCategories: ((category.sub_categories || category.subCategories || []) as unknown[]).map((sub: unknown) => {
      return transformSubCategory(sub as Record<string, unknown>)
    }),
  }
}

export interface CategoryConfig {
  id: string
  name: string
  label?: string
  description?: string
  icon: string
  color: string
  order: number
  enabled: boolean
}

export interface SubCategory {
  id: string
  name: string
  description?: string
  tools: ToolItem[]
}

export type ToolType = 'GUI' | 'CLI' | 'JAR' | 'Python' | '网页' | 'HTML' | 'LNK' | '其他'

export interface ToolItem {
  id: string
  name: string
  description?: string
  iconUrl?: string // 工具头像/图标 URL（原始路径，用于保存）
  _iconBase64?: string // 图标的 base64 数据（仅用于显示，不保存到 JSON）
  wikiUrl?: string
  toolType?: ToolType
  execPath?: string
  args?: string[]
  workingDir?: string
  // Java JAR 类型工具的专门配置
  jarConfig?: {
    jarPath: string // JAR 文件路径
    javaPath?: string // Java 可执行文件路径（可选，为空时使用 PATH）
    jvmArgs?: string[] // JVM 参数（如 -Xmx512m -Dxxx=yyy）
    programArgs?: string[] // 程序参数
  }
}

export interface CategoryPageData {
  id: string
  name: string
  label?: string
  description?: string
  subCategories: SubCategory[]
}

// 从JSON文件加载分类配置，如果没有则使用默认值
const loadCategoriesConfig = async (): Promise<CategoryConfig[]> => {
  const defaultConfig: CategoryConfig[] = [
  {
    id: 'web',
    name: 'WEB',
    label: 'Web 攻击与防御',
    description: 'Web 相关攻击与防御工具集合。',
    icon: 'globe',
    color: '#4DA3FF',
    order: 1,
    enabled: true,
  },
  {
    id: 'misc',
    name: 'MISC',
    label: '杂项工具',
    description: '杂项安全工具与小脚本集合。',
    icon: 'apps',
    color: '#A78BFA',
    order: 2,
    enabled: true,
  },
  {
    id: 'pwn',
    name: 'PWN',
    label: 'Pwn 漏洞利用',
    description: '二进制漏洞利用与堆栈攻击相关工具。',
    icon: 'bug',
    color: '#FF8F3D',
    order: 3,
    enabled: true,
  },
  {
    id: 'crypto',
    name: 'CRYPTO',
    label: '密码与编码',
    description: '常见密码学算法与编码分析工具。',
    icon: 'lock',
    color: '#2DD4BF',
    order: 4,
    enabled: true,
  },
  {
    id: 're',
    name: 'RE',
    label: '逆向工程',
    description: '逆向分析、调试与文件分析相关工具。',
    icon: 'search',
    color: '#9CA3AF',
    order: 5,
    enabled: true,
  },
  {
    id: 'forensics',
    name: '电子取证',
    label: '电子取证',
    description: '应急响应与电子取证相关辅助工具。',
    icon: 'fingerprint',
    color: '#22D3EE',
    order: 6,
    enabled: true,
  },
  {
    id: 'nav',
    name: '网址导航',
    label: '网址导航',
    description: '常用安全社区、情报源与在线工具导航。',
    icon: 'link',
    color: '#60A5FA',
    order: 7,
    enabled: true,
  },
  {
    id: 'post',
    name: '后渗透',
    label: '后渗透',
    description: '上线后控制、权限提升与横向移动工具。',
    icon: 'command',
    color: '#F87171',
    order: 8,
    enabled: true,
  },
  ]
  
  try {
    const fileContent = await readConfigFile('categories.json')
    if (fileContent && fileContent !== '{}') {
      const parsed = JSON.parse(fileContent)
      // 如果文件是数组格式（新格式），直接使用
      if (Array.isArray(parsed) && parsed.length > 0) {
        return parsed.map((cat: unknown) => {
          const c = cat as Record<string, unknown>
          return {
            id: String(c.id || ''),
            name: String(c.name || ''),
            label: c.label ? String(c.label) : undefined,
            description: c.description ? String(c.description) : undefined,
            icon: String(c.icon || 'apps'),
            color: String(c.color || '#4DA3FF'),
            order: typeof c.order === 'number' ? c.order : Number(c.order) || 0,
            enabled: typeof c.enabled === 'boolean' ? c.enabled : c.enabled !== false,
          }
        })
      }
      // 兼容旧格式：如果文件包含 categories 字段
      const config = parsed.categories
      if (Array.isArray(config) && config.length > 0) {
        // 验证配置格式，确保所有必需字段都存在
        return config.map((cat: unknown) => {
          const c = cat as Record<string, unknown>
          return {
            id: String(c.id || ''),
            name: String(c.name || ''),
            label: c.label ? String(c.label) : undefined,
            description: c.description ? String(c.description) : undefined,
            icon: String(c.icon || 'apps'),
            color: String(c.color || '#4DA3FF'),
            order: typeof c.order === 'number' ? c.order : Number(c.order) || 0,
            enabled: typeof c.enabled === 'boolean' ? c.enabled : c.enabled !== false,
          }
        })
      }
      // 兼容旧格式：如果整个文件就是数组
      if (Array.isArray(parsed) && parsed.length > 0) {
        return parsed.map((cat: unknown) => {
          const c = cat as Record<string, unknown>
          return {
            id: String(c.id || ''),
            name: String(c.name || ''),
            label: c.label ? String(c.label) : undefined,
            description: c.description ? String(c.description) : undefined,
            icon: String(c.icon || 'apps'),
            color: String(c.color || '#4DA3FF'),
            order: typeof c.order === 'number' ? c.order : Number(c.order) || 0,
            enabled: typeof c.enabled === 'boolean' ? c.enabled : c.enabled !== false,
          }
        })
      }
    }
  } catch (error) {
    logError('Failed to load categories config from file:', error)
  }
  
  // 如果文件不存在或解析失败，返回默认配置
  return defaultConfig
}

// 首页分类配置（用于Dashboard）
export const categoriesConfig = ref<CategoryConfig[]>([])

// 保存分类配置到 JSON 文件
const saveCategoriesConfig = async () => {
  try {
    // 转换分类配置格式
    const categoriesToSave = categoriesConfig.value.map((cat) => ({
      id: cat.id,
      name: cat.name,
      label: cat.label,
      description: cat.description,
      icon: cat.icon,
      color: cat.color,
      order: cat.order,
      enabled: cat.enabled,
    }))

    const content = JSON.stringify(categoriesToSave, null, 2)
    
    debug('保存分类配置:', {
      categoriesCount: categoriesToSave.length,
      contentLength: content.length,
    })
    
    await writeConfigFile('categories.json', content)
    
    info('✅ 分类配置文件保存成功')
  } catch (error) {
    logError('Failed to save categories config:', error)
  }
}

// 保存工具数据到 JSON 文件
export const saveToolsData = async () => {
  try {
    // 转换工具数据格式
    const dataToSave = await Promise.all(categoriesData.value.map(async (cat) => ({
      id: cat.id,
      name: cat.name,
      label: cat.label,
      description: cat.description,
      sub_categories: await Promise.all(cat.subCategories.map(async (sub) => ({
        id: sub.id,
        name: sub.name,
        description: sub.description,
        tools: await Promise.all(sub.tools.map(async (tool) => {
          // 如果图标是 base64 数据 URL，保存到缓存并转换为相对路径
          let iconUrl = tool.iconUrl
          if (iconUrl && iconUrl.startsWith('data:image')) {
            try {
              const { saveIconToCache } = await import('../utils/fileStorage')
              const iconPath = await saveIconToCache(iconUrl)
              iconUrl = iconPath
              debug('保存时转换 base64 图标为相对路径:', { toolId: tool.id, toolName: tool.name, iconPath })
            } catch (error) {
              logError('保存图标到缓存失败:', { toolId: tool.id, toolName: tool.name, error })
              // 如果保存失败，继续使用 base64（向后兼容）
            }
          }
          // 其他格式的路径（icons/, .config/icons/, 绝对路径等）直接保存，不进行转换
          
          return {
            id: tool.id,
            name: tool.name,
            description: tool.description,
            icon_url: iconUrl,
            wiki_url: tool.wikiUrl,
            tool_type: tool.toolType || null,
            exec_path: tool.execPath,
            args: tool.args,
            working_dir: tool.workingDir,
            jar_config: tool.jarConfig ? {
              jar_path: tool.jarConfig.jarPath,
              java_path: tool.jarConfig.javaPath,
              jvm_args: tool.jarConfig.jvmArgs,
              program_args: tool.jarConfig.programArgs,
            } : undefined,
          }
        })),
      }))),
    })))

    const content = JSON.stringify(dataToSave, null, 2)
    
    debug('保存工具数据:', {
      dataCount: dataToSave.length,
      contentLength: content.length,
      categories: dataToSave.map(c => ({
        id: c.id,
        name: c.name,
        subCategoriesCount: c.sub_categories.length,
        toolsCount: c.sub_categories.reduce((sum, sub) => sum + sub.tools.length, 0)
      }))
    })
    
    await writeConfigFile('tools.json', content)
    
    info('✅ 工具数据文件保存成功')
  } catch (error) {
    logError('Failed to save tools data:', error)
  }
}

// 监听分类配置变化，自动保存到分类配置文件
watch(
  categoriesConfig,
  async () => {
    // 只有在数据初始化完成后才保存，避免初始化时覆盖文件
    if (isDataInitialized) {
      await saveCategoriesConfig()
    }
  },
  { deep: true },
)

// 从JSON文件加载分类页面数据，如果没有则使用默认值
const loadCategoriesData = async (): Promise<CategoryPageData[]> => {
  const defaultData: CategoryPageData[] = [
  {
    id: 'web',
    name: 'WEB',
    label: 'Web 攻击与防御',
    description: '面向 Web 场景的信息收集、扫描与利用工具集合。',
    subCategories: [
      {
        id: 'web-info',
        name: '信息收集',
        description: '基础资产信息、指纹识别、子域名枚举。',
        tools: [
          {
            id: 'host-info',
            name: '主机信息探测',
            description: '对域名/IP 进行 whois、地理位置、ASN 等查询。',
            iconEmoji: '🌐',
            execPath: 'C:\\\\Tools\\\\whois.exe',
          },
          {
            id: 'subdomain',
            name: '子域名收集器',
            description: '综合被动源与爆破，对目标域名进行子域枚举。',
            iconEmoji: '🧭',
            execPath: 'C:\\\\Tools\\\\subfinder.exe',
            args: ['-d', 'example.com'],
          },
          {
            id: 'fingerprint',
            name: '网站指纹识别',
            description: '识别 Web 服务器、中间件、CMS 与常见 WAF。',
            iconEmoji: '🔍',
            execPath: 'C:\\\\Tools\\\\fingerprint.exe',
          },
        ],
      },
      {
        id: 'web-dir',
        name: '目录与文件扫描',
        description: '敏感目录/文件爆破、备份文件探测。',
        tools: [
          {
            id: 'dirscan',
            name: '目录扫描',
            description: '基于字典的目录/文件暴破，可设置线程与状态过滤。',
            iconEmoji: '📂',
            execPath: 'C:\\\\Tools\\\\dirscan.exe',
          },
          {
            id: 'backup-scan',
            name: '备份文件探测',
            description: '常见备份与历史文件名探测，支持自定义规则。',
            iconEmoji: '🗂️',
            execPath: 'C:\\\\Tools\\\\backupscan.exe',
          },
        ],
      },
      {
        id: 'web-port',
        name: '端口与服务探测',
        description: 'Web 相关端口扫描与服务识别。',
        tools: [
          {
            id: 'web-nmap',
            name: 'Web 端口扫描',
            description: '快速扫描常见 Web 端口并识别服务。',
            iconEmoji: '📡',
            execPath: 'C:\\\\Tools\\\\nmap.exe',
            args: ['-Pn', '-sV'],
          },
        ],
      },
      {
        id: 'web-vuln',
        name: '漏洞探测与利用',
        description: '常见 Web 漏洞扫描与 POC/EXP 执行。',
        tools: [
          {
            id: 'poc-runner',
            name: 'POC 运行器',
            description: '管理与运行多种 Web POC，统一输出结果。',
            iconEmoji: '⚡',
            execPath: 'C:\\\\Tools\\\\pocrunner.exe',
          },
        ],
      },
    ],
  },
  ]
  
  try {
    const fileContent = await readConfigFile('tools.json')
    if (fileContent && fileContent.trim() && fileContent !== '{}' && fileContent !== '[]') {
      const parsed = JSON.parse(fileContent)
      // 如果文件是数组格式（新格式），直接使用
      if (Array.isArray(parsed) && parsed.length > 0) {
        // 转换数据格式
        return parsed.map((cat: unknown) => {
          return transformCategoryPageData(cat as Record<string, unknown>)
        })
      }
      // 兼容旧格式：如果文件包含 data 字段
      const data = parsed.data
      if (Array.isArray(data) && data.length > 0) {
        // 转换数据格式（如果是从旧格式）
        return data.map((cat: unknown) => {
          return transformCategoryPageData(cat as Record<string, unknown>)
        })
      }
    }
  } catch (error) {
    logError('Failed to load categories data from file:', error)
  }
  
  return defaultData
}

// 分类页面数据（用于CategoryView）
export const categoriesData = ref<CategoryPageData[]>([])

// 标志：数据是否已初始化完成（用于防止 watch 在初始化时触发保存）
let isDataInitialized = false

// 初始化加载配置和数据，并检查文件是否存在
Promise.all([
  loadCategoriesConfig(),
  loadCategoriesData(),
]).then(async ([config, data]) => {
  // 先设置值
  categoriesConfig.value = config
  categoriesData.value = data
  
  // 标记数据已初始化完成
  isDataInitialized = true
  
  info('数据加载完成:', {
    categoriesCount: config.length,
    dataCount: data.length,
    categoriesIds: config.map(c => c.id),
  })
  
  // 检查配置文件是否存在，如果不存在则创建
  try {
    // 获取配置文件路径（用于显示给用户）
    const invoker = getTauriInvoke()
    let categoriesPath = '未知'
    let toolsPath = '未知'
    if (invoker) {
      try {
        categoriesPath = await invoker<string>('get_config_file_path', { filename: 'categories.json' })
        toolsPath = await invoker<string>('get_config_file_path', { filename: 'tools.json' })
        info('配置文件位置:', {
          categories: categoriesPath,
          tools: toolsPath,
        })
      } catch (err) {
        debug('无法获取配置文件路径:', err)
      }
    }
    
    const categoriesFileExists = await configFileExists('categories.json')
    const toolsFileExists = await configFileExists('tools.json')
    
    info('配置文件存在检查结果:', {
      categories: categoriesFileExists,
      categoriesPath,
      tools: toolsFileExists,
      toolsPath,
      loadedCategoriesCount: config.length,
      loadedDataCount: data.length,
    })
    
    // 如果分类配置文件不存在，使用默认值创建
    if (!categoriesFileExists) {
      info('⚠️ 分类配置文件不存在，使用默认值创建', {
        categoriesCount: config.length,
        path: categoriesPath,
      })
      // 临时禁用 watch，避免触发保存
      const wasInitialized = isDataInitialized
      isDataInitialized = false
      await saveCategoriesConfig()
      isDataInitialized = wasInitialized
      
      const created = await configFileExists('categories.json')
      if (created) {
        info('✅ 分类配置文件创建成功', { path: categoriesPath })
      } else {
        logError('❌ 分类配置文件创建失败，请检查 Tauri API 是否可用', { path: categoriesPath })
      }
    } else {
      debug('分类配置文件已存在，跳过创建', { path: categoriesPath })
    }
    
    // 如果工具数据文件不存在，使用默认值创建
    if (!toolsFileExists) {
      info('⚠️ 工具数据文件不存在，使用默认值创建', {
        dataCount: data.length,
        path: toolsPath,
      })
      // 临时禁用 watch，避免触发保存
      const wasInitialized = isDataInitialized
      isDataInitialized = false
      await saveToolsData()
      isDataInitialized = wasInitialized
      
      const created = await configFileExists('tools.json')
      if (created) {
        info('✅ 工具数据文件创建成功', { path: toolsPath })
      } else {
        logError('❌ 工具数据文件创建失败，请检查 Tauri API 是否可用', { path: toolsPath })
      }
    } else {
      debug('工具数据文件已存在，跳过创建', { path: toolsPath })
    }
  } catch (error) {
    logError('检查或创建配置文件时出错:', error)
  }
})

// 监听分类数据变化，自动保存到工具数据文件
watch(
  categoriesData,
  async () => {
    // 只有在数据初始化完成后才保存，避免初始化时覆盖文件
    if (isDataInitialized) {
      await saveToolsData()
    }
  },
  { deep: true },
)

// 根据分类ID获取或创建分类页面数据
export function getOrCreateCategoryData(categoryId: string): CategoryPageData {
  const config = categoriesConfig.value.find((c) => c.id === categoryId)
  if (!config) {
    // 如果配置中不存在，返回一个默认的空分类
    return {
      id: categoryId,
      name: categoryId.toUpperCase(),
      label: categoryId,
      description: '新分类',
      subCategories: [],
    }
  }

  let data = categoriesData.value.find((c) => c.id === categoryId)
  if (!data) {
    // 如果数据中不存在，根据配置创建一个新的分类数据
    data = {
      id: config.id,
      name: config.name,
      label: config.label,
      description: config.description,
      subCategories: [],
    }
    categoriesData.value.push(data)
  }

  return data
}

// 同步分类配置到分类数据
export function syncCategoryConfigToData(categoryId: string) {
  const config = categoriesConfig.value.find((c) => c.id === categoryId)
  if (!config) return

  const data = categoriesData.value.find((c) => c.id === categoryId)
  if (data) {
    // 更新现有数据
    data.name = config.name
    data.label = config.label
    data.description = config.description
  } else {
    // 创建新数据
    categoriesData.value.push({
      id: config.id,
      name: config.name,
      label: config.label,
      description: config.description,
      subCategories: [],
    })
  }
}

