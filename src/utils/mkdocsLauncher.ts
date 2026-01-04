import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'
import { debug, error as logError } from './logger'

export const getMkDocsBaseUrl = async (): Promise<string> => {
  const port = await invoke<number>('start_mkdocs_server')
  return `http://127.0.0.1:${port}`
}

export const openMkDocs = async (path?: string) => {
  try {
    const base = await getMkDocsBaseUrl()
    const url = path ? `${base}/${path.replace(/^\/+/, '')}` : base
    debug('Opening MkDocs URL:', url)
    await open(url)
  } catch (err) {
    logError('Failed to open MkDocs:', err)
    throw err
  }
}

export const stopMkDocs = async (): Promise<void> => {
  try {
    await invoke('stop_mkdocs_server')
    debug('Stopped MkDocs server')
  } catch (err) {
    logError('Failed to stop MkDocs:', err)
    // Don't throw to avoid crashing callers
  }
}
