<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore, type ScanProgress } from '../stores/app'

const router = useRouter()
const store = useAppStore()

const polling = ref<ReturnType<typeof setInterval> | null>(null)
const completed = ref(false)
const notification = ref('')
const error = ref<string | null>(null)

const phases = ['Walking', 'Hashing', 'Deduplicating']
const phaseLabels: Record<string, string> = {
  Walking: '遍历文件',
  Hashing: '计算哈希',
  Deduplicating: '去重分析',
}

const progress = computed(() => store.progress)

const percentage = computed(() => {
  if (!progress.value || progress.value.total_files === 0) return 0
  return Math.min(100, Math.round((progress.value.scanned_files / progress.value.total_files) * 100))
})

const currentPhaseIndex = computed(() => {
  if (!progress.value) return -1
  if (progress.value.is_complete) return phases.length // all phases completed
  return phases.indexOf(progress.value.phase)
})

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return (bytes / Math.pow(k, i)).toFixed(2) + ' ' + units[i]
}

function formatPath(path: string): string {
  if (!path) return ''
  if (path.length > 60) {
    return '...' + path.slice(-57)
  }
  return path
}

onMounted(async () => {
  store.isScanning = true

  // Start polling
  polling.value = setInterval(async () => {
    try {
      const p = await invoke<ScanProgress>('get_scan_progress')
      store.progress = p

      // Check if scan is complete (use is_complete flag from backend)
      if (p.is_complete) {
        completed.value = true
        notification.value = '扫描完成！'
        clearInterval(polling.value!)

        // Load result into store
        try {
          const result = await invoke<any>('get_scan_result')
          store.scanResult = result
        } catch (e) {
          error.value = String(e)
        }
      }
    } catch (e) {
      error.value = String(e)
    }
  }, 500)

  // If already complete (back navigation), skip start_scan — polling will pick up existing state
  if (store.progress?.is_complete) {
    return
  }

  // Start a new scan
  try {
    if (store.config) {
      await invoke('start_scan', {
        config: {
          wechat_dir: store.config.wechat_dir,
          archive_dirs: store.config.archive_dirs,
        },
      })
    } else {
      const cfg = await invoke<any>('get_config')
      await invoke('start_scan', {
        config: {
          wechat_dir: cfg.wechat_dir,
          archive_dirs: cfg.archive_dirs,
        },
      })
    }
  } catch (e) {
    error.value = String(e)
    store.isScanning = false
    if (polling.value) clearInterval(polling.value)
  }
})

onUnmounted(() => {
  if (polling.value) {
    clearInterval(polling.value)
  }
})

async function pauseResume() {
  if (!progress.value) return
  try {
    if (progress.value.is_paused) {
      await invoke('resume_scan')
    } else {
      await invoke('pause_scan')
    }
  } catch (e) {
    error.value = String(e)
  }
}

async function cancelScan() {
  try {
    await invoke('cancel_scan')
    store.isScanning = false
    if (polling.value) clearInterval(polling.value)
    router.push('/config')
  } catch (e) {
    error.value = String(e)
  }
}
</script>

<template>
  <div class="max-w-3xl mx-auto space-y-6 overflow-y-auto h-full pr-1">
    <!-- Error -->
    <div v-if="error" class="bg-red-900/30 border border-red-800 rounded-lg p-4 text-red-300 text-sm">
      <div class="font-medium mb-1">扫描出错</div>
      {{ error }}
    </div>

    <!-- Notification -->
    <div
      v-if="notification"
      class="bg-green-900/30 border border-green-800 rounded-lg p-4 text-green-300 text-sm flex items-center gap-2"
    >
      <span class="text-lg">✅</span>
      <span class="font-medium">{{ notification }}</span>
    </div>

    <!-- Progress Section -->
    <div class="bg-gray-800 rounded-xl border border-gray-700 p-6">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-lg font-semibold text-white flex items-center gap-2">
          <span class="text-xl">🔍</span> 扫描进度
        </h2>
        <span class="text-2xl font-bold text-blue-400">{{ percentage }}%</span>
      </div>

      <!-- Progress Bar -->
      <div class="w-full bg-gray-700 rounded-full h-4 mb-6 overflow-hidden">
        <div
          class="h-full bg-gradient-to-r from-blue-600 to-blue-400 rounded-full transition-all duration-500 ease-out"
          :style="{ width: percentage + '%' }"
        />
      </div>

      <!-- Phase Indicator -->
      <div class="flex items-center justify-center gap-2 mb-6">
        <template v-for="(phase, idx) in phases" :key="phase">
          <div
            class="flex items-center gap-2 px-4 py-2 rounded-full text-sm font-medium transition-all duration-300"
            :class="
              idx < currentPhaseIndex
                ? 'bg-green-900/40 text-green-400 border border-green-800'
                : idx === currentPhaseIndex
                  ? 'bg-blue-900/40 text-blue-400 border border-blue-800 animate-pulse'
                  : 'bg-gray-700/50 text-gray-500 border border-gray-700'
            "
          >
            <span v-if="idx < currentPhaseIndex" class="text-green-400">✓</span>
            <span v-else-if="idx === currentPhaseIndex" class="text-blue-400">●</span>
            <span v-else class="text-gray-500">○</span>
            {{ phaseLabels[phase] }}
          </div>
          <div
            v-if="idx < phases.length - 1"
            class="w-8 h-px"
            :class="idx < currentPhaseIndex ? 'bg-green-700' : 'bg-gray-700'"
          />
        </template>
      </div>

      <!-- Current Path -->
      <div v-if="progress?.current_path" class="bg-gray-700/50 rounded-lg px-4 py-3 mb-6">
        <div class="text-xs text-gray-500 mb-1">当前扫描路径</div>
        <div class="text-sm text-gray-300 font-mono truncate" :title="progress.current_path">
          {{ formatPath(progress.current_path) }}
        </div>
      </div>
    </div>

    <!-- Stats Cards -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <div class="bg-gray-800 rounded-xl border border-gray-700 p-4">
        <div class="text-xs text-gray-500 mb-1">已扫描文件</div>
        <div class="text-xl font-bold text-white">
          {{ progress?.scanned_files ?? 0 }}
          <span class="text-sm font-normal text-gray-500">/ {{ progress?.total_files ?? 0 }}</span>
        </div>
      </div>
      <div class="bg-gray-800 rounded-xl border border-gray-700 p-4">
        <div class="text-xs text-gray-500 mb-1">总大小</div>
        <div class="text-xl font-bold text-white">
          {{ formatSize(progress?.total_size ?? 0) }}
        </div>
      </div>
      <div class="bg-gray-800 rounded-xl border border-gray-700 p-4">
        <div class="text-xs text-gray-500 mb-1">已发现冗余</div>
        <div class="text-xl font-bold text-amber-400">
          {{ formatSize(progress?.redundant_size ?? 0) }}
        </div>
      </div>
      <div class="bg-gray-800 rounded-xl border border-gray-700 p-4">
        <div class="text-xs text-gray-500 mb-1">可释放空间</div>
        <div class="text-xl font-bold text-green-400">
          {{ formatSize(progress?.redundant_size ?? 0) }}
        </div>
      </div>
    </div>

    <!-- Control Buttons -->
    <div class="flex items-center gap-3 justify-center">
      <button
        v-if="completed && store.scanResult"
        @click="router.push('/results')"
        class="px-6 py-2.5 bg-blue-600 hover:bg-blue-500 rounded-lg text-sm font-medium text-white transition-all duration-200 flex items-center gap-2"
      >
        📊 查看结果
      </button>
      <button
        v-if="!completed"
        @click="pauseResume"
        class="px-6 py-2.5 rounded-lg text-sm font-medium transition-all duration-200 flex items-center gap-2"
        :class="
          progress?.is_paused
            ? 'bg-blue-600 hover:bg-blue-500 text-white'
            : 'bg-gray-700 hover:bg-gray-600 text-gray-300 border border-gray-600'
        "
      >
        <span v-if="progress?.is_paused">▶ 恢复</span>
        <span v-else>⏸ 暂停</span>
      </button>
      <button
        v-if="!completed"
        @click="cancelScan"
        class="px-6 py-2.5 bg-gray-700 hover:bg-red-900/50 border border-gray-600 hover:border-red-800 rounded-lg text-sm text-gray-300 hover:text-red-300 transition-all duration-200"
      >
        ✕ 取消
      </button>
    </div>
  </div>
</template>
