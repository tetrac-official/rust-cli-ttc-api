# BUG-001: camelCase Payload Mismatch

**Date:** 2026-03-27
**Status:** Fixed
**Severity:** Critical — all API calls failed

---

## Summary

The Rust CLI was serializing JSON request bodies in `snake_case` but the TTC Box API (`ttc.box/api/v1/exchanges`) expects `camelCase` field names. This caused the server to reject exchange credentials and the top-level request envelope entirely.

---

## Symptoms

- `account balance` returned `{"error": "API credentials required"}` even with valid credentials set
- The server received `api_key` / `api_secret` but checked for `apiKey` / `apiSecret`
- The server received `exchange_name` but checked for `exchangeName`

---

## Root Cause

Rust's `serde` serializes struct fields as-is (snake_case by default). The affected structs had no `rename_all` directive:

```rust
// BEFORE — serialized as snake_case
pub struct ExchangeRequest<T> {
    pub exchange_name: String,   // → "exchange_name"
    pub method: String,
    pub params: T,
    pub credentials: Credentials,
}

pub struct Credentials {
    pub api_key: String,         // → "api_key"
    pub api_secret: String,      // → "api_secret"
}
```

The server route (`/api/v1/exchanges/route.ts`) destructures the body as:

```typescript
const { exchangeName, method, params, credentials } = body;
// credentials?.apiKey
// credentials?.apiSecret
```

So `exchangeName` was `undefined`, and `credentials.apiKey` was `undefined`, causing the credentials check to fail.

---

## Fix

Added `#[serde(rename_all = "camelCase")]` to all request structs that are serialized into the POST body:

```rust
// AFTER — serialized as camelCase
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRequest<T> {
    pub exchange_name: String,   // → "exchangeName"
    pub method: String,
    pub params: T,
    pub credentials: Credentials,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub api_key: String,         // → "apiKey"
    pub api_secret: String,      // → "apiSecret"
}
```

The following structs were all updated in `src/models.rs`:

| Struct | Fields affected |
|---|---|
| `ExchangeRequest` | `exchange_name` → `exchangeName` |
| `Credentials` | `api_key`, `api_secret` |
| `ExchangeCredentials` | `api_key`, `api_secret`, `passphrase` |
| `LimitOrderParams` | `position_side`, `time_in_force`, `reduce_only`, `take_profit_price`, `stop_loss_price`, `client_order_id` |
| `MarketOrderParams` | `position_side`, `reduce_only`, `client_order_id` |
| `StopOrderParams` | `stop_price`, `position_side`, `trigger_type`, `close_position`, `reduce_only`, `client_order_id` |
| `CancelOrderParams` | `order_id`, `client_order_id` |
| `ClosePositionParams` | `position_side` |
| `SetMarginModeParams` | `margin_mode` |

---

## Verification

```bash
curl -X POST https://ttc.box/api/v1/exchanges \
  -H "ttc-auth-token: <token>" \
  -H "ttc-public-key: <pubkey>" \
  -H "Content-Type: application/json" \
  -d '{"exchangeName":"asterdex","method":"getBalance","params":{"asset":null},"credentials":{"apiKey":"...","apiSecret":"..."}}'
# → {"code":200,"msg":"success","data":[],"exchange":"asterdex","success":true}
```
