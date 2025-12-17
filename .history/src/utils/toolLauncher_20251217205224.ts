/**
 * 工具启动器
 * 统一的工具启动逻辑
 */

import type { ToolItem } from '../stores/categories'
import { getTauriInvoke, waitForTauriAPI, isTauriEnvironment } from './tauri'
import { error } from './logger'

/**
 * 显示确认对话框的回调类型
 */
export type ShowConfirmCallback = (
  title: string,
  message: string,
  callback: () => void,
  type?: 'danger' | 'warning' | 'info'
) => void

/**
 * 启动工具
 * @param tool 工具对象
 * @param showConfirm 显示确认对话框的函数
 * @returns Promise<void>
 */
export async function launchTool(
  tool: ToolItem,
  showConfirm: ShowConfirmCallback
): Promise<void> {
  const toolType = tool.toolType || 'GUI'
  
  try {
    let invoker = getTauriInvoke()
    
    if (!invoker) {
      const apiLoaded = await waitForTauriAPI()
      if (apiLoaded) {
        invoker = getTauriInvoke()
      }
    }
    
    if (!invoker) {
      const isTauri = isTauriEnvironment()
      
      // 对于网页工具，在非 Tauri 环境中可以降级到 window.open
      if (toolType === '网页' && !isTauri) {
        const url = tool.execPath
        if (url) {
          try {
            new URL(url)
            const opened = window.open(url, '_blank', 'noopener,noreferrer')
            if (!opened) {
              showConfirm('提示', '浏览器阻止了弹窗，请允许弹窗后重试', () => {}, 'warning')
            }
            return
          } catch {
            showConfirm('提示', 'URL 地址格式无效', () => {}, 'warning')
            return
          }
        }
      }
      
      if (isTauri) {
        showConfirm('错误', 'Tauri API 加载失败，请刷新页面重试。', () => {}, 'warning')
      } else {
        showConfirm('错误', '无法连接到后端服务。请确保在 Tauri 桌面应用中运行。', () => {}, 'warning')
      }
      return
    }
    
    // 根据工具类型准备参数
    let execPath: string | undefined
    let workingDir: string | undefined
    let jarConfig: ToolItem['jarConfig'] | undefined
    
    if (toolType === 'JAR' && tool.jarConfig) {
      jarConfig = tool.jarConfig
    } else if (toolType === 'Python' || toolType === 'CLI') {
      execPath = tool.execPath
      if (!execPath) {
        showConfirm('提示', '工具路径未配置', () => {}, 'warning')
        return
      }
    } else if (toolType === 'HTML' || toolType === 'LNK') {
      execPath = tool.execPath
      if (!execPath) {
        showConfirm('提示', '文件路径未配置', () => {}, 'warning')
        return
      }
    } else if (toolType === '网页') {
      execPath = tool.execPath
      if (!execPath) {
        showConfirm('提示', 'URL 地址未配置', () => {}, 'warning')
        return
      }
      try {
        new URL(execPath)
      } catch {
        showConfirm('提示', 'URL 地址格式无效（必须以 http:// 或 https:// 开头）', () => {}, 'warning')
        return
      }
    } else {
      execPath = tool.execPath
      workingDir = tool.workingDir
      if (!execPath) {
        showConfirm('提示', '工具路径未配置', () => {}, 'warning')
        return
      }
    }
    
    const actualToolType = (tool.toolType && tool.toolType !== 'null' && tool.toolType !== 'undefined') 
      ? tool.toolType 
      : 'GUI'
    const toolTypeStr = String(actualToolType).trim()
    
    if (!toolTypeStr || toolTypeStr === 'undefined' || toolTypeStr === 'null') {
      showConfirm('错误', `工具类型无效: ${toolType}`, () => {}, 'warning')
      return
    }
    
    // 构建调用参数
    const invokeParams: Record<string, unknown> = {
      tool_type: toolTypeStr,
    }
    
    if (execPath !== undefined) {
      invokeParams.exec_path = execPath
    }
    if (tool.args && tool.args.length > 0) {
      invokeParams.args = tool.args
    }
    if (workingDir !== undefined) {
      invokeParams.working_dir = workingDir
    }
    if (jarConfig !== undefined) {
      invokeParams.jar_config = jarConfig
    }
    
    await invoker('launch_tool', { params: invokeParams })
  } catch (err: unknown) {
    const errorMessage = err instanceof Error ? err.message : String(err) || '未知错误'
    error('启动工具失败:', errorMessage, tool)
    showConfirm('错误', `启动工具失败：${errorMessage}`, () => {}, 'warning')
  }
}

