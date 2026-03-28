# Signal Patrol Watchlist

Edit this file to add or remove symbols from the patrol.

## Format

Each row: `SYMBOL | primary timeframe | notes`

The patrol runs the scanner on the primary timeframe first. If it qualifies (HIGH confidence, R/R ≥ 3.0), it also checks 4h for confirmation.

---

## Active Watchlist

| Symbol | Primary TF | Notes |
|--------|-----------|-------|
| BTCUSDT | 1h | Market leader — always check first |
| ETHUSDT | 1h | Sector indicator for DeFi/L1s |
| SOLUSDT | 1h | High liquidity, strong moves |
| NEARUSDT | 1h | |
| BNBUSDT | 1h | |
| XRPUSDT | 1h | |
| AVAXUSDT | 1h | |
| LINKUSDT | 1h | |
| INJUSDT | 1h | |
| ARBUSDT | 1h | |
| OPUSDT | 1h | |
| SUIUSDT | 1h | |
| APTUSDT | 1h | |
| DOTUSDT | 1h | |
| ADAUSDT | 1h | |

---

## How to Add a Symbol

Add a row to the table above. Use the exact symbol format the exchange accepts (e.g., `NEARUSDT`, not `NEAR/USDT`).

## How to Remove a Symbol

Delete or comment out (prefix with `#`) the row.

## How to Change Timeframe

Change the Primary TF column. Valid values: `1m`, `5m`, `15m`, `1h`, `4h`, `1d`.

Use `4h` as primary for symbols you want to hold for days, `1h` for intraday.
