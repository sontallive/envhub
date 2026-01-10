use std::fmt;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Io,
    Json,
    InvalidState,
    AppNotFound,
    ProfileNotFound,
    Permission,
    InstallPath,
    MissingLauncher,
    TargetNotFound,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            ErrorCode::Io => "io_error",
            ErrorCode::Json => "json_error",
            ErrorCode::InvalidState => "invalid_state",
            ErrorCode::AppNotFound => "app_not_found",
            ErrorCode::ProfileNotFound => "profile_not_found",
            ErrorCode::Permission => "permission_error",
            ErrorCode::InstallPath => "install_path_error",
            ErrorCode::MissingLauncher => "missing_launcher",
            ErrorCode::TargetNotFound => "target_not_found",
        };
        write!(f, "{code}")
    }
}

#[derive(Debug, Error, Clone)]
#[error("{code}: {message}")]
pub struct CoreError {
    pub code: ErrorCode,
    pub message: String,
}

impl CoreError {
    pub fn new(code: ErrorCode, message: String) -> Self {
        Self { code, message }
    }
}
