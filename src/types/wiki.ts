/**
 * Wiki 相关类型定义
 */

export interface WikiFileInfo {
  path: string
  name: string
  title: string
  is_dir: boolean
  children?: WikiFileInfo[]
}

export interface RenderResult {
  html: string
  title: string
  toc?: TocItem[]
}

export interface TocItem {
  level: number
  id: string
  text: string
  children?: TocItem[]
}

export interface SearchResult {
  file_path: string
  title: string
  matches?: SearchMatch[]
}

export interface SearchMatch {
  line: number
  text: string
}

