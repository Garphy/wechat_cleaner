import { ref, computed } from 'vue'
import { defineStore } from 'pinia'

export interface WechatAccount {
  name: string
  wxid: string
  data_path: string
}

export interface AppConfig {
  wechat_dir: string
  archive_dirs: string[]
  selected_account: string | null
  trash_mode: string
  debug_enabled: boolean
}

export interface ScanProgress {
  total_files: number
  scanned_files: number
  total_size: number
  redundant_size: number
  current_path: string
  phase: string
  is_paused: boolean
  is_cancelled: boolean
}

export interface FileEntry {
  path: string
  size: number
  modified: number
  hash: string
  status: string
  source: string
}

export interface FileGroup {
  id: string
  group_type: string
  base_name: string
  total_size: number
  reclaimable_size: number
  files: FileEntry[]
  suggested_keep: number
}

export interface ScanResult {
  groups: FileGroup[]
  total_files: number
  total_size: number
  redundant_files: number
  redundant_size: number
  duration_ms: number
}

export interface PagedResults {
  groups: FileGroup[]
  total: number
  page: number
  page_size: number
}

export interface CleanupReport {
  files_removed: number
  space_freed: number
  errors: Array<{ path: string; error: string }>
}

export const useAppStore = defineStore('app', () => {
  const config = ref<AppConfig | null>(null)
  const accounts = ref<WechatAccount[]>([])
  const progress = ref<ScanProgress | null>(null)
  const scanResult = ref<ScanResult | null>(null)
  const isScanning = ref(false)
  const selectedGroupIds = ref<Set<string>>(new Set())

  const totalSelectedSize = computed(() => {
    if (!scanResult.value) return 0
    return scanResult.value.groups
      .filter((g) => selectedGroupIds.value.has(g.id))
      .reduce((acc, g) => acc + g.reclaimable_size, 0)
  })

  const totalSelectedFiles = computed(() => {
    if (!scanResult.value) return 0
    return scanResult.value.groups
      .filter((g) => selectedGroupIds.value.has(g.id))
      .reduce((acc, g) => acc + g.files.filter((f) => f.status === 'Remove').length, 0)
  })

  function toggleGroup(id: string) {
    if (selectedGroupIds.value.has(id)) {
      selectedGroupIds.value.delete(id)
    } else {
      selectedGroupIds.value.add(id)
    }
  }

  function selectAllGroups() {
    scanResult.value?.groups.forEach((g) => selectedGroupIds.value.add(g.id))
  }

  function deselectAllGroups() {
    selectedGroupIds.value.clear()
  }

  return {
    config,
    accounts,
    progress,
    scanResult,
    isScanning,
    selectedGroupIds,
    totalSelectedSize,
    totalSelectedFiles,
    toggleGroup,
    selectAllGroups,
    deselectAllGroups,
  }
})
