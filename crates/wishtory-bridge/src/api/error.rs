use wishtory_core::error::AppError;

/// What went wrong, as a code the UI can act on. The old UI matched on message text to pick a
/// recovery plan; with a code that contract is explicit and survives rewording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    GameNotFound,
    HistoryPageNotOpened,
    AuthExpired,
    InvalidUrl,
    RateLimited,
    Api,
    Network,
    Database,
    File,
    Format,
    Other,
}

/// Thrown into Dart by every fallible call. `message` is already written for the person.
#[derive(Debug, Clone)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
}

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        let code = match &e {
            AppError::GameNotFound(_) => ErrorCode::GameNotFound,
            AppError::HistoryPageNotOpened => ErrorCode::HistoryPageNotOpened,
            AppError::AuthExpired => ErrorCode::AuthExpired,
            AppError::InvalidUrl => ErrorCode::InvalidUrl,
            AppError::RateLimited => ErrorCode::RateLimited,
            AppError::Api(_) => ErrorCode::Api,
            AppError::Network(_) => ErrorCode::Network,
            AppError::Db(_) => ErrorCode::Database,
            AppError::Io(_) => ErrorCode::File,
            AppError::Format(_) => ErrorCode::Format,
            AppError::Message(_) => ErrorCode::Other,
        };
        Self {
            code,
            message: e.to_string(),
        }
    }
}
