---
name: skill-market-overview
description: Full market briefing. Use when the user wants to understand current market conditions before trading — what regime BTC is in, where money is flowing, which side funding favors, and which symbols have the most open interest. Run this at the start of every trading session.
---

# skill-market-overview — Market Briefing

This skill produces a structured market briefing from live data. Run it before any trading session. It tells you the macro context so every trade decision is grounded in current conditions.

---

## WHEN TO RUN

- At the start of every trading session
- After a significant BTC price move (> 3%)
- When you are unsure which direction to favor
- Before running skill-signal-patrol or skill-momentum

---

## STEP 1 — Market Regime (BTC Trend)

Scan BTC on two timeframes to establish the primary trend direction.

```
skill-trading market scanner --symbol BTCUSDT --timeframe 4h
skill-trading market scanner --symbol BTCUSDT --timeframe 1h
```

Interpret:
- If both 4h and 1h show **LONG** → bull regime. Favor long setups in other symbols.
- If both show **SHORT** → bear regime. Favor short setups.
- If they disagree → ranging or transitioning. Reduce size and be selective.
- Check the confidence and R/R. Weak BTC signal = uncertain market. Do not force trades.

Also check ETH — it often leads or lags BTC:

```
skill-trading market scanner --symbol ETHUSDT --timeframe 4h
```

If ETH direction differs from BTC, note the divergence. The stronger signal is usually more reliable.

---

## STEP 2 — Funding Sentiment

Funding rates show which side the market is leaning. Extreme funding is a mean-reversion warning and a directional signal.

```
skill-trading market funding-rates --symbol BTCUSDT
skill-trading market funding-rates --symbol ETHUSDT
```

Interpret:
- Most exchanges **positive** (> +0.01%): market is long-heavy. Longs paying shorts. Expect potential long squeeze.
- Most exchanges **negative** (< -0.01%): market is short-heavy. Shorts paying longs. Expect potential short squeeze.
- Mixed (near zero): no strong positioning bias.
- Any exchange showing > +0.1% or < -0.1%: extreme. That exchange is highly biased.

**What to do with this:**
- Extreme positive funding → lean short or be cautious going long
- Extreme negative funding → lean long or be cautious going short
- Neutral funding → direction is open; rely on scanner signal

---

## STEP 3 — Open Interest Leaders

OI shows where the largest positions are concentrated.

```
skill-trading market open-interest
```

This returns the top symbols by total open interest. These are the most actively traded markets — highest liquidity, tightest spreads, most reliable scanner signals.

Note the symbols with highest OI. These are the best candidates for skill-signal-patrol and skill-shark.

---

## STEP 4 — Momentum Scan (What Is Moving)

Find where money is flowing right now.

```
skill-trading market hybrid-tickers --up 5 --min-volume 5000000 --market-type futures
```

This returns futures markets that are up ≥ 5% today with ≥ $5M volume. High-volume movers are where the market attention is focused.

Also check the downside:

```
skill-trading market hybrid-tickers --down 5 --min-volume 5000000 --market-type futures
```

**What to look for:**
- A cluster of related symbols moving together (e.g., AI tokens, DeFi, L2s) → a sector rotation is happening
- A few high-volume outliers with 20%+ moves → potential momentum trade, check scanner before entering
- Negative funding on up-movers → short squeeze rally. Be cautious adding longs.
- Positive funding on down-movers → long liquidation cascade. Be cautious adding shorts.

---

## STEP 5 — Synthesize the Briefing

After running all four steps, produce a structured summary in this format:

```
MARKET BRIEFING — [date/time]

REGIME:
  BTC 4h: [LONG/SHORT/WEAK] (confidence: [HIGH/MED/LOW], R/R: [X]x)
  BTC 1h: [LONG/SHORT/WEAK]
  ETH 4h: [LONG/SHORT/WEAK]
  Overall: [BULL / BEAR / RANGING]

FUNDING:
  BTC: [NET POSITIVE / NET NEGATIVE / NEUTRAL] (avg across exchanges: [X]%)
  ETH: [NET POSITIVE / NET NEGATIVE / NEUTRAL]
  Bias: [Long-heavy — fade longs / Short-heavy — fade shorts / Neutral]

OPEN INTEREST LEADERS:
  1. [SYMBOL] — $[OI] OI, $[VOL] 24h vol
  2. [SYMBOL] — ...
  (top 3 only)

MOVERS (UP):
  [SYMBOL] +[X]%  Vol: $[X]M  Fund: [X]%
  (top 3 only, highest volume)

MOVERS (DOWN):
  [SYMBOL] -[X]%  Vol: $[X]M  Fund: [X]%
  (top 3 only)

TRADEABLE BIAS: [LONG / SHORT / NEUTRAL — brief reason]
```

---

## RULES

- **Never skip the regime check.** Trading without knowing BTC direction is gambling.
- **If BTC is RANGING**, reduce position sizing by 50% on all trades and widen stop-losses.
- **Negative funding does not mean sell.** It means shorts are already overcrowded — a squeeze is possible.
- **High OI + high volume** = institutional activity. These moves tend to be more sustained.
- **Low OI + high volume** = retail-driven pump. Can reverse quickly.
- **After completing the briefing**, share it with the user before doing anything else.
