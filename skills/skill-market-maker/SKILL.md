---
name: skill-market-maker
description: Automated limit-order market-making loop. Use when the user wants to earn the spread by repeatedly entering at best bid/ask and exiting at a fixed price offset. Requires skill-trading to be installed and the exchange to support limit orders with low or zero maker fees.
---

# skill-market-maker — Limit-Order Spread Capture

skill-market-maker runs an automated loop that enters at the current best bid (or best ask) and exits at a configurable spread once the entry fills. It is designed for exchanges with zero or near-zero limit-order fees. The default spread is **0.1% of entry price**, computed fresh each round so it scales automatically with market price.

---

## HOW IT WORKS

Each round consists of exactly two fills:

```
Round N:
  1. Fetch best bid/ask
  2. Place limit BUY at best bid  (or SELL at best ask)
  3. Wait for entry fill (poll every poll_ms)
  4. Compute spread for this round: entry_price × spread_pct / 100
  5. Place limit SELL at entry + spread  (or BUY at entry − spread), reduce_only
  6. Wait for exit fill
  7. Report: gross PnL, commission cost, net PnL, cumulative total
  8. Repeat
```

The loop runs for a fixed number of rounds (`--rounds`) or indefinitely until Ctrl-C (`--rounds 0`).

### Position change handling

If the position size or entry price changes mid-loop (e.g., a manual DCA buy fills), the exit order is now wrong. The correct response is:

1. Cancel the stale exit order
2. Place a new exit at `new_entry_price + spread`, `quantity = new_pos_size`, `reduce_only`

When running the loop as an agent-controlled cron job, the state machine must check on every tick:
- `sell_order_qty == pos_size`
- `sell_order_price == pos_entry + spread`

If either fails → **Case B**: cancel and replace the exit order.

---

## COMMAND REFERENCE

```bash
skill-trading market-maker \
  -e <exchange> \
  -s <SYMBOL> \
  --buy | --sell \
  -q <quantity> \
  [--spread-pct <pct>]      # % of entry price (default 0.1 = 0.1%), overrides --spread
  [--spread <price_offset>] # absolute offset, used only when --spread-pct is not set
  [--rounds <N>]            # 0 = infinite (default)
  [--price-decimals <N>]    # default 4
  [--qty-decimals <N>]      # default 0
  [--poll-ms <N>]           # fill poll interval, default 500ms
  [--timeout-secs <N>]      # cancel entry if unfilled after N sec, default 60
  [--dry-run]               # preview without placing orders
```

Alias: `skill-trading mm`

### Key flags

| Flag | Default | Purpose |
|------|---------|---------|
| `--spread-pct` | 0.1 | Exit spread as % of entry price (0.1 = 0.1%). Takes precedence over `--spread`. |
| `--spread` | — | Absolute price offset (ignored when `--spread-pct` is set) |
| `--poll-ms` | 500 | How often (ms) to poll for order fill |
| `--timeout-secs` | 60 | Cancel unfilled entry after N seconds; exit gets 3× longer |
| `--rounds` | 0 | 0 = run until Ctrl-C |
| `--dry-run` | off | Shows projected entry/exit prices and PnL without placing orders |

---

## CONFIGURATION

```toml
[market-maker]
limit_order_commission = 0.00   # 0% maker fee (Orderly Network). Adjust for your exchange.
min_spread = 0.001              # 0.1% minimum spread floor (fraction, not percent)
```

**`limit_order_commission`** — fee rate per side (applied twice per round). Affects only net PnL display; the exchange enforces its own fees independently.

**`min_spread`** — stored as a fraction (0.001 = 0.1%). On every round, the effective spread is:

```
effective_spread = max(requested_spread, entry_price × min_spread)
```

This guarantees that even when `--spread` or `--spread-pct` would produce a sub-minimum value, the exit order is never placed closer than `min_spread %` to entry. At BTC $68,000 with `min_spread = 0.001`, the floor is **$68.00** per round.

---

## PRE-FLIGHT CHECKLIST

Before starting any live market-maker run:

```bash
# 1. Confirm session is valid and credentials are present
skill-trading status

# 2. Check available balance
skill-trading account balance -e <exchange>

# 3. Dry-run to preview prices and expected PnL
skill-trading mm -e <exchange> -s <SYMBOL> --buy -q <qty> --spread-pct 0.1 --rounds 5 --dry-run

# 4. Check existing positions and open orders (avoid conflicts)
skill-trading position get -e <exchange> -s <SYMBOL>
skill-trading order open -e <exchange> -s <SYMBOL>
```

---

## PROFITABILITY CALCULATION

**Effective spread per round** = `max(requested_spread, entry_price × min_spread)`

**Gross PnL per round** = `effective_spread × quantity`

**Commission cost per round** = `2 × limit_order_commission × entry_price × quantity`

**Net PnL per round** = `gross − commission_cost`

**Break-even spread %** = `2 × limit_order_commission × 100`

### Example — zero-fee exchange, 0.1% spread (Orderly Network)

```
Symbol:       BTCUSDT
Quantity:     0.001
Spread %:     0.1%
Entry:        $68,000 (best bid)
Spread $:     $68,000 × 0.001 = $68.00
Exit:         $68,068.0

Gross/round:  $68.00 × 0.001 = $0.068
Commission:   $0.00 × 2      = $0.000
Net/round:    $0.068

100 rounds:   $6.80 net profit
```

### Example — 0.05% maker fee exchange

```
Break-even spread % = 2 × 0.0005 × 100 = 0.10%
→ Use --spread-pct 0.15 or higher to clear fees
```

---

## SIZING GUIDELINES

- **Quantity**: Start small — 5–20 contracts for NEAR-class assets, 0.001–0.01 for BTC. Scale up only after confirming consistent fills.
- **Spread %**: 0.1% works well for liquid assets on zero-fee exchanges. Increase to 0.15–0.2% on fee-bearing exchanges or in thin markets.
- **Timeout**: If the market is thin, increase `--timeout-secs`. If the market is fast, tighten it to 30s to avoid stale fills.

---

## EXAMPLE RUNS

### Standard run — buy side, 0.1% spread (default)

```bash
skill-trading mm -e orderly -s NEARUSDT --buy -q 10 --price-decimals 4 --rounds 20
```

### BTC with 0.1% spread, 1 decimal price

```bash
skill-trading mm -e orderly -s BTCUSDT --buy -q 0.001 --price-decimals 1 --qty-decimals 3
```

### Sell-side (fading rallies)

```bash
skill-trading mm -e orderly -s BTCUSDT --sell -q 0.001 --price-decimals 1 --qty-decimals 3
```

### Explicit percentage spread

```bash
skill-trading mm -e orderly -s NEARUSDT --buy -q 10 --spread-pct 0.15 --price-decimals 4
```

### Absolute spread (legacy)

```bash
skill-trading mm -e orderly -s NEARUSDT --buy -q 10 --spread 0.002 --price-decimals 4
```

### Infinite loop until stopped

```bash
skill-trading mm -e orderly -s NEARUSDT --buy -q 10
# Stop with Ctrl-C — final summary prints automatically
```

### Fast market — tighter polling

```bash
skill-trading mm -e orderly -s BTCUSDT --buy -q 0.001 --poll-ms 200 --timeout-secs 30 --price-decimals 1 --qty-decimals 3
```

---

## READING THE OUTPUT

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  MARKET MAKER  BTCUSDT  BUY  qty: 0.001  spread: 0.100%
  Exchange: orderly  Commission: 0.0000%/side  Poll: 500ms  Timeout: 60s
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  [R1]  BUY    Entry @ $68000.0  qty: 0.001  order: 209754950
  [R1]  ✓      Entry filled @ $68000.0
  [R1]  SELL   Exit @ $68068.0  order: 209754951
  [R1]  ✓ ROUND COMPLETE  gross: $0.0680  fee: $0.0000  net: $0.0680  cumulative: $0.0680

  [R2]  BUY    Entry @ $67995.0  qty: 0.001  order: 209754961
  ...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Market maker stopped  |  20 rounds  |  Total net PnL: $1.3600
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## AGENT-CONTROLLED MONITORING LOOP

When the CLI's internal loop stops (e.g., exit order times out), run an agent-controlled state machine via `CronCreate` to keep the exit order alive independently.

### Spread calculation

The agent loop always computes the exit price using `min_spread` from `config.toml`:

```
spread_dollars = round(pos_entry × min_spread, price_decimals)
exit_price     = pos_entry + spread_dollars          # buy side
exit_price     = pos_entry - spread_dollars          # sell side
```

Example — BTC @ $68,154.4, `min_spread = 0.001` (0.1%), `price_decimals = 1`:
```
spread_dollars = 68154.4 × 0.001 = $68.2
exit_price     = 68154.4 + 68.2 = $68,222.6
```

This gives **$0.068 gross per round** on 0.001 BTC vs $0.001 with a $1 flat spread — 68× more profit per round.

### State machine (run every minute)

**Step 1** — `skill-trading order open -e <exchange> -s <SYMBOL>`

**Step 2** — `skill-trading position get -e <exchange> -s <SYMBOL>`

**Step 3** — Parse:
- `pos_size`, `pos_entry`, `sell_order_id`, `sell_order_price`, `sell_order_qty`, `has_buy_order`
- Compute: `spread = round(pos_entry × min_spread, price_decimals)`
- Compute: `expected_exit = pos_entry + spread`  (or `pos_entry - spread` for sell-side)

**Step 4** — Cases:

| Case | Condition | Action |
|------|-----------|--------|
| A | `pos_size > 0` AND sell in sync (`sell_order_qty == pos_size` AND `sell_order_price == expected_exit`) | Wait — print status |
| B | `pos_size > 0` AND sell missing/wrong size/wrong price | Cancel old sell → place new SELL at `expected_exit`, `qty = pos_size`, `reduce_only` |
| C | `pos_size == 0` AND no orders | Place new entry BUY at best bid |
| D | `pos_size == 0` AND buy order pending | Wait |
| E | `pos_size == 0` AND orphaned sell | Cancel all → restart next tick |

**Case B** handles mid-loop DCA buys or any position change: it automatically re-sizes and re-prices the exit order to match the new position and the current `min_spread` floor.

### Sync check (Case A condition)

Because spread is computed from entry price and rounded, always compare with a small tolerance or recompute:

```
expected_exit = round(pos_entry + pos_entry × min_spread, price_decimals)
in_sync = (sell_order_qty == pos_size) AND (sell_order_price == expected_exit)
```

---

## RISKS AND RULES

**Inventory risk** — If entry fills but exit never fills (market moved away), the position remains open. The loop stops automatically and warns you. Switch to the agent-controlled monitoring loop (above) to keep the exit alive.

**Spread risk** — In fast markets, the best bid can move more than the spread between entry and exit. The exit order may sit behind the market. The 0.1% default gives more room than a fixed-tick spread.

**Position change risk** — If a manual DCA or another strategy fills on the same symbol mid-loop, the exit order is now wrong size/price. Case B in the monitoring loop handles this automatically by cancelling and replacing.

**Do not run simultaneously with another strategy on the same symbol** — market-maker opens and closes positions. If a TWAP, trail-watch, or existing position is active on the same symbol, the reduce-only exit orders will conflict.

**Always run `--dry-run` first** on a new symbol to confirm price decimals and spread math are correct.

### Hard rules

- **Never skip `skill-trading status`** before starting a live run
- **Never use `--spread-pct` below your exchange's break-even level** — check: `break-even % = 2 × maker_fee_pct`
- **Always confirm exit order ID is printed** — if exit placement fails, the position is unprotected. The loop will stop and warn; handle it immediately
- **Set `--timeout-secs` to a non-zero value** — waiting forever for an entry that never fills ties up capital and blocks the loop

---

## WHEN NOT TO USE MARKET MAKER

- **High volatility environments** — during news events, funding resets, or liquidation cascades, the spread capture is overwhelmed by adverse price moves. Pause the loop.
- **Trending markets** — in a strong trend, the buy-side entry fills quickly but the sell-side exit lags or never fills at the narrow spread. Use `skill-shark` for directional trades instead.
- **Exchanges with taker fees on limit orders** — some exchanges classify aggressively-priced limit orders as taker. Verify your fee tier before running.
- **Illiquid symbols** — if the bid/ask spread is already wider than your `--spread-pct` equivalent, the exit order will be behind the market immediately after entry. Check the best bid/ask first: `skill-trading market best-bid-ask -e <exchange> --symbol <SYMBOL>`
