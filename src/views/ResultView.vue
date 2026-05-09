<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore, type FileGroup, type CleanupReport } from '../stores/app'

const store = useAppStore()

const loading = ref(true)
const error = ref<string | null>(null)
const sortField = ref<'size' | 'time' | 'name'>('size')
const sortOrder = ref<'desc' | 'asc'>('desc')
const expandedGroups = ref<Set<string>>(new Set())
const currentPage = ref(0)
const pageSize = 100
const hasMore = ref(true)
const loadingMore = ref(false)

const showCleanupModal = ref(false)
const cleaningUp = ref(false)
const cleanupReport = ref<CleanupReport | null>(null)
const showReport = ref(false)

const allGroups = ref<FileGroup[]>([])

const totalFiles = computed(() => allGroups.value.reduce((acc, g) => acc + g.files.length, 0))
const totalSize = computed(() => allGroups.value.reduce((acc, g) => acc + g.total_size, 0))
const redundantFiles = computed(() =>
  allGroups.value.reduce((acc, g) => acc + g.files.filter((f) => f.status === 'Remove').length, 0)
)
const redundantSize = computed(() => allGroups.value.reduce((acc, g) => acc + g.reclaimable_size, 0))
const durationMs = computed(() => store.scanResult?.duration_ms ?? 0)
const selectedFileCount = computed(() => store.getSelectedFileCount())
const selectedFilesToDelete = computed(() => store.getSelectedFiles(allGroups.value))
const selectedDeleteSize = computed(() => selectedFilesToDelete.value.reduce((acc, f) => acc + f.size, 0))

// ── Path shortening ─────────────────────────────────────────────
function shortenPath(path: string): string {
  if (!path) return ''
  const config = store.config
  if (!config) return path
  const prefixes = [config.wechat_dir, ...(config.archive_dirs || [])]
    .filter(Boolean)
    .sort((a, b) => b.length - a.length)
  for (const prefix of prefixes) {
    if (path.startsWith(prefix)) {
      const rest = path.slice(prefix.length).replace(/^[/\\]/, '')
      return rest || path
    }
  }
  return path
}

// ── Formatters ──────────────────────────────────────────────────
function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return (bytes / Math.pow(k, i)).toFixed(2) + ' ' + units[i]
}

function formatTimestamp(ts: number): string {
  if (!ts) return '-'
  const date = new Date(ts * 1000)
  return date.toLocaleDateString('zh-CN') + ' ' + date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
}

function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000)
  if (seconds < 60) return seconds + ' 秒'
  return `${Math.floor(seconds / 60)} 分 ${seconds % 60} 秒`
}

function getGroupTypeLabel(type: string): string {
  if (type === 'CrossDedup') return '跨目录去重'
  if (type === 'VersionConverge') return '版本收敛'
  return type
}

function toggleExpand(id: string) {
  if (expandedGroups.value.has(id)) expandedGroups.value.delete(id)
  else expandedGroups.value.add(id)
}

function isExpanded(id: string): boolean {
  return expandedGroups.value.has(id)
}

// ── File selection helpers ───────────────────────────────────────
function getCheckboxClass(group: FileGroup): string {
  if (store.isGroupFullySelected(group)) return 'text-red-500'
  if (store.isGroupPartiallySelected(group)) return 'text-amber-500'
  return 'text-gray-500'
}

function getCheckboxIndeterminate(group: FileGroup): boolean {
  return store.isGroupPartiallySelected(group)
}

// ── Data loading ────────────────────────────────────────────────
async function loadResults(append = false) {
  if (loadingMore.value) return
  loadingMore.value = true
  error.value = null
  try {
    const result = await invoke<any>('get_paged_results', {
      page: currentPage.value,
      pageSize: pageSize,
      sort: sortField.value,
      order: sortOrder.value,
    })
    const newGroups: FileGroup[] = result.groups
    if (append) {
      allGroups.value = [...allGroups.value, ...newGroups]
    } else {
      allGroups.value = newGroups
      store.initFileSelection(newGroups)
    }
    if (newGroups.length < pageSize) hasMore.value = false
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
    loadingMore.value = false
  }
}

function handleSort(field: 'size' | 'time' | 'name') {
  if (sortField.value === field) sortOrder.value = sortOrder.value === 'desc' ? 'asc' : 'desc'
  else { sortField.value = field; sortOrder.value = 'desc' }
  currentPage.value = 0
  hasMore.value = true
  allGroups.value = []
  loadResults()
}

async function executeCleanup() {
  cleaningUp.value = true
  try {
    const filePaths = selectedFilesToDelete.value.map((f) => f.path)
    const report = await invoke<CleanupReport>('execute_cleanup', {
      filePaths,
      mode: store.config?.trash_mode || 'trash',
    })
    cleanupReport.value = report
    showReport.value = true
    showCleanupModal.value = false
    const cleanedPaths = new Set(filePaths)
    allGroups.value = allGroups.value
      .map((g) => ({ ...g, files: g.files.filter((f) => !cleanedPaths.has(f.path)) }))
      .filter((g) => g.files.length > 0)
    filePaths.forEach((p) => store.selectedFiles.delete(p))
  } catch (e) {
    error.value = String(e)
  } finally {
    cleaningUp.value = false
  }
}

// ── Infinite scroll ─────────────────────────────────────────────
const scrollContainer = ref<HTMLElement | null>(null)

function handleScroll() {
  if (!scrollContainer.value || loadingMore.value || !hasMore.value) return
  const { scrollTop, scrollHeight, clientHeight } = scrollContainer.value
  if (scrollTop + clientHeight >= scrollHeight - 100) {
    currentPage.value++
    loadResults(true)
  }
}

// ── Init ────────────────────────────────────────────────────────
onMounted(async () => {
  if (store.scanResult?.groups && store.scanResult.groups.length > 0) {
    allGroups.value = store.scanResult.groups
    store.initFileSelection(store.scanResult.groups)
    loading.value = false
  } else {
    loadResults()
  }
})
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-20">
      <div class="flex items-center gap-3 text-gray-400">
        <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24" fill="none">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
        </svg>
        <span>正在加载扫描结果...</span>
      </div>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="bg-red-900/30 border border-red-800 rounded-lg p-3 text-red-300 text-sm">
      <div class="font-medium mb-1">加载出错</div>
      {{ error }}
    </div>

    <!-- Report -->
    <div v-if="showReport && cleanupReport" class="bg-green-900/20 border border-green-800 rounded-xl p-4">
      <h3 class="text-base font-semibold text-green-400 mb-2">✅ 清理完成</h3>
      <div class="grid grid-cols-2 gap-3 text-sm">
        <div>
          <span class="text-gray-500">已清理文件：</span>
          <span class="text-white font-medium">{{ cleanupReport.files_removed }}</span>
        </div>
        <div>
          <span class="text-gray-500">释放空间：</span>
          <span class="text-green-400 font-medium">{{ formatSize(cleanupReport.space_freed) }}</span>
        </div>
      </div>
      <div v-if="cleanupReport.errors.length > 0" class="mt-2">
        <div class="text-xs text-red-400 font-medium mb-1">错误：</div>
        <div v-for="err in cleanupReport.errors" :key="err.path" class="text-xs text-red-300/70 font-mono">
          {{ err.path }}: {{ err.error }}
        </div>
      </div>
    </div>

    <!-- Stats Bar -->
    <div class="grid grid-cols-5 gap-2 mb-3">
      <div class="bg-gray-800 rounded-lg border border-gray-700 px-2 py-1.5">
        <div class="text-[10px] text-gray-500">总文件数</div>
        <div class="text-sm font-bold text-white">{{ totalFiles }}</div>
      </div>
      <div class="bg-gray-800 rounded-lg border border-gray-700 px-2 py-1.5">
        <div class="text-[10px] text-gray-500">总大小</div>
        <div class="text-sm font-bold text-white">{{ formatSize(totalSize) }}</div>
      </div>
      <div class="bg-gray-800 rounded-lg border border-gray-700 px-2 py-1.5">
        <div class="text-[10px] text-gray-500">冗余文件</div>
        <div class="text-sm font-bold text-amber-400">{{ redundantFiles }}</div>
      </div>
      <div class="bg-gray-800 rounded-lg border border-gray-700 px-2 py-1.5">
        <div class="text-[10px] text-gray-500">可释放</div>
        <div class="text-sm font-bold text-green-400">{{ formatSize(redundantSize) }}</div>
      </div>
      <div class="bg-gray-800 rounded-lg border border-gray-700 px-2 py-1.5">
        <div class="text-[10px] text-gray-500">耗时</div>
        <div class="text-sm font-bold text-white">{{ formatDuration(durationMs) }}</div>
      </div>
    </div>

    <!-- Sort Controls -->
    <div class="flex items-center gap-2 mb-3">
      <span class="text-xs text-gray-500">排序：</span>
      <button
        v-for="f in ['size', 'time', 'name'] as const"
        :key="f"
        @click="handleSort(f)"
        class="px-2 py-1 rounded text-xs font-medium transition-all"
        :class="sortField === f
          ? 'bg-blue-600/20 text-blue-400 border border-blue-800'
          : 'bg-gray-800 text-gray-400 border border-gray-700 hover:bg-gray-700'"
      >
        {{ f === 'size' ? '大小' : f === 'time' ? '时间' : '名称' }}
        {{ sortField === f ? (sortOrder === 'desc' ? '↓' : '↑') : '' }}
      </button>

      <div class="flex-1" />

      <button @click="store.selectAllFiles(allGroups)" class="px-2 py-1 bg-gray-800 border border-gray-700 rounded text-xs text-gray-400 hover:bg-gray-700">全选删除</button>
      <button @click="store.deselectAllFiles()" class="px-2 py-1 bg-gray-800 border border-gray-700 rounded text-xs text-gray-400 hover:bg-gray-700">全部保留</button>
    </div>

    <!-- Empty state -->
    <div v-if="!loading && allGroups.length === 0 && !error" class="text-center py-16 text-gray-500">
      <div class="text-4xl mb-3">📭</div>
      <div class="text-sm">未发现冗余文件</div>
    </div>

    <!-- Group List -->
    <div
      ref="scrollContainer"
      class="flex-1 min-h-0 overflow-y-auto space-y-2 pr-1"
      @scroll="handleScroll"
    >
      <div
        v-for="group in allGroups"
        :key="group.id"
        class="bg-gray-800 rounded-lg border border-gray-700 overflow-hidden"
      >
        <!-- Group Header -->
        <div class="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-gray-750 transition-colors" @click="toggleExpand(group.id)">
          <!-- Group Checkbox: toggles all files in group -->
          <label class="flex items-center" @click.stop>
            <input
              type="checkbox"
              :checked="store.isGroupFullySelected(group)"
              :indeterminate="getCheckboxIndeterminate(group)"
              @change="store.toggleGroupFiles(group)"
              class="w-3.5 h-3.5 rounded border-gray-600 bg-gray-700 focus:ring-offset-0 cursor-pointer"
              :class="getCheckboxClass(group)"
            />
          </label>

          <span class="px-1.5 py-0.5 rounded text-[10px] font-medium shrink-0"
            :class="group.group_type === 'CrossDedup'
              ? 'bg-purple-900/40 text-purple-400 border border-purple-800'
              : 'bg-cyan-900/40 text-cyan-400 border border-cyan-800'">
            {{ getGroupTypeLabel(group.group_type) }}
          </span>

          <div class="flex-1 min-w-0">
            <div class="text-xs text-white font-medium truncate">{{ group.base_name }}</div>
          </div>

          <div class="text-right shrink-0 flex items-center gap-3">
            <span class="text-[10px] text-gray-500">{{ group.files.length }} 文件</span>
            <span class="text-[10px] text-green-400">可释放 {{ formatSize(group.reclaimable_size) }}</span>
          </div>

          <svg class="w-3.5 h-3.5 text-gray-500 transition-transform duration-200 shrink-0"
            :class="isExpanded(group.id) ? 'rotate-180' : ''"
            fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </div>

        <!-- Expanded File List -->
        <div v-if="isExpanded(group.id)" class="border-t border-gray-700">
          <div
            v-for="file in group.files"
            :key="file.path"
            class="flex items-center gap-2 px-3 py-1.5 border-b border-gray-700/50 last:border-b-0 hover:bg-gray-750 transition-colors"
          >
            <!-- File Checkbox: checked = will be deleted -->
            <label class="flex items-center" @click.stop>
              <input
                type="checkbox"
                :checked="store.isFileSelected(file.path)"
                @change="store.toggleFile(file.path)"
                class="w-3 h-3 rounded border-gray-600 bg-gray-700 focus:ring-offset-0 cursor-pointer"
                :class="store.isFileSelected(file.path) ? 'text-red-500' : 'text-green-500'"
              />
            </label>

            <!-- Status badge: linked to checkbox -->
            <span class="px-1 py-0.5 rounded text-[9px] font-medium border shrink-0"
              :class="store.isFileSelected(file.path)
                ? 'text-red-400 bg-red-900/30 border-red-800'
                : 'text-green-400 bg-green-900/30 border-green-800'">
              {{ store.isFileSelected(file.path) ? '删除' : '保留' }}
            </span>

            <div class="flex-1 min-w-0">
              <div class="text-[11px] text-gray-300 font-mono truncate" :title="file.path">
                {{ shortenPath(file.path) }}
              </div>
            </div>

            <span class="text-[9px] text-gray-600 shrink-0">{{ file.source === 'WechatDir' ? '微信' : '归档' }}</span>
            <div class="text-[11px] text-gray-400 shrink-0 w-14 text-right">{{ formatSize(file.size) }}</div>
            <div class="text-[11px] text-gray-500 shrink-0 w-24 text-right">{{ formatTimestamp(file.modified) }}</div>
          </div>
        </div>
      </div>

      <!-- Loading more -->
      <div v-if="loadingMore" class="text-center py-3 text-gray-500 text-xs">
        <svg class="animate-spin h-3.5 w-3.5 inline-block mr-1" viewBox="0 0 24 24" fill="none">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
        </svg>
        加载更多...
      </div>
    </div>

    <!-- Bottom Action Bar -->
    <div v-if="allGroups.length > 0"
      class="flex-shrink-0 bg-gray-800/95 backdrop-blur border border-gray-700 rounded-lg p-3 flex items-center gap-4 mt-2">
      <div class="flex-1">
        <span class="text-xs text-gray-400">
          待删除 <span class="text-red-400 font-medium">{{ selectedFileCount }}</span> 个文件,
          释放 <span class="text-green-400 font-medium">{{ formatSize(selectedDeleteSize) }}</span>
        </span>
      </div>
      <button
        @click="showCleanupModal = true"
        :disabled="selectedFileCount === 0"
        class="px-5 py-2 bg-red-600 hover:bg-red-500 disabled:bg-gray-700 disabled:text-gray-500 rounded-lg text-xs font-medium text-white transition-all"
      >
        🗑️ 清理选中文件
      </button>
    </div>

    <!-- Cleanup Confirmation Modal -->
    <Teleport to="body">
      <div v-if="showCleanupModal" class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" @click.self="showCleanupModal = false">
        <div class="bg-gray-800 rounded-xl border border-gray-700 p-6 max-w-lg w-full mx-4 max-h-[80vh] flex flex-col">
          <h3 class="text-lg font-semibold text-white mb-2">确认清理</h3>
          <p class="text-sm text-gray-400 mb-3">
            以下 <span class="text-red-400 font-medium">{{ selectedFileCount }}</span> 个文件将被
            <span class="font-medium" :class="store.config?.trash_mode === 'delete' ? 'text-red-400' : 'text-amber-400'">
              {{ store.config?.trash_mode === 'delete' ? '永久删除' : '移到回收站' }}
            </span>，
            共释放 <span class="text-green-400 font-medium">{{ formatSize(selectedDeleteSize) }}</span>：
          </p>

          <!-- File list -->
          <div class="flex-1 min-h-0 overflow-y-auto bg-gray-900 rounded-lg border border-gray-700 p-2 space-y-1">
            <div
              v-for="file in selectedFilesToDelete"
              :key="file.path"
              class="flex items-center gap-2 px-2 py-1 text-xs"
            >
              <span class="text-gray-300 font-mono flex-1 truncate" :title="file.path">{{ shortenPath(file.path) }}</span>
              <span class="text-gray-500 shrink-0">{{ formatSize(file.size) }}</span>
            </div>
          </div>

          <div class="flex gap-3 justify-end mt-4">
            <button @click="showCleanupModal = false" class="px-4 py-2 text-sm text-gray-400 hover:text-white transition-colors">取消</button>
            <button @click="executeCleanup" :disabled="cleaningUp"
              class="px-4 py-2 bg-red-600 hover:bg-red-500 disabled:bg-gray-700 rounded-lg text-sm font-medium text-white transition-all">
              {{ cleaningUp ? '清理中...' : '确认清理' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
