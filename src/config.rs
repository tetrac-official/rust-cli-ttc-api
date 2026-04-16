//! Configuration management for skill-trading

use crate::error::{Result, TtcError};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_api_secret: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_api_passphrase: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,

    pub api: ApiConfig,

    pub trading: TradingConfig,

    pub output: OutputConfig,

    #[serde(default)]
    pub portfolio: PortfolioConfig,

    #[serde(default)]
    pub watchlist: WatchlistConfig,

    #[serde(default, rename = "market-maker")]
    pub market_maker: MarketMakerConfig,

    /// Exchange-specific credentials
    #[serde(default)]
    pub exchanges: HashMap<String, ExchangeCredentialConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeCredentialConfig {
    pub api_key: String,
    pub api_secret: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub base_url: String,
    pub timeout: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://ttc.box/api/v1".to_string(),
            timeout: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub default_size: f64,
    pub default_leverage: u32,
    pub confirm_orders: bool,
    pub dry_run: bool,
    /// Minimum USD notional per order slice (used by TWAP to auto-calculate slice count)
    #[serde(default = "TradingConfig::default_min_usd_entry")]
    pub min_usd_entry: f64,
}

impl TradingConfig {
    fn default_min_usd_entry() -> f64 {
        15.0
    }
}

impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            default_size: 0.001,
            default_leverage: 10,
            confirm_orders: true,
            dry_run: false,
            min_usd_entry: Self::default_min_usd_entry(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub format: String,
    pub color: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: "table".to_string(),
            color: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioConfig {
    /// Warn if locked margin / total balance exceeds this % (0–100)
    #[serde(default = "PortfolioConfig::default_max_margin_utilization")]
    pub max_margin_utilization: f64,
    /// Warn if any position's liquidation price is fewer than this % away from mark price
    #[serde(default = "PortfolioConfig::default_min_liq_distance_pct")]
    pub min_liq_distance_pct: f64,
    /// Warn if any single position's notional (USD value) exceeds this amount
    #[serde(default = "PortfolioConfig::default_max_position_notional")]
    pub max_position_notional: f64,
}

impl PortfolioConfig {
    fn default_max_margin_utilization() -> f64 {
        80.0
    }
    fn default_min_liq_distance_pct() -> f64 {
        10.0
    }
    fn default_max_position_notional() -> f64 {
        5000.0
    }
}

impl Default for PortfolioConfig {
    fn default() -> Self {
        Self {
            max_margin_utilization: Self::default_max_margin_utilization(),
            min_liq_distance_pct: Self::default_min_liq_distance_pct(),
            max_position_notional: Self::default_max_position_notional(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMakerConfig {
    /// Limit order fee rate per side (e.g., 0.001 = 0.1%). Use 0.0 for zero-fee exchanges.
    #[serde(
        default = "MarketMakerConfig::default_commission",
        rename = "limit_order_commission"
    )]
    pub limit_order_commission: f64,

    /// Minimum spread as a fraction of entry price (e.g., 0.001 = 0.1%).
    /// The effective spread per round is max(requested_spread, entry_price × min_spread).
    #[serde(default = "MarketMakerConfig::default_min_spread")]
    pub min_spread: f64,
}

impl MarketMakerConfig {
    fn default_commission() -> f64 {
        0.001 // 0.1% — a common taker default; override in config.toml
    }

    fn default_min_spread() -> f64 {
        0.001 // 0.1% floor
    }
}

impl Default for MarketMakerConfig {
    fn default() -> Self {
        Self {
            limit_order_commission: Self::default_commission(),
            min_spread: Self::default_min_spread(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistConfig {
    /// Symbols to show in morning brief and price alerts
    #[serde(default = "WatchlistConfig::default_symbols")]
    pub symbols: Vec<String>,
}

impl WatchlistConfig {
    fn default_symbols() -> Vec<String> {
        vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "NEARUSDT".to_string(),
        ]
    }
}

impl Default for WatchlistConfig {
    fn default() -> Self {
        Self {
            symbols: Self::default_symbols(),
        }
    }
}

impl AppConfig {
    /// Load configuration from file with cascading discovery
    ///
    /// Priority order:
    /// 1. Explicit path provided via CLI argument
    /// 2. ./config.toml (current directory)
    /// 3. User-level config (~/Library/Application Support/com.ttcbox.skill-trading/config.toml)
    pub fn load_from_file(path: &Option<PathBuf>) -> Result<Self> {
        let config_path = match path {
            Some(explicit_path) => explicit_path.clone(),
            None => Self::discover_config_path(),
        };

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(&config_path)
            .map_err(|e| TtcError::Config(format!("Failed to read config file: {}", e)))?;

        let config: Self = toml::from_str(&contents)
            .map_err(|e| TtcError::Config(format!("Failed to parse config file: {}", e)))?;

        Ok(config)
    }

    /// Discover config file using cascading lookup
    ///
    /// Checks in order:
    /// 1. ./config.toml (current directory)
    /// 2. User-level config directory
    pub fn discover_config_path() -> PathBuf {
        // First check for local config in current directory
        let local_config = PathBuf::from("./config.toml");
        if local_config.exists() {
            return local_config;
        }

        // Fall back to user-level config
        Self::user_config_path()
    }

    /// Get the user-level configuration file path
    pub fn user_config_path() -> PathBuf {
        ProjectDirs::from("com", "ttcbox", "skill-trading")
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from(".skill-trading.toml"))
    }

    /// Get the default configuration file path (for backwards compatibility)
    pub fn default_config_path() -> PathBuf {
        Self::user_config_path()
    }

    /// Get credentials for a specific exchange.
    ///
    /// Priority order:
    /// 1. CLI flags (applied before this is called, via exchange_api_key/secret fields)
    /// 2. Per-exchange env vars: `{EXCHANGE}_API_KEY`, `{EXCHANGE}_API_SECRET`, `{EXCHANGE}_API_PASSPHRASE`
    ///    e.g. ASTERDEX_API_KEY, BYBIT_API_KEY, OKX_API_PASSPHRASE
    /// 3. config.toml [exchanges.xxx] sections
    pub fn get_credentials(&self, exchange: &str) -> Option<ExchangeCredentialConfig> {
        // CLI flag overrides (exchange_api_key set from --exchange-api-key or EXCHANGE_API_KEY)
        if let (Some(api_key), Some(api_secret)) =
            (&self.exchange_api_key, &self.exchange_api_secret)
        {
            return Some(ExchangeCredentialConfig {
                api_key: api_key.clone(),
                api_secret: api_secret.clone(),
                passphrase: self.exchange_api_passphrase.clone(),
            });
        }

        // Per-exchange env vars: ASTERDEX_API_KEY, BYBIT_API_KEY, etc.
        let prefix = exchange.to_uppercase();
        if let (Ok(key), Ok(secret)) = (
            std::env::var(format!("{}_API_KEY", prefix)),
            std::env::var(format!("{}_API_SECRET", prefix)),
        ) {
            return Some(ExchangeCredentialConfig {
                api_key: key,
                api_secret: secret,
                passphrase: std::env::var(format!("{}_API_PASSPHRASE", prefix)).ok(),
            });
        }

        // config.toml [exchanges.xxx] sections
        if let Some(creds) = self.exchanges.get(exchange) {
            return Some(creds.clone());
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.api.base_url, "https://ttc.box/api/v1");
        assert_eq!(config.api.timeout, 30);
        assert_eq!(config.api.max_retries, 3);
        assert_eq!(config.trading.default_leverage, 10);
        assert!(config.trading.confirm_orders);
        assert!(!config.trading.dry_run);
        assert!(config.exchanges.is_empty());
    }

    #[test]
    fn test_get_credentials_global_fallback() {
        let config = AppConfig {
            exchange_api_key: Some("global-key".into()),
            exchange_api_secret: Some("global-secret".into()),
            ..Default::default()
        };

        // No exchange-specific creds, should fall back to global
        let creds = config.get_credentials("bybit").unwrap();
        assert_eq!(creds.api_key, "global-key");
        assert_eq!(creds.api_secret, "global-secret");
    }

    #[test]
    fn test_get_credentials_none() {
        let config = AppConfig::default();
        assert!(config.get_credentials("binance").is_none());
    }

    #[test]
    fn test_get_credentials_partial_global() {
        let config = AppConfig {
            exchange_api_key: Some("key-only".into()),
            ..Default::default()
        };
        // No secret set
        assert!(config.get_credentials("binance").is_none());
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = Some(PathBuf::from("/tmp/nonexistent-skill-trading-config.toml"));
        let config = AppConfig::load_from_file(&path).unwrap();
        assert_eq!(config.api.base_url, "https://ttc.box/api/v1");
    }

    #[test]
    fn test_config_toml_roundtrip() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.api.base_url, config.api.base_url);
        assert_eq!(parsed.api.timeout, config.api.timeout);
        assert_eq!(
            parsed.trading.default_leverage,
            config.trading.default_leverage
        );
    }
}
