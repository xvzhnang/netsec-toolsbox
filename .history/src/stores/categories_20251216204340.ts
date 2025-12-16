import { ref, watch } from 'vue'
import { saveToStorage, loadFromStorage } from '../utils/storage'

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

export type ToolType = 'GUI' | 'CLI' | 'JAR' | 'Python' | '网页' | '其他'

export interface ToolItem {
  id: string
  name: string
  description?: string
  iconEmoji?: string
  wikiUrl?: string
  toolType?: ToolType
  execPath?: string
  args?: string[]
  workingDir?: string
}

export interface CategoryPageData {
  id: string
  name: string
  label?: string
  description?: string
  subCategories: SubCategory[]
}

// 从本地存储加载分类配置，如果没有则使用默认值
const loadCategoriesConfig = (): CategoryConfig[] => {
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
loadCategoriesConfig().then((config) => {
  categoriesConfig.value = config
})

// 监听分类配置变化，自动保存到JSON文件
watch(
  categoriesConfig,
  async (newConfig) => {
    try {
      const content = JSON.stringify({ categories: newConfig }, null, 2)
      await writeConfigFile(content)
    } catch (error) {
      console.error('Failed to save categories config to file:', error)
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
  return loadFromStorage('categoriesData', defaultData)
}

// 分类页面数据（用于CategoryView）
export const categoriesData = ref<CategoryPageData[]>(loadCategoriesData())

// 监听分类数据变化，自动保存到本地存储
watch(
  categoriesData,
  (newData) => {
    saveToStorage('categoriesData', newData)
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

