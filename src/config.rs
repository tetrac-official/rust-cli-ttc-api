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

    // ========================================================================
    // Priority resolution
    //
    // Notes for future readers:
    // - Top-level CLI > env > config.toml > defaults priority is wired in
    //   src/main.rs via clap `env = "..."` attributes plus the explicit
    //   `if let Some(ref x) = cli.x` overrides. CLI/env-driven precedence
    //   for the binary itself is exercised in tests/integration_test.rs.
    // - get_credentials() owns the per-exchange CLI/env/config priority and
    //   is unit-tested below.
    // ========================================================================

    use std::sync::Mutex;
    use uuid::Uuid;

    /// Serializes tests that mutate process-wide env vars. Without this, two
    /// tests setting different `XYZ_API_KEY` values race each other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct TempConfigFile(PathBuf);

    impl TempConfigFile {
        fn new(contents: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("skill-trading-test-{}.toml", Uuid::new_v4()));
            std::fs::write(&path, contents).expect("write temp config");
            Self(path)
        }

        fn path(&self) -> PathBuf {
            self.0.clone()
        }
    }

    impl Drop for TempConfigFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    // ---- load_from_file ----------------------------------------------------

    #[test]
    fn load_from_file_parses_full_config() {
        let tmp = TempConfigFile::new(
            r#"
exchange = "bybit"

[api]
base_url = "https://example.test/api"
timeout = 60
max_retries = 5
retry_delay_ms = 250

[trading]
default_size = 0.5
default_leverage = 25
confirm_orders = false
dry_run = true
min_usd_entry = 20.0

[output]
format = "json"
color = false

[portfolio]
max_margin_utilization = 70.0
min_liq_distance_pct = 15.0
max_position_notional = 10000.0

[watchlist]
symbols = ["SOLUSDT", "AVAXUSDT"]

[market-maker]
limit_order_commission = 0.0
min_spread = 0.002

[exchanges.bybit]
api_key = "bybit-key-from-toml"
api_secret = "bybit-secret-from-toml"

[exchanges.okx]
api_key = "okx-key"
api_secret = "okx-secret"
passphrase = "okx-pass"
"#,
        );

        let cfg = AppConfig::load_from_file(&Some(tmp.path())).expect("load");
        assert_eq!(cfg.exchange.as_deref(), Some("bybit"));
        assert_eq!(cfg.api.base_url, "https://example.test/api");
        assert_eq!(cfg.api.timeout, 60);
        assert_eq!(cfg.api.max_retries, 5);
        assert_eq!(cfg.api.retry_delay_ms, 250);
        assert_eq!(cfg.trading.default_size, 0.5);
        assert_eq!(cfg.trading.default_leverage, 25);
        assert!(!cfg.trading.confirm_orders);
        assert!(cfg.trading.dry_run);
        assert_eq!(cfg.trading.min_usd_entry, 20.0);
        assert_eq!(cfg.output.format, "json");
        assert!(!cfg.output.color);
        assert_eq!(cfg.portfolio.max_margin_utilization, 70.0);
        assert_eq!(cfg.portfolio.min_liq_distance_pct, 15.0);
        assert_eq!(cfg.portfolio.max_position_notional, 10_000.0);
        assert_eq!(cfg.watchlist.symbols, vec!["SOLUSDT", "AVAXUSDT"]);
        assert_eq!(cfg.market_maker.limit_order_commission, 0.0);
        assert_eq!(cfg.market_maker.min_spread, 0.002);

        let bybit = cfg.exchanges.get("bybit").expect("bybit creds");
        assert_eq!(bybit.api_key, "bybit-key-from-toml");
        assert_eq!(bybit.api_secret, "bybit-secret-from-toml");
        assert!(bybit.passphrase.is_none());

        let okx = cfg.exchanges.get("okx").expect("okx creds");
        assert_eq!(okx.passphrase.as_deref(), Some("okx-pass"));
    }

    #[test]
    fn load_from_file_optional_sections_use_defaults_when_omitted() {
        // [api], [trading], [output] are NOT #[serde(default)] and must be
        // present. [portfolio], [watchlist], [market-maker], [exchanges] are
        // optional and default when missing.
        let tmp = TempConfigFile::new(
            r#"
[api]
base_url = "https://ttc.box/api/v1"
timeout = 30
max_retries = 3
retry_delay_ms = 1000

[trading]
default_size = 0.001
default_leverage = 50
confirm_orders = true
dry_run = false

[output]
format = "table"
color = true
"#,
        );
        let cfg = AppConfig::load_from_file(&Some(tmp.path())).expect("load");
        assert_eq!(cfg.trading.default_leverage, 50);
        // Optional sections should fall back to defaults
        assert_eq!(cfg.portfolio.max_margin_utilization, 80.0);
        assert_eq!(cfg.portfolio.min_liq_distance_pct, 10.0);
        assert_eq!(cfg.portfolio.max_position_notional, 5_000.0);
        assert_eq!(
            cfg.watchlist.symbols,
            vec!["BTCUSDT", "ETHUSDT", "NEARUSDT"]
        );
        assert_eq!(cfg.market_maker.limit_order_commission, 0.001);
        assert_eq!(cfg.market_maker.min_spread, 0.001);
        assert!(cfg.exchanges.is_empty());
    }

    #[test]
    fn load_from_file_missing_required_section_errors() {
        // No [api] section — must fail with a parse error mentioning the field
        let tmp = TempConfigFile::new(
            r#"
[trading]
default_size = 0.001
default_leverage = 10
confirm_orders = true
dry_run = false

[output]
format = "table"
color = true
"#,
        );
        let err = AppConfig::load_from_file(&Some(tmp.path())).expect_err("must fail");
        let msg = format!("{}", err);
        assert!(msg.contains("Failed to parse config file"));
        assert!(
            msg.contains("api"),
            "should mention the missing field: {msg}"
        );
    }

    #[test]
    fn load_from_file_malformed_toml_returns_clear_error() {
        let tmp = TempConfigFile::new("this is not [valid toml = =");
        let err = AppConfig::load_from_file(&Some(tmp.path())).expect_err("must fail");
        let msg = format!("{}", err);
        assert!(
            msg.contains("Failed to parse config file"),
            "expected parse-error hint, got: {msg}"
        );
    }

    #[test]
    fn load_from_file_explicit_path_is_honored() {
        // Two configs differentiated by api.base_url; load_from_file with an
        // explicit path must return that file's contents (not anything from
        // discovery).
        let make = |url: &str| {
            format!(
                r#"
[api]
base_url = "{url}"
timeout = 30
max_retries = 3
retry_delay_ms = 1000

[trading]
default_size = 0.001
default_leverage = 10
confirm_orders = true
dry_run = false

[output]
format = "table"
color = true
"#
            )
        };
        let a = TempConfigFile::new(&make("https://a.example/api"));
        let b = TempConfigFile::new(&make("https://b.example/api"));
        assert_eq!(
            AppConfig::load_from_file(&Some(a.path()))
                .unwrap()
                .api
                .base_url,
            "https://a.example/api"
        );
        assert_eq!(
            AppConfig::load_from_file(&Some(b.path()))
                .unwrap()
                .api
                .base_url,
            "https://b.example/api"
        );
    }

    // ---- get_credentials priority -----------------------------------------

    #[test]
    fn cli_global_creds_beat_per_exchange_config() {
        // Global flags (exchange_api_key/secret) win over [exchanges.bybit]
        let mut config = AppConfig {
            exchange_api_key: Some("from-cli".into()),
            exchange_api_secret: Some("cli-secret".into()),
            ..Default::default()
        };
        config.exchanges.insert(
            "bybit".to_string(),
            ExchangeCredentialConfig {
                api_key: "from-config".into(),
                api_secret: "config-secret".into(),
                passphrase: None,
            },
        );
        let creds = config.get_credentials("bybit").unwrap();
        assert_eq!(creds.api_key, "from-cli");
        assert_eq!(creds.api_secret, "cli-secret");
    }

    #[test]
    fn per_exchange_config_used_when_no_cli_or_env() {
        // Use a fictitious exchange name so no real env var collides
        let exch = "configonlyexa";
        let mut config = AppConfig::default();
        config.exchanges.insert(
            exch.to_string(),
            ExchangeCredentialConfig {
                api_key: "config-key".into(),
                api_secret: "config-secret".into(),
                passphrase: Some("config-pass".into()),
            },
        );
        let creds = config.get_credentials(exch).unwrap();
        assert_eq!(creds.api_key, "config-key");
        assert_eq!(creds.api_secret, "config-secret");
        assert_eq!(creds.passphrase.as_deref(), Some("config-pass"));
    }

    #[test]
    fn per_exchange_env_beats_config() {
        let _g = ENV_LOCK.lock().unwrap();
        // Unique fictitious exchange to dodge collisions with real exchanges
        let exch = "xenva";
        let prefix = exch.to_uppercase();
        std::env::set_var(format!("{}_API_KEY", prefix), "env-key");
        std::env::set_var(format!("{}_API_SECRET", prefix), "env-secret");

        let mut config = AppConfig::default();
        config.exchanges.insert(
            exch.to_string(),
            ExchangeCredentialConfig {
                api_key: "config-key".into(),
                api_secret: "config-secret".into(),
                passphrase: None,
            },
        );

        let creds = config.get_credentials(exch).unwrap();
        assert_eq!(creds.api_key, "env-key");
        assert_eq!(creds.api_secret, "env-secret");
        assert!(creds.passphrase.is_none());

        std::env::remove_var(format!("{}_API_KEY", prefix));
        std::env::remove_var(format!("{}_API_SECRET", prefix));
    }

    #[test]
    fn per_exchange_env_passphrase_propagates() {
        let _g = ENV_LOCK.lock().unwrap();
        let exch = "xenvb";
        let prefix = exch.to_uppercase();
        std::env::set_var(format!("{}_API_KEY", prefix), "k");
        std::env::set_var(format!("{}_API_SECRET", prefix), "s");
        std::env::set_var(format!("{}_API_PASSPHRASE", prefix), "p");

        let creds = AppConfig::default().get_credentials(exch).unwrap();
        assert_eq!(creds.passphrase.as_deref(), Some("p"));

        std::env::remove_var(format!("{}_API_KEY", prefix));
        std::env::remove_var(format!("{}_API_SECRET", prefix));
        std::env::remove_var(format!("{}_API_PASSPHRASE", prefix));
    }

    #[test]
    fn per_exchange_env_partial_falls_through_to_config() {
        // KEY without SECRET in env → not enough; must fall through to config.
        let _g = ENV_LOCK.lock().unwrap();
        let exch = "xenvc";
        let prefix = exch.to_uppercase();
        std::env::set_var(format!("{}_API_KEY", prefix), "env-only-key");
        // No SECRET set

        let mut config = AppConfig::default();
        config.exchanges.insert(
            exch.to_string(),
            ExchangeCredentialConfig {
                api_key: "config-key".into(),
                api_secret: "config-secret".into(),
                passphrase: None,
            },
        );
        let creds = config.get_credentials(exch).unwrap();
        assert_eq!(creds.api_key, "config-key", "should fall through to config");
        assert_eq!(creds.api_secret, "config-secret");

        std::env::remove_var(format!("{}_API_KEY", prefix));
    }

    #[test]
    fn cli_global_creds_beat_env() {
        // CLI fields are checked BEFORE per-exchange env, so CLI must win.
        let _g = ENV_LOCK.lock().unwrap();
        let exch = "xenvd";
        let prefix = exch.to_uppercase();
        std::env::set_var(format!("{}_API_KEY", prefix), "env-key");
        std::env::set_var(format!("{}_API_SECRET", prefix), "env-secret");

        let config = AppConfig {
            exchange_api_key: Some("cli-key".into()),
            exchange_api_secret: Some("cli-secret".into()),
            ..Default::default()
        };

        let creds = config.get_credentials(exch).unwrap();
        assert_eq!(creds.api_key, "cli-key");
        assert_eq!(creds.api_secret, "cli-secret");

        std::env::remove_var(format!("{}_API_KEY", prefix));
        std::env::remove_var(format!("{}_API_SECRET", prefix));
    }
}
