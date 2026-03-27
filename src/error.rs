//! Error types for skill-trading

use thiserror::Error;

pub type Result<T> = std::result::Result<T, TtcError>;

#[derive(Debug, Error)]
pub enum TtcError {
    #[error("API request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("API error [{code}]: {message}")]
    Api { code: u16, message: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid order parameters: {0}")]
    InvalidOrder(String),

    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    #[error("Missing credentials for exchange: {0}")]
    MissingCredentials(String),

    #[error("Position not found: {0}")]
    PositionNotFound(String),

    #[error("Invalid position: {0}")]
    InvalidPosition(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Rate limited - retry after {0} seconds")]
    RateLimited(u64),
}

impl TtcError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, 
            TtcError::Request(_) | 
            TtcError::RateLimited(_)
        )
    }
}
