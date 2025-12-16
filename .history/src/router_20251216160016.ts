import { createRouter, createWebHashHistory, RouteRecordRaw } from 'vue-router'
import DashboardView from './views/DashboardView.vue'
import CategorySettingsView from './views/CategorySettingsView.vue'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'dashboard',
    component: DashboardView,
  },
  {
    path: '/settings/categories',
    name: 'category-settings',
    component: CategorySettingsView,
  },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})


