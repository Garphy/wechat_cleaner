<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore, type WechatAccount, type AppConfig } from '../stores/app'

const router = useRouter()
const store = useAppStore()

const loading = ref(true)
const error = ref<string | null>(null)
const wechatDir = ref('')
const archiveDirs = ref<string[]>([])
const newArchiveDir = ref('')
const accounts = ref<WechatAccount[]>([])
const selectedAccount = ref<string | null>(null)
const saving = ref(false)

function formatPath(path: string): string {
  if (!path) return ''
  const parts = path.split('/')
  if (parts.length > 3) {
    return '...' + parts.slice(-3).join('/')
  }
  return path
}

onMounted(async () => {
  try {
    // Detect wechat paths
    const detectedAccounts = await invoke<WechatAccount[]>('detect_wechat_paths')
    accounts.value = detectedAccounts
    store.accounts = detectedAccounts

    // Get existing config
    const existingConfig = await invoke<AppConfig>('get_config')
    store.config = existingConfig

    if (existingConfig) {
      wechatDir.value = existingConfig.wechat_dir || ''
      archiveDirs.value = existingConfig.archive_dirs || []
      selectedAccount.value = existingConfig.selected_account
    }

    // Auto-select first account if detected but no config set
    if (detectedAccounts.length > 0 && !wechatDir.value) {
      const firstAccount = detectedAccounts[0]!
      wechatDir.value = firstAccount.data_path
      selectedAccount.value = firstAccount.wxid
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
})

function addArchiveDir() {
  const dir = newArchiveDir.value.trim()
  if (dir && !archiveDirs.value.includes(dir)) {
    archiveDirs.value.push(dir)
    newArchiveDir.value = ''
  }
}

function removeArchiveDir(index: number) {
  archiveDirs.value.splice(index, 1)
}

async function startScan() {
  saving.value = true
  error.value = null
  try {
    const config: AppConfig = {
      wechat_dir: wechatDir.value,
      archive_dirs: archiveDirs.value,
      selected_account: selectedAccount.value,
      trash_mode: 'trash',
    }
    await invoke('save_config', { config })
    store.config = config
    router.push('/scan')
  } catch (e) {
    error.value = String(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="max-w-2xl mx-auto space-y-6">
    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-20">
      <div class="flex items-center gap-3 text-gray-400">
        <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24" fill="none">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
        </svg>
        <span>正在检测微信目录...</span>
      </div>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="bg-red-900/30 border border-red-800 rounded-lg p-4 text-red-300 text-sm">
      <div class="font-medium mb-1">检测出错</div>
      {{ error }}
    </div>

    <!-- Config Form -->
    <template v-else>
      <!-- Wechat Directory -->
      <div class="bg-gray-800 rounded-xl border border-gray-700 p-5">
        <h2 class="text-lg font-semibold text-white mb-4 flex items-center gap-2">
          <span class="text-xl">📁</span> 微信目录
        </h2>

        <div v-if="accounts.length > 0" class="mb-4">
          <label class="block text-sm text-gray-400 mb-2">检测到的微信账号</label>
          <select
            v-model="selectedAccount"
            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-gray-200 focus:outline-none focus:border-blue-500 transition-colors"
          >
            <option v-for="account in accounts" :key="account.wxid" :value="account.wxid">
              {{ account.name }} ({{ account.wxid }})
            </option>
          </select>
        </div>

        <label class="block text-sm text-gray-400 mb-2">微信数据目录</label>
        <div v-if="wechatDir" class="bg-gray-700/50 rounded-lg px-3 py-2 text-sm text-gray-300 mb-3 font-mono">
          {{ wechatDir }}
        </div>
        <div v-else class="bg-gray-700/50 rounded-lg px-3 py-2 text-sm text-gray-500 italic mb-3">
          未检测到微信目录
        </div>

        <input
          v-model="wechatDir"
          type="text"
          placeholder="手动输入微信目录路径..."
          class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-blue-500 transition-colors"
        />
      </div>

      <!-- Archive Directories -->
      <div class="bg-gray-800 rounded-xl border border-gray-700 p-5">
        <h2 class="text-lg font-semibold text-white mb-4 flex items-center gap-2">
          <span class="text-xl">📦</span> 归档目录
        </h2>

        <div v-if="archiveDirs.length === 0" class="text-sm text-gray-500 italic mb-3">
          暂无归档目录
        </div>

        <div v-else class="space-y-2 mb-4">
          <div
            v-for="(dir, index) in archiveDirs"
            :key="dir"
            class="flex items-center gap-2 bg-gray-700/50 rounded-lg px-3 py-2"
          >
            <span class="text-sm text-gray-300 font-mono flex-1 truncate">{{ dir }}</span>
            <button
              @click="removeArchiveDir(index)"
              class="text-gray-500 hover:text-red-400 transition-colors p-1"
              title="移除"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <div class="flex gap-2">
          <input
            v-model="newArchiveDir"
            type="text"
            placeholder="输入归档目录路径..."
            class="flex-1 bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-blue-500 transition-colors"
            @keyup.enter="addArchiveDir"
          />
          <button
            @click="addArchiveDir"
            class="px-4 py-2 bg-gray-700 hover:bg-gray-600 border border-gray-600 rounded-lg text-sm text-gray-300 transition-colors shrink-0"
          >
            + 添加
          </button>
        </div>
      </div>

      <!-- Start Scan Button -->
      <div class="pt-2">
        <button
          @click="startScan"
          :disabled="!wechatDir || saving"
          class="w-full py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 disabled:text-gray-500 rounded-xl text-white font-semibold text-base transition-all duration-200 flex items-center justify-center gap-2"
        >
          <svg v-if="saving" class="animate-spin h-4 w-4" viewBox="0 0 24 24" fill="none">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
          </svg>
          <span>{{ saving ? '正在保存...' : '🔍 开始扫描' }}</span>
        </button>
      </div>
    </template>
  </div>
</template>
