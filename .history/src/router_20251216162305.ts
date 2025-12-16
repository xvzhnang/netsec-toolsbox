import { createRouter, createWebHashHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import DashboardView from './views/DashboardView.vue'
import SettingsView from './views/SettingsView.vue'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'dashboard',
    component: DashboardView,
  },
  {
    path: '/settings',
    name: 'settings',
    component: SettingsView,
  },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})


