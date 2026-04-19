pub mod provider;

pub use provider::MacOSDesktopProvider;

#[derive(Debug, thiserror::Error)]
pub enum DesktopError {
    #[error("AppleScript error: {0}")]
    AppleScript(String),
    #[allow(dead_code)]
    #[error("Application not found: {0}")]
    AppNotFound(String),
    #[allow(dead_code)]
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}