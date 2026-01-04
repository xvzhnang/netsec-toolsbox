import { createRouter, createWebHashHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

// 关键优化：所有视图使用懒加载，减小首屏体积
const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'dashboard',
    component: () => import('./views/DashboardView.vue'),
    meta: {
      transition: 'fade',
    },
  },
  {
    path: '/category/:id',
    name: 'category',
    component: () => import('./views/CategoryView.vue'),
    meta: {
      transition: 'slide',
    },
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('./views/SettingsView.vue'),
    meta: {
      transition: 'fade',
    },
  },
  {
    // 捕获所有未匹配的路由，重定向到首页
    path: '/:pathMatch(.*)*',
    name: 'not-found',
    redirect: '/',
  },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

