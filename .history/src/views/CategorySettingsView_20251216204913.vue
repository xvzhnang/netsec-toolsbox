<script setup lang="ts">
import { ref, computed } from 'vue'

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

const initialCategories: CategoryConfig[] = [
  {
    id: 'web',
    name: 'WEB',
    label: 'Web 攻击与防御',
    description: 'Web 相关攻击与防御工具集合。',
    icon: 'globe',
    color: '#4DA3FF',
    order: 1,
    enabled: true,
    subCategories: [
      {
        id: 'web-info',
        name: '信息收集',
        description: '基础资产信息、指纹识别、子域名枚举。',
        order: 1,
        enabled: true,
      },
      {
        id: 'web-dir',
        name: '目录与文件扫描',
        description: '敏感目录/文件爆破、备份文件探测。',
        order: 2,
        enabled: true,
      },
    ],
  },
  {
    id: 'misc',
    name: 'MISC',
    label: '杂项工具',
    description: '杂项安全工具与小脚本集合。',
    icon: 'apps',
    color: '#A78BFA',
    order: 2,
    enabled: true,
    subCategories: [],
  },
]

const categories = ref<CategoryConfig[]>([...initialCategories])
const selectedId = ref<string | null>(categories.value[0]?.id ?? null)

const selected = computed({
  get() {
    return categories.value.find((c) => c.id === selectedId.value) ?? null
  },
  set(value) {
    if (!value) return
    const idx = categories.value.findIndex((c) => c.id === value.id)
    if (idx !== -1) {
      categories.value[idx] = { ...value }
    }
  },
})

const onSelect = (id: string) => {
  selectedId.value = id
}

const onAdd = () => {
  const nextOrder =
    categories.value.reduce((max, c) => Math.max(max, c.order), 0) + 1
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
  selectedId.value = newId
}

const editingSub = ref<SubCategoryConfig | null>(null)
const isAddingSub = ref(false)

const onAddSub = () => {
  if (!selected.value) return
  const subCategories = selected.value.subCategories || []
  const nextOrder = subCategories.reduce((max, s) => Math.max(max, s.order), 0) + 1
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
  const idx = subCategories.findIndex((s) => s.id === editingSub.value!.id)
  if (idx !== -1) {
    subCategories[idx] = { ...editingSub.value }
  } else {
    subCategories.push({ ...editingSub.value })
  }
  editingSub.value = null
  isAddingSub.value = false
}

const onDeleteSub = (subId: string) => {
  if (!selected.value) return
  if (!confirm('确定删除此子分类？')) return
  const subCategories = selected.value.subCategories || []
  const idx = subCategories.findIndex((s) => s.id === subId)
  if (idx !== -1) {
    subCategories.splice(idx, 1)
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
  background: radial-gradient(circle at top, #020617 0, #000000 70%);
  color: #e5e7eb;
}

.sidebar {
  width: 280px;
  border-right: 1px solid rgba(148, 163, 184, 0.25);
  padding: 12px 14px 20px;
  background: linear-gradient(to bottom, rgba(15, 23, 42, 0.96), rgba(15, 23, 42, 0.98));
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
}

.sidebar-title p {
  margin: 4px 0 12px;
  font-size: 12px;
  color: #9ca3af;
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
  padding: 6px 8px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: inherit;
  cursor: pointer;
  transition: background 0.16s ease-out;
}

.category-list-item:hover {
  background: rgba(15, 23, 42, 0.9);
}

.category-list-item.active {
  background: rgba(15, 23, 42, 0.98);
  box-shadow: inset 0 0 0 1px rgba(148, 163, 184, 0.4);
}

.color-dot {
  width: 10px;
  height: 10px;
  border-radius: 999px;
}

.item-main {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}

.item-name {
  font-size: 13px;
}

.item-sub {
  font-size: 11px;
  color: #9ca3af;
}

.add-category {
  margin-top: 8px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: 999px;
  border: 1px dashed rgba(148, 163, 184, 0.6);
  background: transparent;
  color: #e5e7eb;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.16s ease-out;
}

.add-category:hover {
  border-style: solid;
  background: rgba(15, 23, 42, 0.95);
}

.add-symbol {
  font-size: 16px;
}

.editor {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 20px 24px;
}

.editor-header h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.editor-header p {
  margin: 4px 0 0;
  font-size: 13px;
  color: #9ca3af;
}

.editor-body {
  flex: 1;
  margin-top: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.form-section h3 {
  margin: 0 0 8px;
  font-size: 14px;
  font-weight: 500;
  color: #9ca3af;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-label {
  font-size: 12px;
  color: #9ca3af;
}

.field-input,
.field-textarea {
  border-radius: 8px;
  border: 1px solid rgba(148, 163, 184, 0.4);
  background: rgba(15, 23, 42, 0.9);
  color: #e5e7eb;
  padding: 6px 8px;
  font-size: 13px;
  outline: none;
  transition: border-color 0.16s ease-out, box-shadow 0.16s ease-out, background 0.16s ease-out;
}

.field-input:focus,
.field-textarea:focus {
  border-color: #4da3ff;
  box-shadow: 0 0 0 1px rgba(77, 163, 255, 0.5);
  background: rgba(15, 23, 42, 0.96);
}

.color-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.color-picker {
  width: 40px;
  height: 28px;
  padding: 0;
  border-radius: 6px;
  border: 1px solid rgba(148, 163, 184, 0.6);
  background: transparent;
}

.icon-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.icon-preview {
  width: 32px;
  height: 32px;
  border-radius: 999px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(148, 163, 184, 0.6);
  font-size: 16px;
}

.editor-footer {
  margin-top: 16px;
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
  color: #9ca3af;
  margin-bottom: 6px;
}

.preview-card {
  display: flex;
  align-items: stretch;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.4);
  background:
    radial-gradient(circle at top left, rgba(148, 163, 184, 0.16), transparent 55%),
    linear-gradient(135deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.94));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 10px 20px rgba(0, 0, 0, 0.7);
}

.preview-icon {
  flex: 0 0 auto;
  width: 32px;
  height: 32px;
  border-radius: 999px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 10px;
  background: radial-gradient(circle at 30% 0, #ffffff30, transparent 55%);
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 0 0 1px color-mix(in srgb, var(--card-color) 40%, transparent);
}

.preview-icon span {
  font-weight: 600;
  font-size: 16px;
  color: color-mix(in srgb, var(--card-color) 80%, #e5e7eb 20%);
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
  gap: 6px;
}

.preview-title-row h4 {
  margin: 0;
  font-size: 14px;
  letter-spacing: 0.14em;
}

.preview-label-text {
  font-size: 11px;
  color: #9ca3af;
}

.preview-content p {
  margin: 0;
  font-size: 12px;
  color: #9ca3af;
}

.actions {
  display: flex;
  gap: 8px;
}

.btn {
  min-width: 80px;
  padding: 6px 14px;
  border-radius: 999px;
  font-size: 13px;
  cursor: pointer;
  border: 1px solid transparent;
  background: transparent;
  color: #e5e7eb;
  transition: all 0.16s ease-out;
}

.btn.ghost {
  border-color: rgba(148, 163, 184, 0.7);
}

.btn.ghost:hover {
  background: rgba(15, 23, 42, 0.96);
}

.btn.primary {
  border-color: #4da3ff;
  background: linear-gradient(135deg, #4da3ff, #22d3ee);
  color: #0b1120;
}

.btn.primary:hover {
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 10px 24px rgba(37, 99, 235, 0.9);
  transform: translateY(-1px);
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.btn-small {
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background: rgba(15, 23, 42, 0.9);
  color: #e5e7eb;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.16s ease-out;
}

.btn-small:hover {
  border-color: #4da3ff;
  background: rgba(15, 23, 42, 0.96);
}

.sub-list-editor {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 8px;
}

.sub-item {
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid rgba(148, 163, 184, 0.3);
  background: rgba(15, 23, 42, 0.85);
  transition: all 0.16s ease-out;
}

.sub-item.editing {
  border-color: #4da3ff;
  background: rgba(77, 163, 255, 0.1);
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
  color: #e5e7eb;
}

.sub-item-desc {
  font-size: 12px;
  color: #9ca3af;
  margin-top: 2px;
}

.sub-item-actions {
  display: flex;
  gap: 6px;
}

.btn-tiny {
  padding: 3px 8px;
  border-radius: 4px;
  border: 1px solid rgba(148, 163, 184, 0.4);
  background: rgba(15, 23, 42, 0.9);
  color: #e5e7eb;
  font-size: 11px;
  cursor: pointer;
  transition: all 0.14s ease-out;
}

.btn-tiny:hover {
  border-color: #4da3ff;
  background: rgba(15, 23, 42, 0.96);
}

.btn-tiny.danger {
  color: #f87171;
  border-color: rgba(248, 113, 113, 0.4);
}

.btn-tiny.danger:hover {
  border-color: #f87171;
  background: rgba(248, 113, 113, 0.1);
}

.sub-form-editor {
  margin-top: 12px;
  padding: 12px;
  border-radius: 8px;
  border: 1px dashed rgba(148, 163, 184, 0.4);
  background: rgba(15, 23, 42, 0.85);
}

.form-actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}

.empty-hint {
  margin-top: 8px;
  padding: 12px;
  text-align: center;
  color: #9ca3af;
  font-size: 12px;
  border-radius: 8px;
  background: rgba(15, 23, 42, 0.6);
}

@media (max-width: 900px) {
  .settings-page {
    flex-direction: column;
  }

  .sidebar {
    width: 100%;
    border-right: none;
    border-bottom: 1px solid rgba(148, 163, 184, 0.25);
  }
}
</style>


