<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ContextMenu, { type MenuItem } from '../components/ContextMenu.vue'

interface ToolItem {
  id: string
  name: string
  description?: string
  iconEmoji?: string
  wikiUrl?: string
  execPath?: string
  args?: string[]
  workingDir?: string
}

interface SubCategory {
  id: string
  name: string
  description?: string
  tools: ToolItem[]
}

interface CategoryPageData {
  id: string
  name: string
  label?: string
  description?: string
  subCategories: SubCategory[]
}

const route = useRoute()
const router = useRouter()

const categories = ref<CategoryPageData[]>([
  {
    id: 'web',
    name: 'WEB',
    label: 'Web 攻击与防御',
    description: '面向 Web 场景的信息收集、扫描与利用工具集合。',
    subCategories: [
      {
        id: 'web-info',
        name: '信息收集',
        description: '基础资产信息、指纹识别、子域名枚举。',
        tools: [
          {
            id: 'host-info',
            name: '主机信息探测',
            description: '对域名/IP 进行 whois、地理位置、ASN 等查询。',
            iconEmoji: '🌐',
            execPath: 'C:\\\\Tools\\\\whois.exe',
          },
          {
            id: 'subdomain',
            name: '子域名收集器',
            description: '综合被动源与爆破，对目标域名进行子域枚举。',
            iconEmoji: '🧭',
            execPath: 'C:\\\\Tools\\\\subfinder.exe',
            args: ['-d', 'example.com'],
          },
          {
            id: 'fingerprint',
            name: '网站指纹识别',
            description: '识别 Web 服务器、中间件、CMS 与常见 WAF。',
            iconEmoji: '🔍',
            execPath: 'C:\\\\Tools\\\\fingerprint.exe',
          },
        ],
      },
      {
        id: 'web-dir',
        name: '目录与文件扫描',
        description: '敏感目录/文件爆破、备份文件探测。',
        tools: [
          {
            id: 'dirscan',
            name: '目录扫描',
            description: '基于字典的目录/文件暴破，可设置线程与状态过滤。',
            iconEmoji: '📂',
            execPath: 'C:\\\\Tools\\\\dirscan.exe',
          },
          {
            id: 'backup-scan',
            name: '备份文件探测',
            description: '常见备份与历史文件名探测，支持自定义规则。',
            iconEmoji: '🗂️',
            execPath: 'C:\\\\Tools\\\\backupscan.exe',
          },
        ],
      },
      {
        id: 'web-port',
        name: '端口与服务探测',
        description: 'Web 相关端口扫描与服务识别。',
        tools: [
          {
            id: 'web-nmap',
            name: 'Web 端口扫描',
            description: '快速扫描常见 Web 端口并识别服务。',
            iconEmoji: '📡',
            execPath: 'C:\\\\Tools\\\\nmap.exe',
            args: ['-Pn', '-sV'],
          },
        ],
      },
      {
        id: 'web-vuln',
        name: '漏洞探测与利用',
        description: '常见 Web 漏洞扫描与 POC/EXP 执行。',
        tools: [
          {
            id: 'poc-runner',
            name: 'POC 运行器',
            description: '管理与运行多种 Web POC，统一输出结果。',
            iconEmoji: '⚡',
            execPath: 'C:\\\\Tools\\\\pocrunner.exe',
          },
        ],
      },
    ],
  },
])

const categoryId = computed(() => (route.params.id as string) || 'web')
const category = computed(
  () => categories.value.find((c) => c.id === categoryId.value) ?? categories.value[0],
)

const selectedSubId = ref<string | null>(category.value?.subCategories[0]?.id ?? null)
const searchQuery = ref('')

const subCategories = computed(() => category.value?.subCategories ?? [])

const currentSub = computed(() =>
  subCategories.value.find((s) => s.id === selectedSubId.value) ?? subCategories.value[0],
)

const tools = computed(() => currentSub.value?.tools ?? [])

const filteredTools = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return tools.value
  return tools.value.filter(
    (t) =>
      t.name.toLowerCase().includes(q) ||
      (t.description && t.description.toLowerCase().includes(q)),
  )
})

const selectSub = (id: string) => {
  selectedSubId.value = id
  const target = subCategories.value.find((s) => s.id === id)
  if (target) {
    subForm.value = { id: target.id, name: target.name, description: target.description ?? '' }
    toolForm.value = emptyToolForm()
    editingToolId.value = null
  }
}

const goBack = () => {
  router.back()
}

const openTool = async (toolId: string) => {
  const tool = tools.value.find((t) => t.id === toolId)
  if (!tool) return
  // 占位：调用后端命令启动外部程序；若后端未实现，则降级为日志
  try {
    const invoker = (window as any).__TAURI__?.invoke
    if (invoker && tool.execPath) {
      await invoker('launch_tool', {
        execPath: tool.execPath,
        args: tool.args ?? [],
        workingDir: tool.workingDir ?? null,
      })
    } else {
      console.log('launch tool (placeholder):', tool.execPath || tool.name, tool.args)
    }
  } catch (err) {
    console.error('launch tool failed', err)
    alert('启动工具失败，请检查配置或后端命令实现。')
  }
}

const openWiki = (wikiUrl?: string) => {
  if (!wikiUrl) return
  window.open(wikiUrl, '_blank')
}

const goSettings = () => {
  router.push({ name: 'settings' })
}

const openWikiHome = () => {
  try {
    const invoker = (window as any).__TAURI__?.invoke
    if (invoker) {
      invoker('start_wiki_server').catch(() => {})
    }
    window.open('http://127.0.0.1:8777', '_blank')
  } catch (err) {
    console.error(err)
  }
}

const onOverlayClick = (toolId: string) => {
  openTool(toolId)
  searchQuery.value = ''
}

const subForm = ref<{ id: string; name: string; description: string }>({
  id: '',
  name: '',
  description: '',
})
const isNewSub = ref(false)

const startNewSub = () => {
  subForm.value = {
    id: `sub_${Date.now()}`,
    name: '',
    description: '',
  }
  isNewSub.value = true
}

const editSub = (sub: SubCategory) => {
  subForm.value = { id: sub.id, name: sub.name, description: sub.description ?? '' }
  isNewSub.value = false
}

const saveSub = () => {
  if (!subForm.value.name.trim()) return
  const list = subCategories.value
  const idx = list.findIndex((s) => s.id === subForm.value.id)
  if (idx >= 0) {
    list[idx] = {
      ...list[idx],
      name: subForm.value.name.trim(),
      description: subForm.value.description.trim(),
    }
  } else {
    list.push({
      id: subForm.value.id,
      name: subForm.value.name.trim(),
      description: subForm.value.description.trim(),
      tools: [],
    })
  }
  selectedSubId.value = subForm.value.id
  isNewSub.value = false
}

const deleteSub = (id: string) => {
  const list = subCategories.value
  const idx = list.findIndex((s) => s.id === id)
  if (idx >= 0) {
    list.splice(idx, 1)
    if (selectedSubId.value === id) {
      selectedSubId.value = list[0]?.id ?? null
    }
  }
}

const emptyToolForm = () => ({
  id: '',
  name: '',
  description: '',
  iconEmoji: '🛠️',
  execPath: '',
  argsText: '',
})

const toolForm = ref<{
  id: string
  name: string
  description: string
  iconEmoji: string
  execPath: string
  argsText: string
}>({
  ...emptyToolForm(),
})
const editingToolId = ref<string | null>(null)

const startNewTool = () => {
  toolForm.value = { ...emptyToolForm(), id: `tool_${Date.now()}` }
  editingToolId.value = null
}

const editTool = (tool: ToolItem) => {
  toolForm.value = {
    id: tool.id,
    name: tool.name,
    description: tool.description ?? '',
    iconEmoji: tool.iconEmoji || '🛠️',
    execPath: tool.execPath || '',
    argsText: tool.args?.join(' ') || '',
  }
  editingToolId.value = tool.id
}

const saveTool = () => {
  if (!currentSub.value) return
  if (!toolForm.value.name.trim()) return
  const args = toolForm.value.argsText
    .split(' ')
    .map((s) => s.trim())
    .filter(Boolean)
  const list = currentSub.value.tools
  const idx = list.findIndex((t) => t.id === toolForm.value.id)
  const base: ToolItem = {
    id: toolForm.value.id,
    name: toolForm.value.name.trim(),
    description: toolForm.value.description.trim(),
    iconEmoji: toolForm.value.iconEmoji || '🛠️',
    execPath: toolForm.value.execPath || undefined,
    args: args.length ? args : undefined,
  }
  if (idx >= 0) {
    list[idx] = { ...list[idx], ...base }
  } else {
    list.push(base)
  }
  editingToolId.value = null
}

const deleteTool = (id: string) => {
  if (!currentSub.value) return
  const idx = currentSub.value.tools.findIndex((t) => t.id === id)
  if (idx >= 0) currentSub.value.tools.splice(idx, 1)
}

const contextMenuRef = ref<InstanceType<typeof ContextMenu> | null>(null)
const contextMenuVisible = ref(false)
const contextMenuType = ref<'sub' | 'tool' | null>(null)
const contextMenuTarget = ref<SubCategory | ToolItem | null>(null)

const showSubMenu = (e: MouseEvent, sub: SubCategory) => {
  e.preventDefault()
  e.stopPropagation()
  contextMenuType.value = 'sub'
  contextMenuTarget.value = sub
  if (contextMenuRef.value) {
    contextMenuRef.value.show(e.clientX, e.clientY)
  }
  contextMenuVisible.value = true
}

const showToolMenu = (e: MouseEvent, tool: ToolItem) => {
  e.preventDefault()
  e.stopPropagation()
  contextMenuType.value = 'tool'
  contextMenuTarget.value = tool
  if (contextMenuRef.value) {
    contextMenuRef.value.show(e.clientX, e.clientY)
  }
  contextMenuVisible.value = true
}

const subMenuItems = computed<MenuItem[]>(() => {
  if (contextMenuType.value !== 'sub' || !contextMenuTarget.value) return []
  const sub = contextMenuTarget.value as SubCategory
  return [
    {
      label: '编辑子分类',
      icon: '✏️',
      action: () => editSub(sub),
    },
    {
      label: '删除子分类',
      icon: '🗑️',
      action: () => {
        if (confirm(`确定删除子分类「${sub.name}」？`)) {
          deleteSub(sub.id)
        }
      },
      danger: true,
    },
  ]
})

const toolMenuItems = computed<MenuItem[]>(() => {
  if (contextMenuType.value !== 'tool' || !contextMenuTarget.value) return []
  const tool = contextMenuTarget.value as ToolItem
  return [
    {
      label: '编辑工具',
      icon: '✏️',
      action: () => editTool(tool),
    },
    {
      label: '打开工具',
      icon: '▶️',
      action: () => openTool(tool.id),
    },
    ...(tool.wikiUrl
      ? [
          {
            label: '在 Wiki 中查看',
            icon: '📚',
            action: () => openWiki(tool.wikiUrl),
          },
        ]
      : []),
    {
      label: '删除工具',
      icon: '🗑️',
      action: () => {
        if (confirm(`确定删除工具「${tool.name}」？`)) {
          deleteTool(tool.id)
        }
      },
      danger: true,
    },
  ]
})

const contextMenuItems = computed<MenuItem[]>(() => {
  if (contextMenuType.value === 'sub') return subMenuItems.value
  if (contextMenuType.value === 'tool') return toolMenuItems.value
  return []
})

const closeContextMenu = () => {
  contextMenuVisible.value = false
  contextMenuType.value = null
  contextMenuTarget.value = null
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div class="header-left">
        <button type="button" class="back-button" title="返回上层" @click="goBack">
          ←
        </button>
        <div class="title-block">
          <h1 class="title">{{ category?.name }}</h1>
          <p class="subtitle">
            {{ category?.label }} · {{ category?.description }}
          </p>
        </div>
      </div>
      <div class="header-actions">
        <button type="button" class="icon-button" @click="openWikiHome">
          <span class="icon">📚</span>
          <span class="icon-label">Wiki</span>
        </button>
        <button type="button" class="icon-button" @click="goSettings">
          <span class="icon">⚙</span>
          <span class="icon-label">设置</span>
        </button>
      </div>
    </header>

    <main class="page-main">
      <div class="search-row">
        <div class="search-box">
          <span class="search-icon">🔍</span>
          <input
            v-model="searchQuery"
            class="search-input"
            type="search"
            placeholder="搜索当前子分类的工具名称或描述"
          />
        </div>
      </div>
      <div class="content-row">
        <aside class="sub-list">
          <div class="sub-title">子分类</div>
          <div class="sub-cards">
            <button
              v-for="sub in subCategories"
              :key="sub.id"
              type="button"
              class="sub-card"
              :class="{ active: sub.id === selectedSubId }"
              @click="selectSub(sub.id)"
              @contextmenu="showSubMenu($event, sub)"
            >
              <div class="sub-name">{{ sub.name }}</div>
              <div class="sub-desc">{{ sub.description }}</div>
            </button>
          </div>
          <div class="sub-form">
            <div class="sub-form-header">
              <div class="sub-form-title">{{ isNewSub ? '新增子分类' : '编辑子分类' }}</div>
              <button type="button" class="chip primary" @click="startNewSub">新建</button>
            </div>
            <label class="field">
              <span class="field-label">名称</span>
              <input v-model="subForm.name" class="field-input" placeholder="请输入子分类名称" />
            </label>
            <label class="field">
              <span class="field-label">描述</span>
              <textarea
                v-model="subForm.description"
                class="field-textarea"
                rows="2"
                placeholder="简单说明"
              />
            </label>
            <button type="button" class="btn primary full" @click="saveSub">保存子分类</button>
          </div>
        </aside>

        <section class="tools-area">
          <div class="tools-header">
            <div>
              <h2>{{ currentSub?.name }}</h2>
              <p>{{ currentSub?.description }}</p>
            </div>
            <div class="tools-header-actions">
              <button type="button" class="icon-button" @click="startNewTool">
                <span class="icon">＋</span>
                <span class="icon-label">新增工具</span>
              </button>
            </div>
          </div>

          <div class="tool-form">
            <div class="sub-form-title">{{ editingToolId ? '编辑工具' : '新增工具' }}</div>
            <div class="tool-form-grid">
              <label class="field">
                <span class="field-label">名称</span>
                <input v-model="toolForm.name" class="field-input" placeholder="工具名称" />
              </label>
              <label class="field">
                <span class="field-label">图标(emoji)</span>
                <input v-model="toolForm.iconEmoji" class="field-input" placeholder="例如 🛠️" />
              </label>
            </div>
            <label class="field">
              <span class="field-label">描述</span>
              <textarea
                v-model="toolForm.description"
                class="field-textarea"
                rows="2"
                placeholder="工具用途简介"
              />
            </label>
            <div class="tool-form-grid">
              <label class="field">
                <span class="field-label">可执行路径</span>
                <input v-model="toolForm.execPath" class="field-input" placeholder="C:\\Tools\\tool.exe" />
              </label>
              <label class="field">
                <span class="field-label">参数(空格分隔)</span>
                <input v-model="toolForm.argsText" class="field-input" placeholder="-d example.com -v" />
              </label>
            </div>
            <div class="tool-form-actions">
              <button type="button" class="btn primary" @click="saveTool">保存工具</button>
              <button type="button" class="btn ghost" @click="startNewTool">重置</button>
            </div>
          </div>

          <div
            v-if="searchQuery && filteredTools.length"
            class="search-overlay"
          >
            <div class="overlay-title">搜索结果</div>
            <div class="overlay-list">
              <button
                v-for="tool in filteredTools"
                :key="tool.id"
                type="button"
                class="overlay-item"
                @click="onOverlayClick(tool.id)"
              >
                <span class="overlay-icon">{{ tool.iconEmoji || '🛠️' }}</span>
                <span class="overlay-text">
                  <span class="overlay-name">{{ tool.name }}</span>
                  <span class="overlay-desc">{{ tool.description }}</span>
                </span>
                <span class="overlay-action">打开</span>
              </button>
            </div>
          </div>

          <div class="tools-grid">
            <div
              v-for="tool in filteredTools"
              :key="tool.id"
              class="tool-card"
            >
              <div class="tool-header">
                <div class="tool-icon">{{ tool.iconEmoji || '🛠️' }}</div>
                <div class="tool-name">{{ tool.name }}</div>
              </div>
              <p class="tool-desc">{{ tool.description }}</p>
              <div class="tool-actions">
                <div class="tool-meta">工具ID：{{ tool.id }}</div>
                <div class="tool-buttons">
                  <button type="button" class="btn ghost" @click="editTool(tool)">编辑</button>
                  <button type="button" class="btn danger" @click="deleteTool(tool.id)">删除</button>
                  <button type="button" class="btn ghost" @click="openWiki(tool.wikiUrl)">📚</button>
                  <button type="button" class="btn primary" @click="openTool(tool.id)">打开工具</button>
                </div>
              </div>
            </div>
          </div>
        </section>
      </div>
    </main>
  </div>
</template>

<style scoped>
.page {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: radial-gradient(circle at top, #020617 0, #000000 80%);
  color: #e5e7eb;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 24px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.2);
  background: linear-gradient(to bottom, rgba(15, 23, 42, 0.96), rgba(15, 23, 42, 0.9));
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.back-button {
  width: 30px;
  height: 30px;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.7);
  background: rgba(15, 23, 42, 0.98);
  color: #e5e7eb;
  cursor: pointer;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.16s ease-out;
}

.back-button:hover {
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 8px 18px rgba(15, 23, 42, 0.9);
  transform: translateY(-1px);
}

.title-block h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.title-block p {
  margin: 2px 0 0;
  font-size: 13px;
  color: #9ca3af;
}

.icon-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background: radial-gradient(circle at top left, rgba(148, 163, 184, 0.12), rgba(15, 23, 42, 0.95));
  color: #e5e7eb;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.18s ease-out;
}

.icon-button:hover {
  border-color: #e5e7eb;
  box-shadow: 0 0 0 1px rgba(148, 163, 184, 0.5), 0 12px 24px rgba(15, 23, 42, 0.9);
  transform: translateY(-1px);
}

.icon {
  font-size: 14px;
}

.icon-label {
  white-space: nowrap;
}

.page-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 16px 16px;
}

.search-row {
  display: flex;
  justify-content: center;
}

.search-box {
  position: relative;
  width: 100%;
  max-width: 760px;
  margin: 0 auto;
}

.search-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 14px;
  color: #6b7280;
  pointer-events: none;
}

.search-input {
  width: 100%;
  padding: 8px 12px 8px 30px;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.5);
  background: rgba(15, 23, 42, 0.95);
  color: #e5e7eb;
  font-size: 13px;
  outline: none;
  transition: border-color 0.16s ease-out, box-shadow 0.16s ease-out, background 0.16s ease-out;
}

.search-input::placeholder {
  color: #6b7280;
}

.search-input:focus {
  border-color: #4da3ff;
  box-shadow: 0 0 0 1px rgba(77, 163, 255, 0.5);
  background: rgba(15, 23, 42, 0.98);
}

.content-row {
  display: flex;
  gap: 14px;
  width: 100%;
  align-items: flex-start;
}

.sub-list {
  flex: 0 0 260px;
  border-right: 1px solid rgba(148, 163, 184, 0.2);
  padding-right: 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.sub-title {
  font-size: 13px;
  color: #9ca3af;
  margin-bottom: 10px;
}

.sub-cards {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sub-card {
  text-align: left;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.3);
  background: rgba(15, 23, 42, 0.92);
  color: #e5e7eb;
  cursor: pointer;
  transition: all 0.16s ease-out;
}

.sub-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 10px 20px rgba(0, 0, 0, 0.45);
}

.sub-card.active {
  border-color: #4da3ff;
  background: linear-gradient(135deg, rgba(77, 163, 255, 0.12), rgba(15, 23, 42, 0.96));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 14px 30px rgba(0, 0, 0, 0.6);
}

.sub-name {
  font-size: 14px;
  font-weight: 600;
}

.sub-desc {
  font-size: 12px;
  color: #9ca3af;
  margin-top: 2px;
}

.sub-actions {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}

.sub-form {
  border: 1px dashed rgba(148, 163, 184, 0.4);
  border-radius: 12px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: rgba(15, 23, 42, 0.85);
}

.sub-form-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.sub-form-title {
  font-size: 13px;
  color: #e5e7eb;
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

.tools-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  position: relative;
}

.tools-header h2 {
  margin: 0;
  font-size: 16px;
}

.tools-header p {
  margin: 4px 0 0;
  color: #9ca3af;
  font-size: 13px;
}

.tools-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.tools-header-actions {
  display: flex;
  align-items: center;
}

.tool-form {
  border: 1px dashed rgba(148, 163, 184, 0.4);
  border-radius: 12px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: rgba(15, 23, 42, 0.85);
}

.tool-form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 8px;
}

.tool-form-actions {
  display: flex;
  gap: 6px;
}

.tools-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 14px;
}

.tool-card {
  padding: 12px;
  border-radius: 14px;
  border: 1px solid rgba(148, 163, 184, 0.3);
  background:
    radial-gradient(circle at top left, rgba(148, 163, 184, 0.14), transparent 55%),
    linear-gradient(140deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.94));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 16px 32px rgba(0, 0, 0, 0.75);
  display: flex;
  flex-direction: column;
  gap: 8px;
  transition: transform 0.18s ease-out, box-shadow 0.18s ease-out, border-color 0.18s ease-out;
}

.tool-card:hover {
  transform: translateY(-3px);
  border-color: rgba(77, 163, 255, 0.7);
  box-shadow:
    0 0 0 1px rgba(77, 163, 255, 0.3),
    0 22px 40px rgba(0, 0, 0, 0.85);
}

.tool-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.tool-icon {
  width: 38px;
  height: 38px;
  border-radius: 12px;
  background: rgba(77, 163, 255, 0.08);
  border: 1px solid rgba(148, 163, 184, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
}

.tool-name {
  font-size: 15px;
  font-weight: 600;
}

.tool-desc {
  margin: 0;
  font-size: 13px;
  color: #9ca3af;
  line-height: 1.5;
}

.tool-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-top: auto;
}

.tool-meta {
  font-size: 12px;
  color: #9ca3af;
}

.tool-buttons {
  display: flex;
  gap: 6px;
}

.search-overlay {
  position: absolute;
  inset: 0;
  background: rgba(2, 6, 23, 0.78);
  backdrop-filter: blur(6px);
  border-radius: 14px;
  border: 1px solid rgba(148, 163, 184, 0.35);
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 18px 40px rgba(0, 0, 0, 0.75);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 5;
}

.overlay-title {
  font-size: 13px;
  color: #9ca3af;
}

.overlay-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  overflow-y: auto;
}

.overlay-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid rgba(148, 163, 184, 0.25);
  background: rgba(15, 23, 42, 0.9);
  color: #e5e7eb;
  cursor: pointer;
  text-align: left;
  transition: all 0.16s ease-out;
}

.overlay-item:hover {
  border-color: rgba(77, 163, 255, 0.6);
  box-shadow: 0 10px 22px rgba(0, 0, 0, 0.65);
}

.overlay-icon {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: rgba(77, 163, 255, 0.1);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
}

.overlay-text {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.overlay-name {
  font-size: 14px;
  font-weight: 600;
}

.overlay-desc {
  font-size: 12px;
  color: #9ca3af;
  line-height: 1.4;
}

.overlay-action {
  font-size: 12px;
  color: #4da3ff;
}

.btn {
  border-radius: 999px;
  border: 1px solid transparent;
  padding: 6px 12px;
  font-size: 12px;
  cursor: pointer;
  color: #e5e7eb;
  background: transparent;
  transition: all 0.16s ease-out;
}

.btn.ghost {
  border-color: rgba(148, 163, 184, 0.6);
}

.btn.ghost:hover {
  background: rgba(15, 23, 42, 0.96);
}

.btn.danger {
  border-color: #f87171;
  color: #fca5a5;
}

.btn.danger:hover {
  background: rgba(248, 113, 113, 0.12);
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 10px 20px rgba(248, 113, 113, 0.35);
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

.chip {
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.6);
  background: transparent;
  color: #e5e7eb;
  padding: 4px 8px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.14s ease-out;
}

.chip:hover {
  background: rgba(15, 23, 42, 0.96);
}

.chip.primary {
  border-color: #4da3ff;
  color: #4da3ff;
}

.chip.danger {
  border-color: #f87171;
  color: #fca5a5;
}

.btn.full {
  width: 100%;
}

@media (max-width: 960px) {
  .page-main {
    padding-top: 12px;
    gap: 6px;
  }

  .content-row {
    flex-direction: column;
  }

  .sub-list {
    width: 100%;
    border-right: none;
    border-bottom: 1px solid rgba(148, 163, 184, 0.2);
    padding-bottom: 14px;
    margin-bottom: 8px;
  }
}
</style>


