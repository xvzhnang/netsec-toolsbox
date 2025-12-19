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
        <p>管理全局配置。</p>
      </div>
    </header>

    <div class="settings-main">
      <aside class="settings-nav">
        <div class="nav-section">
          <h3 class="nav-section-title">设置</h3>
          <div class="nav-placeholder">
            <p>功能规划中</p>
          </div>
        </div>
      </aside>

      <section class="settings-content">
        <div class="config-panel">
          <div class="panel-header">
            <h2>设置</h2>
            <p>功能规划中</p>
          </div>
          <div class="config-placeholder">
            <div class="placeholder-icon">⚙️</div>
            <h3>功能规划中</h3>
            <p>设置功能正在规划中，敬请期待。</p>
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
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: radial-gradient(circle at top, #020617 0, #000000 75%);
  color: #e5e7eb;
  overflow: hidden;
}

.settings-header {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 22px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.25);
  background: linear-gradient(to bottom, rgba(15, 23, 42, 0.96), rgba(15, 23, 42, 0.92));
  z-index: 10;
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
  color: #e5e7eb;
  letter-spacing: 0.3px;
}

.header-text p {
  margin: 2px 0 0;
  font-size: 12px;
  color: rgba(148, 163, 184, 0.8);
}

.settings-main {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

.settings-nav {
  flex: 0 0 240px;
  padding: 20px 0;
  border-right: 1px solid rgba(148, 163, 184, 0.15);
  background: rgba(15, 23, 42, 0.4);
  overflow-y: auto;
}

.nav-section {
  padding: 0 16px;
}

.nav-section-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: rgba(148, 163, 184, 0.6);
  margin: 0 0 12px;
  padding: 0 12px;
}

.nav-placeholder {
  padding: 20px;
  text-align: center;
  color: rgba(148, 163, 184, 0.6);
}

.settings-content {
  flex: 1;
  min-width: 0;
  padding: 24px 32px;
  overflow-y: auto;
  background: rgba(15, 23, 42, 0.2);
}

.config-panel {
  max-width: 900px;
  margin: 0 auto;
}

.panel-header {
  margin-bottom: 24px;
}

.panel-header h2 {
  margin: 0 0 8px;
  font-size: 24px;
  font-weight: 600;
  color: #e5e7eb;
}

.panel-header p {
  margin: 0;
  font-size: 14px;
  color: rgba(148, 163, 184, 0.8);
}

.config-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  text-align: center;
  background: rgba(15, 23, 42, 0.3);
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.1);
}

.placeholder-icon {
  font-size: 48px;
  margin-bottom: 16px;
  opacity: 0.6;
}

.config-placeholder h3 {
  margin: 0 0 8px;
  font-size: 18px;
  font-weight: 600;
  color: #e5e7eb;
}

.config-placeholder p {
  margin: 0;
  font-size: 14px;
  color: rgba(148, 163, 184, 0.7);
}

.page-footer {
  flex: 0 0 auto;
  padding: 12px 24px;
  border-top: 1px solid rgba(148, 163, 184, 0.15);
  background: rgba(15, 23, 42, 0.4);
}

.footer-content {
  max-width: 1400px;
  margin: 0 auto;
  text-align: center;
}

.copyright {
  font-size: 12px;
  color: rgba(148, 163, 184, 0.6);
}
</style>
