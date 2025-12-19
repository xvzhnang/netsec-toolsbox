<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { readAIConfig, writeAIConfig, type AIConfig, type AIProviderConfig } from '../utils/aiConfig'
import { getTauriInvoke } from '../utils/tauri'
import { debug, error as logError, info, warn } from '../utils/logger'

const router = useRouter()

const goBack = () => {
  router.back()
}

// AI 设置相关状态
const activeAiTab = ref<'providers' | 'mcp'>('providers')

// AI 配置状态
const aiConfig = ref<AIConfig>({})
const isSaving = ref(false)
const saveStatus = ref<'idle' | 'saving' | 'success' | 'error'>('idle')

// 自定义提供商管理
const showAddCustomProvider = ref(false)
const editingCustomProvider = ref<string | null>(null)
const newCustomProviderName = ref('')
const newCustomProviderConfig = ref<AIProviderConfig>({
  enabled: true,
  mode: 'openai',
  api_key: '',
  base_url: '',
  api_url: '',
  model: '',
  temperature: 0.7,
  max_tokens: 2000,
  timeout: 60
})

// 加载配置
const loadConfig = async () => {
  try {
    const config = await readAIConfig()
    
    // 确保配置是对象，不是字符串
    if (typeof config === 'string') {
      try {
        aiConfig.value = JSON.parse(config)
      } catch (parseError) {
        debug('解析配置失败，使用空配置:', parseError)
        aiConfig.value = {}
      }
    } else if (config && typeof config === 'object' && !Array.isArray(config)) {
      aiConfig.value = config
    } else {
      aiConfig.value = {}
    }
    
    debug('AI 配置已加载:', aiConfig.value)
  } catch (error) {
    logError('加载 AI 配置失败:', error)
    aiConfig.value = {}
  }
}

// 保存配置
const saveConfig = async () => {
  isSaving.value = true
  saveStatus.value = 'saving'
  
  try {
    // 确保所有配置都正确保存（包括自定义提供商）
    const configToSave = { ...aiConfig.value }
    
    // 确保 custom_providers 存在
    if (!configToSave.custom_providers) {
      configToSave.custom_providers = {}
    }
    
    await writeAIConfig(configToSave)
    saveStatus.value = 'success'
    info('AI 配置已保存')
    
    // 3 秒后重置状态
    setTimeout(() => {
      saveStatus.value = 'idle'
    }, 3000)
  } catch (error) {
    logError('保存 AI 配置失败:', error)
    saveStatus.value = 'error'
    
    setTimeout(() => {
      saveStatus.value = 'idle'
    }, 3000)
  } finally {
    isSaving.value = false
  }
}

// 添加自定义提供商
const addCustomProvider = () => {
  if (!newCustomProviderName.value.trim()) {
    warn('请输入提供商名称')
    return
  }
  
  if (!aiConfig.value.custom_providers) {
    aiConfig.value.custom_providers = {}
  }
  
  aiConfig.value.custom_providers[newCustomProviderName.value] = {
    ...newCustomProviderConfig.value,
    name: newCustomProviderName.value
  }
  
  // 重置表单
  newCustomProviderName.value = ''
  newCustomProviderConfig.value = {
    enabled: true,
    mode: 'openai',
    api_key: '',
    base_url: '',
    api_url: '',
    model: '',
    temperature: 0.7,
    max_tokens: 2000,
    timeout: 60
  }
  showAddCustomProvider.value = false
  
  info('自定义提供商已添加，请点击"保存配置"保存更改')
}

// 编辑自定义提供商
const editCustomProvider = (name: string) => {
  if (!aiConfig.value.custom_providers || !aiConfig.value.custom_providers[name]) {
    return
  }
  
  editingCustomProvider.value = name
  newCustomProviderName.value = name
  newCustomProviderConfig.value = { ...aiConfig.value.custom_providers[name] }
  showAddCustomProvider.value = true
}

// 更新自定义提供商
const updateCustomProvider = () => {
  if (!editingCustomProvider.value || !newCustomProviderName.value.trim()) {
    return
  }
  
  if (!aiConfig.value.custom_providers) {
    aiConfig.value.custom_providers = {}
  }
  
  // 如果名称改变，删除旧的
  if (editingCustomProvider.value !== newCustomProviderName.value) {
    delete aiConfig.value.custom_providers[editingCustomProvider.value]
  }
  
  // 添加/更新新的配置
  aiConfig.value.custom_providers[newCustomProviderName.value] = {
    ...newCustomProviderConfig.value,
    name: newCustomProviderName.value
  }
  
  // 重置表单
  editingCustomProvider.value = null
  newCustomProviderName.value = ''
  newCustomProviderConfig.value = {
    enabled: true,
    mode: 'openai',
    api_key: '',
    base_url: '',
    api_url: '',
    model: '',
    temperature: 0.7,
    max_tokens: 2000,
    timeout: 60
  }
  showAddCustomProvider.value = false
  
  info('自定义提供商已更新，请点击"保存配置"保存更改')
}

// 删除自定义提供商
const deleteCustomProvider = (name: string) => {
  if (!aiConfig.value.custom_providers || !aiConfig.value.custom_providers[name]) {
    return
  }
  
  if (confirm(`确定要删除自定义提供商 "${name}" 吗？`)) {
    delete aiConfig.value.custom_providers[name]
    info('自定义提供商已删除，请点击"保存配置"保存更改')
  }
}

// 取消编辑
const cancelEditCustomProvider = () => {
  editingCustomProvider.value = null
  newCustomProviderName.value = ''
  newCustomProviderConfig.value = {
    enabled: true,
    mode: 'openai',
    api_key: '',
    base_url: '',
    api_url: '',
    model: '',
    temperature: 0.7,
    max_tokens: 2000,
    timeout: 60
  }
  showAddCustomProvider.value = false
}

// 更新提供商配置
const updateConfig = (provider: 'openai' | 'deepseek' | 'claude' | 'gemini' | 'zhipu' | 'qwen' | 'mistral' | 'ollama' | 'lmstudio' | 'llamacpp', field: keyof AIProviderConfig, value: string | number) => {
  try {
    // 确保 aiConfig.value 是对象
    if (!aiConfig.value || typeof aiConfig.value !== 'object' || Array.isArray(aiConfig.value)) {
      debug('aiConfig.value 不是对象，重置为空对象')
      aiConfig.value = {}
    }
    
    // 确保 provider 配置是对象（检查是否是字符串）
    const providerConfig = aiConfig.value[provider]
    if (typeof providerConfig === 'string') {
      // 如果是字符串，尝试解析
      try {
        const parsed = JSON.parse(providerConfig)
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
          aiConfig.value[provider] = parsed
        } else {
          aiConfig.value[provider] = {}
        }
      } catch {
        aiConfig.value[provider] = {}
      }
    } else if (!providerConfig || typeof providerConfig !== 'object' || Array.isArray(providerConfig)) {
      aiConfig.value[provider] = {}
    }
    
    // 再次确保是对象
    const finalConfig = aiConfig.value[provider]
    if (!finalConfig || typeof finalConfig !== 'object' || Array.isArray(finalConfig)) {
      aiConfig.value[provider] = {}
    }
    
    if (typeof value === 'string' && value.trim() === '') {
      delete (aiConfig.value[provider] as any)[field]
    } else {
      (aiConfig.value[provider] as any)[field] = typeof value === 'string' ? value.trim() : value
    }
  } catch (error) {
    logError('更新配置失败:', error)
    // 重置为安全的空对象
    if (!aiConfig.value || typeof aiConfig.value !== 'object') {
      aiConfig.value = {}
    }
    if (!aiConfig.value[provider] || typeof aiConfig.value[provider] !== 'object') {
      aiConfig.value[provider] = {}
    }
  }
}

// 启动/停止 AI 服务
const aiServiceRunning = ref(false)
const isTogglingService = ref(false)

const checkServiceStatus = async () => {
  const invoker = getTauriInvoke()
  if (!invoker) return
  
  try {
    const status = await invoker('check_ai_service_status')
    aiServiceRunning.value = status as boolean
  } catch (error) {
    debug('检查 AI 服务状态失败:', error)
  }
}

const toggleAIService = async () => {
  const invoker = getTauriInvoke()
  if (!invoker) {
    logError('Tauri API 不可用')
    return
  }
  
  isTogglingService.value = true
  
  try {
    if (aiServiceRunning.value) {
      await invoker('stop_ai_service')
      info('AI 服务已停止')
      aiServiceRunning.value = false
    } else {
      const result = await invoker('start_ai_service')
      info(result || 'AI 服务已启动')
      
      // 等待服务启动（最多等待 5 秒）
      for (let i = 0; i < 10; i++) {
        await new Promise(resolve => setTimeout(resolve, 500))
        const status = await invoker('check_ai_service_status')
        if (status) {
          aiServiceRunning.value = true
          break
        }
      }
    }
    
    // 再次检查状态
    await checkServiceStatus()
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    logError('操作 AI 服务失败:', errorMsg)
    // 显示用户友好的错误提示
    if (errorMsg.includes('不存在')) {
      warn('请确保 Python 313 和 ai_service 目录存在')
    } else if (errorMsg.includes('已在运行')) {
      // 忽略这个错误，只是状态不同步
    } else {
      warn('启动失败，请查看控制台日志获取详细信息')
    }
  } finally {
    isTogglingService.value = false
  }
}

onMounted(async () => {
  await loadConfig()
  await checkServiceStatus()
  
  // 定期检查服务状态
  setInterval(checkServiceStatus, 5000)
})
</script>

<template>
  <div class="settings-root">
    <header class="settings-header">
      <button
        type="button"
        class="back-button"
        title="返回上层"
        @click="goBack"
      >
        ←
      </button>
      <div class="header-text">
        <h1>设置中心</h1>
        <p>管理 AI 以及后续的全局配置。</p>
      </div>
    </header>

    <div class="settings-main">
      <aside class="settings-nav">
        <div class="nav-section">
          <h3 class="nav-section-title">AI 配置</h3>
          <button
            type="button"
            class="nav-item"
            :class="{ active: activeAiTab === 'providers' }"
            @click="activeAiTab = 'providers'"
          >
            <span class="nav-icon">🤖</span>
            <div class="nav-content">
              <span class="nav-title">AI 提供商</span>
              <span class="nav-subtitle">配置模型提供商与 API</span>
            </div>
          </button>
          <button
            type="button"
            class="nav-item"
            :class="{ active: activeAiTab === 'mcp' }"
            @click="activeAiTab = 'mcp'"
          >
            <span class="nav-icon">🔌</span>
            <div class="nav-content">
              <span class="nav-title">MCP 工具</span>
              <span class="nav-subtitle">挂接 MCP 安全工具</span>
            </div>
          </button>
        </div>
      </aside>

      <section class="settings-content">
        <div v-if="activeAiTab === 'providers'" class="ai-config-panel">
          <div class="panel-header">
            <h2>AI 提供商配置</h2>
            <p>配置 AI 模型提供商、API 密钥以及模型列表。</p>
            <div class="service-control">
              <button
                type="button"
                class="service-toggle-btn"
                :class="{ running: aiServiceRunning }"
                :disabled="isTogglingService"
                @click="toggleAIService"
              >
                <span class="status-dot" :class="{ active: aiServiceRunning }"></span>
                {{ aiServiceRunning ? '服务运行中' : '服务已停止' }}
              </button>
            </div>
          </div>
          
          <div class="config-content">
            <!-- OpenAI 配置 -->
            <div class="provider-config">
              <h3>OpenAI</h3>
              <div class="config-form">
                <div class="form-group">
                  <label>API 密钥</label>
                  <input
                    type="password"
                    :value="aiConfig.openai?.api_key || ''"
                    @input="(e) => updateConfig('openai', 'api_key', (e.target as HTMLInputElement).value)"
                    placeholder="sk-..."
                  />
                </div>
                <div class="form-group">
                  <label>模型</label>
                  <input
                    type="text"
                    :value="aiConfig.openai?.model || 'gpt-3.5-turbo'"
                    @input="(e) => updateConfig('openai', 'model', (e.target as HTMLInputElement).value)"
                    placeholder="gpt-3.5-turbo"
                  />
                </div>
                <div class="form-group">
                  <label>Base URL（可选，用于代理）</label>
                  <input
                    type="text"
                    :value="aiConfig.openai?.base_url || ''"
                    @input="(e) => updateConfig('openai', 'base_url', (e.target as HTMLInputElement).value)"
                    placeholder="https://api.openai.com/v1"
                  />
                </div>
              </div>
            </div>
            
            <!-- DeepSeek 配置 -->
            <div class="provider-config">
              <h3>DeepSeek</h3>
              <div class="config-form">
                <div class="form-group">
                  <label>API 密钥</label>
                  <input
                    type="password"
                    :value="aiConfig.deepseek?.api_key || ''"
                    @input="(e) => updateConfig('deepseek', 'api_key', (e.target as HTMLInputElement).value)"
                    placeholder="sk-..."
                  />
                </div>
                <div class="form-group">
                  <label>模型</label>
                  <input
                    type="text"
                    :value="aiConfig.deepseek?.model || 'deepseek-chat'"
                    @input="(e) => updateConfig('deepseek', 'model', (e.target as HTMLInputElement).value)"
                    placeholder="deepseek-chat"
                  />
                </div>
                <div class="form-group">
                  <label>Base URL</label>
                  <input
                    type="text"
                    :value="aiConfig.deepseek?.base_url || 'https://api.deepseek.com/v1'"
                    @input="(e) => updateConfig('deepseek', 'base_url', (e.target as HTMLInputElement).value)"
                    placeholder="https://api.deepseek.com/v1"
                  />
                </div>
                <div class="form-group">
                  <label>Temperature</label>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                    :value="aiConfig.deepseek?.temperature ?? 0.7"
                    @input="(e) => updateConfig('deepseek', 'temperature', parseFloat((e.target as HTMLInputElement).value) || 0.7)"
                    placeholder="0.7"
                  />
                </div>
                <div class="form-group">
                  <label>Max Tokens</label>
                  <input
                    type="number"
                    :value="aiConfig.deepseek?.max_tokens ?? 2000"
                    @input="(e) => updateConfig('deepseek', 'max_tokens', parseInt((e.target as HTMLInputElement).value) || 2000)"
                    placeholder="2000"
                  />
                </div>
                <div class="form-group">
                  <label>超时时间（秒）</label>
                  <input
                    type="number"
                    :value="aiConfig.deepseek?.timeout ?? 60"
                    @input="(e) => updateConfig('deepseek', 'timeout', parseInt((e.target as HTMLInputElement).value) || 60)"
                    placeholder="60"
                  />
                </div>
              </div>
            </div>
            
            <!-- Claude 配置 -->
            <div class="provider-config">
              <h3>Claude (Anthropic)</h3>
              <div class="config-form">
                <div class="form-group">
                  <label>API 密钥</label>
                  <input
                    type="password"
                    :value="aiConfig.claude?.api_key || ''"
                    @input="(e) => updateConfig('claude', 'api_key', (e.target as HTMLInputElement).value)"
                    placeholder="sk-ant-..."
                  />
                </div>
                <div class="form-group">
                  <label>模型</label>
                  <input
                    type="text"
                    :value="aiConfig.claude?.model || 'claude-3-5-sonnet-20241022'"
                    @input="(e) => updateConfig('claude', 'model', (e.target as HTMLInputElement).value)"
                    placeholder="claude-3-5-sonnet-20241022"
                  />
                </div>
                <div class="form-group">
                  <label>Temperature</label>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                    :value="aiConfig.claude?.temperature ?? 0.7"
                    @input="(e) => updateConfig('claude', 'temperature', parseFloat((e.target as HTMLInputElement).value) || 0.7)"
                    placeholder="0.7"
                  />
                </div>
                <div class="form-group">
                  <label>Max Tokens</label>
                  <input
                    type="number"
                    :value="aiConfig.claude?.max_tokens ?? 4096"
                    @input="(e) => updateConfig('claude', 'max_tokens', parseInt((e.target as HTMLInputElement).value) || 4096)"
                    placeholder="4096"
                  />
                </div>
                <div class="form-group">
                  <label>超时时间（秒）</label>
                  <input
                    type="number"
                    :value="aiConfig.claude?.timeout ?? 60"
                    @input="(e) => updateConfig('claude', 'timeout', parseInt((e.target as HTMLInputElement).value) || 60)"
                    placeholder="60"
                  />
                </div>
              </div>
            </div>
            
            <!-- Gemini 配置 -->
            <div class="provider-config">
              <h3>Gemini (Google)</h3>
              <div class="config-form">
                <div class="form-group">
                  <label>API 密钥</label>
                  <input
                    type="password"
                    :value="aiConfig.gemini?.api_key || ''"
                    @input="(e) => updateConfig('gemini', 'api_key', (e.target as HTMLInputElement).value)"
                    placeholder="..."
                  />
                </div>
                <div class="form-group">
                  <label>模型</label>
                  <input
                    type="text"
                    :value="aiConfig.gemini?.model || 'gemini-pro'"
                    @input="(e) => updateConfig('gemini', 'model', (e.target as HTMLInputElement).value)"
                    placeholder="gemini-pro"
                  />
                </div>
                <div class="form-group">
                  <label>Temperature</label>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                    :value="aiConfig.gemini?.temperature ?? 0.7"
                    @input="(e) => updateConfig('gemini', 'temperature', parseFloat((e.target as HTMLInputElement).value) || 0.7)"
                    placeholder="0.7"
                  />
                </div>
                <div class="form-group">
                  <label>Max Tokens</label>
                  <input
                    type="number"
                    :value="aiConfig.gemini?.max_tokens ?? 2048"
                    @input="(e) => updateConfig('gemini', 'max_tokens', parseInt((e.target as HTMLInputElement).value) || 2048)"
                    placeholder="2048"
                  />
                </div>
                <div class="form-group">
                  <label>超时时间（秒）</label>
                  <input
                    type="number"
                    :value="aiConfig.gemini?.timeout ?? 60"
                    @input="(e) => updateConfig('gemini', 'timeout', parseInt((e.target as HTMLInputElement).value) || 60)"
                    placeholder="60"
                  />
                </div>
              </div>
            </div>
            
            <!-- 智谱AI 配置 -->
            <div class="provider-config">
              <h3>智谱AI (GLM)</h3>
              <div class="config-form">
                <div class="form-group">
                  <label>API 密钥</label>
                  <input
                    type="password"
                    :value="aiConfig.zhipu?.api_key || ''"
                    @input="(e) => updateConfig('zhipu', 'api_key', (e.target as HTMLInputElement).value)"
                    placeholder="..."
                  />
                </div>
                <div class="form-group">
                  <label>模型</label>
                  <input
                    type="text"
                    :value="aiConfig.zhipu?.model || 'glm-4'"
                    @input="(e) => updateConfig('zhipu', 'model', (e.target as HTMLInputElement).value)"
                    placeholder="glm-4"
                  />
                </div>
                <div class="form-group">
                  <label>Temperature</label>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                    :value="aiConfig.zhipu?.temperature ?? 0.7"
                    @input="(e) => updateConfig('zhipu', 'temperature', parseFloat((e.target as HTMLInputElement).value) || 0.7)"
                    placeholder="0.7"
                  />
                </div>
                <div class="form-group">
                  <label>Max Tokens</label>
                  <input
                    type="number"
                    :value="aiConfig.zhipu?.max_tokens ?? 2048"
                    @input="(e) => updateConfig('zhipu', 'max_tokens', parseInt((e.target as HTMLInputElement).value) || 2048)"
                    placeholder="2048"
                  />
                </div>
                <div class="form-group">
                  <label>超时时间（秒）</label>
                  <input
                    type="number"
                    :value="aiConfig.zhipu?.timeout ?? 60"
                    @input="(e) => updateConfig('zhipu', 'timeout', parseInt((e.target as HTMLInputElement).value) || 60)"
                    placeholder="60"
                  />
                </div>
              </div>
            </div>
            
            <!-- 通义千问 配置 -->
            <div class="provider-config">
              <h3>通义千问 (Qwen)</h3>
              <div class="config-form">
                <div class="form-group">
                  <label>API 密钥</label>
                  <input
                    type="password"
                    :value="aiConfig.qwen?.api_key || ''"
                    @input="(e) => updateConfig('qwen', 'api_key', (e.target as HTMLInputElement).value)"
                    placeholder="..."
                  />
                </div>
                <div class="form-group">
                  <label>模型</label>
                  <input
                    type="text"
                    :value="aiConfig.qwen?.model || 'qwen-turbo'"
                    @input="(e) => updateConfig('qwen', 'model', (e.target as HTMLInputElement).value)"
                    placeholder="qwen-turbo"
                  />
                </div>
                <div class="form-group">
                  <label>Temperature</label>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                    :value="aiConfig.qwen?.temperature ?? 0.7"
                    @input="(e) => updateConfig('qwen', 'temperature', parseFloat((e.target as HTMLInputElement).value) || 0.7)"
                    placeholder="0.7"
                  />
                </div>
                <div class="form-group">
                  <label>Max Tokens</label>
                  <input
                    type="number"
                    :value="aiConfig.qwen?.max_tokens ?? 2000"
                    @input="(e) => updateConfig('qwen', 'max_tokens', parseInt((e.target as HTMLInputElement).value) || 2000)"
                    placeholder="2000"
                  />
                </div>
                <div class="form-group">
                  <label>超时时间（秒）</label>
                  <input
                    type="number"
                    :value="aiConfig.qwen?.timeout ?? 60"
                    @input="(e) => updateConfig('qwen', 'timeout', parseInt((e.target as HTMLInputElement).value) || 60)"
                    placeholder="60"
                  />
                </div>
              </div>
            </div>
            
            <!-- Mistral 配置 -->
            <div class="provider-config">
              <h3>Mistral AI</h3>
              <div class="config-form">
                <div class="form-group">
                  <label>API 密钥</label>
                  <input
                    type="password"
                    :value="aiConfig.mistral?.api_key || ''"
                    @input="(e) => updateConfig('mistral', 'api_key', (e.target as HTMLInputElement).value)"
                    placeholder="..."
                  />
                </div>
                <div class="form-group">
                  <label>模型</label>
                  <input
                    type="text"
                    :value="aiConfig.mistral?.model || 'mistral-medium'"
                    @input="(e) => updateConfig('mistral', 'model', (e.target as HTMLInputElement).value)"
                    placeholder="mistral-medium"
                  />
                </div>
                <div class="form-group">
                  <label>Base URL</label>
                  <input
                    type="text"
                    :value="aiConfig.mistral?.base_url || 'https://api.mistral.ai/v1'"
                    @input="(e) => updateConfig('mistral', 'base_url', (e.target as HTMLInputElement).value)"
                    placeholder="https://api.mistral.ai/v1"
                  />
                </div>
                <div class="form-group">
                  <label>Temperature</label>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                    :value="aiConfig.mistral?.temperature ?? 0.7"
                    @input="(e) => updateConfig('mistral', 'temperature', parseFloat((e.target as HTMLInputElement).value) || 0.7)"
                    placeholder="0.7"
                  />
                </div>
                <div class="form-group">
                  <label>Max Tokens</label>
                  <input
                    type="number"
                    :value="aiConfig.mistral?.max_tokens ?? 2000"
                    @input="(e) => updateConfig('mistral', 'max_tokens', parseInt((e.target as HTMLInputElement).value) || 2000)"
                    placeholder="2000"
                  />
                </div>
                <div class="form-group">
                  <label>超时时间（秒）</label>
                  <input
                    type="number"
                    :value="aiConfig.mistral?.timeout ?? 60"
                    @input="(e) => updateConfig('mistral', 'timeout', parseInt((e.target as HTMLInputElement).value) || 60)"
                    placeholder="60"
                  />
                </div>
              </div>
            </div>
            
            <!-- Ollama 配置 -->
            <div class="provider-config">
              <h3>Ollama（本地模型）</h3>
              <div class="config-form">
                <div class="form-group">
                  <label>API URL</label>
                  <input
                    type="text"
                    :value="aiConfig.ollama?.api_url || 'http://localhost:11434'"
                    @input="(e) => updateConfig('ollama', 'api_url', (e.target as HTMLInputElement).value)"
                    placeholder="http://localhost:11434"
                  />
                </div>
                <div class="form-group">
                  <label>模型名称</label>
                  <input
                    type="text"
                    :value="aiConfig.ollama?.model || 'llama2'"
                    @input="(e) => updateConfig('ollama', 'model', (e.target as HTMLInputElement).value)"
                    placeholder="llama2"
                  />
                </div>
                <div class="form-group">
                  <label>超时时间（秒）</label>
                  <input
                    type="number"
                    :value="aiConfig.ollama?.timeout || 120"
                    @input="(e) => updateConfig('ollama', 'timeout', (e.target as HTMLInputElement).value)"
                    placeholder="120"
                  />
                </div>
              </div>
            </div>
            
            <!-- LM Studio 配置 -->
            <div class="provider-config">
              <h3>LM Studio（本地模型）</h3>
              <div class="config-form">
                <div class="form-group">
                  <label>API URL</label>
                  <input
                    type="text"
                    :value="aiConfig.lmstudio?.api_url || 'http://localhost:1234/v1'"
                    @input="(e) => updateConfig('lmstudio', 'api_url', (e.target as HTMLInputElement).value)"
                    placeholder="http://localhost:1234/v1"
                  />
                </div>
                <div class="form-group">
                  <label>模型名称</label>
                  <input
                    type="text"
                    :value="aiConfig.lmstudio?.model || 'local-model'"
                    @input="(e) => updateConfig('lmstudio', 'model', (e.target as HTMLInputElement).value)"
                    placeholder="local-model"
                  />
                </div>
                <div class="form-group">
                  <label>超时时间（秒）</label>
                  <input
                    type="number"
                    :value="aiConfig.lmstudio?.timeout || 120"
                    @input="(e) => updateConfig('lmstudio', 'timeout', parseInt((e.target as HTMLInputElement).value) || 120)"
                    placeholder="120"
                  />
                </div>
              </div>
            </div>
            
            <!-- llama.cpp 配置 -->
            <div class="provider-config">
              <h3>llama.cpp（本地模型）</h3>
              <div class="config-form">
                <div class="form-group">
                  <label>API URL</label>
                  <input
                    type="text"
                    :value="aiConfig.llamacpp?.api_url || 'http://localhost:8080/v1'"
                    @input="(e) => updateConfig('llamacpp', 'api_url', (e.target as HTMLInputElement).value)"
                    placeholder="http://localhost:8080/v1"
                  />
                </div>
                <div class="form-group">
                  <label>模型名称</label>
                  <input
                    type="text"
                    :value="aiConfig.llamacpp?.model || 'local'"
                    @input="(e) => updateConfig('llamacpp', 'model', (e.target as HTMLInputElement).value)"
                    placeholder="local"
                  />
                </div>
                <div class="form-group">
                  <label>超时时间（秒）</label>
                  <input
                    type="number"
                    :value="aiConfig.llamacpp?.timeout || 120"
                    @input="(e) => updateConfig('llamacpp', 'timeout', parseInt((e.target as HTMLInputElement).value) || 120)"
                    placeholder="120"
                  />
                </div>
              </div>
            </div>
            
            <!-- 自定义提供商管理 -->
            <div class="custom-providers-section">
              <div class="section-header">
                <h3>自定义 AI 提供商</h3>
                <button
                  type="button"
                  class="add-button"
                  @click="showAddCustomProvider = true"
                >
                  + 添加自定义提供商
                </button>
              </div>
              
              <!-- 自定义提供商列表 -->
              <div v-if="aiConfig.custom_providers && Object.keys(aiConfig.custom_providers).length > 0" class="custom-providers-list">
                <div
                  v-for="(provider, name) in aiConfig.custom_providers"
                  :key="name"
                  class="custom-provider-item"
                >
                  <div class="provider-info">
                    <span class="provider-name">{{ name }}</span>
                    <span class="provider-mode">{{ provider.mode === 'openai' ? 'OpenAI 兼容' : '自定义 HTTP' }}</span>
                    <span class="provider-status" :class="{ enabled: provider.enabled }">
                      {{ provider.enabled ? '已启用' : '已禁用' }}
                    </span>
                  </div>
                  <div class="provider-actions">
                    <button
                      type="button"
                      class="edit-button"
                      @click="editCustomProvider(name)"
                    >
                      编辑
                    </button>
                    <button
                      type="button"
                      class="delete-button"
                      @click="deleteCustomProvider(name)"
                    >
                      删除
                    </button>
                  </div>
                </div>
              </div>
              <div v-else class="empty-state">
                <p>暂无自定义提供商</p>
                <p class="hint">点击"添加自定义提供商"按钮添加新的 AI 服务</p>
              </div>
              
              <!-- 添加/编辑自定义提供商表单 -->
              <div v-if="showAddCustomProvider" class="custom-provider-form">
                <h4>{{ editingCustomProvider ? '编辑自定义提供商' : '添加自定义提供商' }}</h4>
                
                <div class="form-group">
                  <label>提供商名称 *</label>
                  <input
                    type="text"
                    v-model="newCustomProviderName"
                    :disabled="!!editingCustomProvider"
                    placeholder="例如: my-custom-ai"
                  />
                </div>
                
                <div class="form-group">
                  <label>模式 *</label>
                  <select v-model="newCustomProviderConfig.mode">
                    <option value="openai">OpenAI 兼容模式（推荐）</option>
                    <option value="custom">自定义 HTTP 模式</option>
                  </select>
                </div>
                
                <div class="form-group">
                  <label>API 密钥 *</label>
                  <input
                    type="password"
                    v-model="newCustomProviderConfig.api_key"
                    placeholder="输入 API 密钥"
                  />
                </div>
                
                <!-- OpenAI 兼容模式配置 -->
                <template v-if="newCustomProviderConfig.mode === 'openai'">
                  <div class="form-group">
                    <label>Base URL *</label>
                    <input
                      type="text"
                      v-model="newCustomProviderConfig.base_url"
                      placeholder="https://api.example.com/v1"
                    />
                  </div>
                  <div class="form-group">
                    <label>模型名称 *</label>
                    <input
                      type="text"
                      v-model="newCustomProviderConfig.model"
                      placeholder="model-name"
                    />
                  </div>
                </template>
                
                <!-- 自定义 HTTP 模式配置 -->
                <template v-else>
                  <div class="form-group">
                    <label>API URL *</label>
                    <input
                      type="text"
                      v-model="newCustomProviderConfig.api_url"
                      placeholder="https://api.example.com"
                    />
                  </div>
                  <div class="form-group">
                    <label>端点路径</label>
                    <input
                      type="text"
                      v-model="newCustomProviderConfig.endpoint"
                      placeholder="/v1/chat"
                    />
                  </div>
                  <div class="form-group">
                    <label>模型名称</label>
                    <input
                      type="text"
                      v-model="newCustomProviderConfig.model"
                      placeholder="model-name"
                    />
                  </div>
                  <div class="form-group">
                    <label>认证类型</label>
                    <select v-model="newCustomProviderConfig.auth_type">
                      <option value="bearer">Bearer Token</option>
                      <option value="header">自定义 Header</option>
                      <option value="api_key">API Key</option>
                    </select>
                  </div>
                  <div v-if="newCustomProviderConfig.auth_type === 'header'" class="form-group">
                    <label>认证 Header 名称</label>
                    <input
                      type="text"
                      v-model="newCustomProviderConfig.auth_header"
                      placeholder="X-API-Key"
                    />
                  </div>
                  <div class="form-group">
                    <label>响应路径</label>
                    <input
                      type="text"
                      v-model="newCustomProviderConfig.response_path"
                      placeholder="choices[0].message.content"
                    />
                    <small>JSON 路径，用于提取响应文本</small>
                  </div>
                </template>
                
                <!-- 通用配置 -->
                <div class="form-group">
                  <label>Temperature</label>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                    v-model.number="newCustomProviderConfig.temperature"
                    placeholder="0.7"
                  />
                </div>
                <div class="form-group">
                  <label>Max Tokens</label>
                  <input
                    type="number"
                    v-model.number="newCustomProviderConfig.max_tokens"
                    placeholder="2000"
                  />
                </div>
                <div class="form-group">
                  <label>超时时间（秒）</label>
                  <input
                    type="number"
                    v-model.number="newCustomProviderConfig.timeout"
                    placeholder="60"
                  />
                </div>
                
                <div class="form-actions">
                  <button
                    type="button"
                    class="save-btn"
                    @click="editingCustomProvider ? updateCustomProvider() : addCustomProvider()"
                  >
                    {{ editingCustomProvider ? '更新' : '添加' }}
                  </button>
                  <button
                    type="button"
                    class="cancel-button"
                    @click="cancelEditCustomProvider"
                  >
                    取消
                  </button>
                </div>
              </div>
            </div>
            
            <!-- 保存按钮 -->
            <div class="config-actions">
              <button
                type="button"
                class="save-btn"
                :disabled="isSaving"
                @click="saveConfig"
              >
                {{ saveStatus === 'saving' ? '保存中...' : saveStatus === 'success' ? '✓ 已保存' : saveStatus === 'error' ? '✗ 保存失败' : '保存配置' }}
              </button>
            </div>
          </div>
        </div>

        <div v-else-if="activeAiTab === 'mcp'" class="ai-config-panel">
          <div class="panel-header">
            <h2>MCP 工具配置</h2>
            <p>配置 MCP（Model Context Protocol）工具，扩展 AI 助手能力。</p>
          </div>
          <div class="config-placeholder">
            <div class="placeholder-icon">🔌</div>
            <h3>功能规划中</h3>
            <p>未来将支持挂接 MCP 工具，让 AI 助手能够调用安全工具。</p>
            <p>当前阶段仅实现 UI 结构设计，后续接入 Tauri + MCP 后生效。</p>
          </div>
        </div>
      </section>
    </div>

    <footer class="page-footer">
      <div class="footer-content">
        <span class="copyright">© 2025 By 序章</span>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.settings-root {
  height: 100vh; /* 固定高度为视口高度 */
  display: flex;
  flex-direction: column;
  background: radial-gradient(circle at top, #020617 0, #000000 75%);
  color: #e5e7eb;
  overflow: hidden; /* 固定整体页面 */
}

.settings-header {
  flex: 0 0 auto; /* 固定头部，不伸缩 */
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 22px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.25);
  background: linear-gradient(to bottom, rgba(15, 23, 42, 0.96), rgba(15, 23, 42, 0.92));
  z-index: 10; /* 确保头部在最上层 */
}

.back-button {
  flex: 0 0 auto;
  width: 28px;
  height: 28px;
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

.header-text h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.header-text p {
  margin: 2px 0 0;
  font-size: 13px;
  color: #9ca3af;
}

.settings-main {
  flex: 1;
  display: flex;
  min-height: 0;
}

.settings-nav {
  width: 280px;
  padding: 16px 12px;
  border-right: 1px solid rgba(148, 163, 184, 0.2);
  background: linear-gradient(to bottom, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.96));
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  /* 确保可以接收鼠标滚轮事件 */
  overscroll-behavior: contain;
  /* 优化滚动条样式 */
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.4) rgba(15, 23, 42, 0.9);
}

.settings-nav::-webkit-scrollbar {
  width: 6px;
}

.settings-nav::-webkit-scrollbar-track {
  background: rgba(15, 23, 42, 0.9);
}

.settings-nav::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.4);
  border-radius: 3px;
}

.settings-nav::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.6);
}

.nav-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.nav-section-title {
  margin: 0 0 4px 0;
  padding: 8px 10px;
  font-size: 12px;
  font-weight: 600;
  color: #9ca3af;
  text-transform: uppercase;
  letter-spacing: 0.1em;
}

.nav-item {
  text-align: left;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid transparent;
  background: transparent;
  color: #e5e7eb;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 10px;
  transition: all 0.16s ease-out;
}

.nav-item:hover {
  background: rgba(15, 23, 42, 0.96);
  border-color: rgba(148, 163, 184, 0.4);
}

.nav-item.active {
  background: rgba(15, 23, 42, 0.98);
  border-color: rgba(77, 163, 255, 0.6);
  box-shadow: 0 0 0 1px rgba(15, 23, 42, 0.9);
}

.nav-icon {
  font-size: 20px;
  flex-shrink: 0;
}

.nav-content {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}

.nav-title {
  font-size: 14px;
  font-weight: 500;
}

.nav-subtitle {
  font-size: 12px;
  color: #9ca3af;
}

.settings-content {
  flex: 1;
  min-width: 0;
  min-height: 0;
  padding: 24px;
  overflow-y: auto;
  overflow-x: hidden;
  /* 确保可以接收鼠标滚轮事件 */
  overscroll-behavior: contain;
  /* 优化滚动条样式 */
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.4) rgba(15, 23, 42, 0.9);
}

.settings-content::-webkit-scrollbar {
  width: 8px;
}

.settings-content::-webkit-scrollbar-track {
  background: rgba(15, 23, 42, 0.9);
}

.settings-content::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.4);
  border-radius: 4px;
}

.settings-content::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.6);
}

.ai-config-panel {
  display: flex;
  flex-direction: column;
  gap: 20px;
  height: 100%;
}

.panel-header {
  padding-bottom: 16px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.2);
}

.panel-header h2 {
  margin: 0 0 6px 0;
  font-size: 20px;
  font-weight: 600;
}

.panel-header p {
  margin: 0;
  font-size: 14px;
  color: #9ca3af;
}

.service-control {
  margin-top: 12px;
}

.service-toggle-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-radius: 6px;
  border: 1px solid rgba(148, 163, 184, 0.3);
  background: rgba(15, 23, 42, 0.6);
  color: #e5e7eb;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}

.service-toggle-btn:hover:not(:disabled) {
  background: rgba(15, 23, 42, 0.8);
  border-color: rgba(148, 163, 184, 0.5);
}

.service-toggle-btn.running {
  border-color: rgba(34, 197, 94, 0.5);
  background: rgba(34, 197, 94, 0.1);
}

.service-toggle-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #ef4444;
  transition: background 0.2s;
}

.status-dot.active {
  background: #22c55e;
}

.config-content {
  flex: 1;
  overflow-y: auto;
  padding: 20px 0;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.provider-config {
  padding: 20px;
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.2);
  background: rgba(15, 23, 42, 0.4);
}

.provider-config h3 {
  margin: 0 0 16px 0;
  font-size: 16px;
  font-weight: 600;
  color: #e5e7eb;
}

.config-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-group label {
  font-size: 13px;
  color: #9ca3af;
  font-weight: 500;
}

.form-group input {
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid rgba(148, 163, 184, 0.3);
  background: rgba(15, 23, 42, 0.8);
  color: #e5e7eb;
  font-size: 13px;
  outline: none;
  transition: border-color 0.2s;
}

.form-group input:focus {
  border-color: #4da3ff;
}

.form-group input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.form-hint {
  margin: 8px 0 0 0;
  font-size: 12px;
  color: #6b7280;
}

.config-actions {
  padding: 20px;
  border-top: 1px solid rgba(148, 163, 184, 0.2);
  display: flex;
  justify-content: flex-end;
}

.save-btn {
  padding: 10px 20px;
  border-radius: 8px;
  border: 1px solid #4da3ff;
  background: linear-gradient(135deg, #4da3ff, #22d3ee);
  color: #0b1120;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.save-btn:hover:not(:disabled) {
  box-shadow: 0 4px 12px rgba(77, 163, 255, 0.4);
  transform: translateY(-1px);
}

.save-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.config-placeholder {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 24px;
  text-align: center;
  gap: 16px;
}

.placeholder-icon {
  font-size: 64px;
  opacity: 0.6;
}

.config-placeholder h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: #e5e7eb;
}

.config-placeholder p {
  margin: 4px 0;
  font-size: 14px;
  color: #9ca3af;
  max-width: 500px;
  line-height: 1.6;
}

@media (max-width: 900px) {
  .settings-main {
    flex-direction: column;
  }

  .settings-nav {
    width: 100%;
    border-right: none;
    border-bottom: 1px solid rgba(148, 163, 184, 0.2);
    flex-direction: row;
    overflow-x: auto;
    padding: 12px;
  }

  .nav-item {
    min-width: 200px;
  }

  .nav-content {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .nav-subtitle {
    display: none;
  }
}

.page-footer {
  flex: 0 0 auto; /* 固定底部，不伸缩 */
  padding: 16px 32px;
  border-top: 1px solid rgba(148, 163, 184, 0.1);
  background: rgba(15, 23, 42, 0.3);
  backdrop-filter: blur(8px);
  z-index: 10; /* 确保底部在最上层 */
}

.footer-content {
  display: flex;
  justify-content: center;
  align-items: center;
}

.copyright {
  font-size: 12px;
  color: #9ca3af;
  letter-spacing: 0.05em;
}
</style>


