/**
 * 本地存储工具
 * 提供类型安全的数据持久化功能
 */

const STORAGE_KEY_PREFIX = 'netsec-toolbox_'

export function saveToStorage<T>(key: string, value: T): void {
  try {
    const serialized = JSON.stringify(value)
    localStorage.setItem(`${STORAGE_KEY_PREFIX}${key}`, serialized)
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error(`Failed to save to localStorage: ${key}`, error)
  }
}

export function loadFromStorage<T>(key: string, defaultValue: T): T {
  try {
    const item = localStorage.getItem(`${STORAGE_KEY_PREFIX}${key}`)
    if (item === null) {
      return defaultValue
    }
    return JSON.parse(item) as T
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error(`Failed to load from localStorage: ${key}`, error)
    return defaultValue
  }
}

export function removeFromStorage(key: string): void {
  try {
    localStorage.removeItem(`${STORAGE_KEY_PREFIX}${key}`)
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error(`Failed to remove from localStorage: ${key}`, error)
  }
}

export function clearStorage(): void {
  try {
    const keys = Object.keys(localStorage)
    keys.forEach((key) => {
      if (key.startsWith(STORAGE_KEY_PREFIX)) {
        localStorage.removeItem(key)
      }
    })
  } catch (error) {
    // eslint-disable-next-line no-console
    console.error('Failed to clear storage', error)
  }
}

