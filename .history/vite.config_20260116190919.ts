import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'
import viteCompression from 'vite-plugin-compression'
import { visualizer } from 'rollup-plugin-visualizer'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    viteCompression({
      verbose: true,
      disable: false,
      threshold: 10240, // 10kb
      algorithm: 'gzip',
      ext: '.gz',
    }),
    viteCompression({
      verbose: true,
      disable: false,
      threshold: 10240,
      algorithm: 'brotliCompress',
      ext: '.br',
    }),
    visualizer({
      open: false,
      gzipSize: true,
      brotliSize: true,
      filename: 'stats.html'
    }),
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  build: {
    target: 'esnext',
    minify: 'esbuild', // esbuild is faster and good enough
    sourcemap: false,
    cssCodeSplit: true, // Enable CSS splitting
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          // Vue Core
          if (id.includes('node_modules/vue') || id.includes('node_modules/@vue') || id.includes('node_modules/pinia')) {
            return 'vue-core'
          }
          
          // Vue Router
          if (id.includes('node_modules/vue-router')) {
            return 'vue-router'
          }
          
          // Markdown related (heavy)
          if (
            id.includes('node_modules/markdown-it') ||
            id.includes('node_modules/katex') ||
            id.includes('node_modules/marked')
          ) {
            return 'markdown-libs'
          }
          
          // Chart/Diagram related (heavy)
          if (id.includes('node_modules/mermaid')) {
            return 'mermaid-libs'
          }
          
          // Syntax Highlighting (heavy)
          if (id.includes('node_modules/highlight.js')) {
            return 'highlight-libs'
          }
          
          // Components that are large or independent
          if (id.includes('components/AiAssistantPanel.vue')) {
            return 'comp-ai-assistant'
          }
          
          if (id.includes('components/Wiki') || id.includes('views/WikiView.vue')) {
            return 'feat-wiki'
          }
        },
      },
    },
    // Increase chunk size warning limit to reduce noise for intentional large chunks
    chunkSizeWarningLimit: 1000,
  },
  optimizeDeps: {
    include: [
      'vue',
      'vue-router',
      'pinia',
      'mermaid',
      'highlight.js',
    ],
    // Exclude markdown-it to ensure it's handled by our split logic if needed, 
    // but usually include is better for dev performance. 
    // Keeping it default or included is fine.
  },
})
