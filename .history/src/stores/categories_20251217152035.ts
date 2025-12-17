import { ref, watch } from 'vue'
import { readConfigFile, writeConfigFile } from '../utils/fileStorage'

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
  iconEmoji?: string
  iconUrl?: string // 工具头像/图标 URL 或 base64 数据URL（优先于 iconEmoji）
  iconSource?: 'url' | 'local' | 'executable' // 图标来源类型
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
    const fileContent = await readConfigFile()
    if (fileContent && fileContent !== '{}') {
      const parsed = JSON.parse(fileContent)
      // 如果文件中有categories字段，使用它；否则假设整个文件就是数组
      const config = parsed.categories || parsed
      if (Array.isArray(config) && config.length > 0) {
        return config
      }
    }
  } catch (error) {
    console.error('Failed to load categories config from file:', error)
  }
  
  return defaultConfig
}

// 首页分类配置（用于Dashboard）
export const categoriesConfig = ref<CategoryConfig[]>([])

// 初始化加载配置
loadCategoriesConfig().then((config: CategoryConfig[]) => {
  categoriesConfig.value = config
})

// 保存配置到 JSON 文件的统一函数
const saveAllConfigToFile = async () => {
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

    // 转换分类数据格式
    const dataToSave = categoriesData.value.map((cat) => ({
      id: cat.id,
      name: cat.name,
      label: cat.label,
      description: cat.description,
      sub_categories: cat.subCategories.map((sub) => ({
        id: sub.id,
        name: sub.name,
        description: sub.description,
        tools: sub.tools.map((tool) => ({
          id: tool.id,
          name: tool.name,
          description: tool.description,
          icon_emoji: tool.iconEmoji,
          icon_url: tool.iconUrl,
          icon_source: tool.iconSource,
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
        })),
      })),
    }))

    // 合并保存到一个 JSON 文件
    const content = JSON.stringify(
      {
        categories: categoriesToSave,
        data: dataToSave,
      },
      null,
      2
    )
    await writeConfigFile(content)
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error('Failed to save config to file:', error)
  }
}

// 监听分类配置变化，自动保存到JSON文件
watch(
  categoriesConfig,
  async () => {
    await saveAllConfigToFile()
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
    const fileContent = await readConfigFile()
    if (fileContent && fileContent !== '{}') {
      const parsed = JSON.parse(fileContent)
      // 优先使用 parsed.data
      const data = parsed.data
      if (Array.isArray(data) && data.length > 0) {
        // 转换数据格式（如果是从旧格式）
        return data.map((cat: unknown) => {
          const category = cat as Record<string, unknown>
          return {
            id: String(category.id || ''),
            name: String(category.name || ''),
            label: category.label ? String(category.label) : undefined,
            description: category.description ? String(category.description) : undefined,
            subCategories: ((category.sub_categories || category.subCategories || []) as unknown[]).map((sub: unknown) => {
              const subCategory = sub as Record<string, unknown>
              return {
                id: String(subCategory.id || ''),
                name: String(subCategory.name || ''),
                description: subCategory.description ? String(subCategory.description) : undefined,
                tools: ((subCategory.tools || []) as unknown[]).map((tool: unknown) => {
                  const toolItem = tool as Record<string, unknown>
                  const jarConfig = toolItem.jar_config || toolItem.jarConfig
                  return {
                    id: String(toolItem.id || ''),
                    name: String(toolItem.name || ''),
                    description: toolItem.description ? String(toolItem.description) : undefined,
                    iconEmoji: (toolItem.icon_emoji || toolItem.iconEmoji) ? String(toolItem.icon_emoji || toolItem.iconEmoji) : undefined,
                    iconUrl: (toolItem.icon_url || toolItem.iconUrl) ? String(toolItem.icon_url || toolItem.iconUrl) : undefined,
                    iconSource: (toolItem.icon_source || toolItem.iconSource) ? String(toolItem.icon_source || toolItem.iconSource) as 'url' | 'local' | 'executable' : undefined,
                    wikiUrl: (toolItem.wiki_url || toolItem.wikiUrl) ? String(toolItem.wiki_url || toolItem.wikiUrl) : undefined,
                    toolType: (() => {
                      const toolTypeValue = toolItem.tool_type || toolItem.toolType
                      if (toolTypeValue && String(toolTypeValue).trim()) {
                        return String(toolTypeValue).trim() as ToolType
                      }
                      return undefined
                    })(),
                    execPath: (toolItem.exec_path || toolItem.execPath) ? String(toolItem.exec_path || toolItem.execPath) : undefined,
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
                }),
              }
            }),
          }
        })
      }
    }
  } catch (error) {
    console.error('Failed to load categories data from file:', error)
  }
  
  return defaultData
}

// 分类页面数据（用于CategoryView）
export const categoriesData = ref<CategoryPageData[]>([])

// 初始化加载数据
loadCategoriesData().then((data) => {
  categoriesData.value = data
})

// 监听分类数据变化，自动保存到JSON文件
watch(
  categoriesData,
  async () => {
    await saveAllConfigToFile()
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

