use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static LOG_FILE: Lazy<Mutex<Option<std::fs::File>>> = Lazy::new(|| Mutex::new(None));
static DEBUG_ENABLED: Lazy<std::sync::atomic::AtomicBool> = Lazy::new(|| std::sync::atomic::AtomicBool::new(false));

pub fn get_log_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
}

pub fn get_log_path() -> PathBuf {
    get_log_dir().join("debug.log")
}

pub fn init_debug_log() {
    let log_dir = get_log_dir();
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Failed to create log dir: {}", e);
        return;
    }

    let log_path = get_log_path();
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(file) => {
            if let Ok(mut f) = LOG_FILE.lock() {
                *f = Some(file);
            }
            // Write header
            log_raw(&format!("\n=== Debug Log Started ===\n"));
        }
        Err(e) => {
            eprintln!("Failed to open log file: {}", e);
        }
    }
}

pub fn set_debug_enabled(enabled: bool) {
    DEBUG_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn log(message: &str) {
    if !is_debug_enabled() {
        return;
    }
    log_raw(message);
}

pub fn log_raw(message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{}] {}\n", timestamp, message);

    if let Ok(mut f) = LOG_FILE.lock() {
        if let Some(file) = f.as_mut() {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }
}

pub fn clear_log() {
    let log_path = get_log_path();
    let _ = std::fs::write(&log_path, "");
    init_debug_log();
}
