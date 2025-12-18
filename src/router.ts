import { createRouter, createWebHashHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import DashboardView from './views/DashboardView.vue'
import SettingsView from './views/SettingsView.vue'
import CategoryView from './views/CategoryView.vue'
import WikiView from './views/WikiView.vue'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'dashboard',
    component: DashboardView,
    meta: {
      transition: 'fade',
    },
  },
  {
    path: '/category/:id',
    name: 'category',
    component: CategoryView,
    meta: {
      transition: 'slide',
    },
  },
  {
    path: '/settings',
    name: 'settings',
    component: SettingsView,
    meta: {
      transition: 'fade',
    },
  },
  {
    path: '/wiki',
    name: 'wiki',
    component: WikiView,
    meta: {
      transition: 'fade',
    },
    props: (route) => ({
      filePath: route.query.filePath as string | undefined,
      toolId: route.query.toolId as string | undefined,
      toolName: route.query.toolName as string | undefined,
    }),
  },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

