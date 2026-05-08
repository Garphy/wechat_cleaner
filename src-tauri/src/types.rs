use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatAccount {
    pub name: String,
    pub wxid: String,
    pub data_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub wechat_dir: PathBuf,
    pub archive_dirs: Vec<PathBuf>,
    pub selected_account: Option<String>,
    pub trash_mode: TrashMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrashMode {
    Trash,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub modified: i64,
    pub hash: String,
    pub status: FileStatus,
    pub source: SourceDir,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    Keep,
    Remove,
    UserDecided,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceDir {
    WechatDir,
    ArchiveDir,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupType {
    CrossDedup,
    VersionConverge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileGroup {
    pub id: String,
    pub group_type: GroupType,
    pub base_name: String,
    pub total_size: u64,
    pub reclaimable_size: u64,
    pub files: Vec<FileEntry>,
    pub suggested_keep: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub total_files: u64,
    pub scanned_files: u64,
    pub total_size: u64,
    pub redundant_size: u64,
    pub current_path: String,
    pub phase: ScanPhase,
    pub is_paused: bool,
    pub is_cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanPhase {
    Walking,
    Hashing,
    Deduplicating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub groups: Vec<FileGroup>,
    pub total_files: u64,
    pub total_size: u64,
    pub redundant_files: u64,
    pub redundant_size: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    pub files_removed: u64,
    pub space_freed: u64,
    pub errors: Vec<CleanupError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupError {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub wechat_dir: PathBuf,
    pub archive_dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResults {
    pub groups: Vec<FileGroup>,
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
}
