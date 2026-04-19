---
name: skill-dca
description: Build or trim a position using a DCA (dollar-cost-average) limit-order ladder. Use when the user wants to layer multiple limit orders at stepped prices to improve average entry — scale into longs at better dip prices, scale into shorts at better rip prices, or scale out of either side in tranches.
---

# skill-dca — DCA Ladder Skill

This skill governs how an AI assistant should use the `skill-trading` CLI to place layered limit-order ladders that improve average entry (or average exit) on a position.

## Reference Files

- `references/examples.md` — worked examples for long accumulate, short scale-in, and reduce-only scale-out

---

## WHAT IS A DCA LADDER

A DCA ladder places **N limit orders at stepped prices** instead of one large order at a single price. Two flavours:

| Flavour | Side | Step direction | Purpose |
|---------|------|----------------|---------|
| **Long accumulate** | BUY | steps **down** from current | Average into a long at better dip prices. Some levels may not fill — that is fine. |
| **Short scale-in** | SELL | steps **up** from current | Average into a short at better rip prices (sell into strength). Some levels may not fill. |
| **Scale-out long** (reduce-only) | SELL | steps **up** from current | Book profits on an existing long in tranches as price rises. |
| **Scale-out short** (reduce-only) | BUY | steps **down** from current | Book profits on an existing short in tranches as price falls. |

**Key principle:** a ladder trades *fill certainty* for *better average price*. Not every level needs to fill — partial fills are still a valid outcome.

---

## COMMAND

There are two ways to place a ladder. Pick by how much control you need.

### 1. Automated — `order dca` (fastest, level count is derived)

```
skill-trading order dca -e <exchange> -s <SYMBOL> --buy|--sell --amount <USD> -d <distance%> [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-e, --exchange` | Exchange name (e.g. `orderly`, `dydx`, `asterdex`) |
| `-s, --symbol` | Trading symbol (e.g. `ETHUSDT`) |
| `--buy` / `--sell` | Side. BUY ladders step down; SELL ladders step up. |
| `--amount` | Total USD notional across **all** levels |
| `-d, --distance` | % gap between each level (compound, not linear) |
| `--start-price` | Override base (defaults to current last price) |
| `--price-decimals` | Price rounding (default `4`; use `1` for ETH $0.10 tick, `0` for $1 tick, `-1` not supported — round `--start-price` manually instead) |
| `--qty-decimals` | Quantity rounding (default `0` = integer like NEAR; use `3` for ETH/BTC) |
| `--dry-run` | Preview without placing anything. **Always run first.** |

#### Calculation

```
levels     = floor(amount / min_usd_entry)       # min_usd_entry defaults to $15 in config
level_usd  = amount / levels
price_n    = base_price × (1 − distance%)^(n−1)   # BUY: steps down
           = base_price × (1 + distance%)^(n−1)   # SELL: steps up
qty_n      = floor(level_usd / price_n, qty_decimals)
```

**Caveat — level count is derived, not chosen.** `order dca` has **no `--levels` flag**. The number of levels is always `floor(amount / min_usd_entry)`. If the user asks for exactly N levels, you must either:

- **Option A:** adjust the amount so `amount / min_usd_entry` rounds to N. Example: 4 levels of $25 each → set `--amount 100` *and* edit `config.toml` so `min_usd_entry = 25` (or pick an amount like `$60` with default `$15` min).
- **Option B:** use method 2 below (individual `order limit` calls) — always works, 4 calls, no magic.

### 2. Manual — N individual `order limit` calls (precise control)

Use when:
- The exact level count doesn't fit the `floor(amount / 15)` formula
- You need `--reduce-only` on every rung (scale-out ladders)
- You need different sizes per rung (weighted ladder)

Compute prices yourself, then place each rung:

```
skill-trading order limit -e <exchange> -s <SYMBOL> --buy|--sell \
  --price <PRICE> --quantity <QTY> [--reduce-only]
```

---

## PRE-LADDER CHECKLIST

Before placing **any** ladder, run these in order. Do not skip.

### 1. Check balance
```
skill-trading account balance -e <exchange>
```
- For **BUY** ladders: `available ≥ amount` (every rung locks margin immediately on placement).
- For **SELL scale-in** on a short: same — every rung locks margin to back potential added exposure.
- For **reduce-only** ladders: no margin impact — reduce-only orders don't lock extra margin.

### 2. Check existing position
```
skill-trading position get -e <exchange>
```
- If the user already holds a position on the symbol, determine intent:
  - Same-direction ladder? → **scaling in** (increases exposure if filled)
  - Opposite-direction ladder **without reduce-only**? → dangerous: flips side as fills land. Usually wrong. Confirm intent.
  - Opposite-direction **with reduce-only**? → **scaling out** (books PnL in tranches). Safe.
- Always confirm with the user whether the ladder is meant to **add to** or **exit** the position.

### 3. Check open orders on the symbol
```
skill-trading order open -e <exchange>
```
Flag any existing orders that would collide with the ladder. A prior limit at one of the ladder levels is a duplicate; confirm whether to cancel it or step around it.

### 4. Fetch current price
```
skill-trading market tickers -e <exchange> --symbol <SYM>
```
`market best-bid-ask` is preferred but may fail on some exchanges (dydx has known deserialization bugs) — use `market tickers` as a fallback and read the last price.

### 5. Dry-run the plan
```
skill-trading order dca ... --dry-run
```
Or for manual ladders, list each rung (price, qty, side, reduce-only?) in a table and show the user. **Never place rungs without explicit user approval** of the full plan.

---

## SHOWING THE PLAN TO THE USER

Always present the ladder as a table before placing. Include the **impact on the current position** if fills land:

```
LADDER — ETHUSDT SELL on dydx
Current:  ETHUSDT last $2430.64  |  Existing position: 0.054 short @ $2436.04
Base:     $2479.25 (+2.00% above last)  |  Step: 0.25% per level
Rung size: 0.010 ETH (~$24.80 each)  |  Reduce-only: NO

  L1  $2479.25  0.010 ETH  (+2.00%)
  L2  $2485.45  0.010 ETH  (+2.26%)
  L3  $2491.66  0.010 ETH  (+2.51%)
  L4  $2497.89  0.010 ETH  (+2.77%)

Total notional: $99.54  |  Total qty: 0.040 ETH
If all rungs fill: position becomes 0.094 short @ ~$2462.00 blended.
```

Then ask: *"Confirm to place these 4 orders sequentially?"* — and wait.

---

## EXAMPLE USAGE

### Accumulate $150 of NEAR across 10 levels, 1% apart, dipping down (BUY)
```
skill-trading order dca -e orderly -s NEARUSDT --buy --amount 150 -d 1 --dry-run
```

### Short scale-in: sell $100 of ETH starting 2% above price, 0.5% apart (SELL, 4 rungs)
```
# Method A — auto-derive levels (this gives 6 rungs of ~$16.67 each with default min $15)
skill-trading order dca -e dydx -s ETHUSDT --sell \
  --amount 100 -d 0.5 \
  --start-price 2479.25 \
  --price-decimals 2 --qty-decimals 3 --dry-run

# Method B — exactly 4 rungs of $25, placed manually
skill-trading order limit -e dydx -s ETHUSDT --sell --price 2479.25 --quantity 0.010
skill-trading order limit -e dydx -s ETHUSDT --sell --price 2491.64 --quantity 0.010
skill-trading order limit -e dydx -s ETHUSDT --sell --price 2504.10 --quantity 0.010
skill-trading order limit -e dydx -s ETHUSDT --sell --price 2516.62 --quantity 0.010
```

### Scale-out of an existing long: sell $300 of SOL in 6 tranches, 2% apart, reduce-only
```
# Manual is required here — order dca does not take --reduce-only
skill-trading order limit -e orderly -s SOLUSDT --sell --price 90.00 --quantity 0.5 --reduce-only
skill-trading order limit -e orderly -s SOLUSDT --sell --price 91.80 --quantity 0.5 --reduce-only
# ... repeat for all rungs
```

---

## LONG vs SHORT — IMPROVING AVERAGE ENTRY

DCA math works identically in both directions but the intent differs:

### Long (BUY ladder stepping down)
- You believe the asset will rise **eventually**, but the path is down first.
- Fills land on dips → **lower blended entry** → higher PnL when price recovers.
- Risk: if price keeps dropping, you end up fully loaded near a continued downtrend.
- Mitigation: set a final invalidation level below the lowest rung as a hard stop.

### Short (SELL ladder stepping up)
- You believe the asset will fall **eventually**, but it is ripping up now.
- Fills land on rips → **higher blended entry for the short** → better PnL when price falls.
- Risk: if price keeps rising, every fill deepens an underwater short.
- Mitigation: set a final invalidation level above the highest rung as a hard stop.
- **Reality check:** a losing short that keeps scaling up has no ceiling. A budget-capped ladder (fixed total notional) is critical.

### Scale-out (reduce-only)
- Book profits in tranches as price moves in your favour.
- Reduce-only guarantees each rung only **reduces** the position — never flips it.
- Benefit: you don't need to be right about the top/bottom — you capture the move in pieces.

---

## RULES

- **Always `--dry-run` first** and show the full ladder table to the user before placing anything.
- **Cap total notional.** The `--amount` flag is your risk budget — once it's spent, the ladder stops adding.
- **Match step to volatility.** On quiet majors (BTC/ETH) a 0.25% step is tight; on meme alts a 2% step may not even clear the noise. Check the scanner's `Avg range/bar` on the symbol's timeframe.
- **Tick-size aware.** If `--price-decimals` rounds two adjacent rungs to the same price, they'll collide (one may be rejected as a duplicate, or both sit at the same level wasting a rung). Widen the step or use tighter decimals.
- **Confirm reduce-only intent.** A SELL ladder on an existing long that is *not* `--reduce-only` flips the position once enough size sells. This is almost always a mistake.
- **Do not run two ladders on the same symbol.** They compete for margin and confuse the fill book.
- **Cancel cleanly.** `skill-trading order cancel-all -e <exchange> -s <SYMBOL>` wipes the whole ladder. Use before adjusting a plan mid-flight.

---

## WHAT NOT TO DO

- Do not place a ladder without running the pre-ladder checklist.
- Do not use `order dca` when you need exactly N rungs unless `amount / min_usd_entry` cleanly equals N — use individual `order limit` calls instead.
- Do not use `order dca` for scale-out — it has no `--reduce-only` flag. Scale-out ladders must be manual.
- Do not assume a fill. `order open` is the source of truth after placement — always refresh before referencing levels.
- Do not scale a short up indefinitely. If price keeps running against you, cut. A ladder is not a thesis — it is an execution technique.

---

## AFTER PLACING A LADDER

1. `skill-trading order open -e <exchange> -s <SYMBOL>` — confirm every rung was accepted.
2. Tell the user which rungs made it in and which (if any) were rejected (usually price-tick or min-qty issues).
3. Offer to set a paired invalidation stop (for scale-in ladders) via `skill-trading risk sl`.
4. If the ladder was a scale-in, remind the user that filled rungs will raise their liquidation distance on each fill — monitor.
