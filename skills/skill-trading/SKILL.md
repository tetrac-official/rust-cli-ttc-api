# skill-trading — Order Management Skill

This skill governs how an AI assistant should interact with the `skill-trading` CLI.
It exists to prevent hallucination and unsafe order execution.

---

## MANDATORY PRE-ORDER CHECKLIST

Before placing **any** order (`limit`, `market`, `stop`), you MUST run these checks in order.
Do not skip steps. Do not place an order if any check fails.

### Step 1 — Check available balance

```
skill-trading account balance
```

Parse the `Available` value from the output.

- If `Available` is **negative** or **zero** → **STOP. Do not place the order.**
  - Tell the user: "Available balance is [amount]. Cannot place order."
  - Suggest: close a position, reduce size, or add funds.

- If `Available` is **positive** → continue to Step 2.

### Step 2 — Check existing positions

```
skill-trading position get
```

Review open positions:
- Note symbols, sizes, side (long/short), PnL, and liquidation prices.
- If the user wants to open in the same direction as an existing position, flag it.
- If the user wants to reduce/close, use `position close` not a new order.

### Step 3 — Validate order size vs available margin

Estimate required margin:
```
required_margin = (quantity × price) / leverage
```

- If `required_margin > available` → **STOP. Reduce quantity or skip.**
- Always confirm with the user before placing if margin is tight (< 20% headroom).

### Step 4 — Check open orders

```
skill-trading order open
```

- If there are existing orders on the same symbol/side, flag potential duplicates.
- Ask the user to confirm before adding another order.

---

## ORDER PLACEMENT RULES

- **Never place a real order without user confirmation** unless explicitly told to automate.
- **Always show the order summary** before executing:
  ```
  Symbol:    NEARUSDT
  Side:      BUY
  Type:      LIMIT
  Price:     $1.168
  Quantity:  17
  Value:     ~$19.86
  Exchange:  orderly
  ```
- Use `--dry-run` first when testing a new order type or exchange.
- If the user says "place an order at 1% under price", always fetch the current price first via:
  ```
  skill-trading market best-bid-ask --symbol <SYMBOL>
  ```
  Then calculate: `price = bid × 0.99`, `quantity = budget / price`.

---

## INTERPRETING BALANCE OUTPUT

| Field       | Meaning                                              |
|-------------|------------------------------------------------------|
| `balance`   | Total USDT in account                                |
| `locked`    | Margin currently committed to open positions/orders  |
| `available` | Free margin = balance − locked. **This is what matters for new orders.** |

A **negative available** means the account is over-committed. No new orders can be placed until a position is closed or margin is freed.

---

## INTERPRETING POSITION OUTPUT

| Field        | Meaning                                         |
|--------------|-------------------------------------------------|
| `size`       | Position size in base asset                     |
| `entry_price`| Average entry price                             |
| `mark_price` | Current market price                            |
| `pnl`        | Unrealized profit/loss                          |
| `liq`        | Liquidation price — dangerous if approached     |
| `leverage`   | Current leverage multiplier                     |

If `mark_price` is approaching `liq`, warn the user immediately.

---

## MARKET DATA COMMANDS

These are cross-exchange, public endpoints — no API key required.

### Hybrid Tickers (aggregated across all exchanges)
```
skill-trading market hybrid-tickers [OPTIONS]
```
Options:
- `--market-type spot|futures` — filter by market type (default: both)
- `--source <exchange>` — filter by exchange name (e.g. `binance`, `orderly`)
- `--symbol <SYM>` — filter by symbol (e.g. `NEARUSDT`)
- `--min-volume <USD>` — minimum 24h volume in USD
- `--min-price <price>` — minimum price filter
- `--max-price <price>` — maximum price filter
- `--up <pct>` — show only markets up ≥ N% today (e.g. `--up 5`)
- `--down <pct>` — show only markets down ≥ N% today

> **Note:** Do NOT pass `-e` / `--exchange` here — use `--source` to filter by exchange.
> The global `TTC_EXCHANGE` env var does not affect this command.

### Funding Rates
```
skill-trading market funding-rates [--symbol <SYM>]
```
Shows current funding rates across all exchanges for a symbol (or all symbols).

### Open Interest
```
skill-trading market open-interest [--symbol <SYM>]
```
Shows open interest in USD across all exchanges.

### Volume Snapshot
```
skill-trading market volume-snapshot
```
Shows 24h volume, open interest, and TVL per exchange (CEX + DEX).

### Scanner — Technical Analysis
```
skill-trading market scanner --symbol <SYM> [--timeframe 1h] [--bars 1000] [--swing-strength 10]
```
Runs fan analysis on a symbol. Returns:
- **Signal** — direction (LONG/SHORT), confidence, entry price, stop-loss, TP1/TP2/TP3, R/R ratio
- **Scoring** — long score, short score, preferred direction, active fan line per side
- **Fan lines** — all angle levels with current price, % distance, and time-to-reach for nearby lines

Parameters:
- `--timeframe` — kline interval: `1m`, `5m`, `15m`, `1h`, `4h`, `1d` (default: `1h`)
- `--bars` — number of bars to analyze, max 1000 (default: 1000)
- `--swing-strength` — lookback for swing detection (default: 10)

> **Usage tip:** Run this before opening a position to get an objective entry/exit framework.
> The signal includes a ready-to-use stop-loss and three take-profit targets.

### Tickers (exchange-specific)
```
skill-trading market tickers --symbol <SYM>
```
Shows ticker data for a specific exchange (requires `-e <exchange>`).

### Best Bid/Ask
```
skill-trading market best-bid-ask --symbol <SYM>
```
Shows the best bid and ask on a specific exchange (requires `-e <exchange>`).

---

## COMMON WORKFLOWS

### Scan for movers
```
skill-trading market hybrid-tickers --up 5 --min-volume 1000000
skill-trading market hybrid-tickers --down 5 --market-type futures
```

### Check funding rates for a symbol
```
skill-trading market funding-rates --symbol BTCUSDT
```

### Open a new position
1. `account balance` → verify available > 0
2. `position get` → check no conflicting positions
3. `market best-bid-ask --symbol <SYM>` → get current price
4. Calculate price and quantity
5. Show order summary → confirm with user
6. `order limit` or `order market`

### Close a position
1. `position get` → get symbol and size
2. `position close --symbol <SYM>` (uses market order)
   OR `order limit --sell --reduce-only` for a limit close

### Check account health
```
skill-trading account balance
skill-trading position get
```
Together these give a full picture of margin usage and risk.

---

## EXCHANGE DEFAULTS

- Default exchange is set in `config.toml` via `skill-trading config set-default <exchange>`
- Override per-command with `-e <exchange>`
- Credentials are loaded from `.env` using `{EXCHANGE}_API_KEY` / `{EXCHANGE}_API_SECRET` / `{EXCHANGE}_API_PASSPHRASE`
- For Orderly: passphrase = broker ID (e.g. `what_exchange`, `woofi_pro`, `ttc`)

---

## WHAT NOT TO DO

- Do not place an order immediately after being asked — always run the checklist first.
- Do not assume the account has funds — always verify.
- Do not guess the current price — always fetch it.
- Do not place duplicate orders without confirming with the user.
- Do not use market orders unless the user explicitly requests it — prefer limit orders.
