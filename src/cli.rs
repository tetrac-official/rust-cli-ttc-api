//! CLI command definitions using clap

use clap::{Args, Subcommand, ValueEnum};

use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Place, cancel, and manage orders
    Order(OrderCommands),

    /// View and close positions
    #[command(alias = "positions", alias = "pos")]
    Position(PositionCommands),

    /// Account operations: balance, leverage, margin mode
    #[command(alias = "acct")]
    Account(AccountCommands),

    /// Manage open orders
    #[command(alias = "o")]
    Orders(OrdersCommands),

    /// Get market data (tickers, best bid/ask)
    #[command(alias = "m")]
    Market(MarketCommands),

    /// Risk management: stop losses, take profits
    Risk(RiskCommands),

    /// Configuration management
    Config(ConfigCommands),

    /// Show version and info
    #[command(alias = "version")]
    Info,

    /// Login to TTC Box with email and passkey
    #[command(alias = "auth")]
    Login(LoginArgs),

    /// Register a new TTC Box account with email and passkey
    Register(RegisterArgs),
}

// ============================================================================
// Order Commands
// ============================================================================

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct OrderCommands {
    #[command(subcommand)]
    pub command: OrderSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum OrderSubcommands {
    /// Place a limit order at a specific price
    Limit(OrderLimitArgs),

    /// Place a market order (executes immediately)
    Market(OrderMarketArgs),

    /// Place a stop-loss order
    Stop(OrderStopArgs),

    /// Place a take-profit order
    TakeProfit(OrderTakeProfitArgs),

    /// Cancel an open order
    #[command(alias = "cxl")]
    Cancel(OrderCancelArgs),

    /// Cancel all open orders
    #[command(alias = "cxl-all")]
    CancelAll(OrderCancelAllArgs),

    /// List open orders
    #[command(alias = "list", alias = "ls")]
    Open(OrderOpenArgs),
}

#[derive(Debug, Args)]
pub struct OrderLimitArgs {
    /// Exchange name (e.g., phemex, bybit, binance)
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol (e.g., BTCUSDT, ETHUSDT)
    #[arg(short, long)]
    pub symbol: String,

    /// Order side: buy
    #[arg(long, conflicts_with = "sell")]
    pub buy: bool,

    /// Order side: sell
    #[arg(long, conflicts_with = "buy")]
    pub sell: bool,

    /// Position side for hedge mode: long, short, or both
    #[arg(long, value_enum)]
    pub position_side: Option<PositionSideArg>,

    /// Order quantity
    #[arg(short = 'q', long)]
    pub quantity: f64,

    /// Limit price
    #[arg(short = 'p', long)]
    pub price: f64,

    /// Time in force (GTC, IOC, FOK, PostOnly)
    #[arg(long, value_enum, default_value = "gtc")]
    pub time_in_force: TimeInForceArg,

    /// Reduce only (don't open new position)
    #[arg(long)]
    pub reduce_only: bool,

    /// Custom client order ID
    #[arg(long)]
    pub client_order_id: Option<String>,

    /// Exchange API key (overrides config)
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret (overrides config)
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct OrderMarketArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Order side: buy
    #[arg(long, conflicts_with = "sell")]
    pub buy: bool,

    /// Order side: sell
    #[arg(long, conflicts_with = "buy")]
    pub sell: bool,

    /// Position side for hedge mode
    #[arg(long, value_enum)]
    pub position_side: Option<PositionSideArg>,

    /// Order quantity
    #[arg(short = 'q', long)]
    pub quantity: f64,

    /// Reduce only
    #[arg(long)]
    pub reduce_only: bool,

    /// Custom client order ID
    #[arg(long)]
    pub client_order_id: Option<String>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct OrderStopArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Order side: buy
    #[arg(long, conflicts_with = "sell")]
    pub buy: bool,

    /// Order side: sell
    #[arg(long, conflicts_with = "buy")]
    pub sell: bool,

    /// Position side for hedge mode
    #[arg(long, value_enum)]
    pub position_side: Option<PositionSideArg>,

    /// Order quantity
    #[arg(short = 'q', long)]
    pub quantity: f64,

    /// Stop trigger price
    #[arg(short = 's', long)]
    pub stop_price: f64,

    /// Trigger type: ByLastPrice, ByMarkPrice, ByIndexPrice
    #[arg(long, value_enum, default_value = "mark")]
    pub trigger: TriggerTypeArg,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct OrderTakeProfitArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Order side: buy
    #[arg(long, conflicts_with = "sell")]
    pub buy: bool,

    /// Order side: sell
    #[arg(long, conflicts_with = "buy")]
    pub sell: bool,

    /// Position side for hedge mode
    #[arg(long, value_enum)]
    pub position_side: Option<PositionSideArg>,

    /// Order quantity
    #[arg(short = 'q', long)]
    pub quantity: f64,

    /// Take profit trigger price
    #[arg(short = 't', long)]
    pub tp_price: f64,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct OrderCancelArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Order ID to cancel
    #[arg(short = 'o', long)]
    pub order_id: Option<String>,

    /// Client order ID to cancel
    #[arg(long)]
    pub client_order_id: Option<String>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct OrderCancelAllArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Symbol (optional - cancels all if not specified)
    #[arg(short, long)]
    pub symbol: Option<String>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct OrderOpenArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Symbol (optional - shows all if not specified)
    #[arg(short, long)]
    pub symbol: Option<String>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

// ============================================================================
// Position Commands
// ============================================================================

#[derive(Debug, Args)]
pub struct PositionCommands {
    #[command(subcommand)]
    pub command: PositionSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum PositionSubcommands {
    /// List all positions
    #[command(alias = "ls", alias = "list")]
    Get(PositionGetArgs),

    /// Close a position (market order)
    #[command(alias = "exit")]
    Close(PositionCloseArgs),

    /// Close all positions
    CloseAll(PositionCloseAllArgs),
}

#[derive(Debug, Args)]
pub struct PositionGetArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Symbol filter (optional)
    #[arg(short, long)]
    pub symbol: Option<String>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct PositionCloseArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Position side to close (long, short)
    #[arg(long, value_enum)]
    pub position_side: Option<PositionSideArg>,

    /// Quantity to close (default: all)
    #[arg(short = 'q', long)]
    pub quantity: Option<f64>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct PositionCloseAllArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

// ============================================================================
// Account Commands
// ============================================================================

#[derive(Debug, Args)]
pub struct AccountCommands {
    #[command(subcommand)]
    pub command: AccountSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum AccountSubcommands {
    /// Get account balance
    #[command(alias = "bal")]
    Balance(AccountBalanceArgs),

    /// Set leverage for a symbol
    #[command(alias = "lev")]
    Leverage(AccountLeverageArgs),

    /// Set margin mode (isolated or cross)
    Margin(AccountMarginArgs),

    /// Set hedge mode (one-way vs hedge)
    Hedge(AccountHedgeArgs),
}

#[derive(Debug, Args)]
pub struct AccountBalanceArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct AccountLeverageArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Leverage (e.g., 10, 20, 50)
    #[arg(short = 'l', long)]
    pub leverage: u32,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct AccountMarginArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Margin mode: isolated or cross
    #[arg(short = 'm', long, value_enum)]
    pub mode: MarginModeArg,

    /// Symbol (some exchanges require this)
    #[arg(short, long)]
    pub symbol: Option<String>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct AccountHedgeArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Enable hedge mode
    #[arg(long)]
    pub enable: bool,

    /// Disable hedge mode (one-way)
    #[arg(long, conflicts_with = "enable")]
    pub disable: bool,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

// ============================================================================
// Risk Commands
// ============================================================================

#[derive(Debug, Args)]
pub struct RiskCommands {
    #[command(subcommand)]
    pub command: RiskSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum RiskSubcommands {
    /// Set stop loss for a position
    Sl(RiskStopLossArgs),

    /// Set take profit for a position
    Tp(RiskTakeProfitArgs),

    /// Set trailing stop
    Trail(RiskTrailingStopArgs),
}

#[derive(Debug, Args)]
pub struct RiskStopLossArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Stop loss price
    #[arg(short = 's', long)]
    pub stop_price: f64,

    /// Position side (long, short)
    #[arg(long, value_enum)]
    pub position_side: Option<PositionSideArg>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct RiskTakeProfitArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Take profit price
    #[arg(short = 't', long)]
    pub tp_price: f64,

    /// Position side (long, short)
    #[arg(long, value_enum)]
    pub position_side: Option<PositionSideArg>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct RiskTrailingStopArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Trailing stop distance/percentage
    #[arg(short = 'd', long)]
    pub distance: f64,

    /// Position side (long, short)
    #[arg(long, value_enum)]
    pub position_side: Option<PositionSideArg>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

// ============================================================================
// Config Commands
// ============================================================================

#[derive(Debug, Args)]
pub struct ConfigCommands {
    #[command(subcommand)]
    pub command: ConfigSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommands {
    /// Initialize a new config file
    Init,

    /// Show current config
    Show,

    /// Set default exchange
    SetDefault(ConfigSetDefaultArgs),

    /// Add exchange credentials
    AddExchange(ConfigAddExchangeArgs),

    /// Remove exchange credentials
    RmExchange(ConfigRmExchangeArgs),

    /// Show config file path
    Path,
}

#[derive(Debug, Args)]
pub struct ConfigSetDefaultArgs {
    /// Default exchange name
    pub exchange: String,
}

#[derive(Debug, Args)]
pub struct ConfigAddExchangeArgs {
    /// Exchange name
    pub exchange: String,

    /// API key
    #[arg(short, long)]
    pub api_key: String,

    /// API secret
    #[arg(short, long)]
    pub api_secret: String,

    /// Passphrase (for some exchanges like OKX)
    #[arg(long)]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct ConfigRmExchangeArgs {
    /// Exchange name to remove
    pub exchange: String,
}

// ============================================================================
// Orders Commands
// ============================================================================

#[derive(Debug, Args)]
pub struct OrdersCommands {
    #[command(subcommand)]
    pub command: OrdersSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum OrdersSubcommands {
    /// Get open orders
    #[command(alias = "ls", alias = "list")]
    Get(OrdersGetArgs),

    /// Cancel all orders
    #[command(alias = "cancel-all")]
    CancelAll(OrdersCancelAllArgs),

    /// Cancel a specific order
    Cancel(OrdersCancelArgs),
}

#[derive(Debug, Args)]
pub struct OrdersGetArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Symbol (optional - shows all if not specified)
    #[arg(short, long)]
    pub symbol: Option<String>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct OrdersCancelAllArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Symbol (optional - cancels all if not specified)
    #[arg(short, long)]
    pub symbol: Option<String>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct OrdersCancelArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Order ID to cancel
    #[arg(short = 'o', long)]
    pub order_id: Option<String>,

    /// Client order ID to cancel
    #[arg(long)]
    pub client_order_id: Option<String>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

// ============================================================================
// Market Commands
// ============================================================================

#[derive(Debug, Args)]
pub struct MarketCommands {
    #[command(subcommand)]
    pub command: MarketSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum MarketSubcommands {
    /// Get current ticker information
    #[command(alias = "t")]
    Tickers(MarketTickersArgs),

    /// Get best bid and ask prices
    #[command(alias = "bb", alias = "book")]
    BestBidAsk(MarketBestBidAskArgs),

    /// Get aggregated tickers across all exchanges (spot + futures, with OI and funding)
    #[command(alias = "ht", alias = "agg")]
    HybridTickers(MarketHybridTickersArgs),

    /// Get funding rates across all exchanges
    #[command(alias = "fr", alias = "funding")]
    FundingRates(MarketFundingRatesArgs),

    /// Get open interest across all exchanges
    #[command(alias = "oi")]
    OpenInterest(MarketOpenInterestArgs),

    /// Get volume snapshot for DEX and CEX exchanges
    #[command(alias = "vol", alias = "vs")]
    VolumeSnapshot(MarketVolumeSnapshotArgs),

    /// Fan analysis — entry, stop-loss, and take-profit levels
    #[command(alias = "scan")]
    Scanner(MarketScannerArgs),
}

#[derive(Debug, Args)]
pub struct MarketTickersArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Symbol (optional, returns all if not specified)
    #[arg(short, long)]
    pub symbol: Option<String>,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct MarketBestBidAskArgs {
    /// Exchange name
    #[arg(short, long, env = "TTC_EXCHANGE")]
    pub exchange: String,

    /// Trading symbol
    #[arg(short, long)]
    pub symbol: String,

    /// Exchange API key
    #[arg(long, env = "EXCHANGE_API_KEY")]
    pub api_key: Option<String>,

    /// Exchange API secret
    #[arg(long, env = "EXCHANGE_API_SECRET")]
    pub api_secret: Option<String>,

    /// Exchange API passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Args)]
pub struct MarketHybridTickersArgs {
    /// Market type: spot or futures (default: all)
    #[arg(long, value_enum)]
    pub market_type: Option<MarketTypeArg>,

    /// Filter by exchange source (e.g. binance, orderly, hyperliquid)
    #[arg(long = "source")]
    pub source: Option<String>,

    /// Filter by symbol (e.g. NEARUSDT)
    #[arg(short, long)]
    pub symbol: Option<String>,

    /// Minimum 24h volume in USD
    #[arg(long)]
    pub min_volume: Option<f64>,

    /// Minimum price filter
    #[arg(long)]
    pub min_price: Option<f64>,

    /// Maximum price filter
    #[arg(long)]
    pub max_price: Option<f64>,

    /// Minimum % gain (e.g. 5 for +5%)
    #[arg(long)]
    pub up: Option<f64>,

    /// Minimum % loss magnitude (e.g. 5 for -5%)
    #[arg(long)]
    pub down: Option<f64>,
}

#[derive(Debug, Args)]
pub struct MarketFundingRatesArgs {
    /// Filter by symbol (e.g. NEARUSDT)
    #[arg(short, long)]
    pub symbol: Option<String>,
}

#[derive(Debug, Args)]
pub struct MarketOpenInterestArgs {
    /// Filter by symbol (e.g. NEARUSDT)
    #[arg(short, long)]
    pub symbol: Option<String>,
}

#[derive(Debug, Args)]
pub struct MarketVolumeSnapshotArgs {}

#[derive(Debug, Args)]
pub struct MarketScannerArgs {
    /// Market symbol to scan (e.g. BTCUSDT)
    #[arg(short, long)]
    pub symbol: String,

    /// Kline timeframe (e.g. 1m, 5m, 1h, 4h, 1d)
    #[arg(short, long, default_value = "1h")]
    pub timeframe: String,

    /// Number of bars to analyze (max 1000)
    #[arg(short, long)]
    pub bars: Option<u32>,

    /// Lookback period for swing detection
    #[arg(long)]
    pub swing_strength: Option<u32>,
}

// ============================================================================
// Value Enums
// ============================================================================

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MarketTypeArg {
    Spot,
    Futures,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PositionSideArg {
    Long,
    Short,
    Both,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TimeInForceArg {
    Gtc,
    Ioc,
    Fok,
    Postonly,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TriggerTypeArg {
    Last,
    Mark,
    Index,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MarginModeArg {
    Isolated,
    Cross,
}

// ============================================================================
// Login Args
// ============================================================================

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// Email address (uses TTC_EMAIL env var if not provided)
    #[arg(long, env = "TTC_EMAIL")]
    pub email: Option<String>,
}

#[derive(Debug, Args)]
pub struct RegisterArgs {
    /// Email address (auto-generated if not provided)
    #[arg(long, env = "TTC_EMAIL")]
    pub email: Option<String>,
}

impl ValueEnum for OutputFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[OutputFormat::Table, OutputFormat::Json, OutputFormat::Csv, OutputFormat::Quiet]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            OutputFormat::Table => Some(clap::builder::PossibleValue::new("table")),
            OutputFormat::Json => Some(clap::builder::PossibleValue::new("json")),
            OutputFormat::Csv => Some(clap::builder::PossibleValue::new("csv")),
            OutputFormat::Quiet => Some(clap::builder::PossibleValue::new("quiet")),
        }
    }
}
