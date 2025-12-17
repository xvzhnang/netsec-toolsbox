/**
 * 应用常量定义
 */

/**
 * 分类图标映射
 */
export const CATEGORY_ICON_MAP: Record<string, string> = {
  'globe': '🌐',
  'apps': '🔧',
  'bug': '🐞',
  'lock': '🔒',
  'search': '🔍',
  'fingerprint': '🆔',
  'link': '🔗',
  'command': '⌘',
  'shield': '🛡️',
  'key': '🔑',
  'database': '💾',
  'network': '🌐',
  'code': '💻',
  'terminal': '💻',
}

/**
 * 默认分类图标
 */
export const DEFAULT_CATEGORY_ICON = '📁'

/**
 * 子分类图标
 */
export const SUBCATEGORY_ICON = '📂'

/**
 * 默认工具图标
 */
export const DEFAULT_TOOL_ICON = '🛠️'

/**
 * 虚拟滚动阈值
 */
export const VIRTUAL_SCROLL_THRESHOLD = 50

/**
 * 图标尺寸
 */
export const ICON_SIZE = {
  DEFAULT: 160,
  SMALL: 40,
  MEDIUM: 80,
  LARGE: 160,
} as const

/**
 * 图片处理配置
 */
export const IMAGE_PROCESSING = {
  TARGET_SIZE: 160,
  QUALITY: 0.9,
} as const

/**
 * Tauri API 等待配置
 */
export const TAURI_API_CONFIG = {
  TIMEOUT: 5000,
  RETRY_INTERVAL: 50,
} as const

