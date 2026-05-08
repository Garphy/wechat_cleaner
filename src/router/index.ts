import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    { path: '/', redirect: '/config' },
    { path: '/config', name: 'config', component: () => import('../views/ConfigView.vue') },
    { path: '/scan', name: 'scan', component: () => import('../views/ScanView.vue') },
    { path: '/results', name: 'results', component: () => import('../views/ResultView.vue') },
  ],
})

export default router
