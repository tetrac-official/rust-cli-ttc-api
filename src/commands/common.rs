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
        std::env::var("ORDERLY_MAIN_WALLET_ADDRESS").ok().filter(|s| !s.is_empty())
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
            let creds = settings.get_credentials(exchange).ok_or_else(|| {
                TtcError::MissingCredentials(exchange.to_string())
            })?;
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
