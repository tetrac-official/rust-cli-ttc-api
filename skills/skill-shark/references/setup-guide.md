# skill-shark Setup Guide

## Prerequisites

skill-shark requires `skill-trading` to be installed and working. The CLI binary must be available as `skill-trading` in your PATH, or at `../skill-trading/scripts/skill-trading` relative to this skill.

## Verify Installation

```bash
skill-trading info
skill-trading account balance
```

Both should succeed without errors. If not, see `skill-trading/references/exchanges.md` for credential setup.

## Strategy Parameters

These defaults are used unless the user specifies otherwise:

| Parameter         | Default | Notes                                      |
|-------------------|---------|--------------------------------------------|
| Minimum R/R       | 2.0     | Below this, hunt for another market        |
| Default timeframe | 1h      | Always try 4h as a second opinion          |
| TP split          | 50/50   | Half at TP1, half at TP2                   |
| Stop loss         | Info only | No stop order placed unless user asks    |

## Market Hunt Order (when R/R < 2)

1. `market open-interest` → scan top 5 by OI
2. `market hybrid-tickers --up 5 --min-volume 1000000 --market-type futures` → scan top movers
3. Stop at first symbol with R/R ≥ 2.0

## Tick Size

Always check before placing any order:
```bash
skill-trading market best-bid-ask --symbol <SYMBOL>
```
The spread between bid and ask reveals the tick size. Round all order prices to that increment.

## Example Full Run

```bash
# 1. Scan
skill-trading market scanner --symbol NEARUSDT --timeframe 1h

# 2. Check account
skill-trading account balance
skill-trading position get

# 3. Check tick size
skill-trading market best-bid-ask --symbol NEARUSDT

# 4. Place bracket (example: LONG, 824 NEAR, split 412/412)
skill-trading order limit --symbol NEARUSDT --buy --quantity 824 --price 1.1710
skill-trading order limit --symbol NEARUSDT --sell --quantity 412 --price 1.3695 --reduce-only
skill-trading order limit --symbol NEARUSDT --sell --quantity 412 --price 1.7880 --reduce-only
```
