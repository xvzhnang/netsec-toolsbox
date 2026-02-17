<script setup lang="ts">
import { ref, computed, watch, type Ref } from 'vue'
import AppLayout from '../layouts/AppLayout.vue'
import { categoriesConfig, syncCategoryConfigToData } from '@/stores/categories'

interface SubCategoryConfig {
  id: string
  name: string
  description?: string
  order: number
  enabled: boolean
}

interface CategoryConfig {
  id: string
  name: string
  label?: string
  description?: string
  icon: string
  color: string
  order: number
  enabled: boolean
  subCategories?: SubCategoryConfig[]
}

// 使用 store 中的 categoriesConfig
const categories = categoriesConfig as unknown as Ref<CategoryConfig[]>
const selectedId = ref<string | null>(categories.value[0]?.id ?? null)

const selected = computed({
  get() {
    return categories.value.find((c: CategoryConfig) => c.id === selectedId.value) ?? null
  },
  set(value) {
    if (!value) return
    const idx = categories.value.findIndex((c: CategoryConfig) => c.id === value.id)
    if (idx !== -1) {
      categories.value[idx] = { ...value }
      // 触发响应式更新
      categories.value = [...categories.value]
      // 同步配置到分类数据
      syncCategoryConfigToData(value.id)
    }
  },
})

const onSelect = (id: string) => {
  selectedId.value = id
}

const onAdd = () => {
  const nextOrder =
    categories.value.reduce((max: number, c: CategoryConfig) => Math.max(max, c.order), 0) + 1
  const newId = `category_${Date.now()}`
  const newCategory: CategoryConfig = {
    id: newId,
    name: 'NEW',
    label: '新分类',
    description: '请编辑此分类信息。',
    icon: 'apps',
    color: '#4DA3FF',
    order: nextOrder,
    enabled: true,
    subCategories: [],
  }
  categories.value.push(newCategory)
  // 同步配置到分类数据
  syncCategoryConfigToData(newId)
  // 触发响应式更新
  categories.value = [...categories.value]
  selectedId.value = newId
}

const editingSub = ref<SubCategoryConfig | null>(null)
const isAddingSub = ref(false)

const onAddSub = () => {
  if (!selected.value) return
  const subCategories = selected.value.subCategories || []
  const nextOrder =
    subCategories.reduce((max: number, s: SubCategoryConfig) => Math.max(max, s.order), 0) + 1
  const newSub: SubCategoryConfig = {
    id: `sub_${Date.now()}`,
    name: '新子分类',
    description: '请编辑子分类信息。',
    order: nextOrder,
    enabled: true,
  }
  if (!selected.value.subCategories) {
    selected.value.subCategories = []
  }
  selected.value.subCategories.push(newSub)
  // 触发响应式更新
  const categoryIndex = categories.value.findIndex((c: CategoryConfig) => c.id === selected.value?.id)
  if (categoryIndex >= 0) {
    const current = categories.value[categoryIndex]
    if (current) {
      categories.value[categoryIndex] = { ...current }
    }
    categories.value = [...categories.value]
  }
  editingSub.value = newSub
  isAddingSub.value = true
}

const onEditSub = (sub: SubCategoryConfig) => {
  editingSub.value = { ...sub }
  isAddingSub.value = false
}

const onSaveSub = () => {
  if (!selected.value || !editingSub.value) return
  const subCategories = selected.value.subCategories || []
  const idx = subCategories.findIndex((s: SubCategoryConfig) => s.id === editingSub.value!.id)
  if (idx !== -1) {
    subCategories[idx] = { ...editingSub.value }
  } else {
    subCategories.push({ ...editingSub.value })
  }
  // 触发响应式更新
  const categoryIndex = categories.value.findIndex((c: CategoryConfig) => c.id === selected.value?.id)
  if (categoryIndex >= 0) {
    const current = categories.value[categoryIndex]
    if (current) {
      categories.value[categoryIndex] = { ...current }
    }
    categories.value = [...categories.value]
  }
  editingSub.value = null
  isAddingSub.value = false
}

const onDeleteSub = (subId: string) => {
  if (!selected.value) return
  if (!confirm('确定删除此子分类？')) return
  const subCategories = selected.value.subCategories || []
  const idx = subCategories.findIndex((s: SubCategoryConfig) => s.id === subId)
  if (idx !== -1) {
    subCategories.splice(idx, 1)
    // 触发响应式更新
    const categoryIndex = categories.value.findIndex((c: CategoryConfig) => c.id === selected.value?.id)
    if (categoryIndex >= 0) {
      const current = categories.value[categoryIndex]
      if (current) {
        categories.value[categoryIndex] = { ...current }
      }
      categories.value = [...categories.value]
    }
  }
  if (editingSub.value?.id === subId) {
    editingSub.value = null
    isAddingSub.value = false
  }
}

const onCancelSub = () => {
  editingSub.value = null
  isAddingSub.value = false
}

// 删除分类
const onDeleteCategory = (categoryId: string) => {
  if (!confirm('确定删除此分类？删除后无法恢复。')) return
  const idx = categories.value.findIndex((c: CategoryConfig) => c.id === categoryId)
  if (idx !== -1) {
    categories.value.splice(idx, 1)
    // 触发响应式更新
    categories.value = [...categories.value]
    // 如果删除的是当前选中的分类，切换到第一个分类
    if (selectedId.value === categoryId) {
      selectedId.value = categories.value[0]?.id ?? null
    }
  }
}

// 监听 selected 的变化，确保通过 v-model 修改属性时也能触发响应式更新
watch(
  () => selected.value,
  (newVal) => {
    if (newVal) {
      // 当 selected 变化时，触发响应式更新
      const categoryIndex = categories.value.findIndex((c: CategoryConfig) => c.id === newVal.id)
      if (categoryIndex >= 0) {
        const current = categories.value[categoryIndex]
        if (current) {
          categories.value[categoryIndex] = { ...current }
        }
        categories.value = [...categories.value]
        // 同步配置到分类数据
        syncCategoryConfigToData(newVal.id)
      }
    }
  },
  { deep: true, immediate: false }
)
</script>

<template>
  <div class="settings-page">
    <aside class="sidebar">
      <div class="sidebar-header">
        <div class="sidebar-title">
          <h2>分类管理</h2>
          <p>拖拽调整顺序，点击编辑详细信息。</p>
        </div>
      </div>
      <div class="category-list">
        <button
          v-for="item in categories"
          :key="item.id"
          type="button"
          class="category-list-item"
          :class="{ active: item.id === selectedId }"
          @click="onSelect(item.id)"
        >
          <span class="color-dot" :style="{ backgroundColor: item.color }" />
          <span class="item-main">
            <span class="item-name">{{ item.label || item.name }}</span>
            <span class="item-sub">{{ item.name }}</span>
          </span>
        </button>

        <button type="button" class="add-category" @click="onAdd">
          <span class="add-symbol">+</span>
          <span>新增分类</span>
        </button>
      </div>
    </aside>

    <section class="editor" v-if="selected">
      <header class="editor-header">
        <div>
          <h2>{{ selected.label || selected.name }}</h2>
          <p>编辑分类的基础信息与展示样式。</p>
        </div>
        <button type="button" class="btn-tiny danger" @click="onDeleteCategory(selected.id)">
          删除分类
        </button>
      </header>

      <div class="editor-body">
        <div class="form-section">
          <h3>基础信息</h3>
          <div class="form-grid">
            <label class="field">
              <span class="field-label">名称（代号）</span>
              <input v-model="selected.name" class="field-input" />
            </label>
            <label class="field">
              <span class="field-label">显示名称</span>
              <input v-model="selected.label" class="field-input" />
            </label>
          </div>

          <label class="field">
            <span class="field-label">简要说明</span>
            <textarea v-model="selected.description" class="field-textarea" rows="3" />
          </label>
        </div>

        <div class="form-section">
          <h3>视觉风格</h3>
          <div class="form-grid">
            <label class="field">
              <span class="field-label">图标</span>
              <div class="icon-row">
                <span class="icon-preview">
                  <span v-if="selected.icon === 'globe'">🌐</span>
                  <span v-else-if="selected.icon === 'apps'">🔧</span>
                  <span v-else-if="selected.icon === 'bug'">🐞</span>
                  <span v-else-if="selected.icon === 'lock'">🔒</span>
                  <span v-else-if="selected.icon === 'search'">🔍</span>
                  <span v-else-if="selected.icon === 'fingerprint'">🆔</span>
                  <span v-else-if="selected.icon === 'link'">🔗</span>
                  <span v-else-if="selected.icon === 'command'">⌘</span>
                  <span v-else>★</span>
                </span>
                <select v-model="selected.icon" class="field-input">
                  <option value="globe">🌐 globe</option>
                  <option value="apps">🔧 apps</option>
                  <option value="bug">🐞 bug</option>
                  <option value="lock">🔒 lock</option>
                  <option value="search">🔍 search</option>
                  <option value="fingerprint">🆔 fingerprint</option>
                  <option value="link">🔗 link</option>
                  <option value="command">⌘ command</option>
                </select>
              </div>
            </label>
            <label class="field">
              <span class="field-label">颜色（HEX）</span>
              <div class="color-row">
                <input v-model="selected.color" type="color" class="color-picker" />
                <input v-model="selected.color" class="field-input" />
              </div>
            </label>
          </div>
        </div>

        <div class="form-section">
          <div class="section-header">
            <h3>子分类管理</h3>
            <button type="button" class="btn-small" @click="onAddSub">+ 新增子分类</button>
          </div>
          <div v-if="selected.subCategories && selected.subCategories.length" class="sub-list-editor">
            <div
              v-for="sub in selected.subCategories"
              :key="sub.id"
              class="sub-item"
              :class="{ editing: editingSub?.id === sub.id }"
            >
              <div class="sub-item-content">
                <div>
                  <div class="sub-item-name">{{ sub.name }}</div>
                  <div class="sub-item-desc">{{ sub.description || '无描述' }}</div>
                </div>
                <div class="sub-item-actions">
                  <button type="button" class="btn-tiny" @click="onEditSub(sub)">编辑</button>
                  <button type="button" class="btn-tiny danger" @click="onDeleteSub(sub.id)">删除</button>
                </div>
              </div>
            </div>
          </div>
          <div v-else class="empty-hint">暂无子分类，点击上方按钮添加</div>

          <div v-if="editingSub" class="sub-form-editor">
            <div class="form-grid">
              <label class="field">
                <span class="field-label">名称</span>
                <input v-model="editingSub.name" class="field-input" />
              </label>
              <label class="field">
                <span class="field-label">描述</span>
                <input v-model="editingSub.description" class="field-input" />
              </label>
            </div>
            <div class="form-actions">
              <button type="button" class="btn ghost" @click="onCancelSub">取消</button>
              <button type="button" class="btn primary" @click="onSaveSub">保存</button>
            </div>
          </div>
        </div>
      </div>

      <footer class="editor-footer">
        <div class="preview">
          <div class="preview-label">预览</div>
          <div class="preview-card" :style="{ '--card-color': selected.color }">
            <div class="preview-icon">
              <span>{{ selected.name.charAt(0) }}</span>
            </div>
            <div class="preview-content">
              <div class="preview-title-row">
                <h4>{{ selected.name }}</h4>
                <span v-if="selected.label" class="preview-label-text">
                  {{ selected.label }}
                </span>
              </div>
              <p>{{ selected.description }}</p>
            </div>
          </div>
        </div>

        <div class="actions">
          <button type="button" class="btn ghost">还原</button>
          <button type="button" class="btn primary">应用</button>
        </div>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.settings-page {
  min-height: 100vh;
  display: flex;
  background-color: var(--bg-primary);
  color: var(--text-primary);
}

.sidebar {
  width: 280px;
  border-right: 1px solid var(--border-color);
  padding: 12px 14px 20px;
  background-color: var(--bg-secondary);
}

.sidebar-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.sidebar-title h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.sidebar-title p {
  margin: 4px 0 12px;
  font-size: 12px;
  color: var(--text-secondary);
}

.category-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.category-list-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s ease;
}

.category-list-item:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.category-list-item.active {
  background-color: var(--bg-active);
  border-color: var(--border-color);
  color: var(--text-primary);
}

.color-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.item-main {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}

.item-name {
  font-size: 13px;
  font-weight: 500;
}

.item-sub {
  font-size: 11px;
  color: var(--text-tertiary);
}

.add-category {
  margin-top: 8px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px dashed var(--border-color);
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s ease;
}

.add-category:hover {
  border-style: solid;
  border-color: var(--primary-color);
  color: var(--primary-color);
  background-color: var(--bg-hover);
}

.add-symbol {
  font-size: 16px;
}

.editor {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 20px 24px;
  background-color: var(--bg-primary);
}

.editor-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 24px;
}

.editor-header > div {
  flex: 1;
}

.editor-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary);
}

.editor-header p {
  margin: 4px 0 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.editor-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.form-section h3 {
  margin: 0 0 12px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
}

.field-input,
.field-textarea {
  border-radius: 6px;
  border: 1px solid var(--input-border);
  background-color: var(--input-bg);
  color: var(--text-primary);
  padding: 8px 10px;
  font-size: 13px;
  outline: none;
  transition: all 0.2s ease;
}

.field-input:focus,
.field-textarea:focus {
  border-color: var(--primary-color);
  box-shadow: 0 0 0 2px var(--primary-color-alpha);
}

.color-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.color-picker {
  width: 40px;
  height: 34px;
  padding: 0;
  border-radius: 6px;
  border: 1px solid var(--input-border);
  background: transparent;
  cursor: pointer;
}

.icon-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.icon-preview {
  width: 34px;
  height: 34px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  font-size: 16px;
}

.editor-footer {
  margin-top: 24px;
  padding-top: 20px;
  border-top: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.preview {
  flex: 1;
}

.preview-label {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.preview-card {
  display: flex;
  align-items: stretch;
  padding: 16px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-secondary);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.preview-icon {
  flex: 0 0 auto;
  width: 40px;
  height: 40px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 12px;
  background-color: var(--bg-tertiary);
  color: var(--card-color);
}

.preview-icon span {
  font-weight: 600;
  font-size: 20px;
}

.preview-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.preview-title-row {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.preview-title-row h4 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.preview-label-text {
  font-size: 12px;
  color: var(--text-secondary);
  background-color: var(--bg-tertiary);
  padding: 2px 6px;
  border-radius: 4px;
}

.preview-content p {
  margin: 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.actions {
  display: flex;
  gap: 10px;
}

.btn {
  min-width: 80px;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.2s ease;
}

.btn.ghost {
  background-color: transparent;
  border-color: var(--border-color);
  color: var(--text-primary);
}

.btn.ghost:hover {
  background-color: var(--bg-hover);
  border-color: var(--text-secondary);
}

.btn.primary {
  background-color: var(--primary-color);
  color: #ffffff;
}

.btn.primary:hover {
  background-color: var(--primary-hover);
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.btn-small {
  padding: 4px 10px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-small:hover {
  background-color: var(--bg-hover);
  border-color: var(--text-secondary);
}

.sub-list-editor {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sub-item {
  padding: 10px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-secondary);
  transition: all 0.2s ease;
}

.sub-item.editing {
  border-color: var(--primary-color);
  background-color: var(--bg-active);
}

.sub-item-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.sub-item-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.sub-item-desc {
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 2px;
}

.sub-item-actions {
  display: flex;
  gap: 6px;
}

.btn-tiny {
  padding: 4px 8px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-tertiary);
  color: var(--text-secondary);
  font-size: 11px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-tiny:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--text-secondary);
}

.btn-tiny.danger {
  color: var(--danger-color);
  border-color: var(--danger-color);
  background-color: transparent;
}

.btn-tiny.danger:hover {
  background-color: var(--danger-color);
  color: #ffffff;
}

.sub-form-editor {
  margin-top: 12px;
  padding: 16px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-secondary);
}

.form-actions {
  display: flex;
  gap: 10px;
  margin-top: 16px;
  justify-content: flex-end;
}

.empty-hint {
  padding: 20px;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 13px;
  border-radius: 6px;
  background-color: var(--bg-secondary);
  border: 1px dashed var(--border-color);
}

@media (max-width: 900px) {
  .settings-page {
    flex-direction: column;
  }

  .sidebar {
    width: 100%;
    border-right: none;
    border-bottom: 1px solid var(--border-color);
  }
}
</style>


