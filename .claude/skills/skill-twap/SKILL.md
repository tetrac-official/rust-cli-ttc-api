---
name: skill-twap
description: Build a position using Time-Weighted Average Price (TWAP) execution. Splits a USD budget into equal slices and places market orders at regular intervals over a specified duration. Use when the user wants to accumulate or distribute a position gradually without moving the market.
---

# skill-twap — TWAP Position Builder

This skill governs how an AI assistant should use the `skill-trading` CLI to execute TWAP orders. TWAP spreads a fixed USD budget across equal time intervals to achieve an average entry price and reduce market impact.

## Reference Files

- `references/examples.md` — worked examples for common TWAP scenarios

---

## WHAT IS TWAP

TWAP (Time-Weighted Average Price) breaks a large order into smaller slices executed at regular intervals. For example:

- **$1000 over 24 hours** → 48 orders of $20.83 every 30 minutes
- **$500 over 4 hours** → 8 orders of $62.50 every 30 minutes
- **$200 over 1 hour** → 4 orders of $50 every 15 minutes (using `--interval 15`)

Each slice fetches the current price and places a market order for the calculated quantity. The result is an average entry price spread across the full duration.

---

## COMMAND

```
skill-trading twap -e <exchange> -s <SYMBOL> --buy|--sell --budget <USD> --hours <H> [OPTIONS]
```

### Required Flags

| Flag | Description |
|------|-------------|
| `-e, --exchange` | Exchange name (e.g. `orderly`, `bybit`) |
| `-s, --symbol` | Trading symbol (e.g. `NEARUSDT`) |
| `--buy` or `--sell` | Order direction |
| `--budget` | Total USD to deploy |
| `--hours` | Duration in hours |

### Optional Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--interval` | `30` | Minutes between each slice |
| `--slices` | auto | Override slice count (recalculates interval from hours) |
| `--decimals` | `0` | Quantity decimal precision (`0` = integer like NEAR; `3` = BTC/ETH style). If too low, slice qty rounds to zero and every slice is skipped. |
| `--leverage` | unset | Sets leverage on the exchange before slice 1 and uses it to compute required margin as `budget / leverage`. See skill-trading SKILL.md § "LEVERAGE MANAGEMENT" — only accepted when the symbol has no open position at the moment of the first tick. |
| `--resume` | off | Resume a previous TWAP from saved state after a crash |
| `--dry-run` | off | Preview plan without placing any orders |

### Auto-Calculation

When `--slices` is not specified:
```
slices   = ceil(hours × 60 / interval_mins)
slice_usd = budget / slices
interval_secs = interval_mins × 60
```

When `--slices` is specified, the interval is recalculated:
```
interval_secs = (hours × 3600) / slices
```

---

## PRE-TWAP CHECKLIST

Before starting a TWAP, always:

### 1. Check available balance
```
skill-trading account balance -e <exchange>
```
Verify `Available ≥ budget`. TWAP does not pre-lock margin — it places orders one at a time, so available balance must remain sufficient throughout the run.

### 2. Preview the plan with --dry-run
```
skill-trading twap -e orderly -s NEARUSDT --buy --budget 1000 --hours 24 --dry-run
```
Confirm slices, slice size, and interval with the user before running live.

### 3. Confirm with the user
Show the plan summary and get explicit approval:
```
TWAP — NEARUSDT BUY on orderly
Budget:   $1000.00 over 24.0h
Slices:   48 orders × $20.83 each
Interval: 30 min between orders
```

---

## EXAMPLE USAGE

### Accumulate $1000 of NEAR over 24 hours (default 30min interval)
```
skill-trading twap -e orderly -s NEARUSDT --buy --budget 1000 --hours 24
```

### Distribute $500 of SOL over 4 hours, every 15 minutes
```
skill-trading twap -e orderly -s SOLUSDT --sell --budget 500 --hours 4 --interval 15
```

### 10 equal slices over 2 hours (custom slice count)
```
skill-trading twap -e orderly -s BTCUSDT --buy --budget 300 --hours 2 --slices 10
```

### Dry-run first (always recommended)
```
skill-trading twap -e orderly -s NEARUSDT --buy --budget 1000 --hours 24 --dry-run
```

---

## READING TWAP OUTPUT

```
  TWAP — NEARUSDT BUY on orderly
  Budget:   $1000.00 over 24.0h
  Slices:   48 orders × $20.83 each
  Interval: 30 min between orders
  ─────────────────────────────────────────────────────

  [1/48]  Price: $1.1615  Qty: 17.9  Order: 20975270001  [deployed: $20.83 / $1000.00]
          Next order in 30m 0s...
  [2/48]  Price: $1.1580  Qty: 17.9  Order: 20975270002  [deployed: $41.66 / $1000.00]
          Next order in 30m 0s...
  ...
  ─────────────────────────────────────────────────────
  TWAP complete — 48 / 48 slices filled
  Total deployed: $1000.00  |  Total qty: 860.2  |  Avg price: $1.1625
```

| Field | Meaning |
|-------|---------|
| `[i/n]` | Current slice number out of total |
| `Price` | Market price at time of order |
| `Qty` | Units purchased/sold in this slice |
| `Order` | Exchange order ID |
| `deployed` | USD spent so far vs total budget |
| `Avg price` | Final summary: total_deployed / total_qty |

---

## IMPORTANT NOTES

- **TWAP runs in the foreground** — keep the terminal open for the full duration. Press `Ctrl+C` to stop early. Partial runs are valid — slices already executed are filled.
- **Orders are market orders** — each slice executes at the current best price. No partial fills or timeout logic — the exchange fills it immediately.
- **Balance is not pre-locked** — if balance drops mid-run (e.g. other positions are closed or liquidated), later slices may fail. Monitor with `account balance` in another terminal if concerned.
- **Skipped slices are logged** — if a slice fails (e.g. price fetch error, zero qty, API error), it is skipped and the next slice runs on schedule.
- **Use `--dry-run` first** — always preview the plan before committing a large budget.

---

## WHAT NOT TO DO

- Do not start a TWAP without running `--dry-run` first and confirming the plan with the user.
- Do not start a TWAP if `available balance < budget`.
- Do not run two TWAPs on the same symbol simultaneously — they will race each other for fills.
- Do not use TWAP for very small budgets with many slices — if `slice_usd / price` rounds to 0, slices will be skipped. Ensure each slice is above the exchange minimum order size (typically $5-$10).
