use std::path::PathBuf;
use crate::types::WechatAccount;

pub fn detect_wechat_accounts() -> Vec<WechatAccount> {
    let platform = crate::platform::get_platform();
    let paths = platform.detect_wechat_paths();
    let mut accounts = vec![];

    for path in paths {
        // Look for wxid_* subdirectories
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    // WeChat account dirs typically start with wxid_ or are custom names
                    // Check if it has a FileStorage subdirectory (indicating it's an account)
                    if entry.path().join("FileStorage").exists() {
                        accounts.push(WechatAccount {
                            name: name.clone(),
                            wxid: name,
                            data_path: entry.path(),
                        });
                    }
                }
            }
        }
    }

    accounts
}

pub fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
        .join("wechat-cleaner")
        .join("config.json")
}

pub fn load_config() -> crate::types::AppConfig {
    let path = get_config_path();
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&data) {
                return config;
            }
        }
    }
    // Default config
    crate::types::AppConfig {
        wechat_dir: dirs::document_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
            .join("WeChat Files"),
        archive_dirs: vec![],
        selected_account: None,
        trash_mode: crate::types::TrashMode::Trash,
        debug_enabled: true,
    }
}

pub fn save_config(config: &crate::types::AppConfig) -> Result<(), std::io::Error> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, data)
}
