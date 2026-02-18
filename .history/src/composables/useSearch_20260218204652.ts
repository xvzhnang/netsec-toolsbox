import { computed, type Ref } from 'vue'

export interface SearchItem {
  id: string
  name: string
  description?: string
  iconUrl?: string // For tools/categories
  iconEmoji?: string // For categories/tools that use emoji
  type: 'category' | 'subcategory' | 'tool'
  originalData: any // The original object (ToolItem, CategoryConfig, etc.)
  // Additional context for navigation
  categoryId?: string
  subCategoryId?: string
}

export function useSearch(items: Ref<SearchItem[]>, searchQuery: Ref<string>) {
  const filteredResults = computed(() => {
    const q = searchQuery.value.trim().toLowerCase()
    if (!q) return []

    // Split query into keywords
    const keywords = q.split(/\s+/).filter((k) => k.length > 0)

    return items.value.filter((item) => {
      // Match every keyword
      return keywords.every((keyword) => {
        const nameMatch = item.name.toLowerCase().includes(keyword)
        const descMatch = item.description?.toLowerCase().includes(keyword) ?? false
        return nameMatch || descMatch
      })
    })
  })

  return {
    filteredResults,
  }
}
