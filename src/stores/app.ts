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

  // selectedFiles: set of file paths to delete
  const selectedFiles = ref<Set<string>>(new Set())

  // Initialize selection: all 'Remove' files are selected by default
  function initFileSelection(groups: FileGroup[]) {
    selectedFiles.value = new Set()
    groups.forEach((g) => {
      g.files.forEach((f) => {
        if (f.status === 'Remove') {
          selectedFiles.value.add(f.path)
        }
      })
    })
  }

  function toggleFile(path: string) {
    if (selectedFiles.value.has(path)) {
      selectedFiles.value.delete(path)
    } else {
      selectedFiles.value.add(path)
    }
  }

  function isFileSelected(path: string): boolean {
    return selectedFiles.value.has(path)
  }

  function selectAllFiles(groups: FileGroup[]) {
    groups.forEach((g) => g.files.forEach((f) => selectedFiles.value.add(f.path)))
  }

  function deselectAllFiles() {
    selectedFiles.value.clear()
  }

  function toggleGroupFiles(group: FileGroup) {
    const allSelected = group.files.every((f) => selectedFiles.value.has(f.path))
    group.files.forEach((f) => {
      if (allSelected) {
        selectedFiles.value.delete(f.path)
      } else {
        selectedFiles.value.add(f.path)
      }
    })
  }

  function isGroupFullySelected(group: FileGroup): boolean {
    return group.files.length > 0 && group.files.every((f) => selectedFiles.value.has(f.path))
  }

  function isGroupPartiallySelected(group: FileGroup): boolean {
    const count = group.files.filter((f) => selectedFiles.value.has(f.path)).length
    return count > 0 && count < group.files.length
  }

  function getSelectedFileCount(): number {
    return selectedFiles.value.size
  }

  function getSelectedFiles(groups: FileGroup[]): FileEntry[] {
    const result: FileEntry[] = []
    groups.forEach((g) => g.files.forEach((f) => {
      if (selectedFiles.value.has(f.path)) result.push(f)
    }))
    return result
  }

  return {
    config,
    accounts,
    progress,
    scanResult,
    isScanning,
    selectedFiles,
    initFileSelection,
    toggleFile,
    isFileSelected,
    selectAllFiles,
    deselectAllFiles,
    toggleGroupFiles,
    isGroupFullySelected,
    isGroupPartiallySelected,
    getSelectedFileCount,
    getSelectedFiles,
  }
})
