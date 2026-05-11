pub mod dedup;
pub mod hash;
pub mod walker;

use crate::scanner::dedup::{find_cross_dedup, find_version_groups};
use crate::scanner::walker::{FileWalker, ScannedFile};
use crate::types::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct ScanEngine;

impl ScanEngine {
    /// Start a full scan operation.
    ///
    /// 1. Walk wechat_dir → get wechat_files
    /// 2. Walk each archive_dir → get archive_files
    /// 3. find_cross_dedup(wechat_files, archive_files) — hashes on-demand via size→partial→full filter
    /// 4. find_version_groups(wechat_files)
    /// 5. Merge all groups, compute stats, return ScanResult
    pub fn start_scan(
        config: ScanConfig,
        progress: Arc<Mutex<ScanProgress>>,
        cancel: Arc<AtomicBool>,
        pause: Arc<AtomicBool>,
    ) -> ScanResult {
        let start_time = Instant::now();
        let walker = FileWalker::new();

        let modified_after = config.date_range.as_ref().and_then(|d| d.after);
        crate::debug::log(&format!("Scan started. wechat_dir={}, archive_dirs={:?}, modified_after={:?}", config.wechat_dir.display(), config.archive_dirs, modified_after));

        // Phase 1: Walking
        {
            let mut p = progress.lock().unwrap();
            p.phase = ScanPhase::Walking;
            p.current_path = "Scanning directories...".to_string();
        }

        if cancel.load(Ordering::Relaxed) {
            return Self::make_result(Vec::new(), 0, 0, start_time);
        }

        // Walk wechat directory
        let wechat_files = walker.walk(&config.wechat_dir, modified_after);
        crate::debug::log(&format!("WeChat files found: {}", wechat_files.len()));

        if cancel.load(Ordering::Relaxed) {
            return Self::make_result(Vec::new(), 0, 0, start_time);
        }
        Self::check_pause(&pause);

        // Walk each archive directory
        let mut archive_files = Vec::new();
        for archive_dir in &config.archive_dirs {
            if cancel.load(Ordering::Relaxed) {
                return Self::make_result(Vec::new(), 0, 0, start_time);
            }
            let mut files = walker.walk(archive_dir, modified_after);
            archive_files.append(&mut files);
        }

        // Update progress after walking
        let total_files = (wechat_files.len() + archive_files.len()) as u64;
        let total_size: u64 = wechat_files.iter().map(|f| f.size).sum::<u64>()
            + archive_files.iter().map(|f| f.size).sum::<u64>();
        crate::debug::log(&format!("Walking complete. total_files={}, total_size={}", total_files, total_size));
        {
            let mut p = progress.lock().unwrap();
            p.total_files = total_files;
            p.total_size = total_size;
            p.scanned_files = total_files; // walking is done, hashing happens on-demand in dedup
            p.current_path = "Walking complete, starting dedup...".to_string();
        }

        if cancel.load(Ordering::Relaxed) {
            return Self::make_result(Vec::new(), 0, 0, start_time);
        }
        Self::check_pause(&pause);

        // Phase 2: Deduplicating
        // No upfront full hashing — find_cross_dedup does size → partial_hash → full_hash filtering,
        // only computing full SHA-256 for the small subset of files that match.
        {
            let mut p = progress.lock().unwrap();
            p.phase = ScanPhase::Deduplicating;
            p.current_path = "Cross-directory deduplication...".to_string();
        }

        crate::debug::log("Starting cross-directory dedup (on-demand hashing)...");
        let mut groups = find_cross_dedup(&wechat_files, &archive_files);
        crate::debug::log(&format!("Cross-dedup groups found: {}", groups.len()));

        if cancel.load(Ordering::Relaxed) {
            return Self::make_result(Vec::new(), 0, 0, start_time);
        }
        Self::check_pause(&pause);

        // Version convergence (no hashing needed — uses filename pattern + modified time)
        {
            let mut p = progress.lock().unwrap();
            p.current_path = "Version convergence analysis...".to_string();
        }
        crate::debug::log("Starting version convergence analysis...");
        let version_groups = find_version_groups(&wechat_files);
        crate::debug::log(&format!("Version groups found: {}", version_groups.len()));
        groups.extend(version_groups);

        // Compute stats
        let redundant_files: u64 = groups
            .iter()
            .flat_map(|g| &g.files)
            .filter(|f| f.status == FileStatus::Remove)
            .count() as u64;
        let redundant_size: u64 = groups
            .iter()
            .map(|g| g.reclaimable_size)
            .sum();

        // Mark scan as complete
        {
            let mut p = progress.lock().unwrap();
            p.scanned_files = p.total_files;
            p.redundant_size = redundant_size;
            p.current_path = "Scan complete".to_string();
            p.is_complete = true;
        }

        crate::debug::log(&format!(
            "Scan complete. groups={}, redundant_files={}, redundant_size={}, duration={:?}",
            groups.len(), redundant_files, redundant_size, start_time.elapsed()
        ));

        Self::make_result(groups, redundant_files, redundant_size, start_time)
    }

    fn check_pause(pause: &AtomicBool) {
        while pause.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    fn make_result(
        groups: Vec<FileGroup>,
        redundant_files: u64,
        redundant_size: u64,
        start_time: Instant,
    ) -> ScanResult {
        let total_files: u64 = groups
            .iter()
            .flat_map(|g| &g.files)
            .count() as u64;
        let total_size: u64 = groups.iter().map(|g| g.total_size).sum();

        let wechat_files: u64 = groups
            .iter()
            .flat_map(|g| &g.files)
            .filter(|f| f.source == SourceDir::WechatDir)
            .count() as u64;
        let wechat_size: u64 = groups
            .iter()
            .flat_map(|g| &g.files)
            .filter(|f| f.source == SourceDir::WechatDir)
            .map(|f| f.size)
            .sum();

        ScanResult {
            groups,
            total_files,
            total_size,
            redundant_files,
            redundant_size,
            wechat_files,
            wechat_size,
            duration_ms: start_time.elapsed().as_millis() as u64,
        }
    }
}
