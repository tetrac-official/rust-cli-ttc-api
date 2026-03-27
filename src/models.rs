use serde::{Deserialize, Deserializer, Serialize};

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
#[serde(rename_all = "camelCase")]
pub struct CancelOrderParams {
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub code: u16,
    #[serde(default)]
    pub message: Option<String>,
}

// ============================================================================
// Domain Models
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub position_side: String,
    pub order_type: String,
    pub price: f64,
    pub quantity: f64,
    pub status: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_quantity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
