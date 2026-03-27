//! Configuration management for skill-trading

use crate::error::{Result, TtcError};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

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
}

impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            default_size: 0.001,
            default_leverage: 10,
            confirm_orders: true,
            dry_run: false,
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
    fn discover_config_path() -> PathBuf {
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
        if let (Some(api_key), Some(api_secret)) = (&self.exchange_api_key, &self.exchange_api_secret) {
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
    fn test_get_credentials_exchange_specific() {
        let mut config = AppConfig::default();
        config.exchanges.insert("phemex".into(), ExchangeCredentialConfig {
            api_key: "phemex-key".into(),
            api_secret: "phemex-secret".into(),
            passphrase: None,
        });

        config.exchange_api_key = Some("global-key".into());
        config.exchange_api_secret = Some("global-secret".into());

        // Should return exchange-specific, not global
        let creds = config.get_credentials("phemex").unwrap();
        assert_eq!(creds.api_key, "phemex-key");
        assert_eq!(creds.api_secret, "phemex-secret");
    }

    #[test]
    fn test_get_credentials_global_fallback() {
        let mut config = AppConfig::default();
        config.exchange_api_key = Some("global-key".into());
        config.exchange_api_secret = Some("global-secret".into());

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
        let mut config = AppConfig::default();
        config.exchange_api_key = Some("key-only".into());
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
        assert_eq!(parsed.trading.default_leverage, config.trading.default_leverage);
    }
}
