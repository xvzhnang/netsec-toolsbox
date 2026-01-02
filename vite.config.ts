import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  build: {
    target: 'esnext',
    minify: 'esbuild',
    sourcemap: false,
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          // Vue 核心库
          if (id.includes('node_modules/vue') || id.includes('node_modules/@vue')) {
            return 'vue-vendor'
          }
          
          // Vue Router
          if (id.includes('node_modules/vue-router')) {
            return 'vue-router'
          }
          
          // Markdown 相关（markdown-it 及其插件）
          if (
            id.includes('node_modules/markdown-it') ||
            id.includes('node_modules/katex') ||
            id.includes('node_modules/marked')
          ) {
            return 'markdown'
          }
          
          // Mermaid 图表库
          if (id.includes('node_modules/mermaid')) {
            return 'mermaid'
          }
          
          // Highlight.js 代码高亮
          if (id.includes('node_modules/highlight.js')) {
            return 'highlight'
          }
          
          // AI 助手组件（懒加载）
          if (id.includes('components/AiAssistantPanel.vue')) {
            return 'ai-assistant'
          }
          
          // Wiki 相关组件
          if (id.includes('components/Wiki') || id.includes('views/WikiView.vue')) {
            return 'wiki'
          }
          
          // 其他 node_modules
          if (id.includes('node_modules')) {
            return 'vendor'
          }
        },
      },
    },
    // 优化：增加 chunk 大小警告阈值
    chunkSizeWarningLimit: 1000,
  },
  optimizeDeps: {
    include: [
      'vue',
      'vue-router',
      'mermaid',
      'highlight.js',
    ],
    // 排除 markdown-it（使用动态加载）
    exclude: ['markdown-it'],
  },
})
