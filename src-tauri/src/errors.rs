use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error("I/O error while {context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("network error while {context}: {source}")]
    Network {
        context: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("invalid data while {context}: {message}")]
    InvalidData { context: String, message: String },
    #[error("operation failed while {context}: {message}")]
    Operation { context: String, message: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDto {
    pub kind: String,
    pub message: String,
}

impl From<LauncherError> for ErrorDto {
    fn from(error: LauncherError) -> Self {
        let kind = match &error {
            LauncherError::Io { .. } => "io",
            LauncherError::Network { .. } => "network",
            LauncherError::InvalidData { .. } => "invalidData",
            LauncherError::Operation { .. } => "operation",
        };
        Self {
            kind: kind.to_string(),
            message: error.to_string(),
        }
    }
}

pub type LauncherResult<T> = Result<T, LauncherError>;

pub fn io_context(context: impl Into<String>, source: std::io::Error) -> LauncherError {
    LauncherError::Io {
        context: context.into(),
        source,
    }
}
