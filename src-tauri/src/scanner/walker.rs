use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub size: u64,
    pub modified: i64,
}

pub struct FileWalker {
    skip_patterns: Vec<&'static str>,
}

impl FileWalker {
    pub fn new() -> Self {
        Self {
            skip_patterns: vec![".db-shm", ".db-wal", ".ini"],
        }
    }

    pub fn should_skip(&self, path: &Path) -> bool {
        let name = path.to_string_lossy();
        // Skip database files, INI files
        if self.skip_patterns.iter().any(|p| name.contains(p)) {
            return true;
        }
        // Skip hidden directories (starting with .)
        if let Some(parent) = path.parent() {
            if let Some(dir_name) = parent.file_name() {
                if dir_name.to_string_lossy().starts_with('.') {
                    return true;
                }
            }
        }
        false
    }

    pub fn walk(&self, root: &Path) -> Vec<ScannedFile> {
        WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| !self.should_skip(e.path()))
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                Some(ScannedFile {
                    path: e.path().to_path_buf(),
                    size: meta.len(),
                    modified,
                })
            })
            .collect()
    }
}
