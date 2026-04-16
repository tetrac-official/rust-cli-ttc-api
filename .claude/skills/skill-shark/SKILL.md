---
name: skill-shark
description: Signal-driven trade setup strategy. Use when the user wants to find a trade, scan a market for an entry, get analysis, or automatically build a bracketed order (entry + take-profits) based on technical analysis. Requires skill-trading to be installed.
---

# skill-shark — Trade Setup Strategy

skill-shark is a signal-driven trade setup strategy. It uses the TTC Scanner to find high-probability entries with defined risk, then sizes and places orders accordingly. If no qualifying setup exists on the requested symbol, it hunts for better setups across the market using volume and open interest data.

---

## STRATEGY OVERVIEW

1. **Scan** the requested symbol for a signal (entry, SL, TP1/2/3, R/R)
2. **Qualify** the setup — R/R must be ≥ 2.0 to proceed
3. **If R/R < 2** — hunt the market for a better setup
4. **Size** the orders relative to available balance
5. **Place** the bracket: buy/sell limit at entry zone + TPs as reduce-only buy/sells (opposite direction of the entry limit)
6. **Place** the bracket: buy/sell limit at SL zone to dollar cost average
---

## STEP 1 — SCAN THE SYMBOL

```
skill-trading market scanner --symbol <SYMBOL> --timeframe <TF>
```

Default timeframe: `1h`. Always try `4h` as a second opinion if the signal is weak.

Extract from the output:
- `direction` — LONG or SHORT
- `entry` — limit order price
- `stopLoss` — defines max risk per trade
- `takeProfit1`, `takeProfit2`, `takeProfit3` — exit targets
- `riskRewardRatio` — must be ≥ 2.0 to qualify

---

## STEP 2 — QUALIFY THE SETUP

```
if riskRewardRatio >= 2.0 → PROCEED to Step 3
if riskRewardRatio < 2.0  → HUNT for a better market (Step 2b)
```

### Step 2b — Market Hunt (R/R failed)

Run these scans in order to find candidates:

**By open interest** (most liquid futures markets):
```
skill-trading market open-interest
```
Take the top 5 symbols by OI. Scan each with the scanner.

**By momentum** (markets already moving):
```
skill-trading market hybrid-tickers --up 5 --min-volume 1000000 --market-type futures
```
Take the top movers. Scan each.

For each candidate, run the scanner and check R/R. Stop at the first symbol that qualifies (R/R ≥ 2.0). Report back to the user with the found symbol and signal before proceeding.

---

## STEP 3 — CHECK ACCOUNT STATE

Before placing any orders, run the mandatory checks from `skill-trading`:

```
skill-trading account balance -e <exchange>
skill-trading position get -e <exchange>
skill-trading order open -e <exchange>
```

- Confirm available balance > 0
- Flag any existing position on the same symbol
- Flag duplicate orders

---

## STEP 4 — SIZE THE ORDERS

Default sizing (adjust if user specifies a different amount):
- **Total risk per trade**: 2% of available balance, or user-specified USD amount
- **Entry order**: full trade size
- **TP1**: 50% of position size (reduce-only)
- **TP2**: 50% of position size (reduce-only)

Calculate quantity:
```
quantity = trade_usd / entry_price
half_qty = quantity / 2   (round down to valid lot size)
```

Always fetch the tick size from the bid/ask spread before placing:
```
skill-trading market best-bid-ask -e <exchange> --symbol <SYMBOL>
```
Round all prices to match the observed tick size.

---

## STEP 5 — PLACE THE BRACKET

### LONG setup

```
# Entry — buy limit at signal entry price
skill-trading order limit -e <exchange> --symbol <SYM> --buy --quantity <QTY> --price <ENTRY>

# TP1 — sell half at TP1 (reduce-only)
skill-trading order limit -e <exchange> --symbol <SYM> --sell --quantity <HALF_QTY> --price <TP1> --reduce-only

# TP2 — sell half at TP2 (reduce-only)
skill-trading order limit -e <exchange> --symbol <SYM> --sell --quantity <HALF_QTY> --price <TP2> --reduce-only
```

### SHORT setup

```
# Entry — sell limit at signal entry price
skill-trading order limit -e <exchange> --symbol <SYM> --sell --quantity <QTY> --price <ENTRY>

# TP1 — buy half at TP1 (reduce-only)
skill-trading order limit -e <exchange> --symbol <SYM> --buy --quantity <HALF_QTY> --price <TP1> --reduce-only

# TP2 — buy half at TP2 (reduce-only)
skill-trading order limit -e <exchange> --symbol <SYM> --buy --quantity <HALF_QTY> --price <TP2> --reduce-only
```

---

## ORDER SUMMARY FORMAT

Always print this before placing:

```
Symbol:    NEARUSDT
Direction: LONG
Timeframe: 1h
R/R:       18.47x  ✓

Entry:     $1.1710  →  BUY  824 NEAR  (~$963)
Stop Loss: $1.1603  (0.92% / ~$8.80 risk)
TP1:       $1.3695  →  SELL 412 NEAR  (+16.95%)
TP2:       $1.7880  →  SELL 412 NEAR  (+52.69%)
```

---

## RULES

- **Never skip the R/R check.** A setup with R/R < 2 is not a skill-shark setup.
- **Always use reduce-only on TP orders** for existing positions.
- **Never place a market order** for entry — always a limit.
- **Stop loss is informational** — do not place a stop order unless the user explicitly asks.
- **If R/R > 10**, flag it to the user as unusually high — could indicate a stale or extreme pivot.
- **Do not place orders on multiple symbols simultaneously** without user confirmation.
- **Always confirm with the user** before executing if trade size > $100 or if an existing position on the same symbol is open.
