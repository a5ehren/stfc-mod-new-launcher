use crate::errors::{io_context, LauncherResult};
use chrono::Utc;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogEntry<'a> {
    timestamp: String,
    level: &'a str,
    category: &'a str,
    message: &'a str,
}

#[derive(Debug, Clone)]
pub struct DiagnosticsService {
    logs_dir: PathBuf,
    log_file: PathBuf,
    write_lock: Arc<Mutex<()>>,
}

impl DiagnosticsService {
    pub fn new(logs_dir: PathBuf) -> Self {
        Self {
            log_file: logs_dir.join("launcher.log.jsonl"),
            logs_dir,
            write_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn log_file(&self) -> &Path {
        &self.log_file
    }

    pub fn ensure_logs_dir(&self) -> LauncherResult<()> {
        fs::create_dir_all(&self.logs_dir)
            .map_err(|err| io_context(format!("creating {}", self.logs_dir.display()), err))
    }

    pub fn info(&self, category: &str, message: &str) -> LauncherResult<()> {
        self.write("info", category, message)
    }

    pub fn warn(&self, category: &str, message: &str) -> LauncherResult<()> {
        self.write("warn", category, message)
    }

    pub fn error(&self, category: &str, message: &str) -> LauncherResult<()> {
        self.write("error", category, message)
    }

    fn write(&self, level: &str, category: &str, message: &str) -> LauncherResult<()> {
        let _guard =
            self.write_lock
                .lock()
                .map_err(|_| crate::errors::LauncherError::Operation {
                    context: "writing diagnostics log".into(),
                    message: "diagnostics log lock is poisoned".into(),
                })?;
        self.ensure_logs_dir()?;
        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level,
            category,
            message,
        };
        let line = serde_json::to_string(&entry).map_err(|err| {
            crate::errors::LauncherError::InvalidData {
                context: "serializing log entry".into(),
                message: err.to_string(),
            }
        })?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .map_err(|err| io_context(format!("opening {}", self.log_file.display()), err))?;
        writeln!(file, "{line}")
            .map_err(|err| io_context(format!("writing {}", self.log_file.display()), err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_log_line() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = crate::storage::ManagedPaths::from_root(root.path().to_path_buf());
        paths.ensure_dirs().expect("dirs");
        let diagnostics = DiagnosticsService::new(paths.logs_dir.clone());

        diagnostics
            .info("startup", "launcher started")
            .expect("write log");
        let log_text = std::fs::read_to_string(diagnostics.log_file()).expect("read log");

        assert!(log_text.contains("\"category\":\"startup\""));
        assert!(log_text.contains("\"message\":\"launcher started\""));
    }

    #[test]
    fn ensure_logs_dir_only_creates_logs_path() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = crate::storage::ManagedPaths::from_root(root.path().to_path_buf());
        let diagnostics = DiagnosticsService::new(paths.logs_dir.clone());

        diagnostics.ensure_logs_dir().expect("logs dir");

        assert!(paths.logs_dir.is_dir());
        assert!(!paths.mods_dir.exists());
        assert!(!paths.staging_dir.exists());
    }

    #[test]
    fn writes_valid_json_lines_from_clones() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = crate::storage::ManagedPaths::from_root(root.path().to_path_buf());
        paths.ensure_dirs().expect("dirs");
        let diagnostics = DiagnosticsService::new(paths.logs_dir.clone());
        let writes = [
            (diagnostics.clone(), "info", "startup", "first"),
            (diagnostics.clone(), "warn", "startup", "second"),
            (diagnostics.clone(), "error", "state", "third"),
        ];

        let handles = writes
            .into_iter()
            .map(|(service, level, category, message)| {
                std::thread::spawn(move || match level {
                    "info" => service.info(category, message),
                    "warn" => service.warn(category, message),
                    "error" => service.error(category, message),
                    _ => unreachable!("unexpected test level"),
                })
            });
        for handle in handles {
            handle.join().expect("thread").expect("write log");
        }

        let log_text = std::fs::read_to_string(diagnostics.log_file()).expect("read log");
        let mut entries = log_text
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("valid json line"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry["message"].as_str().unwrap().to_string());

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0]["level"], "info");
        assert_eq!(entries[0]["category"], "startup");
        assert_eq!(entries[0]["message"], "first");
        assert_eq!(entries[1]["level"], "warn");
        assert_eq!(entries[1]["category"], "startup");
        assert_eq!(entries[1]["message"], "second");
        assert_eq!(entries[2]["level"], "error");
        assert_eq!(entries[2]["category"], "state");
        assert_eq!(entries[2]["message"], "third");
    }
}
