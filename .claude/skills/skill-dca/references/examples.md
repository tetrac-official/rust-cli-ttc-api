# DCA Ladder Examples

Worked scenarios for common use cases. Every example starts with `--dry-run` — always preview before going live.

## 1. Long accumulate — buy a dip in tranches

**Scenario:** NEAR at $1.20, you want to buy $150 total if it dips, 10 levels 1% apart.

```bash
skill-trading order dca -e orderly -s NEARUSDT --buy \
  --amount 150 -d 1 --dry-run
```

Output (approx):
```
DCA Ladder — NEARUSDT BUY on orderly
Amount:  $150.00 total  |  Levels: 10  |  Per level: $15.00
Base:    $1.2000  |  Step: 1.00% per level
  Level 1/10   $1.2000  Qty: 12
  Level 2/10   $1.1880  Qty: 12
  ...
  Level 10/10  $1.0966  Qty: 13
```

After confirming, re-run without `--dry-run`.

---

## 2. Short scale-in — sell into strength (exact level count)

**Scenario:** ETH at $2430, you want **exactly 4 SELL limits** of $25 each, starting +2% above price, spaced 0.5% apart.

Default `min_usd_entry = $15` means `order dca --amount 100` gives **6 levels** of $16.67 — not 4. Two fixes:

### Option A — tweak the amount and config
Edit `config.toml`:
```toml
[order]
min_usd_entry = 25
```
Then:
```bash
skill-trading order dca -e dydx -s ETHUSDT --sell \
  --amount 100 -d 0.5 \
  --start-price 2479.25 \
  --price-decimals 2 --qty-decimals 3 --dry-run
```

### Option B — manual limit calls (recommended for one-offs)
Compute rung prices yourself:

```
Base = 2430 × 1.02 = 2479.25
Step = 0.5% compound
L1 = 2479.25 × 1.0000 = 2479.25
L2 = 2479.25 × 1.0050 = 2491.65
L3 = 2479.25 × 1.0050² = 2504.10
L4 = 2479.25 × 1.0050³ = 2516.62
```

Then:
```bash
skill-trading order limit -e dydx -s ETHUSDT --sell --price 2479.25 --quantity 0.010
skill-trading order limit -e dydx -s ETHUSDT --sell --price 2491.65 --quantity 0.010
skill-trading order limit -e dydx -s ETHUSDT --sell --price 2504.10 --quantity 0.010
skill-trading order limit -e dydx -s ETHUSDT --sell --price 2516.62 --quantity 0.010
```

Verify with:
```bash
skill-trading order open -e dydx -s ETHUSDT
```

---

## 3. Scale-out of a long — reduce-only sell ladder

**Scenario:** You hold 100 SOL long @ $85. Price is at $90 and you want to trim $300 in 6 tranches as it climbs, 2% apart, keeping PnL if it reverses.

`order dca` has no `--reduce-only` flag, so this must be manual.

```
Start at $90.00, step up 2% each:
L1 = 90.00
L2 = 91.80
L3 = 93.64
L4 = 95.51
L5 = 97.42
L6 = 99.37

Per-level size: $300 / 6 = $50 notional
Qty per level ≈ 50 / price ≈ 0.55 SOL per rung (use --qty-decimals 2)
```

```bash
for PRICE in 90.00 91.80 93.64 95.51 97.42 99.37; do
  skill-trading order limit -e orderly -s SOLUSDT --sell \
    --price "$PRICE" --quantity 0.55 --reduce-only
done
```

Each rung only reduces — position can never flip short from this ladder.

---

## 4. Scale-out of a short — reduce-only BUY ladder

**Scenario:** You hold 10 RAVE short @ $9.43, mark has dropped to $7.50 and you want to book 50% of the position as it keeps falling.

```
Target: close 5 of 10 units across 5 rungs of 1 unit each, 3% apart, stepping DOWN.
L1 = 7.50
L2 = 7.28  (−3%)
L3 = 7.06
L4 = 6.85
L5 = 6.65
```

```bash
for PRICE in 7.50 7.28 7.06 6.85 6.65; do
  skill-trading order limit -e asterdex -s RAVEUSDT --buy \
    --price "$PRICE" --quantity 1 --reduce-only
done
```

---

## Rung-count reference table

For `order dca` with default `min_usd_entry = $15`:

| `--amount` | Levels (= floor(amount/15)) | Per-level $ |
|-----------:|----------------------------:|------------:|
| $30 | 2 | $15.00 |
| $45 | 3 | $15.00 |
| $60 | 4 | $15.00 |
| $90 | 6 | $15.00 |
| $100 | 6 | $16.67 |
| $150 | 10 | $15.00 |
| $200 | 13 | $15.38 |
| $500 | 33 | $15.15 |

If you need an exact level count with a specific per-level size, either (a) change `min_usd_entry` in `config.toml` or (b) place individual `order limit` calls.

---

## Tick-size cheat sheet (common exchanges)

| Exchange | ETHUSDT price tick | ETH qty step |
|----------|-------------------|--------------|
| orderly | $0.01 (`--price-decimals 2`) | 0.0001 (`--qty-decimals 4`) |
| dydx | $0.10 (`--price-decimals 1`) | 0.001 (`--qty-decimals 3`) |
| bybit | $0.01 (`--price-decimals 2`) | 0.001 (`--qty-decimals 3`) |
| asterdex / binance-family | varies per symbol — check `exchangeInfo` | varies |

If a rung is rejected with a tick-size error, tighten `--price-decimals` or bump the rung manually.
