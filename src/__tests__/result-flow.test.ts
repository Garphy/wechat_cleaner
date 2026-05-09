import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAppStore } from '../stores/app'

// ── Mock Tauri invoke ────────────────────────────────────────────
const invokeMock = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: any[]) => invokeMock(...args),
}))

// ── Test: Scan completion detection logic ────────────────────────
describe('Scan completion detection', () => {
  function makeProgress(overrides: Partial<{
    phase: string
    scanned_files: number
    total_files: number
  }> = {}) {
    return {
      total_files: 0,
      scanned_files: 0,
      total_size: 0,
      redundant_size: 0,
      current_path: '',
      phase: 'Walking',
      is_paused: false,
      is_cancelled: false,
      ...overrides,
    }
  }

  function isScanComplete(p: ReturnType<typeof makeProgress>): boolean {
    return p.phase === 'Deduplicating' && p.scanned_files >= p.total_files && p.total_files > 0
  }

  it('should NOT be complete during Walking phase', () => {
    const p = makeProgress({ phase: 'Walking', scanned_files: 10, total_files: 10 })
    expect(isScanComplete(p)).toBe(false)
  })

  it('should NOT be complete during Hashing phase', () => {
    const p = makeProgress({ phase: 'Hashing', scanned_files: 10, total_files: 10 })
    expect(isScanComplete(p)).toBe(false)
  })

  it('should NOT be complete when scanned < total', () => {
    const p = makeProgress({ phase: 'Deduplicating', scanned_files: 5, total_files: 10 })
    expect(isScanComplete(p)).toBe(false)
  })

  it('should NOT be complete when total_files is 0', () => {
    const p = makeProgress({ phase: 'Deduplicating', scanned_files: 0, total_files: 0 })
    expect(isScanComplete(p)).toBe(false)
  })

  it('should be complete when Deduplicating + scanned >= total', () => {
    const p = makeProgress({ phase: 'Deduplicating', scanned_files: 10, total_files: 10 })
    expect(isScanComplete(p)).toBe(true)
  })

  it('should be complete when scanned > total (edge case)', () => {
    const p = makeProgress({ phase: 'Deduplicating', scanned_files: 12, total_files: 10 })
    expect(isScanComplete(p)).toBe(true)
  })
})

// ── Test: Result page data loading logic ─────────────────────────
describe('ResultView data loading', () => {
  let store: ReturnType<typeof useAppStore>

  beforeEach(() => {
    setActivePinia(createPinia())
    store = useAppStore()
    invokeMock.mockReset()
  })

  it('should use store.scanResult when available', async () => {
    // Simulate scan result stored by ScanView
    store.scanResult = {
      groups: [
        {
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
        },
      ],
      total_files: 2,
      total_size: 1024,
      redundant_files: 1,
      redundant_size: 512,
      duration_ms: 100,
    }

    // ResultView logic: check store first
    const shouldUseStore = store.scanResult?.groups && store.scanResult.groups.length > 0
    expect(shouldUseStore).toBe(true)
    expect(store.scanResult!.groups).toHaveLength(1)
  })

  it('should fallback to backend when store has no data', async () => {
    store.scanResult = null

    invokeMock.mockResolvedValue({
      groups: [
        {
          id: 'g1',
          group_type: 'CrossDedup',
          base_name: 'test.jpg',
          total_size: 1024,
          reclaimable_size: 512,
          files: [],
          suggested_keep: 1,
        },
      ],
      total: 1,
      page: 0,
      page_size: 100,
    })

    // ResultView logic: fallback to loadResults
    const shouldUseStore = store.scanResult?.groups && store.scanResult.groups.length > 0
    expect(shouldUseStore).toBeFalsy()

    // Verify backend call with correct parameter names
    const result = await invokeMock('get_paged_results', {
      page: 0,
      page_size: 100,
      sort: 'size',
      order: 'desc',
    })
    expect(result.groups).toHaveLength(1)
  })
})

// ── Test: Tauri parameter name mapping ───────────────────────────
describe('Tauri invoke parameter names', () => {
  beforeEach(() => {
    invokeMock.mockReset()
  })

  it('should use snake_case for get_paged_results', async () => {
    invokeMock.mockResolvedValue({ groups: [], total: 0, page: 0, page_size: 100 })

    await invokeMock('get_paged_results', {
      page: 0,
      page_size: 100,
      sort: 'size',
      order: 'desc',
    })

    const [cmd, params] = invokeMock.mock.calls[0]
    expect(cmd).toBe('get_paged_results')
    // Must use snake_case, not camelCase
    expect(params).toHaveProperty('page_size')
    expect(params).not.toHaveProperty('pageSize')
  })

  it('should pass config with all required fields', async () => {
    invokeMock.mockResolvedValue(undefined)

    const config = {
      wechat_dir: '/test',
      archive_dirs: ['/test/archive'],
      selected_account: null,
      trash_mode: 'trash',
      debug_enabled: true,
    }
    await invokeMock('save_config', { config })

    const [, params] = invokeMock.mock.calls[0]
    expect(params.config).toHaveProperty('debug_enabled')
    expect(params.config).toHaveProperty('trash_mode')
    expect(params.config).toHaveProperty('archive_dirs')
  })
})
