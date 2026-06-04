use crate::errors::{io_context, LauncherResult};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ConfigService {
    config_file: PathBuf,
}

impl ConfigService {
    pub fn new(config_file: PathBuf) -> Self {
        Self { config_file }
    }

    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    pub fn read_config(&self) -> LauncherResult<String> {
        if !self.config_file.exists() {
            return Ok(String::new());
        }
        fs::read_to_string(&self.config_file)
            .map_err(|err| io_context(format!("reading {}", self.config_file.display()), err))
    }

    pub fn write_config(&self, text: &str) -> LauncherResult<()> {
        if let Some(parent) = self.config_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| io_context(format!("creating {}", parent.display()), err))?;
        }
        fs::write(&self.config_file, text)
            .map_err(|err| io_context(format!("writing {}", self.config_file.display()), err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_missing_config_as_empty_string() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = crate::storage::ManagedPaths::from_root(root.path().to_path_buf());
        let service = ConfigService::new(paths.config_file.clone());

        assert_eq!(service.read_config().expect("read"), "");
    }

    #[test]
    fn writes_config_and_preserves_text() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = crate::storage::ManagedPaths::from_root(root.path().to_path_buf());
        let service = ConfigService::new(paths.config_file.clone());

        service
            .write_config("[control]\nhotkeys_enabled = true\n")
            .expect("write");

        assert_eq!(
            service.read_config().expect("read"),
            "[control]\nhotkeys_enabled = true\n"
        );
    }
}
