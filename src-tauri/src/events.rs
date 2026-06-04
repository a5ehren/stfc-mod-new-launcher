use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub operation: String,
    pub phase: String,
    pub message: String,
    pub current: Option<u64>,
    pub total: Option<u64>,
}

impl ProgressEvent {
    pub fn message(operation: &str, phase: &str, message: impl Into<String>) -> Self {
        Self {
            operation: operation.to_string(),
            phase: phase.to_string(),
            message: message.into(),
            current: None,
            total: None,
        }
    }

    pub fn counted(
        operation: &str,
        phase: &str,
        message: impl Into<String>,
        current: u64,
        total: u64,
    ) -> Self {
        Self {
            operation: operation.to_string(),
            phase: phase.to_string(),
            message: message.into(),
            current: Some(current),
            total: Some(total),
        }
    }
}
