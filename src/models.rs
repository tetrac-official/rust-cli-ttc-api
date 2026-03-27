use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize a field that may be either a JSON number or a quoted string into String.
fn deserialize_string_or_int<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    struct StringOrInt;
    impl<'de> Visitor<'de> for StringOrInt {
        type Value = String;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "a string or integer")
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<String, E> { Ok(v.to_owned()) }
        fn visit_string<E: de::Error>(self, v: String) -> Result<String, E> { Ok(v) }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<String, E> { Ok(v.to_string()) }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<String, E> { Ok(v.to_string()) }
    }
    deserializer.deserialize_any(StringOrInt)
}

/// Deserialize a field that may be either a JSON number or a quoted string into f64.
fn deserialize_f64_or_string<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    struct F64OrString;
    impl<'de> Visitor<'de> for F64OrString {
        type Value = f64;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "a number or a string containing a number")
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<f64, E> { Ok(v) }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<f64, E> { Ok(v as f64) }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<f64, E> { Ok(v as f64) }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<f64, E> {
            v.parse().map_err(de::Error::custom)
        }
    }
    deserializer.deserialize_any(F64OrString)
}

// ============================================================================
// Enum Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "buy"),
            OrderSide::Sell => write!(f, "sell"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionSide {
    Long,
    Short,
    Both,
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionSide::Long => write!(f, "long"),
            PositionSide::Short => write!(f, "short"),
            PositionSide::Both => write!(f, "both"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TimeInForce {
    GoodTillCancel,
    ImmediateOrCancel,
    FillOrKill,
    PostOnly,
}

impl std::fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeInForce::GoodTillCancel => write!(f, "GoodTillCancel"),
            TimeInForce::ImmediateOrCancel => write!(f, "ImmediateOrCancel"),
            TimeInForce::FillOrKill => write!(f, "FillOrKill"),
            TimeInForce::PostOnly => write!(f, "PostOnly"),
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TriggerType {
    ByLastPrice,
    ByMarkPrice,
    ByIndexPrice,
}

impl std::fmt::Display for TriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriggerType::ByLastPrice => write!(f, "ByLastPrice"),
            TriggerType::ByMarkPrice => write!(f, "ByMarkPrice"),
            TriggerType::ByIndexPrice => write!(f, "ByIndexPrice"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarginMode {
    Isolated,
    Cross,
}

impl std::fmt::Display for MarginMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarginMode::Isolated => write!(f, "isolated"),
            MarginMode::Cross => write!(f, "cross"),
        }
    }
}

// ============================================================================
// API Request Models
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRequest<T> {
    pub exchange_name: String,
    pub method: String,
    pub params: T,
    pub credentials: Credentials,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub api_key: String,
    pub api_secret: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

/// Exchange credentials sent in every API request body.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeCredentials {
    pub api_key: String,
    pub api_secret: String,
    /// Required for OKX, KuCoin, Orderly (broker ID), Bitget, BloFin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

// ============================================================================
// Order Parameters
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitOrderParams {
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: f64,
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<PositionSide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub take_profit_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_loss_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketOrderParams {
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<PositionSide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopOrderParams {
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: f64,
    pub stop_price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<PositionSide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_type: Option<TriggerType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_position: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CancelOrderParams {
    pub symbol: String,
    #[serde(rename = "orderID", skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(rename = "clientOrderID", skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SetLeverageParams {
    pub symbol: String,
    pub leverage: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosePositionParams {
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<PositionSide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetMarginModeParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub margin_mode: MarginMode,
}

#[derive(Debug, Serialize)]
pub struct GetPositionsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetBalanceParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetTickersParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetBestBidAskParams {
    pub symbol: String,
}

// ============================================================================
// API Response Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    #[serde(default)]
    pub code: u16,
    #[serde(default, alias = "error")]
    pub message: Option<String>,
}

// ============================================================================
// Domain Models
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    #[serde(default, alias = "id", alias = "order_id", deserialize_with = "deserialize_string_or_int")]
    pub order_id: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub side: String,
    #[serde(default, alias = "positionSide")]
    pub position_side: String,
    #[serde(default, alias = "type", alias = "order_type")]
    pub order_type: String,
    #[serde(default)]
    pub price: f64,
    pub quantity: f64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filled_quantity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average_price: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub symbol: String,
    pub side: String,
    pub position_side: String,
    pub size: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub pnl: f64,
    pub leverage: u32,
    pub liquidation_price: f64,
    pub margin_type: String,
    pub unrealized_pnl: f64,
    pub notional: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Balance {
    pub asset: String,
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub balance: f64,
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub available: f64,
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub locked: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticker {
    pub symbol: String,
    #[serde(rename = "close")]
    pub last_price: f64,
    pub price_change_percent: f64,
    pub volume: f64,
    pub high: f64,
    pub low: f64,
    #[serde(rename = "bidPrice")]
    pub bid: Option<f64>,
    #[serde(rename = "askPrice")]
    pub ask: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BidAskLevel {
    pub price: String,
    pub quantity: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BestBidAsk {
    pub best_bid: BidAskLevel,
    pub best_ask: BidAskLevel,
}

// ============================================================================
// API Result Types
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeverageResult {
    pub symbol: String,
    pub leverage: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_leverage: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginModeResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub margin_mode: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HedgeModeResult {
    pub hedge_mode: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelAllResult {
    pub message: String,
}

// ============================================================================
// Market Data Models (TTC Box direct endpoints)
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HybridTicker {
    pub symbol: String,
    #[serde(default)]
    pub last_price: String,
    #[serde(default)]
    pub price_change_percent: String,
    #[serde(default)]
    pub high_price: String,
    #[serde(default)]
    pub low_price: String,
    #[serde(default)]
    pub volume: String,
    #[serde(default)]
    pub quote_volume: String,
    #[serde(default)]
    pub bid_price: String,
    #[serde(default)]
    pub ask_price: String,
    #[serde(default)]
    pub open_interest: Option<String>,
    #[serde(default)]
    pub funding: Option<String>,
    #[serde(default)]
    pub source: String,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HybridTickerList {
    pub data: Vec<HybridTicker>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HybridTickersData {
    pub spot: HybridTickerList,
    pub futures: HybridTickerList,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRate {
    pub exchange: String,
    pub symbol: String,
    pub funding_rate: f64,
    pub next_funding_time: i64,
    pub timestamp: i64,
    #[serde(default)]
    pub open_interest: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenInterestItem {
    pub symbol: String,
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub open_interest: f64,
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub open_interest_usd: f64,
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub price: f64,
    #[serde(deserialize_with = "deserialize_f64_or_string")]
    pub volume_usd: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSnapshotExchange {
    pub exchange: String,
    pub display_name: String,
    pub chain: String,
    pub total_volume_24h: f64,
    pub total_open_interest: f64,
    #[serde(default)]
    pub tvl: f64,
    pub markets: Vec<VolumeSnapshotMarket>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSnapshotMarket {
    pub symbol: String,
    pub volume_24h: f64,
    #[serde(default)]
    pub open_interest: f64,
    pub price: f64,
    #[serde(default)]
    pub funding_rate: f64,
}

// ============================================================================
// TTC Scanner
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ScannerResult {
    pub symbol: String,
    pub signal: ScanSignal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSignal {
    pub direction: String,
    pub strength: f64,
    pub confidence: String,
    pub entry: f64,
    pub stop_loss: f64,
    pub take_profit1: f64,
    pub take_profit2: f64,
    pub take_profit3: f64,
    pub risk_reward_ratio: f64,
    pub reasoning: String,
}
