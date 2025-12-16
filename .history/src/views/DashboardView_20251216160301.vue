<script setup lang="ts">
import { useRouter } from 'vue-router'

interface CategoryConfig {
  id: string
  name: string
  label?: string
  description?: string
  icon: string
  color: string
  order: number
  enabled: boolean
}

const router = useRouter()

const categories: CategoryConfig[] = [
  {
    id: 'web',
    name: 'WEB',
    label: 'Web 攻击与防御',
    description: 'Web 相关攻击与防御工具集合。',
    icon: 'globe',
    color: '#4DA3FF',
    order: 1,
    enabled: true,
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
  },
  {
    id: 'pwn',
    name: 'PWN',
    label: 'Pwn 漏洞利用',
    description: '二进制漏洞利用与堆栈攻击相关工具。',
    icon: 'bug',
    color: '#FF8F3D',
    order: 3,
    enabled: true,
  },
  {
    id: 'crypto',
    name: 'CRYPTO',
    label: '密码与编码',
    description: '常见密码学算法与编码分析工具。',
    icon: 'lock',
    color: '#2DD4BF',
    order: 4,
    enabled: true,
  },
  {
    id: 're',
    name: 'RE',
    label: '逆向工程',
    description: '逆向分析、调试与文件分析相关工具。',
    icon: 'search',
    color: '#9CA3AF',
    order: 5,
    enabled: true,
  },
  {
    id: 'forensics',
    name: '电子取证',
    label: '电子取证',
    description: '应急响应与电子取证相关辅助工具。',
    icon: 'fingerprint',
    color: '#22D3EE',
    order: 6,
    enabled: true,
  },
  {
    id: 'nav',
    name: '网址导航',
    label: '网址导航',
    description: '常用安全社区、情报源与在线工具导航。',
    icon: 'link',
    color: '#60A5FA',
    order: 7,
    enabled: true,
  },
  {
    id: 'post',
    name: '后渗透',
    label: '后渗透',
    description: '上线后控制、权限提升与横向移动工具。',
    icon: 'command',
    color: '#F87171',
    order: 8,
    enabled: true,
  },
]

const goToSettings = () => {
  router.push({ name: 'category-settings' })
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div class="title-block">
        <h1 class="title">NetSec Toolbox</h1>
        <p class="subtitle">网络攻防工具箱 · 桌面版</p>
      </div>
      <button type="button" class="icon-button" @click="goToSettings">
        <span class="icon">⚙</span>
        <span class="icon-label">分类设置</span>
      </button>
    </header>

    <main class="page-main">
      <div class="card-grid">
        <button
          v-for="category in categories.filter((c) => c.enabled)"
          :key="category.id"
          type="button"
          class="category-card"
          :style="{ '--card-color': category.color }"
        >
          <div class="card-icon">
            <span class="icon-fallback">{{ category.name.charAt(0) }}</span>
          </div>
          <div class="card-content">
            <div class="card-title-row">
              <h2 class="card-title">{{ category.name }}</h2>
              <span v-if="category.label" class="card-label">{{ category.label }}</span>
            </div>
            <p class="card-description">
              {{ category.description }}
            </p>
          </div>
        </button>
      </div>
    </main>
  </div>
</template>

<style scoped>
.page {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: radial-gradient(circle at top, #020617 0, #020617 40%, #000000 100%);
  color: #e5e7eb;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 32px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.2);
  backdrop-filter: blur(14px);
  background: linear-gradient(to bottom, rgba(15, 23, 42, 0.9), rgba(15, 23, 42, 0.7));
}

.title-block {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.subtitle {
  margin: 0;
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
  padding: 24px 32px 32px;
  display: flex;
  justify-content: center;
  align-items: flex-start;
}

.card-grid {
  width: 100%;
  max-width: 1200px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 20px;
}

.category-card {
  position: relative;
  display: flex;
  flex-direction: row;
  align-items: stretch;
  padding: 14px 16px;
  border-radius: 14px;
  border: 1px solid rgba(148, 163, 184, 0.35);
  background:
    radial-gradient(circle at top left, rgba(148, 163, 184, 0.16), transparent 55%),
    linear-gradient(135deg, rgba(15, 23, 42, 0.98), rgba(15, 23, 42, 0.94));
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 0.9),
    0 18px 35px rgba(0, 0, 0, 0.75);
  cursor: pointer;
  text-align: left;
  color: inherit;
  transition: transform 0.2s cubic-bezier(0.22, 0.88, 0.25, 1.05),
    box-shadow 0.2s ease-out,
    border-color 0.2s ease-out,
    background 0.2s ease-out;
}

.category-card::before {
  content: '';
  position: absolute;
  inset: -1px;
  border-radius: inherit;
  background: radial-gradient(circle at top left, color-mix(in srgb, var(--card-color) 45%, transparent), transparent 60%);
  opacity: 0.25;
  pointer-events: none;
  z-index: -1;
}

.category-card:hover {
  transform: translateY(-4px) scale(1.02);
  border-color: color-mix(in srgb, var(--card-color) 70%, #e5e7eb 30%);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--card-color) 40%, rgba(15, 23, 42, 1) 60%),
    0 22px 45px rgba(0, 0, 0, 0.9);
}

.card-icon {
  flex: 0 0 auto;
  width: 42px;
  height: 42px;
  border-radius: 999px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 12px;
  background: radial-gradient(circle at 30% 0, #ffffff30, transparent 55%);
  box-shadow:
    0 0 0 1px rgba(15, 23, 42, 1),
    0 0 0 1px color-mix(in srgb, var(--card-color) 40%, transparent);
}

.icon-fallback {
  font-weight: 600;
  font-size: 18px;
  color: color-mix(in srgb, var(--card-color) 80%, #e5e7eb 20%);
}

.card-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.card-title-row {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.card-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  letter-spacing: 0.16em;
}

.card-label {
  font-size: 12px;
  color: #9ca3af;
}

.card-description {
  margin: 0;
  font-size: 13px;
  color: #9ca3af;
}

@media (max-width: 768px) {
  .page-header {
    padding: 12px 16px;
  }

  .page-main {
    padding: 16px;
  }

  .card-grid {
    gap: 14px;
  }
}
</style>


