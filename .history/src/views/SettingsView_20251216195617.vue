<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()

const goBack = () => {
  router.back()
}

// AI 设置相关状态
const activeAiTab = ref<'providers' | 'mcp'>('providers')
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
          </div>
          <div class="config-placeholder">
            <div class="placeholder-icon">🤖</div>
            <h3>功能规划中</h3>
            <p>未来将支持配置多个 AI 提供商（OpenAI、DeepSeek、本地模型等）。</p>
            <p>当前阶段仅实现 UI 结构设计，后续接入 Tauri + MCP 后生效。</p>
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
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 22px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.25);
  background: linear-gradient(to bottom, rgba(15, 23, 42, 0.96), rgba(15, 23, 42, 0.92));
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
  overflow-y: auto;
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
  padding: 24px;
  overflow-y: auto;
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
  margin-top: auto;
  padding: 16px 32px;
  border-top: 1px solid rgba(148, 163, 184, 0.1);
  background: rgba(15, 23, 42, 0.3);
  backdrop-filter: blur(8px);
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


