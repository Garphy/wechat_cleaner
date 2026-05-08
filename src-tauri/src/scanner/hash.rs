use crate::scanner::walker::ScannedFile;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Compute full SHA-256 hash of a file using 64KB streaming reads.
pub fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536]; // 64KB buffer

    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Compute partial hash: SHA-256 of (first 4KB || last 4KB) for quick comparison.
/// If file is smaller than 4KB, hashes the entire content.
pub fn hash_file_head_tail(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut head_buf = [0u8; 4096]; // 4KB

    // Read head
    let head_bytes = file.read(&mut head_buf)?;
    if head_bytes > 0 {
        hasher.update(&head_buf[..head_bytes]);
    }

    // Read tail: seek to last 4KB if file is larger than 4KB
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    if file_size > 4096 {
        // Read last 4KB
        let mut tail_buf = [0u8; 4096];
        file.seek(SeekFrom::End(-4096))?;
        let tail_bytes = file.read(&mut tail_buf)?;
        if tail_bytes > 0 {
            hasher.update(&tail_buf[..tail_bytes]);
        }
    }
    // If file <= 4KB, the head read already captured everything.

    Ok(format!("{:x}", hasher.finalize()))
}

/// Hash multiple files in parallel using rayon.
/// Returns Vec of (path, full_hash).
pub fn hash_files_parallel(files: &[ScannedFile]) -> Vec<(PathBuf, String)> {
    files
        .par_iter()
        .filter_map(|sf| {
            let hash = hash_file(&sf.path).ok()?;
            Some((sf.path.clone(), hash))
        })
        .collect()
}
