<template>
  <ul class="wiki-tree-list">
    <li
      v-for="file in files"
      :key="file.path"
      :class="file.is_dir ? 'wiki-tree-dir' : 'wiki-tree-file'"
    >
      <template v-if="file.is_dir">
        <span
          class="wiki-tree-toggle"
          :class="{ 'wiki-tree-toggle-empty': !file.children || file.children.length === 0 }"
          @click="toggleDir(file.path)"
          :style="{ cursor: (file.children && file.children.length > 0) ? 'pointer' : 'default' }"
        >
          {{ expandedDirs.has(file.path) ? '▼' : '▶' }} 📁 {{ file.name }}
        </span>
        <div
          v-if="file.children && file.children.length > 0"
          v-show="expandedDirs.has(file.path)"
          class="wiki-tree-children"
        >
          <WikiFileTree :files="file.children" @load-file="$emit('load-file', $event)" />
        </div>
      </template>
      <template v-else>
        <a href="#" @click.prevent="$emit('load-file', file.path)">📄 {{ file.title }}</a>
      </template>
    </li>
  </ul>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import type { WikiFileInfo } from '../types/wiki'

interface Props {
  files: WikiFileInfo[]
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'load-file': [filePath: string]
}>()

// 从 localStorage 恢复展开状态
const getExpandedDirs = (): Set<string> => {
  try {
    const saved = localStorage.getItem('wiki-expanded-dirs')
    if (saved) {
      return new Set(JSON.parse(saved))
    }
  } catch (e) {
    console.warn('恢复文件树展开状态失败:', e)
  }
  return new Set()
}

const expandedDirs = ref<Set<string>>(getExpandedDirs())

// 保存展开状态到 localStorage
const saveExpandedDirs = () => {
  try {
    localStorage.setItem('wiki-expanded-dirs', JSON.stringify(Array.from(expandedDirs.value)))
  } catch (e) {
    console.warn('保存文件树展开状态失败:', e)
  }
}

const toggleDir = (dirId: string) => {
  if (expandedDirs.value.has(dirId)) {
    expandedDirs.value.delete(dirId)
  } else {
    expandedDirs.value.add(dirId)
  }
  saveExpandedDirs()
}

onMounted(() => {
  // 组件挂载时恢复展开状态
  expandedDirs.value = getExpandedDirs()
})
</script>

<style scoped>
.wiki-tree-list {
  list-style: none;
  padding: 0;
  margin: 0;
  line-height: 1.6;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
}

.wiki-tree-dir,
.wiki-tree-file {
  margin: 2px 0;
  position: relative;
}

.wiki-tree-toggle {
  cursor: pointer;
  user-select: none;
  color: #24292f;
  display: flex;
  align-items: center;
  padding: 6px 10px;
  border-radius: 4px;
  font-size: 13px;
  line-height: 1.6;
  transition: all 0.2s;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
}

.wiki-tree-toggle:hover {
  background: #f6f8fa;
  color: #0969da;
}

.wiki-tree-toggle-empty {
  cursor: default;
  opacity: 0.6;
  color: #8c959f;
}

/* 分支线样式 */
.wiki-tree-children {
  margin-left: 16px;
  margin-top: 2px;
  padding-left: 10px;
  position: relative;
  border-left: 1px solid #e1e4e8;
}

/* 分支连接线（竖线） */
.wiki-tree-children::before {
  content: '';
  position: absolute;
  left: -1px;
  top: 0;
  bottom: 0;
  width: 1px;
  background: #e1e4e8;
}

/* 分支连接点 */
.wiki-tree-children::after {
  content: '';
  position: absolute;
  left: -4px;
  top: 10px;
  width: 6px;
  height: 1px;
  background: #e1e4e8;
}

.wiki-tree-file a {
  color: #24292f;
  text-decoration: none;
  display: flex;
  align-items: center;
  padding: 6px 10px;
  border-radius: 4px;
  font-size: 13px;
  line-height: 1.6;
  transition: all 0.2s;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
}

.wiki-tree-file a:hover {
  background: #f6f8fa;
  color: #0969da;
  text-decoration: none;
}

/* 图标间距 */
.wiki-tree-toggle,
.wiki-tree-file a {
  gap: 6px;
}
</style>

