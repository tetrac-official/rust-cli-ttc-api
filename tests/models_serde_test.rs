//! Wire-format tests for src/models.rs.
//!
//! These lock in the JSON shape the CLI sends to TTC Box and the shape it
//! expects back. They catch breakage when:
//! - A struct field gets renamed or its serde rename changes.
//! - An enum's case-mapping (lowercase / PascalCase) drifts.
//! - A response field that some exchanges return as a string-quoted number
//!   stops being accepted.
//! - An alias (e.g. `id` for `order_id`) gets dropped.
//!
//! Wrong wire shape = exchange rejects the request OR CLI fails to parse the
//! response. Both are silent regressions until a user hits them in prod.

use serde_json::{json, Value};
use skill_trading::models::*;

// ============================================================================
// Enum wire format
// ============================================================================

#[test]
fn order_side_serializes_lowercase() {
    assert_eq!(serde_json::to_string(&OrderSide::Buy).unwrap(), "\"buy\"");
    assert_eq!(serde_json::to_string(&OrderSide::Sell).unwrap(), "\"sell\"");
}

#[test]
fn order_side_round_trips() {
    let original = OrderSide::Buy;
    let s = serde_json::to_string(&original).unwrap();
    let back: OrderSide = serde_json::from_str(&s).unwrap();
    assert_eq!(back, original);
}

#[test]
fn order_side_rejects_wrong_case() {
    assert!(serde_json::from_str::<OrderSide>("\"BUY\"").is_err());
    assert!(serde_json::from_str::<OrderSide>("\"Buy\"").is_err());
}

#[test]
fn position_side_serializes_lowercase() {
    assert_eq!(
        serde_json::to_string(&PositionSide::Long).unwrap(),
        "\"long\""
    );
    assert_eq!(
        serde_json::to_string(&PositionSide::Short).unwrap(),
        "\"short\""
    );
    assert_eq!(
        serde_json::to_string(&PositionSide::Both).unwrap(),
        "\"both\""
    );
}

#[test]
fn time_in_force_serializes_pascal_case() {
    // Exchanges expect PascalCase here — wrong casing = order rejected.
    assert_eq!(
        serde_json::to_string(&TimeInForce::GoodTillCancel).unwrap(),
        "\"GoodTillCancel\""
    );
    assert_eq!(
        serde_json::to_string(&TimeInForce::PostOnly).unwrap(),
        "\"PostOnly\""
    );
    assert_eq!(
        serde_json::to_string(&TimeInForce::ImmediateOrCancel).unwrap(),
        "\"ImmediateOrCancel\""
    );
    assert_eq!(
        serde_json::to_string(&TimeInForce::FillOrKill).unwrap(),
        "\"FillOrKill\""
    );
}

#[test]
fn trigger_type_serializes_pascal_case() {
    assert_eq!(
        serde_json::to_string(&TriggerType::ByLastPrice).unwrap(),
        "\"ByLastPrice\""
    );
    assert_eq!(
        serde_json::to_string(&TriggerType::ByMarkPrice).unwrap(),
        "\"ByMarkPrice\""
    );
    assert_eq!(
        serde_json::to_string(&TriggerType::ByIndexPrice).unwrap(),
        "\"ByIndexPrice\""
    );
}

#[test]
fn margin_mode_serializes_lowercase() {
    assert_eq!(
        serde_json::to_string(&MarginMode::Isolated).unwrap(),
        "\"isolated\""
    );
    assert_eq!(
        serde_json::to_string(&MarginMode::Cross).unwrap(),
        "\"cross\""
    );
}

// ============================================================================
// Permissive numeric deserializers (string OR number)
//
// TTC Box aggregates from 15+ exchanges; some return numbers, some return
// quoted strings. The deserialize_f64_or_string / deserialize_opt_f64_or_string
// helpers accept either. If those helpers ever stop accepting one shape, every
// position read from those exchanges starts failing.
// ============================================================================

#[test]
fn balance_accepts_numeric_fields() {
    let b: Balance = serde_json::from_str(
        r#"{"asset":"USDT","balance":1234.5,"available":1000.0,"locked":234.5}"#,
    )
    .expect("parse");
    assert_eq!(b.asset, "USDT");
    assert_eq!(b.balance, 1234.5);
    assert_eq!(b.available, 1000.0);
    assert_eq!(b.locked, Some(234.5));
}

#[test]
fn balance_accepts_string_quoted_numbers() {
    let b: Balance = serde_json::from_str(
        r#"{"asset":"USDT","balance":"1234.5","available":"1000.0","locked":"234.5"}"#,
    )
    .expect("parse");
    assert_eq!(b.balance, 1234.5);
    assert_eq!(b.available, 1000.0);
    assert_eq!(b.locked, Some(234.5));
}

#[test]
fn balance_accepts_missing_locked_field() {
    let b: Balance = serde_json::from_str(r#"{"asset":"USDT","balance":100.0,"available":100.0}"#)
        .expect("parse");
    assert!(b.locked.is_none());
}

#[test]
fn balance_accepts_null_locked() {
    let b: Balance =
        serde_json::from_str(r#"{"asset":"USDT","balance":100.0,"available":100.0,"locked":null}"#)
            .expect("parse");
    assert!(b.locked.is_none());
}

#[test]
fn balance_accepts_empty_string_locked() {
    // deserialize_opt_f64_or_string treats "" as None — exchanges that return
    // "" for unset numeric fields don't break parsing.
    let b: Balance =
        serde_json::from_str(r#"{"asset":"USDT","balance":100.0,"available":100.0,"locked":""}"#)
            .expect("empty string locked should map to None");
    assert!(b.locked.is_none());
}

#[test]
fn position_accepts_empty_string_for_optional_numeric_fields() {
    // If any exchange ever sends "" for liquidationPrice / pnl / unrealizedPnl
    // / notional, the position read must still succeed with those as None.
    let p: Position = serde_json::from_str(
        r#"{
            "symbol":"BTCUSDT","side":"long","positionSide":"long",
            "size":0.5,"entryPrice":30000.0,"markPrice":31000.0,
            "pnl":"","leverage":10,
            "liquidationPrice":"","unrealizedPnl":"","notional":""
        }"#,
    )
    .expect("empty strings should map to None on Option<f64> fields");
    assert!(p.pnl.is_none());
    assert!(p.unrealized_pnl.is_none());
    assert!(p.liquidation_price.is_none());
    assert!(p.notional.is_none());
    assert_eq!(p.effective_pnl(), 0.0);
}

#[test]
fn open_interest_accepts_string_or_number() {
    let numeric: OpenInterestItem = serde_json::from_str(
        r#"{"symbol":"BTCUSDT","openInterest":1.5,"openInterestUsd":50000.0,"price":33333.0,"volumeUsd":1.0,"timestamp":0}"#,
    ).unwrap();
    let stringed: OpenInterestItem = serde_json::from_str(
        r#"{"symbol":"BTCUSDT","openInterest":"1.5","openInterestUsd":"50000.0","price":"33333.0","volumeUsd":"1.0","timestamp":0}"#,
    ).unwrap();
    assert_eq!(numeric.open_interest, stringed.open_interest);
    assert_eq!(numeric.price, stringed.price);
}

// ============================================================================
// Position — pnl/unrealized_pnl/notional/liquidation can be missing or null
// ============================================================================

#[test]
fn position_with_all_optional_pnl_fields_present() {
    let p: Position = serde_json::from_str(
        r#"{
            "symbol":"BTCUSDT","side":"long","positionSide":"long",
            "size":0.5,"entryPrice":30000.0,"markPrice":31000.0,
            "pnl":500.0,"leverage":10,
            "liquidationPrice":25000.0,"marginType":"cross",
            "unrealizedPnl":500.0,"notional":15500.0
        }"#,
    )
    .expect("parse");
    assert_eq!(p.pnl, Some(500.0));
    assert_eq!(p.unrealized_pnl, Some(500.0));
    assert_eq!(p.effective_pnl(), 500.0);
    assert_eq!(p.notional, Some(15500.0));
}

#[test]
fn position_with_null_pnl_uses_unrealized_pnl_fallback() {
    // dYdX returns the real value in `pnl` and leaves `unrealizedPnl` null
    // — but the inverse can also happen. Position::effective_pnl() handles both.
    let p: Position = serde_json::from_str(
        r#"{
            "symbol":"BTCUSDT","side":"long","positionSide":"long",
            "size":0.5,"entryPrice":30000.0,"markPrice":31000.0,
            "pnl":null,"leverage":10,
            "liquidationPrice":null,"unrealizedPnl":250.0
        }"#,
    )
    .expect("parse");
    assert!(p.pnl.is_none());
    assert!(p.liquidation_price.is_none());
    assert_eq!(p.unrealized_pnl, Some(250.0));
    assert_eq!(p.effective_pnl(), 250.0);
}

#[test]
fn position_with_no_pnl_at_all_is_zero() {
    let p: Position = serde_json::from_str(
        r#"{
            "symbol":"BTCUSDT","side":"long","positionSide":"long",
            "size":0.5,"entryPrice":30000.0,"markPrice":31000.0,
            "leverage":10
        }"#,
    )
    .expect("parse");
    assert_eq!(p.effective_pnl(), 0.0);
}

#[test]
fn position_round_trip_preserves_fields() {
    let original_json = r#"{
        "symbol":"NEARUSDT","side":"short","positionSide":"short",
        "size":100.0,"entryPrice":2.5,"markPrice":2.4,
        "pnl":10.0,"leverage":5,"liquidationPrice":3.0,
        "marginType":"isolated","unrealizedPnl":10.0,"notional":240.0
    }"#;
    let p: Position = serde_json::from_str(original_json).unwrap();
    let s = serde_json::to_string(&p).unwrap();
    let back: Position = serde_json::from_str(&s).unwrap();
    assert_eq!(back.symbol, p.symbol);
    assert_eq!(back.size, p.size);
    assert_eq!(back.entry_price, p.entry_price);
    assert_eq!(back.pnl, p.pnl);
    assert_eq!(back.unrealized_pnl, p.unrealized_pnl);
    assert_eq!(back.liquidation_price, p.liquidation_price);
}

// ============================================================================
// Order — aliases for id/order_id/type
// ============================================================================

#[test]
fn order_accepts_id_alias() {
    let o: Order =
        serde_json::from_str(r#"{"id":"abc-123","symbol":"BTCUSDT","quantity":1.0}"#).unwrap();
    assert_eq!(o.order_id, "abc-123");
}

#[test]
fn order_accepts_order_id_field() {
    let o: Order =
        serde_json::from_str(r#"{"orderId":"abc-456","symbol":"BTCUSDT","quantity":1.0}"#).unwrap();
    assert_eq!(o.order_id, "abc-456");
}

#[test]
fn order_accepts_integer_id() {
    // Some exchanges return numeric order IDs.
    let o: Order =
        serde_json::from_str(r#"{"id":123456789,"symbol":"BTCUSDT","quantity":1.0}"#).unwrap();
    assert_eq!(o.order_id, "123456789");
}

#[test]
fn order_accepts_type_alias() {
    let o: Order =
        serde_json::from_str(r#"{"id":"x","symbol":"BTCUSDT","quantity":1.0,"type":"limit"}"#)
            .unwrap();
    assert_eq!(o.order_type, "limit");
}

#[test]
fn order_optional_fill_fields_are_none_by_default() {
    let o: Order = serde_json::from_str(r#"{"id":"x","symbol":"BTCUSDT","quantity":1.0}"#).unwrap();
    assert!(o.filled_quantity.is_none());
    assert!(o.average_price.is_none());
}

#[test]
fn order_accepts_standx_filled_and_avg_fill_price_aliases() {
    // standx returns "filled" / "avgFillPrice" instead of "filledQuantity" /
    // "averagePrice". Aliases must populate the canonical fields.
    let o: Order = serde_json::from_str(
        r#"{"id":"abc","symbol":"BTCUSDT","quantity":0.001,
            "filled":0.0005,"avgFillPrice":31000.0}"#,
    )
    .unwrap();
    assert_eq!(o.filled_quantity, Some(0.0005));
    assert_eq!(o.average_price, Some(31000.0));
}

#[test]
fn order_filled_alias_accepts_empty_string() {
    // If standx (or any exchange) ever sends "filled":"" we must not blow up.
    let o: Order = serde_json::from_str(
        r#"{"id":"abc","symbol":"BTCUSDT","quantity":0.001,
            "filled":"","avgFillPrice":""}"#,
    )
    .unwrap();
    assert!(o.filled_quantity.is_none());
    assert!(o.average_price.is_none());
}

#[test]
fn parses_standx_market_order_response_envelope_and_order() {
    // Real response shape captured from standx — a fresh "new" market order
    // with id="" and zeroed numeric fields.
    let raw = r#"{
        "code":0,"success":true,
        "data":{
            "id":"","symbol":"btcusdt","side":"sell","type":"market",
            "price":0,"stopPrice":0,"quantity":0.001,
            "filled":0,"avgFillPrice":0,"status":"new","timestamp":1777371526222
        },
        "msg":"success","exchange":"standx"
    }"#;

    let env: ApiResponse<Order> = serde_json::from_str(raw).expect("parse envelope");
    assert!(env.success);
    assert_eq!(env.code, 0);
    assert_eq!(env.message.as_deref(), Some("success"), "msg → message");

    let o = env.data;
    assert_eq!(
        o.order_id, "",
        "empty-string id is allowed on String fields"
    );
    assert_eq!(o.symbol, "btcusdt");
    assert_eq!(o.side, "sell");
    assert_eq!(
        o.order_type, "market",
        "type alias must populate order_type"
    );
    assert_eq!(o.quantity, 0.001);
    assert_eq!(o.filled_quantity, Some(0.0), "filled alias");
    assert_eq!(o.average_price, Some(0.0), "avgFillPrice alias");
    assert_eq!(o.status, "new");
    assert_eq!(o.timestamp, 1777371526222);
}

#[test]
fn api_response_msg_alias_parses() {
    // standx uses "msg" instead of "error"/"message" on failure too.
    let r: ApiResponse<i32> =
        serde_json::from_str(r#"{"success":false,"data":0,"msg":"insufficient balance"}"#).unwrap();
    assert_eq!(r.message.as_deref(), Some("insufficient balance"));
}

// ============================================================================
// ScanSignal — stop_loss / TP fields null on NEUTRAL signals
// ============================================================================

#[test]
fn scan_signal_neutral_with_null_levels_parses() {
    // Per CLAUDE.md: NEUTRAL signals have null stop_loss and TP levels.
    let raw = r#"{
        "direction":"NEUTRAL","strength":0.0,"confidence":"LOW",
        "entry":1.0,"stopLoss":null,"takeProfit1":null,
        "takeProfit2":null,"takeProfit3":null,
        "riskRewardRatio":null,"reasoning":"no edge"
    }"#;
    let s: ScanSignal = serde_json::from_str(raw).unwrap();
    assert_eq!(s.direction, "NEUTRAL");
    assert!(s.stop_loss.is_none());
    assert!(s.take_profit1.is_none());
    assert!(s.take_profit2.is_none());
    assert!(s.take_profit3.is_none());
    assert!(s.risk_reward_ratio.is_none());
}

#[test]
fn scan_signal_long_with_full_levels_parses() {
    let raw = r#"{
        "direction":"LONG","strength":2.5,"confidence":"HIGH",
        "entry":1.1700,"stopLoss":1.1500,"takeProfit1":1.2000,
        "takeProfit2":1.2200,"takeProfit3":1.2500,
        "riskRewardRatio":3.0,"reasoning":"breakout above resistance"
    }"#;
    let s: ScanSignal = serde_json::from_str(raw).unwrap();
    assert_eq!(s.entry, 1.17);
    assert_eq!(s.stop_loss, Some(1.15));
    assert_eq!(s.take_profit3, Some(1.25));
    assert_eq!(s.risk_reward_ratio, Some(3.0));
}

// ============================================================================
// ApiResponse — success, error alias, missing optional message
// ============================================================================

#[test]
fn api_response_success_with_data_parses() {
    let r: ApiResponse<Vec<i32>> =
        serde_json::from_str(r#"{"success":true,"data":[1,2,3]}"#).unwrap();
    assert!(r.success);
    assert_eq!(r.data, vec![1, 2, 3]);
}

#[test]
fn api_response_message_field_parses() {
    let r: ApiResponse<i32> =
        serde_json::from_str(r#"{"success":false,"data":0,"message":"upstream timeout"}"#).unwrap();
    assert_eq!(r.message.as_deref(), Some("upstream timeout"));
}

#[test]
fn api_response_error_alias_parses() {
    // route.ts emits {"error": "..."} on failures — alias maps it to `message`.
    let r: ApiResponse<i32> =
        serde_json::from_str(r#"{"success":false,"data":0,"error":"bad symbol"}"#).unwrap();
    assert_eq!(r.message.as_deref(), Some("bad symbol"));
}

// ============================================================================
// ExchangeRequest body shape — what we POST to /exchanges
// ============================================================================

#[test]
fn exchange_request_serializes_camel_case_top_level() {
    let req = ExchangeRequest {
        exchange_name: "orderly".into(),
        method: "getBalance".into(),
        params: GetBalanceParams { asset: None },
        credentials: ExchangeCredentials {
            api_key: "k".into(),
            api_secret: "s".into(),
            passphrase: None,
            wallet_address: None,
        },
    };
    let v: Value = serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();
    let obj = v.as_object().unwrap();
    assert!(obj.contains_key("exchangeName"));
    assert!(obj.contains_key("method"));
    assert!(obj.contains_key("params"));
    assert!(obj.contains_key("credentials"));
    assert_eq!(obj["exchangeName"], json!("orderly"));
    assert_eq!(obj["method"], json!("getBalance"));
}

#[test]
fn exchange_credentials_skips_none_passphrase_and_wallet() {
    let creds = ExchangeCredentials {
        api_key: "k".into(),
        api_secret: "s".into(),
        passphrase: None,
        wallet_address: None,
    };
    let s = serde_json::to_string(&creds).unwrap();
    assert!(s.contains("apiKey"));
    assert!(s.contains("apiSecret"));
    assert!(!s.contains("passphrase"));
    assert!(!s.contains("walletAddress"));
}

#[test]
fn exchange_credentials_includes_set_passphrase_and_wallet() {
    let creds = ExchangeCredentials {
        api_key: "k".into(),
        api_secret: "s".into(),
        passphrase: Some("ttc-broker".into()),
        wallet_address: Some("0xabc".into()),
    };
    let v: Value = serde_json::from_str(&serde_json::to_string(&creds).unwrap()).unwrap();
    assert_eq!(v["passphrase"], json!("ttc-broker"));
    assert_eq!(v["walletAddress"], json!("0xabc"));
}

// ============================================================================
// Order params — key shape + enum casing on the wire
// ============================================================================

#[test]
fn limit_order_params_wire_shape() {
    let p = LimitOrderParams {
        symbol: "BTCUSDT".into(),
        side: OrderSide::Buy,
        quantity: 0.5,
        price: 30000.0,
        position_side: Some(PositionSide::Long),
        time_in_force: Some(TimeInForce::PostOnly),
        reduce_only: Some(false),
        take_profit_price: None,
        stop_loss_price: None,
        client_order_id: Some("ttc-001".into()),
    };
    let v: Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(v["symbol"], json!("BTCUSDT"));
    assert_eq!(v["side"], json!("buy"));
    assert_eq!(v["quantity"], json!(0.5));
    assert_eq!(v["price"], json!(30000.0));
    assert_eq!(v["positionSide"], json!("long"));
    assert_eq!(v["timeInForce"], json!("PostOnly"));
    assert_eq!(v["reduceOnly"], json!(false));
    assert_eq!(v["clientOrderId"], json!("ttc-001"));
    // None fields are skipped
    assert!(v.get("takeProfitPrice").is_none());
    assert!(v.get("stopLossPrice").is_none());
}

#[test]
fn market_order_params_wire_shape() {
    let p = MarketOrderParams {
        symbol: "ETHUSDT".into(),
        side: OrderSide::Sell,
        quantity: 1.0,
        position_side: None,
        reduce_only: Some(true),
        client_order_id: None,
    };
    let v: Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(v["side"], json!("sell"));
    assert_eq!(v["reduceOnly"], json!(true));
    assert!(v.get("positionSide").is_none());
    assert!(v.get("clientOrderId").is_none());
}

#[test]
fn cancel_order_params_uses_uppercase_id_keys() {
    // Exchange APIs are inconsistent; this struct uses orderID and clientOrderID
    // (uppercase ID) — the rename must be preserved.
    let p = CancelOrderParams {
        symbol: "BTCUSDT".into(),
        order_id: Some("abc".into()),
        client_order_id: Some("xyz".into()),
    };
    let v: Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(v["orderID"], json!("abc"));
    assert_eq!(v["clientOrderID"], json!("xyz"));
    assert!(v.get("orderId").is_none(), "lowercase 'd' must not appear");
}

#[test]
fn stop_order_params_wire_shape() {
    let p = StopOrderParams {
        symbol: "BTCUSDT".into(),
        side: OrderSide::Sell,
        quantity: 0.1,
        stop_price: 28000.0,
        position_side: None,
        trigger_type: Some(TriggerType::ByMarkPrice),
        price: None,
        close_position: Some(true),
        reduce_only: Some(true),
        client_order_id: None,
    };
    let v: Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(v["stopPrice"], json!(28000.0));
    assert_eq!(v["triggerType"], json!("ByMarkPrice"));
    assert_eq!(v["closePosition"], json!(true));
    assert_eq!(v["reduceOnly"], json!(true));
}

#[test]
fn set_leverage_params_serializes_snake_case() {
    // SetLeverageParams has no rename_all attribute, so fields stay snake_case.
    let p = SetLeverageParams {
        symbol: "BTCUSDT".into(),
        leverage: 10,
    };
    let v: Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(v["symbol"], json!("BTCUSDT"));
    assert_eq!(v["leverage"], json!(10));
}

// ============================================================================
// Result types — round-trip
// ============================================================================

#[test]
fn leverage_result_round_trip() {
    let r: LeverageResult =
        serde_json::from_str(r#"{"symbol":"BTCUSDT","leverage":10,"maxLeverage":125}"#).unwrap();
    let s = serde_json::to_string(&r).unwrap();
    assert!(s.contains("\"maxLeverage\":125"));
    let back: LeverageResult = serde_json::from_str(&s).unwrap();
    assert_eq!(back.leverage, 10);
    assert_eq!(back.max_leverage, Some(125));
}

#[test]
fn hedge_mode_result_round_trip() {
    let r: HedgeModeResult = serde_json::from_str(r#"{"hedge_mode":true}"#).unwrap();
    assert!(r.hedge_mode);
    // Serialize back — no rename_all, so field stays snake_case.
    let s = serde_json::to_string(&r).unwrap();
    assert!(s.contains("\"hedge_mode\":true"), "got: {s}");
}
