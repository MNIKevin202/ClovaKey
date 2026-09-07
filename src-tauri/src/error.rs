//! Frontend-facing error type.
//!
//! Every command returns `Result<T, AppError>`. `AppError` serializes to a
//! small `{ code, message }` object the UI can branch on and display. It never
//! carries secret material — it is built from [`clovakey_core::CoreError`],
//! whose messages are already redacted.

use clovakey_core::CoreError;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        AppError {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

impl From<CoreError> for AppError {
    fn from(e: CoreError) -> Self {
        AppError {
            code: e.code().to_string(),
            message: e.to_string(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = std::result::Result<T, AppError>;
