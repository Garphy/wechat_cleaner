import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAppStore, type FileGroup } from '../stores/app'

// ── Mock Tauri invoke ────────────────────────────────────────────
const invokeMock = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: any[]) => invokeMock(...args),
}))

// ── Helpers ──────────────────────────────────────────────────────
function makeGroup(overrides: Partial<FileGroup> = {}): FileGroup {
  return {
    id: 'g1',
    group_type: 'CrossDedup',
    base_name: 'test.jpg',
    total_size: 1024,
    reclaimable_size: 512,
    files: [
      { path: '/a/test.jpg', size: 512, modified: 0, hash: 'abc', status: 'Keep', source: 'WechatDir' },
      { path: '/b/test.jpg', size: 512, modified: 0, hash: 'abc', status: 'Remove', source: 'ArchiveDir' },
    ],
    suggested_keep: 1,
    ...overrides,
  }
}

// ── Test: Scan completion detection ──────────────────────────────
describe('Scan completion detection', () => {
  function makeProgress(overrides: Partial<{ phase: string; scanned_files: number; total_files: number }> = {}) {
    return { total_files: 0, scanned_files: 0, total_size: 0, redundant_size: 0, current_path: '', phase: 'Walking', is_paused: false, is_cancelled: false, ...overrides }
  }

  function isScanComplete(p: ReturnType<typeof makeProgress>): boolean {
    return p.phase === 'Deduplicating' && p.scanned_files >= p.total_files && p.total_files > 0
  }

  it('should NOT be complete during Walking phase', () => {
    expect(isScanComplete(makeProgress({ phase: 'Walking', scanned_files: 10, total_files: 10 }))).toBe(false)
  })

  it('should NOT be complete when scanned < total', () => {
    expect(isScanComplete(makeProgress({ phase: 'Deduplicating', scanned_files: 5, total_files: 10 }))).toBe(false)
  })

  it('should NOT be complete when total_files is 0', () => {
    expect(isScanComplete(makeProgress({ phase: 'Deduplicating', scanned_files: 0, total_files: 0 }))).toBe(false)
  })

  it('should be complete when Deduplicating + scanned >= total', () => {
    expect(isScanComplete(makeProgress({ phase: 'Deduplicating', scanned_files: 10, total_files: 10 }))).toBe(true)
  })
})

// ── Test: File selection logic ────────────────────────────────────
describe('File selection (selectedFiles)', () => {
  let store: ReturnType<typeof useAppStore>

  beforeEach(() => {
    setActivePinia(createPinia())
    store = useAppStore()
  })

  it('should initialize with Remove files selected', () => {
    const group = makeGroup()
    store.initFileSelection([group])
    expect(store.isFileSelected('/a/test.jpg')).toBe(false) // Keep
    expect(store.isFileSelected('/b/test.jpg')).toBe(true)  // Remove
  })

  it('should toggle individual file', () => {
    const group = makeGroup()
    store.initFileSelection([group])
    store.toggleFile('/a/test.jpg') // Keep → selected (delete)
    expect(store.isFileSelected('/a/test.jpg')).toBe(true)
    store.toggleFile('/b/test.jpg') // Remove → deselected (keep)
    expect(store.isFileSelected('/b/test.jpg')).toBe(false)
  })

  it('should detect group fully selected', () => {
    const group = makeGroup()
    store.initFileSelection([group])
    expect(store.isGroupFullySelected(group)).toBe(false) // only Remove selected
    store.selectAllFiles([group])
    expect(store.isGroupFullySelected(group)).toBe(true)
  })

  it('should detect group partially selected', () => {
    const group = makeGroup()
    store.initFileSelection([group])
    expect(store.isGroupPartiallySelected(group)).toBe(true)
  })

  it('should toggle group files', () => {
    const group = makeGroup()
    store.initFileSelection([group])
    expect(store.isGroupFullySelected(group)).toBe(false)
    store.toggleGroupFiles(group) // select all
    expect(store.isGroupFullySelected(group)).toBe(true)
    store.toggleGroupFiles(group) // deselect all
    expect(store.getSelectedFileCount()).toBe(0)
  })

  it('should count selected files correctly', () => {
    const g1 = makeGroup({ id: 'g1' })
    const g2 = makeGroup({ id: 'g2', files: [{ path: '/c/test.jpg', size: 100, modified: 0, hash: 'def', status: 'Remove', source: 'WechatDir' }] })
    store.initFileSelection([g1, g2])
    expect(store.getSelectedFileCount()).toBe(2) // g1 Remove + g2 Remove
  })
})

// ── Test: Tauri invoke parameter names ───────────────────────────
describe('Tauri invoke parameter names', () => {
  beforeEach(() => invokeMock.mockReset())

  it('should use camelCase for get_paged_results', async () => {
    invokeMock.mockResolvedValue({ groups: [], total: 0, page: 0, pageSize: 100 })
    await invokeMock('get_paged_results', { page: 0, pageSize: 100, sort: 'size', order: 'desc' })
    const [, params] = invokeMock.mock.calls[0]
    expect(params).toHaveProperty('pageSize')
  })

  it('should send file paths for execute_cleanup', async () => {
    invokeMock.mockResolvedValue({ files_removed: 1, space_freed: 512, errors: [] })
    await invokeMock('execute_cleanup', { filePaths: ['/b/test.jpg'], mode: 'trash' })
    const [, params] = invokeMock.mock.calls[0]
    expect(params.filePaths).toEqual(['/b/test.jpg'])
    expect(params).not.toHaveProperty('selectedIds')
  })
})
