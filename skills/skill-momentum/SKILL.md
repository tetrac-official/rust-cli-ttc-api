---
name: skill-momentum
description: Momentum hunter. Finds futures markets that are moving significantly right now (volume-confirmed breakouts), then scans each for a signal. Use when the user wants to find a trade outside their usual watchlist, or when something is clearly moving and they want to know if there is a valid entry.
---

# skill-momentum — Momentum Hunter

This skill finds what is breaking out right now and checks whether it has a valid trade signal. It works outside the fixed watchlist — it hunts the full market for volume-confirmed moves and then applies the scanner to find entries with defined risk.

---

## WHEN TO RUN

- When the user says "find me something to trade"
- When you see a symbol moving 10%+ in hybrid-tickers and want to know if there is an entry
- After a major BTC move when alts start running
- When the current watchlist patrol returns no qualifying setups

---

## STEP 1 — Find the Movers

Get the top futures markets by momentum (volume × price change):

```
skill-trading market hybrid-tickers --up 5 --min-volume 3000000 --market-type futures
```

This returns markets up ≥ 5% today with ≥ $3M 24h volume. Volume is the filter that separates real moves from illiquid noise.

Also check the downside for short setups:

```
skill-trading market hybrid-tickers --down 5 --min-volume 3000000 --market-type futures
```

### Selecting candidates

From the results, pick the top 8 symbols ranked by this priority:
1. **Volume** — higher is better. A 10% move on $100M volume is more reliable than on $3M.
2. **Open Interest** — non-zero OI means futures traders are actively positioned.
3. **Funding rate** — avoid symbols with extreme funding in the direction of the move (overcrowded).

**Exclude:**
- Symbols with zero or near-zero OI (no futures market depth)
- Symbols you are already positioned in (check `skill-trading position get`)
- Moves that look like a single exchange anomaly (check `sources` — prefer `avr` or multi-exchange)

---

## STEP 2 — Scan Each Candidate

For each of the selected candidates, run the scanner on 1h:

```
skill-trading market scanner --symbol <SYMBOL> --timeframe 1h
```

Record: direction, confidence, R/R, entry, stop-loss, TP1.

Work through the list in order of volume (highest first). Stop as soon as you find a qualifying setup — you do not need to scan all 8 if you find a good one early.

**Qualifying threshold for momentum trades:**
- Confidence: **HIGH**
- R/R: **≥ 2.0** (lower than patrol because the move is already in progress)
- Direction: **same as the price move direction** (never fade a momentum breakout with this skill)

If no candidate qualifies on 1h, try 15m for the top 3 symbols only:

```
skill-trading market scanner --symbol <SYMBOL> --timeframe 15m
```

---

## STEP 3 — Evaluate the Signal

Before proceeding to an order, check one additional thing for momentum trades: funding.

```
skill-trading market funding-rates --symbol <SYMBOL>
```

If the move is **up** and funding is already **strongly positive** (> +0.05% on most exchanges) → the rally is funded by late longs. Risk of reversal is high. Reduce size or skip.

If funding is **neutral or negative** on an up-mover → shorts are still positioned or haven't flipped yet. The move may have more room.

---

## STEP 4 — Report and Hand Off

Report to the user:

```
MOMENTUM HUNT — [time]

TOP MOVERS SCANNED: [list of symbols]

QUALIFYING SETUP:
  Symbol:    ONUSDT
  Direction: LONG
  Timeframe: 1h
  Move:      +80.2% today, Vol: $71M
  Signal:    HIGH confidence, R/R 4.2x
  Entry:     $0.2165
  Stop Loss: $0.2040  (5.8% risk)
  TP1:       $0.2780  (+28.4%)
  TP2:       $0.3650  (+68.7%)
  Funding:   Neutral — no squeeze risk

  → Run skill-shark to place the bracket? Or place manually:
    skill-trading market best-bid-ask -e <exchange> --symbol ONUSDT

NO SETUP:
  KNCUSDT, STGUSDT, TAUSDT, BSBUSDT (below threshold)
```

If no candidates qualify at all:

```
MOMENTUM HUNT — [time]

Scanned: [list]
No HIGH confidence R/R ≥ 2.0 setups found in current movers.

The move may already be extended. Consider waiting for a retest or running
skill-signal-patrol on the watchlist instead.
```

---

## RULES

- **Only trade in the direction of the move.** Do not use this skill to fade a move.
- **Funding is the exit signal, not the entry signal.** Check it after finding the setup, not before.
- **The move being large does not make the trade good.** The signal must qualify independently.
- **If R/R < 2.0 on all scanned symbols**, do not force a trade. Large moves that are already extended often produce low R/R because the stop must be placed far away.
- **Always run the pre-order checklist** (balance, positions, open orders) before handing off to skill-shark.
- **Position sizing on momentum trades**: use no more than 50% of your usual size. Momentum moves are volatile — the stop-loss may be hit even if the direction is correct.
