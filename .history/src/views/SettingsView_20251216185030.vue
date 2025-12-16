<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()

const goBack = () => {
  router.back()
}
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
      <section class="settings-content">
        <div class="ai-placeholder">
          <h2>AI 设置（规划中）</h2>
          <p>未来将在此配置 AI 提供商、模型列表以及挂接到 MCP 的安全工具。</p>
          <p>当前阶段仅实现 UI 与配置结构设计，后续接入 Tauri + MCP 后生效。</p>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-root {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: radial-gradient(circle at top, #020617 0, #000000 75%);
  color: #e5e7eb;
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

.settings-content {
  flex: 1;
  min-width: 0;
  padding: 24px;
}

.ai-placeholder {
  padding: 24px;
}

.ai-placeholder h2 {
  margin: 0 0 8px;
  font-size: 18px;
}

.ai-placeholder p {
  margin: 4px 0;
  font-size: 13px;
  color: #9ca3af;
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
  }

  .nav-item {
    min-width: 170px;
  }
}
</style>


