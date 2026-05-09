pub mod dedup;
pub mod hash;
pub mod walker;

use crate::scanner::dedup::{find_cross_dedup, find_version_groups};
use crate::scanner::hash::hash_files_parallel;
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
    /// 3. Hash wechat_files (parallel)
    /// 4. Hash archive_files (parallel)
    /// 5. find_cross_dedup(wechat_files, archive_files)
    /// 6. find_version_groups(wechat_files)
    /// 7. Merge all groups, compute stats, return ScanResult
    pub fn start_scan(
        config: ScanConfig,
        progress: Arc<Mutex<ScanProgress>>,
        cancel: Arc<AtomicBool>,
        pause: Arc<AtomicBool>,
    ) -> ScanResult {
        let start_time = Instant::now();
        let walker = FileWalker::new();

        crate::debug::log(&format!("Scan started. wechat_dir={}, archive_dirs={:?}", config.wechat_dir.display(), config.archive_dirs));

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
        let wechat_files = walker.walk(&config.wechat_dir);
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
            let mut files = walker.walk(archive_dir);
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
            p.scanned_files = 0;
            p.current_path = "Walking complete".to_string();
        }

        if cancel.load(Ordering::Relaxed) {
            return Self::make_result(Vec::new(), 0, 0, start_time);
        }
        Self::check_pause(&pause);

        // Phase 2: Hashing
        {
            let mut p = progress.lock().unwrap();
            p.phase = ScanPhase::Hashing;
            p.current_path = "Hashing WeChat files...".to_string();
        }

        // Hash wechat files in parallel
        let wechat_hashes = hash_files_parallel(&wechat_files);
        crate::debug::log(&format!("WeChat hashes computed: {}", wechat_hashes.len()));

        if cancel.load(Ordering::Relaxed) {
            return Self::make_result(Vec::new(), 0, 0, start_time);
        }
        Self::check_pause(&pause);

        // Hash archive files in parallel
        {
            let mut p = progress.lock().unwrap();
            p.current_path = "Hashing archive files...".to_string();
        }
        let archive_hashes = hash_files_parallel(&archive_files);
        crate::debug::log(&format!("Archive hashes computed: {}", archive_hashes.len()));

        // Update scanned_files count
        {
            let mut p = progress.lock().unwrap();
            p.scanned_files = wechat_hashes.len() as u64 + archive_hashes.len() as u64;
            p.current_path = "Hashing complete".to_string();
        }

        if cancel.load(Ordering::Relaxed) {
            return Self::make_result(Vec::new(), 0, 0, start_time);
        }
        Self::check_pause(&pause);

        // Phase 3: Deduplicating
        {
            let mut p = progress.lock().unwrap();
            p.phase = ScanPhase::Deduplicating;
            p.current_path = "Cross-directory deduplication...".to_string();
        }

        // Build ScannedFile lists with hashes attached
        // We need to associate hashes with files for the dedup functions
        let wechat_with_hashes: Vec<ScannedFile> = wechat_files
            .iter()
            .map(|f| {
                let sf = f.clone();
                sf
            })
            .collect();

        // Cross-directory dedup
        let mut groups = find_cross_dedup(&wechat_with_hashes, &archive_files);
        crate::debug::log(&format!("Cross-dedup groups found: {}", groups.len()));

        if cancel.load(Ordering::Relaxed) {
            return Self::make_result(Vec::new(), 0, 0, start_time);
        }
        Self::check_pause(&pause);

        // Version convergence
        {
            let mut p = progress.lock().unwrap();
            p.current_path = "Version convergence analysis...".to_string();
        }
        let version_groups = find_version_groups(&wechat_with_hashes);
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

        ScanResult {
            groups,
            total_files,
            total_size,
            redundant_files,
            redundant_size,
            duration_ms: start_time.elapsed().as_millis() as u64,
        }
    }
}
