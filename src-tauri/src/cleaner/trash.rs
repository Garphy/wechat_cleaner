use std::path::Path;
use crate::types::{CleanupReport, CleanupError, TrashMode};

pub fn cleanup_files(paths: &[String], mode: &TrashMode) -> CleanupReport {
    let mut report = CleanupReport {
        files_removed: 0,
        space_freed: 0,
        errors: vec![],
    };

    for path_str in paths {
        let path = Path::new(path_str);
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        let result: Result<(), String> = match mode {
            TrashMode::Trash => trash::delete(path).map_err(|e| e.to_string()),
            TrashMode::Delete => std::fs::remove_file(path).map_err(|e| e.to_string()),
        };

        match result {
            Ok(()) => {
                report.files_removed += 1;
                report.space_freed += size;
            }
            Err(e) => {
                report.errors.push(CleanupError {
                    path: path_str.clone(),
                    error: e,
                });
            }
        }
    }

    report
}

pub fn is_wechat_running() -> bool {
    // Platform-specific check
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq WeChat.exe"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("WeChat.exe"))
            .unwrap_or(false)
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("pgrep")
            .arg("WeChat")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        false
    }
}
