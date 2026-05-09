<script setup lang="ts">
import { RouterView, RouterLink, useRoute } from 'vue-router'

const route = useRoute()

const navItems = [
  { path: '/config', label: '配置目录', icon: '📁' },
  { path: '/scan', label: '智能扫描', icon: '🔍' },
  { path: '/results', label: '结果看板', icon: '📊' },
]
</script>

<template>
  <div class="flex h-screen bg-gray-900 text-gray-100 overflow-hidden">
    <!-- Left Sidebar -->
    <aside class="w-56 bg-gray-800 border-r border-gray-700 flex flex-col shrink-0">
      <!-- Logo area -->
      <div class="px-4 py-5 border-b border-gray-700">
        <div class="flex items-center gap-2">
          <span class="text-2xl">🧹</span>
          <span class="font-semibold text-lg text-white">微信清理工具</span>
        </div>
        <div class="text-xs text-gray-500 mt-1 ml-9">v0.1.0</div>
      </div>

      <!-- Navigation -->
      <nav class="flex-1 py-3">
        <RouterLink
          v-for="item in navItems"
          :key="item.path"
          :to="item.path"
          class="flex items-center gap-3 px-4 py-2.5 mx-2 rounded-lg text-sm transition-all duration-150"
          :class="
            route.path === item.path
              ? 'bg-blue-600/20 text-blue-400 font-medium'
              : 'text-gray-400 hover:bg-gray-700/50 hover:text-gray-200'
          "
        >
          <span class="text-lg">{{ item.icon }}</span>
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>

      <!-- Footer -->
      <div class="px-4 py-3 border-t border-gray-700 text-xs text-gray-600">
        WeChat Cleaner · Tauri
      </div>
    </aside>

    <!-- Main Content Area -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- Top Bar -->
      <header class="h-14 bg-gray-800/50 border-b border-gray-700 flex items-center px-6 shrink-0">
        <h1 class="text-base font-medium text-gray-300">
          🧹 微信清理工具
          <span class="text-xs text-gray-600 ml-2">v0.1.0</span>
        </h1>
      </header>

      <!-- Page Content -->
      <main class="flex-1 overflow-hidden p-6">
        <RouterView />
      </main>
    </div>
  </div>
</template>
