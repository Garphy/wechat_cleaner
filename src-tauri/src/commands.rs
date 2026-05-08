use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::types::*;
use crate::scanner;
use crate::cleaner::trash;
use crate::config::wechat;

/// Application state shared across Tauri commands
pub struct AppState {
    pub scan_progress: Arc<Mutex<ScanProgress>>,
    pub scan_result: Arc<Mutex<Option<ScanResult>>>,
    pub scan_cancel: Arc<AtomicBool>,
    pub scan_pause: Arc<AtomicBool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scan_progress: Arc::new(Mutex::new(ScanProgress {
                total_files: 0,
                scanned_files: 0,
                total_size: 0,
                redundant_size: 0,
                current_path: String::new(),
                phase: ScanPhase::Walking,
                is_paused: false,
                is_cancelled: false,
            })),
            scan_result: Arc::new(Mutex::new(None)),
            scan_cancel: Arc::new(AtomicBool::new(false)),
            scan_pause: Arc::new(AtomicBool::new(false)),
        }
    }
}

// ── Configuration Commands ──────────────────────────────────────────

#[tauri::command]
pub fn detect_wechat_paths() -> Vec<WechatAccount> {
    wechat::detect_wechat_accounts()
}

#[tauri::command]
pub fn get_config() -> AppConfig {
    wechat::load_config()
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    wechat::save_config(&config).map_err(|e| e.to_string())
}

// ── Scan Commands ───────────────────────────────────────────────────

#[tauri::command]
pub fn start_scan(config: ScanConfig, state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Reset state
    state.scan_cancel.store(false, Ordering::SeqCst);
    state.scan_pause.store(false, Ordering::SeqCst);
    {
        let mut progress = state.scan_progress.lock().map_err(|e| e.to_string())?;
        *progress = ScanProgress {
            total_files: 0,
            scanned_files: 0,
            total_size: 0,
            redundant_size: 0,
            current_path: String::new(),
            phase: ScanPhase::Walking,
            is_paused: false,
            is_cancelled: false,
        };
    }

    let progress = Arc::clone(&state.scan_progress);
    let result = Arc::clone(&state.scan_result);
    let cancel = Arc::clone(&state.scan_cancel);
    let pause = Arc::clone(&state.scan_pause);

    // Run scan in a blocking thread (Tauri manages the async runtime)
    std::thread::spawn(move || {
        let scan_result = scanner::ScanEngine::start_scan(config, progress, cancel, pause);
        if let Ok(mut r) = result.lock() {
            *r = Some(scan_result);
        }
    });

    Ok(())
}

#[tauri::command]
pub fn get_scan_progress(state: tauri::State<'_, AppState>) -> Result<ScanProgress, String> {
    state
        .scan_progress
        .lock()
        .map(|p| p.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pause_scan(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.scan_pause.store(true, Ordering::SeqCst);
    if let Ok(mut p) = state.scan_progress.lock() {
        p.is_paused = true;
    }
    Ok(())
}

#[tauri::command]
pub fn resume_scan(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.scan_pause.store(false, Ordering::SeqCst);
    if let Ok(mut p) = state.scan_progress.lock() {
        p.is_paused = false;
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_scan(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.scan_cancel.store(true, Ordering::SeqCst);
    if let Ok(mut p) = state.scan_progress.lock() {
        p.is_cancelled = true;
    }
    Ok(())
}

// ── Result Commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn get_scan_result(state: tauri::State<'_, AppState>) -> Result<Option<ScanResult>, String> {
    state
        .scan_result
        .lock()
        .map(|r| r.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_paged_results(
    page: u32,
    page_size: u32,
    sort: Option<String>,
    order: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<PagedResults, String> {
    let result = state
        .scan_result
        .lock()
        .map_err(|e| e.to_string())?;

    let scan = result.as_ref().ok_or("Scan not completed yet")?;
    let mut groups = scan.groups.clone();

    // Sort
    let sort_key = sort.unwrap_or_else(|| "size".to_string());
    let ascending = order.as_deref() != Some("desc");
    groups.sort_by(|a, b| {
        let cmp = match sort_key.as_str() {
            "name" => a.base_name.cmp(&b.base_name),
            "time" => a
                .files
                .first()
                .map(|f| f.modified)
                .cmp(&b.files.first().map(|f| f.modified))
                .reverse(),
            "count" => a.files.len().cmp(&b.files.len()).reverse(),
            _ => a.reclaimable_size.cmp(&b.reclaimable_size).reverse(),
        };
        if ascending {
            cmp
        } else {
            cmp.reverse()
        }
    });

    let total = groups.len();
    let start = (page * page_size) as usize;
    let end = std::cmp::min(start + page_size as usize, total);
    let paged = if start < total {
        groups[start..end].to_vec()
    } else {
        vec![]
    };

    Ok(PagedResults {
        groups: paged,
        total,
        page,
        page_size,
    })
}

// ── Cleanup Commands ────────────────────────────────────────────────

#[tauri::command]
pub fn execute_cleanup(
    selected_ids: Vec<String>,
    mode: String,
    state: tauri::State<'_, AppState>,
) -> Result<CleanupReport, String> {
    let result = state
        .scan_result
        .lock()
        .map_err(|e| e.to_string())?;

    let scan = result.as_ref().ok_or("Scan not completed yet")?;

    // Collect all file paths from selected groups
    let mut paths: Vec<String> = Vec::new();
    for group in &scan.groups {
        if selected_ids.contains(&group.id) {
            for file in &group.files {
                if file.status == FileStatus::Remove {
                    paths.push(file.path.clone());
                }
            }
        }
    }

    let trash_mode = match mode.as_str() {
        "delete" => TrashMode::Delete,
        _ => TrashMode::Trash,
    };

    Ok(trash::cleanup_files(&paths, &trash_mode))
}

#[tauri::command]
pub fn check_wechat_running() -> bool {
    trash::is_wechat_running()
}
