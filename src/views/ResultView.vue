<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore, type FileGroup, type FileEntry, type CleanupReport } from '../stores/app'
import { useVirtualizer } from '@tanstack/vue-virtual'

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

const virtualScrollRef = ref<HTMLElement | null>(null)

const virtualizer = useVirtualizer<HTMLElement, HTMLDivElement>({
  count: 0,
  getScrollElement: () => virtualScrollRef.value,
  estimateSize: () => 120,
  overscan: 10,
})

// Update virtualizer when groups change
watch(
  allGroups,
  (newGroups) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    ;(virtualizer.value as any).setOptions({ count: newGroups.length })
  },
  { deep: true }
)

const totalFiles = computed(() => {
  return allGroups.value.reduce((acc, g) => acc + g.files.length, 0)
})

const totalSize = computed(() => {
  return allGroups.value.reduce((acc, g) => acc + g.total_size, 0)
})

const redundantFiles = computed(() => {
  return allGroups.value.reduce(
    (acc, g) => acc + g.files.filter((f) => f.status === 'Remove').length,
    0
  )
})

const redundantSize = computed(() => {
  return allGroups.value.reduce((acc, g) => acc + g.reclaimable_size, 0)
})

const durationMs = computed(() => {
  return store.scanResult?.duration_ms ?? 0
})

const totalSelectedSize = computed(() => {
  return allGroups.value
    .filter((g) => store.selectedGroupIds.has(g.id))
    .reduce((acc, g) => acc + g.reclaimable_size, 0)
})

const totalSelectedFiles = computed(() => {
  return allGroups.value
    .filter((g) => store.selectedGroupIds.has(g.id))
    .reduce((acc, g) => acc + g.files.filter((f) => f.status === 'Remove').length, 0)
})

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
  const minutes = Math.floor(seconds / 60)
  const remainSeconds = seconds % 60
  return `${minutes} 分 ${remainSeconds} 秒`
}

function getGroupTypeLabel(type: string): string {
  if (type === 'CrossDedup') return '跨目录去重'
  if (type === 'VersionConverge') return '版本收敛'
  return type
}

function getStatusLabel(status: string): string {
  if (status === 'Keep') return '保留'
  if (status === 'Remove') return '删除'
  if (status === 'UserDecided') return '待定'
  return status
}

function getStatusColor(status: string): string {
  if (status === 'Keep') return 'text-green-400 bg-green-900/30 border-green-800'
  if (status === 'Remove') return 'text-red-400 bg-red-900/30 border-red-800'
  return 'text-amber-400 bg-amber-900/30 border-amber-800'
}

function toggleExpand(id: string) {
  if (expandedGroups.value.has(id)) {
    expandedGroups.value.delete(id)
  } else {
    expandedGroups.value.add(id)
  }
}

function isExpanded(id: string): boolean {
  return expandedGroups.value.has(id)
}

async function loadResults(append = false) {
  if (loadingMore.value) return
  loadingMore.value = true
  error.value = null
  try {
    const result = await invoke<any>('get_paged_results', {
      page: currentPage.value,
      page_size: pageSize,
      sort: sortField.value,
      order: sortOrder.value,
    })
    const newGroups: FileGroup[] = result.groups
    if (append) {
      allGroups.value = [...allGroups.value, ...newGroups]
    } else {
      allGroups.value = newGroups
      // Auto-select all Remove status groups
      newGroups.forEach((g: FileGroup) => {
        store.selectedGroupIds.add(g.id)
      })
    }
    if (newGroups.length < pageSize) {
      hasMore.value = false
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
    loadingMore.value = false
  }
}

function handleSort(field: 'size' | 'time' | 'name') {
  if (sortField.value === field) {
    sortOrder.value = sortOrder.value === 'desc' ? 'asc' : 'desc'
  } else {
    sortField.value = field
    sortOrder.value = 'desc'
  }
  currentPage.value = 0
  hasMore.value = true
  allGroups.value = []
  loadResults()
}

function handleScroll() {
  if (!virtualScrollRef.value || loadingMore.value || !hasMore.value) return
  const { scrollTop, scrollHeight, clientHeight } = virtualScrollRef.value
  if (scrollTop + clientHeight >= scrollHeight - 100) {
    currentPage.value++
    loadResults(true)
  }
}

function getGroupById(index: number): FileGroup {
  const group = allGroups.value[index]
  if (!group) {
    return { id: '', group_type: '', base_name: '', total_size: 0, reclaimable_size: 0, files: [], suggested_keep: 1 }
  }
  return group
}

async function executeCleanup() {
  cleaningUp.value = true
  try {
    const selectedIdsArray = Array.from(store.selectedGroupIds)
    const report = await invoke<CleanupReport>('execute_cleanup', {
      selectedIds: selectedIdsArray,
      mode: 'trash',
    })
    cleanupReport.value = report
    showReport.value = true
    showCleanupModal.value = false

    // Remove cleaned groups from list
    const cleanedIds = new Set(selectedIdsArray)
    allGroups.value = allGroups.value.filter((g) => !cleanedIds.has(g.id))
    selectedIdsArray.forEach((id) => store.selectedGroupIds.delete(id))
  } catch (e) {
    error.value = String(e)
  } finally {
    cleaningUp.value = false
  }
}

onMounted(async () => {
  console.log('[DEBUG] ResultView onMounted. store.scanResult:', store.scanResult)
  console.log('[DEBUG] groups:', store.scanResult?.groups?.length)

  // First try to load from store (set by ScanView)
  if (store.scanResult?.groups && store.scanResult.groups.length > 0) {
    console.log('[DEBUG] Using store data, groups count:', store.scanResult.groups.length)
    allGroups.value = store.scanResult.groups
    loading.value = false
    // Auto-select all Remove status groups
    store.scanResult.groups.forEach((g: FileGroup) => {
      store.selectedGroupIds.add(g.id)
    })
  } else {
    // Fallback: load from backend
    console.log('[DEBUG] No store data, falling back to loadResults()')
    loadResults()
  }
})
</script>

<template>
  <div class="space-y-5">
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
    <div v-else-if="error" class="bg-red-900/30 border border-red-800 rounded-lg p-4 text-red-300 text-sm">
      <div class="font-medium mb-1">加载出错</div>
      {{ error }}
    </div>

    <!-- Report Modal (after cleanup) -->
    <div v-if="showReport && cleanupReport" class="bg-green-900/20 border border-green-800 rounded-xl p-5">
      <h3 class="text-lg font-semibold text-green-400 mb-3 flex items-center gap-2">
        <span class="text-xl">✅</span> 清理完成
      </h3>
      <div class="grid grid-cols-2 gap-4 text-sm">
        <div>
          <span class="text-gray-500">已清理文件：</span>
          <span class="text-white font-medium">{{ cleanupReport.files_removed }}</span>
        </div>
        <div>
          <span class="text-gray-500">释放空间：</span>
          <span class="text-green-400 font-medium">{{ formatSize(cleanupReport.space_freed) }}</span>
        </div>
      </div>
      <div v-if="cleanupReport.errors.length > 0" class="mt-3">
        <div class="text-xs text-red-400 font-medium mb-1">错误：</div>
        <div v-for="err in cleanupReport.errors" :key="err.path" class="text-xs text-red-300/70 font-mono">
          {{ err.path }}: {{ err.error }}
        </div>
      </div>
    </div>

    <!-- Stats Bar -->
    <div class="grid grid-cols-2 md:grid-cols-5 gap-3">
      <div class="bg-gray-800 rounded-lg border border-gray-700 p-3">
        <div class="text-xs text-gray-500">总文件数</div>
        <div class="text-lg font-bold text-white">{{ totalFiles }}</div>
      </div>
      <div class="bg-gray-800 rounded-lg border border-gray-700 p-3">
        <div class="text-xs text-gray-500">总大小</div>
        <div class="text-lg font-bold text-white">{{ formatSize(totalSize) }}</div>
      </div>
      <div class="bg-gray-800 rounded-lg border border-gray-700 p-3">
        <div class="text-xs text-gray-500">冗余文件数</div>
        <div class="text-lg font-bold text-amber-400">{{ redundantFiles }}</div>
      </div>
      <div class="bg-gray-800 rounded-lg border border-gray-700 p-3">
        <div class="text-xs text-gray-500">可释放空间</div>
        <div class="text-lg font-bold text-green-400">{{ formatSize(redundantSize) }}</div>
      </div>
      <div class="bg-gray-800 rounded-lg border border-gray-700 p-3">
        <div class="text-xs text-gray-500">扫描耗时</div>
        <div class="text-lg font-bold text-white">{{ formatDuration(durationMs) }}</div>
      </div>
    </div>

    <!-- Sort Controls -->
    <div class="flex items-center gap-3 flex-wrap">
      <span class="text-sm text-gray-500">排序：</span>
      <button
        @click="handleSort('size')"
        class="px-3 py-1.5 rounded-lg text-xs font-medium transition-all"
        :class="
          sortField === 'size'
            ? 'bg-blue-600/20 text-blue-400 border border-blue-800'
            : 'bg-gray-800 text-gray-400 border border-gray-700 hover:bg-gray-700'
        "
      >
        按大小 {{ sortField === 'size' ? (sortOrder === 'desc' ? '↓' : '↑') : '' }}
      </button>
      <button
        @click="handleSort('time')"
        class="px-3 py-1.5 rounded-lg text-xs font-medium transition-all"
        :class="
          sortField === 'time'
            ? 'bg-blue-600/20 text-blue-400 border border-blue-800'
            : 'bg-gray-800 text-gray-400 border border-gray-700 hover:bg-gray-700'
        "
      >
        按时间 {{ sortField === 'time' ? (sortOrder === 'desc' ? '↓' : '↑') : '' }}
      </button>
      <button
        @click="handleSort('name')"
        class="px-3 py-1.5 rounded-lg text-xs font-medium transition-all"
        :class="
          sortField === 'name'
            ? 'bg-blue-600/20 text-blue-400 border border-blue-800'
            : 'bg-gray-800 text-gray-400 border border-gray-700 hover:bg-gray-700'
        "
      >
        按名称 {{ sortField === 'name' ? (sortOrder === 'desc' ? '↓' : '↑') : '' }}
      </button>

      <div class="flex-1" />

      <button
        @click="store.selectAllGroups()"
        class="px-3 py-1.5 bg-gray-800 border border-gray-700 rounded-lg text-xs text-gray-400 hover:bg-gray-700 transition-colors"
      >
        全选
      </button>
      <button
        @click="store.deselectAllGroups()"
        class="px-3 py-1.5 bg-gray-800 border border-gray-700 rounded-lg text-xs text-gray-400 hover:bg-gray-700 transition-colors"
      >
        取消全选
      </button>
    </div>

    <!-- Empty state -->
    <div
      v-if="!loading && allGroups.length === 0 && !error"
      class="text-center py-16 text-gray-500"
    >
      <div class="text-4xl mb-3">📭</div>
      <div class="text-sm">未发现冗余文件</div>
    </div>

    <!-- Virtual Scroll List -->
    <div
      ref="virtualScrollRef"
      class="space-y-3 max-h-[calc(100vh-320px)] overflow-auto pr-1"
      @scroll="handleScroll"
    >
      <div
        :style="{ height: virtualizer.getTotalSize() + 'px', position: 'relative' }"
      >
        <div
          v-for="virtualRow in virtualizer.getVirtualItems()"
          :key="String(virtualRow.key)"
          :style="{
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            transform: `translateY(${virtualRow.start}px)`,
          }"
        >
          <div class="bg-gray-800 rounded-xl border border-gray-700 mb-3 overflow-hidden">
            <!-- Group Header -->
            <div class="flex items-center gap-3 p-4 cursor-pointer hover:bg-gray-750 transition-colors" @click="toggleExpand(getGroupById(virtualRow.index).id)">
              <!-- Checkbox -->
              <label class="flex items-center" @click.stop>
                <input
                  type="checkbox"
                  :checked="store.selectedGroupIds.has(getGroupById(virtualRow.index).id)"
                  @change="store.toggleGroup(getGroupById(virtualRow.index).id)"
                  class="w-4 h-4 rounded border-gray-600 bg-gray-700 text-blue-500 focus:ring-blue-500 focus:ring-offset-0 cursor-pointer"
                />
              </label>

              <!-- Group Type Badge -->
              <span
                class="px-2 py-0.5 rounded text-xs font-medium shrink-0"
                :class="
                  getGroupById(virtualRow.index).group_type === 'CrossDedup'
                    ? 'bg-purple-900/40 text-purple-400 border border-purple-800'
                    : 'bg-cyan-900/40 text-cyan-400 border border-cyan-800'
                "
              >
                {{ getGroupTypeLabel(getGroupById(virtualRow.index).group_type) }}
              </span>

              <!-- File Name -->
              <div class="flex-1 min-w-0">
                <div class="text-sm text-white font-medium truncate">
                  {{ getGroupById(virtualRow.index).base_name }}
                </div>
              </div>

              <!-- File Count & Size -->
              <div class="text-right shrink-0">
                <div class="text-sm text-white font-medium">
                  {{ getGroupById(virtualRow.index).files.length }} 个文件
                </div>
                <div class="text-xs text-green-400">
                  可释放 {{ formatSize(getGroupById(virtualRow.index).reclaimable_size) }}
                </div>
              </div>

              <!-- Expand Arrow -->
              <svg
                class="w-4 h-4 text-gray-500 transition-transform duration-200 shrink-0"
                :class="isExpanded(getGroupById(virtualRow.index).id) ? 'rotate-180' : ''"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
              </svg>
            </div>

            <!-- Expanded File List -->
            <div
              v-if="isExpanded(getGroupById(virtualRow.index).id)"
              class="border-t border-gray-700 bg-gray-850"
            >
              <div
                v-for="file in getGroupById(virtualRow.index).files"
                :key="file.path"
                class="flex items-center gap-3 px-4 py-2.5 border-b border-gray-700/50 last:border-b-0 hover:bg-gray-750 transition-colors"
              >
                <!-- Status Badge -->
                <span
                  class="px-1.5 py-0.5 rounded text-[10px] font-medium border shrink-0"
                  :class="getStatusColor(file.status)"
                >
                  {{ getStatusLabel(file.status) }}
                </span>

                <!-- Path -->
                <div class="flex-1 min-w-0">
                  <div class="text-xs text-gray-300 font-mono truncate" :title="file.path">
                    {{ file.path }}
                  </div>
                </div>

                <!-- Source -->
                <span class="text-[10px] text-gray-600 shrink-0">
                  {{ file.source === 'WechatDir' ? '微信' : '归档' }}
                </span>

                <!-- Size -->
                <div class="text-xs text-gray-400 shrink-0 w-16 text-right">
                  {{ formatSize(file.size) }}
                </div>

                <!-- Modified -->
                <div class="text-xs text-gray-500 shrink-0 w-28 text-right">
                  {{ formatTimestamp(file.modified) }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Loading more indicator -->
      <div v-if="loadingMore" class="text-center py-4 text-gray-500 text-sm">
        <svg class="animate-spin h-4 w-4 inline-block mr-2" viewBox="0 0 24 24" fill="none">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
        </svg>
        加载更多...
      </div>
    </div>

    <!-- Bottom Action Bar -->
    <div
      v-if="allGroups.length > 0"
      class="sticky bottom-0 bg-gray-800/95 backdrop-blur border border-gray-700 rounded-xl p-4 flex items-center gap-4"
    >
      <div class="flex-1">
        <span class="text-sm text-gray-400">
          已选 <span class="text-white font-medium">{{ totalSelectedFiles }}</span> 个文件,
          释放 <span class="text-green-400 font-medium">{{ formatSize(totalSelectedSize) }}</span>
        </span>
      </div>
      <button
        @click="showCleanupModal = true"
        :disabled="totalSelectedFiles === 0"
        class="px-6 py-2.5 bg-red-600 hover:bg-red-500 disabled:bg-gray-700 disabled:text-gray-500 rounded-lg text-sm font-medium text-white transition-all duration-200 flex items-center gap-2"
      >
        🗑️ 清理到回收站
      </button>
    </div>

    <!-- Cleanup Confirmation Modal -->
    <Teleport to="body">
      <div
        v-if="showCleanupModal"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
        @click.self="showCleanupModal = false"
      >
        <div class="bg-gray-800 border border-gray-700 rounded-2xl p-6 max-w-md w-full mx-4 shadow-2xl">
          <h3 class="text-lg font-semibold text-white mb-2">确认清理</h3>
          <p class="text-sm text-gray-400 mb-4">
            即将删除 <span class="text-white font-medium">{{ totalSelectedFiles }}</span> 个冗余文件,
            预计释放 <span class="text-green-400 font-medium">{{ formatSize(totalSelectedSize) }}</span> 空间。
          </p>
          <p class="text-xs text-gray-500 mb-5">
            文件将移至回收站，如需恢复可从回收站还原。
          </p>
          <div class="flex gap-3 justify-end">
            <button
              @click="showCleanupModal = false"
              class="px-4 py-2 bg-gray-700 hover:bg-gray-600 border border-gray-600 rounded-lg text-sm text-gray-300 transition-colors"
            >
              取消
            </button>
            <button
              @click="executeCleanup"
              :disabled="cleaningUp"
              class="px-4 py-2 bg-red-600 hover:bg-red-500 disabled:bg-red-800 rounded-lg text-sm text-white font-medium transition-colors flex items-center gap-2"
            >
              <svg v-if="cleaningUp" class="animate-spin h-4 w-4" viewBox="0 0 24 24" fill="none">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
              {{ cleaningUp ? '清理中...' : '确认清理' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
