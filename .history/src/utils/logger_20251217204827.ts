/**
 * 统一的日志工具
 * 只在开发环境输出日志，生产环境静默
 */

const isDev = import.meta.env.DEV

/**
 * 日志级别
 */
export enum LogLevel {
  DEBUG = 0,
  INFO = 1,
  WARN = 2,
  ERROR = 3,
}

/**
 * 日志配置
 */
const config = {
  level: isDev ? LogLevel.DEBUG : LogLevel.ERROR,
  enableConsole: isDev,
}

/**
 * 格式化日志消息
 */
function formatMessage(level: string, message: string): string {
  const timestamp = new Date().toISOString()
  return `[${timestamp}] [${level}] ${message}`
}

/**
 * 调试日志（仅在开发环境）
 */
export function debug(message: string, ...args: unknown[]): void {
  if (config.level <= LogLevel.DEBUG && config.enableConsole) {
    // eslint-disable-next-line no-console
    console.debug(formatMessage('DEBUG', message), ...args)
  }
}

/**
 * 信息日志（仅在开发环境）
 */
export function info(message: string, ...args: unknown[]): void {
  if (config.level <= LogLevel.INFO && config.enableConsole) {
    // eslint-disable-next-line no-console
    console.log(formatMessage('INFO', message), ...args)
  }
}

/**
 * 警告日志（仅在开发环境）
 */
export function warn(message: string, ...args: unknown[]): void {
  if (config.level <= LogLevel.WARN && config.enableConsole) {
    // eslint-disable-next-line no-console
    console.warn(formatMessage('WARN', message), ...args)
  }
}

/**
 * 错误日志（始终输出）
 */
export function error(message: string, ...args: unknown[]): void {
  if (config.level <= LogLevel.ERROR) {
    // eslint-disable-next-line no-console
    console.error(formatMessage('ERROR', message), ...args)
  }
}

