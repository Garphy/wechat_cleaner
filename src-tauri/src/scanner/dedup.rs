use crate::scanner::hash::{hash_file, hash_file_head_tail};
use crate::scanner::walker::ScannedFile;
use crate::types::*;
use regex::Regex;
use std::collections::HashMap;
use std::io;

/// Cross-directory deduplication: find files in wechat_dir that are identical
/// to files in archive_dirs.
pub fn find_cross_dedup(
    wechat_files: &[ScannedFile],
    archive_files: &[ScannedFile],
) -> Vec<FileGroup> {
    // Group all files by size
    let mut size_map: HashMap<u64, Vec<&ScannedFile>> = HashMap::new();

    for f in wechat_files {
        size_map.entry(f.size).or_default().push(f);
    }
    for f in archive_files {
        size_map.entry(f.size).or_default().push(f);
    }

    let mut groups = Vec::new();
    let mut group_id_counter = 0u64;

    for (_size, files) in &size_map {
        // Only consider groups with files from BOTH wechat and archive
        let wechat_in_group: Vec<&&ScannedFile> = files
            .iter()
            .filter(|f| wechat_files.iter().any(|wf| wf.path == f.path))
            .collect();
        let archive_in_group: Vec<&&ScannedFile> = files
            .iter()
            .filter(|f| archive_files.iter().any(|af| af.path == f.path))
            .collect();

        if wechat_in_group.is_empty() || archive_in_group.is_empty() {
            continue;
        }

        // Compute partial hashes for all files in this size group
        let wechat_hashes: Vec<(&ScannedFile, io::Result<String>)> = wechat_in_group
            .iter()
            .map(|f| (**f, hash_file_head_tail(&f.path)))
            .collect();
        let archive_hashes: Vec<(&ScannedFile, io::Result<String>)> = archive_in_group
            .iter()
            .map(|f| (**f, hash_file_head_tail(&f.path)))
            .collect();

        // Group wechat files by partial hash
        let mut wechat_by_hash: HashMap<String, Vec<&ScannedFile>> = HashMap::new();
        for (sf, result) in &wechat_hashes {
            if let Ok(h) = result {
                wechat_by_hash.entry(h.clone()).or_default().push(sf);
            }
        }

        // For each archive file, check partial hash match
        for (archive_sf, archive_hash_result) in &archive_hashes {
            let archive_hash = match archive_hash_result {
                Ok(h) => h,
                Err(_) => continue,
            };

            let matching_wechat = match wechat_by_hash.get(archive_hash) {
                Some(m) => m,
                None => continue,
            };

            // Partial match found — verify with full hash
            let archive_full_hash = match hash_file(&archive_sf.path) {
                Ok(h) => h,
                Err(_) => continue,
            };

            for wechat_sf in matching_wechat {
                let wechat_full_hash = match hash_file(&wechat_sf.path) {
                    Ok(h) => h,
                    Err(_) => continue,
                };

                if wechat_full_hash == archive_full_hash {
                    // Found a cross-dedup match: wechat file is redundant
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
                        suggested_keep: 1, // Index 1 is the archive copy
                    });
                }
            }
        }
    }

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
