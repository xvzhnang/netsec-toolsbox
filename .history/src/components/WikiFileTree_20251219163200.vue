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
    // console.warn('恢复文件树展开状态失败:', e)
  }
  return new Set()
}

const expandedDirs = ref<Set<string>>(getExpandedDirs())

// 保存展开状态到 localStorage
const saveExpandedDirs = () => {
  try {
    localStorage.setItem('wiki-expanded-dirs', JSON.stringify(Array.from(expandedDirs.value)))
  } catch (e) {
    // console.warn('保存文件树展开状态失败:', e)
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
  line-height: 1.8;
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
}

.wiki-tree-dir,
.wiki-tree-file {
  margin: 4px 0;
  position: relative;
}

.wiki-tree-toggle {
  cursor: pointer;
  user-select: none;
  color: #333;
  display: flex;
  align-items: center;
  padding: 8px 14px;
  border-radius: 8px;
  font-size: 14px;
  line-height: 1.8;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  font-family: "微软雅黑", "Microsoft YaHei", sans-serif;
  font-weight: 500;
}

.wiki-tree-toggle:hover {
  background: linear-gradient(to right,
    rgba(240, 255, 240, 0.9) 0%,
    rgba(232, 255, 232, 0.7) 100%);
  color: #2d5a2d;
  transform: translateX(4px);
  box-shadow: 0 2px 8px rgba(144, 238, 144, 0.4);
}

.wiki-tree-toggle-empty {
  cursor: default;
  opacity: 0.5;
  color: rgba(45, 90, 45, 0.5);
}

/* 分支线样式 - 淡绿色主题 */
.wiki-tree-children {
  margin-left: 20px;
  margin-top: 4px;
  padding-left: 12px;
  position: relative;
  border-left: 2px solid #90EE90;
}

/* 分支连接线（竖线） */
.wiki-tree-children::before {
  content: '';
  position: absolute;
  left: -2px;
  top: 0;
  bottom: 0;
  width: 2px;
  background: linear-gradient(to bottom,
    #90EE90 0%,
    #98FB98 50%,
    #90EE90 100%);
  opacity: 0.8;
}

/* 分支连接点 */
.wiki-tree-children::after {
  content: '';
  position: absolute;
  left: -6px;
  top: 12px;
  width: 8px;
  height: 2px;
  background: #90EE90;
  border-radius: 1px;
}

.wiki-tree-file a {
  color: rgba(229, 231, 235, 0.85);
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
  background: rgba(255, 255, 255, 0.1);
  color: #4da3ff;
  text-decoration: none;
}

/* 图标间距 */
.wiki-tree-toggle,
.wiki-tree-file a {
  gap: 6px;
}
</style>

