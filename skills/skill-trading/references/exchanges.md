# Supported Exchanges

All exchanges are accessed via the TTC Box proxy at `https://ttc.box`. You never call exchange APIs directly.

## Exchange Names (use with `-e` flag)

| Name         | Type         | Passphrase Required | Notes                                      |
|--------------|--------------|---------------------|--------------------------------------------|
| `orderly`    | Futures      | Yes (`what_exchange`, `woofi_pro`, `ttc`, etc.) | BrokerID = passphrase |
| `binance`    | Spot/Futures | No                  |                                            |
| `bybit`      | Spot/Futures | No                  |                                            |
| `okx`        | Spot/Futures | Yes                 |                                            |
| `phemex`     | Futures      | No                  |                                            |
| `bitget`     | Futures      | Yes                 |                                            |
| `blofin`     | Futures      | Yes                 |                                            |
| `kucoin`     | Spot/Futures | Yes                 |                                            |
| `asterdex`   | Futures      | No                  | Requires funded account                    |
| `hyperliquid`| Futures      | No                  |                                            |
| `avr`        | Futures      | No                  | Aggregated virtual router                  |
| `bingx`      | Futures      | No                  |                                            |

## Exchange-Specific Quirks

### Orderly
- `passphrase` = broker ID (e.g. `what_exchange`), not an API passphrase
- `order_id` is returned as an **integer** from the API
- Stop/algo orders use a separate cancel endpoint (handled server-side)
- Symbol format: `NEARUSDT`, `BTCUSDT` (no prefix)
- Tick sizes vary per symbol — always check `market best-bid-ask` before placing

### Market Data Commands (no `-e` required)
These hit TTC Box aggregation endpoints directly:
- `market hybrid-tickers` — cross-exchange tickers, use `--source` to filter by exchange
- `market funding-rates` — funding rates across all exchanges
- `market open-interest` — open interest across all exchanges
- `market volume-snapshot` — 24h volume per exchange
- `market scanner` — technical analysis signal (requires `--symbol`, `--timeframe`)

## Credentials Setup

Credentials are loaded from `.env` in the working directory:

```env
TTC_AUTH_TOKEN=your_ttc_token
TTC_PUBLIC_KEY=your_public_key
TTC_EXCHANGE=orderly

EXCHANGE_API_KEY=your_exchange_key
EXCHANGE_API_SECRET=your_exchange_secret
EXCHANGE_API_PASSPHRASE=your_passphrase
```

Or pass per-command:
```
skill-trading order limit --exchange orderly --api-key KEY --api-secret SECRET --passphrase PASS ...
```
