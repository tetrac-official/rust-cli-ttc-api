---
name: skill-trading
description: Execute trading operations on TTC Box across 15+ exchanges. Use when the user wants to place orders, cancel orders, check balances, view positions, set leverage, or fetch market data (tickers, funding rates, open interest, scanner signals).
---

# skill-trading — Order Management Skill

This skill governs how an AI assistant should interact with the `skill-trading` CLI.
It exists to prevent hallucination and unsafe order execution.

## Reference Files

Load these on demand when deeper context is needed:

- `references/api-reference.md` — full TTC Box REST API: all methods, param shapes, response formats, supported exchanges, quirks
- `references/exchanges.md` — exchange names, credential setup, `ORDERLY_MAIN_WALLET_ADDRESS` guide
- `references/troubleshooting.md` — every error message with cause and fix

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
skill-trading orders get
```

- Always fetch live — never assume orders from a previous step still exist. They may have been filled, cancelled, or expired.
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
  skill-trading market best-bid-ask -e <exchange> --symbol <SYMBOL>
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

### Detailed PnL Breakdown

For a full breakdown of a position including margin used, % from entry, and liquidation distance:

```
skill-trading position pnl -e <exchange> -s <SYMBOL>
```

Output:
```
── NEARUSDT BUY 10x ─────────────────────────────────
Size:           1160 units  ($1356.62 notional)
Entry price:    $1.2425
Mark price:     $1.1695  (-5.88% from entry)
Unrealized PnL: -52.17 USDT  (-5.88%)
Margin used:    $135.66  (10x leverage, cross mode)
Liquidation:    $1.0816  (7.52% away)
```

Use `position pnl` when you need to assess risk (liquidation distance) or explain the position to the user in detail. Use `position get` for a quick summary.

---

## LEVERAGE MANAGEMENT

Leverage is set **per symbol**, on the exchange, via:

```
skill-trading account leverage -e <exchange> -s <SYMBOL> -l <N>
```

Example — set RAVEUSDT to 5× on asterdex:
```
skill-trading account leverage -e asterdex -s RAVEUSDT -l 5
```

### Critical rule — no open position on that symbol

**Exchanges only accept a leverage change when there is no open position on the symbol.** If a position (or even a resting reduce-only stop/TP for that symbol) is open, the call returns an error like `leverage not modified` or `position exists`.

To change leverage on a symbol you already hold:

1. `skill-trading position get -e <exchange>` — confirm the current position
2. `skill-trading position close -e <exchange> -s <SYMBOL>` (or wait for TP/SL to fill)
3. `skill-trading orders cancel-all -e <exchange> -s <SYMBOL>` — clear any resting reduce-only orders
4. `skill-trading account leverage -e <exchange> -s <SYMBOL> -l <N>` — now accepted
5. Re-enter the position at the new leverage

Open orders that are **not** on the same symbol do not block a leverage change.

### When to set leverage

- **Before the first order** on a new symbol — the exchange default may not match your risk sizing.
- **After flat** — after closing a position, if you want to re-enter at a different multiplier.
- **Not mid-position** — see rule above. The only way to "change leverage on an open position" is close → change → reopen, which realizes PnL and incurs fees.

### Pairing with `twap --leverage`

`skill-trading twap` accepts an optional `--leverage <N>` flag. It does two things:
1. Calls `setLeverage` on the exchange **before slice 1** (same underlying API as `account leverage`).
2. Uses the value to compute margin-required (`budget / leverage`) for balance checks.

Because step 1 runs before any order is placed, `twap --leverage` only succeeds if the symbol has no open position at the moment of the first tick. If you are adding to an existing position via TWAP, either omit `--leverage` (use whatever the exchange already has set) or close the position first.

### Verifying the current leverage

The leverage in effect for a symbol shows up in the `position pnl` output (`── NEARUSDT BUY 10x ──`). When flat, query the exchange via the TTC Box API (`getPositions` returns leverage even for size=0 on most exchanges), or simply set it explicitly before your next order.

### Per-exchange notes

- **Orderly** — cross margin only; leverage is account-wide, not per-symbol. Setting it changes margin requirements across all open symbols.
- **Bybit** — per-symbol; isolated vs cross is a separate setting (`account margin-mode`). Set margin mode before leverage.
- **OKX / Bitget / BloFin** — per-symbol, per-side in hedge mode. In hedge mode you may need to set long and short leverage separately.
- **Binance / asterdex** — per-symbol; valid range commonly 1–125× but capped by tier based on notional. High leverage on large size will be rejected.

If `account leverage` errors, check `references/troubleshooting.md` or run with `-v` for the raw API response.

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

Two modes: single symbol or watchlist scan.

#### Single symbol
```
skill-trading market scanner --symbol <SYM> [--timeframe 1h] [--bars 1000] [--swing-strength 10]
```
Full output: direction, confidence, Vola unit, momentum, stop, TP1-3, R/R, reasoning note.

```
NEARUSDT / 1h — LONG HIGH  (strength 79/100)
Entry:     $1.1940
Vola unit: $0.000558/bar (1x1)  |  Momentum: +0.000700/bar (flat)  |  Avg range: $0.014500/bar
Stop Loss: $1.1862  (0.65% risk)
TP1:       $1.4214  (+19.04%)
TP2:       $1.8918  (+58.44%)
TP3:       $2.3622  (+97.84%)
R/R:       29.14x
Note:      bull composite 79.0 (score 65, R/R 29.14) vs opposite 39.6
```

#### Watchlist scan
```
# Use config.toml [watchlist] symbols (default)
skill-trading market scanner

# Custom list
skill-trading market scanner --watchlist BTCUSDT,ETHUSDT,SOLUSDT,NEARUSDT

# Filters
skill-trading market scanner --only-high            # HIGH confidence only
skill-trading market scanner --min-rr 3.0           # R/R ≥ 3.0 only
skill-trading market scanner --timeframe 4h --only-high --min-rr 3.0
```

Output — compact table, one row per symbol:
```
  Scanner — 1h  │  3 symbols  │  filter: R/R ≥ 2.0
  ──────────────────────────────────────────────────────────────────────────────────────
  Symbol          Dir      Conf    Str         Entry            SL           TP1     R/R
  ──────────────────────────────────────────────────────────────────────────────────────
  NEARUSDT        LONG     HIGH     79       $1.1940       $1.1862       $1.4214   29.1x
  BTCUSDT         NEUTRAL  —
  ETHUSDT         LONG     MEDIUM   66      $2094.01      $2044.89      $2293.77    4.1x
  ──────────────────────────────────────────────────────────────────────────────────────
  2 signal(s) match  │  1 NEUTRAL  │  0 below filter
```

- Signals that don't meet the filter are shown dimmed with `filtered` tag — not hidden
- NEUTRAL signals are always shown (no levels to filter on)
- All scans run in parallel — 10-symbol watchlist takes the same time as 1

**Rules:**
- Use watchlist scan as the morning signal sweep before `brief`
- Only act on signals that pass your R/R threshold — don't lower the bar mid-session
- `--only-high` + `--min-rr 3.0` is the recommended filter for new entries
- Confirm with `brief` before acting: check portfolio health is not DANGER first

Parameters:
- `--timeframe` — `1m`, `5m`, `15m`, `1h`, `4h`, `1d` (default: `1h`)
- `--bars` — bars to analyze, max 1000 (default: 1000)
- `--swing-strength` — lookback for swing detection (default: 10)
- `--min-rr` — minimum R/R to show as a match (default: 2.0)
- `--only-high` — filter to HIGH confidence only

> **Vola fan note:** Descending fan lines from a high pivot can project below zero after many bars — this is mathematically correct, not a bug. Use the Vola unit and momentum to assess whether the move is realistic given the timeframe.

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

## DCA LADDER COMMAND

Places multiple limit orders stepping away from the current price. Levels and per-level size are auto-calculated from your total amount and the `min_usd_entry` in config (default $15).

```
skill-trading order dca -e <exchange> -s <SYMBOL> --buy|--sell --amount <USD> -d <distance%>
```

| Flag | Description |
|------|-------------|
| `--amount` | Total USD notional for the entire ladder |
| `-d, --distance` | % gap between each level (e.g. `1` = 1% per step) |
| `--start-price` | Override base price (defaults to current last price) |
| `--price-decimals` | Decimal places for limit prices (default: 4) |
| `--qty-decimals` | Decimal places for quantities (default: 0 = integer) |

**Calculation:**
```
levels     = floor(amount / min_usd_entry)
level_usd  = amount / levels
price_n    = base_price × (1 - distance%)^n   (buy: steps down)
           = base_price × (1 + distance%)^n   (sell: steps up)
qty_n      = floor(level_usd / price_n)
```

**Example — $150 across 10 levels, 1% apart:**
```
skill-trading order dca -e orderly -s NEARUSDT --buy --amount 150 -d 1 --dry-run

  DCA Ladder — NEARUSDT BUY on orderly
  Amount:  $150.00 total  |  Levels: 10  |  Per level: $15.00
  Base:    $1.1667  |  Step: 1.00% per level  |  Min entry: $15.00
  ─────────────────────────────────────────────────────
  Level 1/10   $1.1667  Qty: 12  Cost: ~$15.00
  Level 2/10   $1.1550  Qty: 12  Cost: ~$15.00
  ...
  Level 10/10  $1.0658  Qty: 14  Cost: ~$15.00
```

**Rules:**
- Always `--dry-run` first to confirm levels and prices before going live
- Check available balance — all levels are placed as open limit orders, locking margin
- Use `orders get` after placing to confirm all levels were accepted
- Cancel with `orders cancel-all` if you want to clear the ladder

---

## RISK MANAGEMENT COMMANDS

### Stop Loss / Take Profit (one-shot)
```
skill-trading risk sl -e <exchange> -s <SYMBOL> --stop-price <price>
skill-trading risk tp -e <exchange> -s <SYMBOL> --tp-price <price>
```
Places a single stop/TP order against the current open position. Reduce-only, triggered by mark price.

### Trailing Stop Watch (polling loop)
```
skill-trading risk trail-watch -e <exchange> -s <SYMBOL> --trail-pct <pct> --interval <seconds>
```
Runs a foreground loop that:
1. **Waits** until the position enters profit (PnL > 0)
2. **Activates** — records peak price, places first stop at `peak × (1 - trail_pct%)`
3. **Trails** — each poll, if price sets a new peak, cancels old stop and places a new one
4. **Exits** automatically when position closes (and removes the progress file)

Default: `--trail-pct 2.0`, `--interval 30`. Press `Ctrl+C` to stop.

**Progress file:** writes JSON state to `~/.trail-watch-{symbol}-{exchange}.json` on every tick. An agent can read this file on demand to check trail-watch status without interrupting the loop.

| Field | Description |
|-------|-------------|
| `active` | `false` while waiting for profit, `true` once trailing |
| `mark_price`, `entry_price` | Current prices |
| `peak_price` | Tracked peak (`null` while inactive) |
| `current_stop`, `stop_order_id` | Active stop level and order ID (`null` if none placed) |
| `unrealized_pnl`, `position_size` | Position state |
| `updated_at` | ISO 8601 timestamp of last tick |

> Use this after entering a position — it watches passively and only activates once you're in profit.

---

## MORNING BRIEF

Run once at the start of every session to get a full picture before touching anything:

```bash
skill-trading brief -e orderly
# aliases: morning, mb
```

Single command. Runs all fetches concurrently and presents them in one report:

| Section | Data |
|---------|------|
| Session | Token validity + time remaining |
| Watchlist Prices | Price, 24h%, volume, OI, funding rate per symbol |
| Signals | Scanner direction, confidence, entry, SL, TP1, R/R per symbol |
| Portfolio | Balance, utilization, all positions with PnL and liq distance |
| Open Orders | All live orders with price and qty |
| Overall | READY / WATCH / DANGER banner |

**Override watchlist on the fly:**
```bash
skill-trading brief -e orderly --watchlist SOLUSDT,BTCUSDT,NEARUSDT
```

**Change signal timeframe:**
```bash
skill-trading brief -e orderly --timeframe 4h
```

**Watchlist** defaults come from `config.toml`:
```toml
[watchlist]
symbols = ["NEARUSDT", "BTCUSDT", "ETHUSDT"]
```

**Rules:**
- Always run `brief` before starting a new session's trading
- If OVERALL is DANGER, address the at-risk position before any new orders
- The brief takes 2–5 seconds (all fetches run in parallel)

---

## PRE-LOOP SESSION CHECK

Before starting any loop (`/loop`, `twap`, `trail-watch`, `portfolio summary` cycle), always verify the session is healthy:

```bash
skill-trading status
```

Output:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  SKILL-TRADING STATUS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ✓  TTC Box API
  ✓  Session token               VALID  23h 43m remaining
  ✓  Exchange credentials        orderly configured

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  STATUS: READY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**Decision tree:**

| Status | Action |
|--------|--------|
| READY | Proceed with loop or order placement |
| NOT READY — session expired | Run `skill-trading login` first |
| NOT READY — no credentials | Set `{EXCHANGE}_API_KEY` / `{EXCHANGE}_API_SECRET` in `.env` |
| NOT READY — API unreachable | Check connectivity; retry in 30s |

**Rules:**
- If `STATUS: NOT READY`, do not start any loop or place any order.
- Pay attention to the remaining time on the session token. If < 2h remain, warn the user to re-login soon.
- Exit code is 0 when READY, 1 when NOT READY — usable in shell scripts as a gate.

---

## WHAT NOT TO DO

- Do not place an order immediately after being asked — always run the checklist first.
- Do not assume the account has funds — always verify.
- Do not guess the current price — always fetch it.
- Do not place duplicate orders without confirming with the user.
- Do not use market orders unless the user explicitly requests it — prefer limit orders.
- **Do not assume orders still exist** — always call `orders get` before referencing open orders. Orders may have been filled, cancelled, or expired since they were last placed.
- **Do not assume positions are unchanged** — always call `position get` for the current state before making decisions based on a position.
- **Do not assume balance is the same** — always re-fetch before placing new orders, especially after fills or PnL changes.
