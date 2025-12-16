import { createRouter, createWebHashHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import DashboardView from './views/DashboardView.vue'
import SettingsView from './views/SettingsView.vue'
import CategoryView from './views/CategoryView.vue'

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
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

