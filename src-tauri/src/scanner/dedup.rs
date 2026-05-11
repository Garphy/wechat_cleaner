use crate::scanner::hash::{hash_file, hash_file_head_tail};
use crate::scanner::walker::ScannedFile;
use crate::types::*;
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::io;

/// Cross-directory deduplication: find files in wechat_dir that are identical
/// to files in archive_dirs.
///
/// Optimized for large file sets (100K+) with remote storage:
/// - Pre-groups by size in a single pass (no O(n²) filtering)
/// - Caches full hashes to avoid recomputation
/// - Parallelizes partial hash computation within each size group
pub fn find_cross_dedup(
    wechat_files: &[ScannedFile],
    archive_files: &[ScannedFile],
) -> Vec<FileGroup> {
    // Pre-group all files by size with source tag — single pass, no filtering needed later
    let mut size_map: HashMap<u64, Vec<(&ScannedFile, SourceDir)>> = HashMap::new();
    for f in wechat_files {
        size_map.entry(f.size).or_default().push((f, SourceDir::WechatDir));
    }
    for f in archive_files {
        size_map.entry(f.size).or_default().push((f, SourceDir::ArchiveDir));
    }

    let total_size_groups = size_map.len();
    crate::debug::log(&format!(
        "find_cross_dedup: {} wechat files, {} archive files, {} unique sizes",
        wechat_files.len(), archive_files.len(), total_size_groups
    ));

    let mut groups = Vec::new();
    let mut group_id_counter = 0u64;
    let mut full_hash_cache: HashMap<String, String> = HashMap::new(); // path → hash
    let mut processed_sizes = 0usize;

    for (_size, files) in &size_map {
        processed_sizes += 1;

        // Split by source — O(n) within this size group only
        let wechat_in_group: Vec<&ScannedFile> = files.iter()
            .filter(|(_, src)| matches!(src, SourceDir::WechatDir))
            .map(|(f, _)| *f)
            .collect();
        let archive_in_group: Vec<&ScannedFile> = files.iter()
            .filter(|(_, src)| matches!(src, SourceDir::ArchiveDir))
            .map(|(f, _)| *f)
            .collect();

        if wechat_in_group.is_empty() || archive_in_group.is_empty() {
            continue;
        }

        // Log progress every 1000 size groups
        if processed_sizes % 1000 == 0 {
            crate::debug::log(&format!(
                "find_cross_dedup: processed {}/{} size groups, {} matches so far",
                processed_sizes, total_size_groups, groups.len()
            ));
        }

        // Compute partial hashes in parallel for this size group
        let wechat_hashes: Vec<(&ScannedFile, io::Result<String>)> = wechat_in_group
            .par_iter()
            .map(|f| (*f, hash_file_head_tail(&f.path)))
            .collect();
        let archive_hashes: Vec<(&ScannedFile, io::Result<String>)> = archive_in_group
            .par_iter()
            .map(|f| (*f, hash_file_head_tail(&f.path)))
            .collect();

        // Group wechat files by partial hash
        let mut wechat_by_hash: HashMap<String, Vec<&ScannedFile>> = HashMap::new();
        for (sf, result) in &wechat_hashes {
            if let Ok(h) = result {
                wechat_by_hash.entry(h.clone()).or_default().push(sf);
            }
        }

        // For each archive file, check partial hash match then verify with full hash
        for (archive_sf, archive_hash_result) in &archive_hashes {
            let archive_hash = match archive_hash_result {
                Ok(h) => h,
                Err(_) => continue,
            };

            let matching_wechat = match wechat_by_hash.get(archive_hash) {
                Some(m) => m,
                None => continue,
            };

            // Compute full hash for archive file (with cache)
            let archive_path_key = archive_sf.path.to_string_lossy().to_string();
            let archive_full_hash = if let Some(cached) = full_hash_cache.get(&archive_path_key) {
                cached.clone()
            } else {
                match hash_file(&archive_sf.path) {
                    Ok(h) => {
                        full_hash_cache.insert(archive_path_key, h.clone());
                        h
                    }
                    Err(_) => continue,
                }
            };

            for wechat_sf in matching_wechat {
                // Compute full hash for wechat file (with cache)
                let wechat_path_key = wechat_sf.path.to_string_lossy().to_string();
                let wechat_full_hash = if let Some(cached) = full_hash_cache.get(&wechat_path_key) {
                    cached.clone()
                } else {
                    match hash_file(&wechat_sf.path) {
                        Ok(h) => {
                            full_hash_cache.insert(wechat_path_key, h.clone());
                            h
                        }
                        Err(_) => continue,
                    }
                };

                if wechat_full_hash == archive_full_hash {
                    let base_name = wechat_sf
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let file_entry_wechat = FileEntry {
                        path: wechat_sf.path.to_string_lossy().to_string(),
                        size: wechat_sf.size,
                        modified: wechat_sf.modified,
                        hash: wechat_full_hash.clone(),
                        status: FileStatus::Remove,
                        source: SourceDir::WechatDir,
                    };

                    let file_entry_archive = FileEntry {
                        path: archive_sf.path.to_string_lossy().to_string(),
                        size: archive_sf.size,
                        modified: archive_sf.modified,
                        hash: archive_full_hash.clone(),
                        status: FileStatus::Keep,
                        source: SourceDir::ArchiveDir,
                    };

                    group_id_counter += 1;
                    groups.push(FileGroup {
                        id: format!("cross-{}", group_id_counter),
                        group_type: GroupType::CrossDedup,
                        base_name,
                        total_size: wechat_sf.size + archive_sf.size,
                        reclaimable_size: wechat_sf.size,
                        files: vec![file_entry_wechat, file_entry_archive],
                        suggested_keep: 1,
                    });
                }
            }
        }
    }

    crate::debug::log(&format!(
        "find_cross_dedup complete: {} groups found, full_hash_cache size={}",
        groups.len(), full_hash_cache.len()
    ));

    groups
}

/// Version convergence: find files with pattern like `name(1).ext`, `name(1)(2).ext`
/// within the same directory, and suggest keeping only the newest.
pub fn find_version_groups(files: &[ScannedFile]) -> Vec<FileGroup> {
    let re = Regex::new(r"^(.+?)(?:\s*\(\d+\))+(\.\w+)$").unwrap();
    let mut groups_map: HashMap<String, Vec<&ScannedFile>> = HashMap::new();

    for f in files {
        let file_name = match f.path.file_name().map(|n| n.to_string_lossy().to_string()) {
            Some(n) => n,
            None => continue,
        };

        if let Some(caps) = re.captures(&file_name) {
            let base_name = caps[1].trim().to_string();
            let ext = caps[2].to_string();
            let key = format!("{}{}", base_name, ext);
            groups_map.entry(key).or_default().push(f);
        }
    }

    let mut groups = Vec::new();
    let mut group_id_counter = 0u64;

    for (base_name, group_files) in &groups_map {
        if group_files.len() <= 1 {
            continue;
        }

        // Sort by modified time descending (newest first)
        let mut sorted_files = group_files.clone();
        sorted_files.sort_by(|a, b| b.modified.cmp(&a.modified));

        let total_size: u64 = sorted_files.iter().map(|f| f.size).sum();

        // Suggested keep is index 0 (newest file)
        let suggested_keep = 0;

        // Reclaimable size is total minus the file we keep
        let reclaimable_size = total_size - sorted_files[0].size;

        let file_entries: Vec<FileEntry> = sorted_files
            .iter()
            .enumerate()
            .map(|(idx, f)| FileEntry {
                path: f.path.to_string_lossy().to_string(),
                size: f.size,
                modified: f.modified,
                hash: String::new(), // Will be filled in during hashing phase
                status: if idx == suggested_keep {
                    FileStatus::Keep
                } else {
                    FileStatus::Remove
                },
                source: SourceDir::WechatDir,
            })
            .collect();

        group_id_counter += 1;
        groups.push(FileGroup {
            id: format!("version-{}", group_id_counter),
            group_type: GroupType::VersionConverge,
            base_name: base_name.clone(),
            total_size,
            reclaimable_size,
            files: file_entries,
            suggested_keep,
        });
    }

    groups
}
