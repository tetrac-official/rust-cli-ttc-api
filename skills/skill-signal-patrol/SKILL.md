---
name: skill-signal-patrol
description: Watchlist scanner. Scans a defined list of symbols on 1h and 4h for high-quality signals (HIGH confidence, R/R ≥ 3.0). Use when the user wants to know which of their watched symbols currently has a tradeable setup. Returns a ranked list of active setups.
---

# skill-signal-patrol — Watchlist Scanner

This skill scans a fixed watchlist of symbols and surfaces any setups that meet the quality threshold (HIGH confidence, R/R ≥ 3.0). Run it periodically during a session to catch new setups as they form.

The default watchlist is in `references/watchlist.md`. The user can edit that file to add or remove symbols.

---

## WHEN TO RUN

- Every 1–2 hours during an active trading session
- After the market-overview briefing to identify specific entry points
- When the user asks "anything setting up?"
- Before the end of a session to plan overnight holds

---

## HOW TO RUN A PATROL

### Step 1 — Load the watchlist

Read `references/watchlist.md` to get the current list of symbols and their assigned timeframes.

### Step 2 — Scan each symbol

For each symbol in the watchlist, run the scanner on its primary timeframe:

```
skill-trading market scanner --symbol <SYMBOL> --timeframe <TF>
```

Record the result: direction, confidence, R/R ratio, entry, stop-loss, TP1, TP2.

### Step 3 — Filter for quality

Keep only results that meet **all** of the following:
- Confidence is **HIGH**
- R/R is **≥ 3.0**
- Direction is consistent with the current market regime (from skill-market-overview)

Discard weak signals (MEDIUM or LOW confidence, R/R < 3.0).

### Step 4 — Double-check qualifying signals on 4h

For any symbol that passes Step 3, run the scanner on 4h as a second opinion:

```
skill-trading market scanner --symbol <SYMBOL> --timeframe 4h
```

- If 4h **agrees** (same direction, HIGH confidence) → mark as **STRONG SETUP**
- If 4h **disagrees** or is weak → mark as **WEAK SETUP** (caution)
- If 4h shows opposite direction → discard

### Step 5 — Rank and report

Rank qualifying setups by:
1. STRONG SETUP first (both 1h + 4h aligned)
2. Then by R/R descending

Report in this format:

```
SIGNAL PATROL — [time]

STRONG SETUPS (1h + 4h aligned):
  NEARUSDT  LONG   1h R/R 18.5x  4h R/R 6.2x  Entry: $1.171  SL: $1.160  TP1: $1.370
  SOLUSDT   SHORT  1h R/R 4.1x   4h R/R 3.8x  Entry: $82.30  SL: $84.10  TP1: $77.20

SINGLE TIMEFRAME SETUPS (1h only):
  ARBUSDT   LONG   1h R/R 3.2x   Entry: $0.425  SL: $0.412  TP1: $0.467

NO SETUP:
  BTCUSDT, ETHUSDT, BNBUSDT, AVAXUSDT (signals below threshold)
```

If there are no qualifying setups at all, say so clearly: "No HIGH confidence R/R ≥ 3.0 signals found on the watchlist right now."

---

## AFTER A PATROL

- For any STRONG SETUP, offer to run skill-shark to place the bracketed order
- For WEAK SETUPS, mention them but do not proceed unless the user asks
- For NO SETUP symbols, do not comment on them individually — just list the symbol names

---

## RULES

- **Never lower the threshold** to find something to trade. If there are no qualifying setups, say so. Patience is a position.
- **Never trade against the regime.** If skill-market-overview says BEAR, skip LONG setups from the patrol unless there is a strong reason.
- **Rerun the patrol** before placing any order from it — signals can change. A setup found 2 hours ago may no longer be valid.
- **Do not scan more than 20 symbols** in one patrol. It takes too long and adds noise. Focus on liquid, high-OI markets.
