#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "macos")]
pub mod macos;

use std::path::PathBuf;

pub trait PlatformOps {
    fn detect_wechat_paths(&self) -> Vec<PathBuf>;
    fn is_wechat_running(&self) -> bool;
    fn get_default_data_dir(&self) -> Option<PathBuf>;
}

pub fn get_platform() -> Box<dyn PlatformOps> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsOps)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsOps)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        panic!("Unsupported platform")
    }
}
