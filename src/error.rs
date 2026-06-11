use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Walk(#[from] ignore::Error),
}

impl AppError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
