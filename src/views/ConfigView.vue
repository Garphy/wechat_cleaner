<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useAppStore, type WechatAccount, type AppConfig } from '../stores/app'

const router = useRouter()
const store = useAppStore()

const loading = ref(true)
const error = ref<string | null>(null)
const wechatDir = ref('')
const archiveDirs = ref<string[]>([''])
const accounts = ref<WechatAccount[]>([])
const selectedAccount = ref<string | null>(null)
const saving = ref(false)
const trashMode = ref<'trash' | 'delete'>('trash')
const dirErrors = ref<Record<number, string>>({})
const debugMode = ref(false)
const logPath = ref('')

// ── Directory validation ──────────────────────────────────────────
async function validateDir(path: string): Promise<boolean> {
  if (!path.trim()) return true
  try {
    return await invoke<boolean>('validate_directory', { path })
  } catch {
    return false
  }
}

async function validateAllDirs() {
  const errors: Record<number, string> = {}
  for (let i = 0; i < archiveDirs.value.length; i++) {
    const dir = archiveDirs.value[i]?.trim() ?? ''
    if (dir && !(await validateDir(dir))) {
      errors[i] = '目录不存在或无效'
    }
  }
  dirErrors.value = errors
  return Object.keys(errors).length === 0
}

// ── Archive directory management ──────────────────────────────────
function addArchiveDir() {
  archiveDirs.value.push('')
}

function removeArchiveDir(index: number) {
  archiveDirs.value.splice(index, 1)
  delete dirErrors.value[index]
}

async function browseDir(index: number) {
  const selected = await open({ directory: true })
  if (selected) {
    archiveDirs.value[index] = selected
    // Validate after selection
    const valid = await validateDir(selected)
    if (!valid) {
      dirErrors.value[index] = '目录不存在或无效'
    } else {
      delete dirErrors.value[index]
    }
  }
}

// ── Auto-save config ──────────────────────────────────────────────
let saveTimer: ReturnType<typeof setTimeout> | null = null

function debouncedSave() {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(async () => {
    try {
      await saveConfig()
    } catch (e) {
      console.error('Auto-save failed:', e)
    }
  }, 500)
}

// Watch all config fields for auto-save
watch([wechatDir, archiveDirs, selectedAccount, trashMode, debugMode], debouncedSave, { deep: true })

// ── Debug mode ────────────────────────────────────────────────────
async function toggleDebug() {
  debugMode.value = !debugMode.value
  // Save to config
  await saveConfig()
}

async function openLogDir() {
  // Extract directory from log path
  const dir = logPath.value.substring(0, logPath.value.lastIndexOf('\\') || logPath.value.lastIndexOf('/'))
  if (dir) {
    await invoke('open', { url: dir })
  }
}

async function clearLogFile() {
  await invoke('clear_debug_log')
}

async function saveConfig() {
  const config: AppConfig = {
    wechat_dir: wechatDir.value,
    archive_dirs: archiveDirs.value.filter((d) => d.trim()),
    selected_account: selectedAccount.value,
    trash_mode: trashMode.value,
    debug_enabled: debugMode.value,
  }
  await invoke('save_config', { config })
  store.config = config
}

// ── Initialize ────────────────────────────────────────────────────
onMounted(async () => {
  try {
    // Load log path
    logPath.value = await invoke<string>('get_log_path')

    const detectedAccounts = await invoke<WechatAccount[]>('detect_wechat_paths')
    accounts.value = detectedAccounts
    store.accounts = detectedAccounts

    const existingConfig = await invoke<AppConfig>('get_config')
    store.config = existingConfig

    if (existingConfig) {
      wechatDir.value = existingConfig.wechat_dir || ''
      archiveDirs.value = existingConfig.archive_dirs?.length > 0 ? existingConfig.archive_dirs : ['']
      selectedAccount.value = existingConfig.selected_account
      trashMode.value = (existingConfig.trash_mode as 'trash' | 'delete') || 'trash'
      debugMode.value = existingConfig.debug_enabled ?? true
    }

    // Sync debug mode to runtime
    await invoke('set_debug_mode', { enabled: debugMode.value })

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

// ── Start scan ────────────────────────────────────────────────────
async function startScan() {
  saving.value = true
  error.value = null
  try {
    // Validate all directories first
    if (!(await validateAllDirs())) {
      error.value = '请修正无效的目录路径后再继续'
      return
    }

    await saveConfig()
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

        <div class="space-y-3 mb-4">
          <div
            v-for="(dir, index) in archiveDirs"
            :key="index"
            class="flex gap-2 items-start"
          >
            <div class="flex-1">
              <div class="flex gap-2">
                <input
                  v-model="archiveDirs[index]"
                  type="text"
                  :placeholder="index === 0 ? '输入归档目录路径...' : '输入另一个归档目录路径...'"
                  class="flex-1 bg-gray-700 border rounded-lg px-3 py-2 text-sm text-gray-200 placeholder-gray-500 focus:outline-none transition-colors"
                  :class="dirErrors[index] ? 'border-red-500 focus:border-red-400' : 'border-gray-600 focus:border-blue-500'"
                />
                <button
                  @click="browseDir(index)"
                  class="px-3 py-2 bg-gray-600 hover:bg-gray-500 border border-gray-500 rounded-lg text-sm text-gray-300 transition-colors shrink-0"
                  title="浏览选择目录"
                >
                  📂
                </button>
                <button
                  v-if="archiveDirs.length > 1"
                  @click="removeArchiveDir(index)"
                  class="px-3 py-2 bg-gray-700 hover:bg-red-900/50 border border-gray-600 hover:border-red-500 rounded-lg text-gray-500 hover:text-red-400 transition-colors shrink-0"
                  title="移除"
                >
                  ✕
                </button>
              </div>
              <div v-if="dirErrors[index]" class="text-red-400 text-xs mt-1">
                {{ dirErrors[index] }}
              </div>
            </div>
          </div>
        </div>

        <button
          @click="addArchiveDir"
          class="px-4 py-2 bg-gray-700 hover:bg-gray-600 border border-gray-600 rounded-lg text-sm text-gray-300 transition-colors"
        >
          + 添加更多归档目录
        </button>
      </div>

      <!-- Cleanup Mode -->
      <div class="bg-gray-800 rounded-xl border border-gray-700 p-5">
        <h2 class="text-lg font-semibold text-white mb-4 flex items-center gap-2">
          <span class="text-xl">⚙️</span> 清理模式
        </h2>

        <div class="space-y-3">
          <label
            class="flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors"
            :class="trashMode === 'trash' ? 'bg-blue-900/30 border border-blue-600' : 'bg-gray-700/50 border border-gray-600 hover:border-gray-500'"
          >
            <input
              v-model="trashMode"
              type="radio"
              value="trash"
              class="w-4 h-4 text-blue-500 focus:ring-blue-500"
            />
            <div>
              <div class="text-sm text-white font-medium">🗑️ 移到回收站</div>
              <div class="text-xs text-gray-400">文件将被移到系统回收站，可恢复</div>
            </div>
          </label>

          <label
            class="flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors"
            :class="trashMode === 'delete' ? 'bg-red-900/30 border border-red-600' : 'bg-gray-700/50 border border-gray-600 hover:border-gray-500'"
          >
            <input
              v-model="trashMode"
              type="radio"
              value="delete"
              class="w-4 h-4 text-red-500 focus:ring-red-500"
            />
            <div>
              <div class="text-sm text-white font-medium">⛔ 永久删除</div>
              <div class="text-xs text-gray-400">文件将被永久删除，无法恢复</div>
            </div>
          </label>
        </div>
      </div>

      <!-- Debug Mode -->
      <div class="bg-gray-800 rounded-xl border border-gray-700 p-5">
        <h2 class="text-lg font-semibold text-white mb-4 flex items-center gap-2">
          <span class="text-xl">🐛</span> 调试模式
        </h2>

        <div class="flex items-center justify-between mb-3">
          <div>
            <div class="text-sm text-white font-medium">启用调试日志</div>
            <div class="text-xs text-gray-400">记录详细日志到文件，便于排查问题</div>
          </div>
          <button
            @click="toggleDebug"
            class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 focus:outline-none"
            :class="debugMode ? 'bg-blue-600' : 'bg-gray-600'"
          >
            <span
              class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform duration-200"
              :class="debugMode ? 'translate-x-6' : 'translate-x-1'"
            />
          </button>
        </div>

        <div v-if="debugMode" class="bg-gray-700/50 rounded-lg p-3 space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-xs text-gray-400 font-mono truncate">{{ logPath }}</span>
            <div class="flex gap-2 shrink-0 ml-2">
              <button
                @click="openLogDir"
                class="px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-xs text-gray-300 transition-colors"
              >
                📁 打开目录
              </button>
              <button
                @click="clearLogFile"
                class="px-2 py-1 bg-gray-700 hover:bg-red-900/50 border border-gray-600 hover:border-red-500 rounded text-xs text-gray-400 hover:text-red-400 transition-colors"
              >
                🗑️ 清空
              </button>
            </div>
          </div>
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
