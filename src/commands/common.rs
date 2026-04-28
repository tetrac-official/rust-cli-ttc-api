//! Shared helpers for command implementations

use crate::cli::PositionSideArg;
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::{ExchangeCredentials, PositionSide};

/// Resolve exchange credentials from CLI args or config.
///
/// Priority: CLI args > exchange-specific config > global config
pub fn get_credentials(
    exchange: &str,
    api_key: Option<String>,
    api_secret: Option<String>,
    passphrase: Option<String>,
    settings: &AppConfig,
) -> Result<ExchangeCredentials> {
    // For Orderly: send walletAddress so the server uses the real trading wallet
    // instead of the ttc-public-key (which may be a random key for email-registered users).
    // Set ORDERLY_MAIN_WALLET_ADDRESS in .env to enable this.
    let wallet_address = if exchange.to_lowercase() == "orderly" {
        std::env::var("ORDERLY_MAIN_WALLET_ADDRESS")
            .ok()
            .filter(|s| !s.is_empty())
    } else {
        None
    };

    match (api_key, api_secret) {
        (Some(key), Some(secret)) => Ok(ExchangeCredentials {
            api_key: key,
            api_secret: secret,
            passphrase,
            wallet_address,
        }),
        _ => {
            let creds = settings
                .get_credentials(exchange)
                .ok_or_else(|| TtcError::MissingCredentials(exchange.to_string()))?;
            Ok(ExchangeCredentials {
                api_key: creds.api_key,
                api_secret: creds.api_secret,
                passphrase: passphrase.or(creds.passphrase),
                wallet_address,
            })
        }
    }
}

/// Convert CLI position side arg to domain model
pub fn convert_position_side(ps: Option<PositionSideArg>) -> Option<PositionSide> {
    ps.map(|p| match p {
        PositionSideArg::Long => PositionSide::Long,
        PositionSideArg::Short => PositionSide::Short,
        PositionSideArg::Both => PositionSide::Both,
    })
}

/// Parse a position side string from the API into a PositionSide enum
pub fn parse_position_side(s: &str) -> PositionSide {
    match s.to_lowercase().as_str() {
        "long" => PositionSide::Long,
        "short" => PositionSide::Short,
        _ => PositionSide::Both,
    }
}

/// Validate inputs to an order placement command.
///
/// Catches malformed inputs before they hit the network — agents get an
/// immediate, descriptive error instead of waiting for the exchange to
/// reject the request with a generic 400.
///
/// Rejects:
/// - Empty / whitespace-only symbol.
/// - Quantity that is non-finite (NaN, ±inf), zero, or negative.
pub fn validate_order_inputs(symbol: &str, quantity: f64) -> Result<()> {
    if symbol.trim().is_empty() {
        return Err(TtcError::InvalidOrder(
            "--symbol must not be empty".into(),
        ));
    }
    if !quantity.is_finite() || quantity <= 0.0 {
        return Err(TtcError::InvalidOrder(format!(
            "--quantity must be a positive finite number (got {quantity})"
        )));
    }
    Ok(())
}

/// Process-wide lock for any test that mutates process env vars or $HOME
/// (state-file tests, status command tests, anything reading per-exchange
/// API key env vars). Without a single shared lock, parallel tests across
/// modules clobber each other's env state.
#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_symbol() {
        assert!(validate_order_inputs("", 1.0).is_err());
        assert!(validate_order_inputs("   ", 1.0).is_err());
        assert!(validate_order_inputs("\t\n", 1.0).is_err());
    }

    #[test]
    fn accepts_well_formed_inputs() {
        assert!(validate_order_inputs("BTCUSDT", 0.001).is_ok());
        assert!(validate_order_inputs("near-perp", 100.0).is_ok());
    }

    #[test]
    fn rejects_negative_quantity() {
        let err = validate_order_inputs("BTCUSDT", -0.1).unwrap_err();
        assert!(format!("{err}").contains("positive"), "got: {err}");
    }

    #[test]
    fn rejects_zero_quantity() {
        assert!(validate_order_inputs("BTCUSDT", 0.0).is_err());
    }

    #[test]
    fn rejects_nan_and_infinite_quantity() {
        assert!(validate_order_inputs("BTCUSDT", f64::NAN).is_err());
        assert!(validate_order_inputs("BTCUSDT", f64::INFINITY).is_err());
        assert!(validate_order_inputs("BTCUSDT", f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn error_message_names_the_field() {
        // Agents parse error text — keep the field name in the message so the
        // recovery protocol knows which arg to fix.
        let e = validate_order_inputs("", 1.0).unwrap_err().to_string();
        assert!(e.contains("--symbol"));
        let e = validate_order_inputs("BTCUSDT", -1.0).unwrap_err().to_string();
        assert!(e.contains("--quantity"));
    }
}
