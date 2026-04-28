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
        matches!(self, TtcError::Request(_) | TtcError::RateLimited(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api(code: u16) -> TtcError {
        TtcError::Api {
            code,
            message: format!("status {code}"),
        }
    }

    // ---- is_retryable ------------------------------------------------------

    #[test]
    fn rate_limited_is_retryable() {
        assert!(TtcError::RateLimited(30).is_retryable());
        assert!(TtcError::RateLimited(0).is_retryable());
    }

    #[test]
    fn auth_and_validation_4xx_are_not_retryable() {
        assert!(!api(400).is_retryable(), "400 must not retry");
        assert!(!api(401).is_retryable(), "401 (auth) must not retry");
        assert!(!api(403).is_retryable(), "403 (forbidden) must not retry");
        assert!(!api(404).is_retryable(), "404 must not retry");
        assert!(!api(422).is_retryable(), "422 (validation) must not retry");
    }

    /// Documents current behavior: 5xx Api responses are NOT retried because
    /// `handle_response` maps them to `TtcError::Api`, and is_retryable only
    /// matches Request and RateLimited. Network-layer 5xx (transport errors)
    /// surface as Request and are retried; an HTTP response with a 5xx body
    /// is not.
    #[test]
    fn api_5xx_is_not_retryable_today() {
        assert!(!api(500).is_retryable());
        assert!(!api(502).is_retryable());
        assert!(!api(503).is_retryable());
        assert!(!api(504).is_retryable());
    }

    #[test]
    fn config_and_input_errors_are_not_retryable() {
        assert!(!TtcError::Config("x".into()).is_retryable());
        assert!(!TtcError::InvalidOrder("x".into()).is_retryable());
        assert!(!TtcError::MissingConfig("x".into()).is_retryable());
        assert!(!TtcError::MissingCredentials("x".into()).is_retryable());
        assert!(!TtcError::PositionNotFound("x".into()).is_retryable());
        assert!(!TtcError::InvalidPosition("x".into()).is_retryable());
    }

    #[test]
    fn parse_and_io_errors_are_not_retryable() {
        let io_err: TtcError = std::io::Error::other("x").into();
        assert!(!io_err.is_retryable());

        let json_err: TtcError = serde_json::from_str::<i32>("not-json").unwrap_err().into();
        assert!(!json_err.is_retryable());

        let toml_err: TtcError = toml::from_str::<toml::Value>("= broken").unwrap_err().into();
        assert!(!toml_err.is_retryable());
    }

    // ---- Display formatting -----------------------------------------------

    #[test]
    fn display_includes_api_code_and_message() {
        let s = format!("{}", api(401));
        assert!(s.contains("401"), "missing code: {s}");
        assert!(s.contains("status 401"), "missing message: {s}");
    }

    #[test]
    fn display_for_each_variant_uses_thiserror_template() {
        assert_eq!(
            format!("{}", TtcError::Config("bad".into())),
            "Configuration error: bad"
        );
        assert_eq!(
            format!("{}", TtcError::InvalidOrder("zero qty".into())),
            "Invalid order parameters: zero qty"
        );
        assert_eq!(
            format!("{}", TtcError::MissingConfig("token".into())),
            "Missing required configuration: token"
        );
        assert_eq!(
            format!("{}", TtcError::MissingCredentials("bybit".into())),
            "Missing credentials for exchange: bybit"
        );
        assert_eq!(
            format!("{}", TtcError::PositionNotFound("BTCUSDT".into())),
            "Position not found: BTCUSDT"
        );
        assert_eq!(
            format!("{}", TtcError::InvalidPosition("flat".into())),
            "Invalid position: flat"
        );
        assert_eq!(
            format!("{}", TtcError::RateLimited(42)),
            "Rate limited - retry after 42 seconds"
        );
    }

    // ---- From conversions --------------------------------------------------

    #[test]
    fn from_io_error_produces_io_variant() {
        let e: TtcError = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "nope").into();
        assert!(matches!(e, TtcError::Io(_)));
    }

    #[test]
    fn from_serde_json_error_produces_serialization_variant() {
        let e: TtcError = serde_json::from_str::<serde_json::Value>("{not json").unwrap_err().into();
        assert!(matches!(e, TtcError::Serialization(_)));
    }

    #[test]
    fn from_toml_error_produces_toml_variant() {
        let e: TtcError = toml::from_str::<toml::Value>("= broken").unwrap_err().into();
        assert!(matches!(e, TtcError::Toml(_)));
    }

    // ---- Sanity: Result alias compiles and short-circuits -----------------

    #[test]
    fn result_alias_propagates_with_question_mark() {
        fn inner() -> Result<i32> {
            Err(TtcError::Config("nope".into()))
        }
        fn outer() -> Result<i32> {
            let v = inner()?;
            Ok(v + 1)
        }
        let err = outer().unwrap_err();
        assert!(matches!(err, TtcError::Config(_)));
    }
}
