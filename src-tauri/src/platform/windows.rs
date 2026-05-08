use super::PlatformOps;
use std::path::PathBuf;

pub struct WindowsOps;

impl PlatformOps for WindowsOps {
    fn detect_wechat_paths(&self) -> Vec<PathBuf> {
        vec![]
    }
    fn is_wechat_running(&self) -> bool {
        false
    }
    fn get_default_data_dir(&self) -> Option<PathBuf> {
        dirs::document_dir().map(|d| d.join("WeChat Files"))
    }
}
