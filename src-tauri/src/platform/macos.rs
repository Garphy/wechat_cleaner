use super::PlatformOps;
use std::path::PathBuf;

pub struct MacOsOps;

impl PlatformOps for MacOsOps {
    fn detect_wechat_paths(&self) -> Vec<PathBuf> {
        // TODO: macOS implementation
        vec![]
    }
    fn is_wechat_running(&self) -> bool {
        // TODO: check pgrep for WeChat
        false
    }
    fn get_default_data_dir(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join("Library/Containers/com.tencent.xinWeChat/Data/Library/Application Support/com.tencent.xinWeChat"))
    }
}
