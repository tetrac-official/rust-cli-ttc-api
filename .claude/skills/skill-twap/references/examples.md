# TWAP Examples

## Common Scenarios

### Dollar-Cost Averaging into a position
```
# Buy $500 of NEAR over 48 hours (2 days), every 30 min = 96 slices
skill-trading twap -e orderly -s NEARUSDT --buy --budget 500 --hours 48
```

### Scaling out of a position over a session
```
# Sell $1000 of SOL over 8 hours, every 30 min = 16 slices of ~$62.50
skill-trading twap -e orderly -s SOLUSDT --sell --budget 1000 --hours 8
```

### Fast accumulation with tight intervals
```
# Buy $200 of ETH over 1 hour, every 5 min = 12 slices of ~$16.67
skill-trading twap -e orderly -s ETHUSDT --buy --budget 200 --hours 1 --interval 5
```

### Fixed slice count (10 orders over 6 hours)
```
# 10 equal slices, interval auto-calculated to 36 min
skill-trading twap -e orderly -s BTCUSDT --buy --budget 1000 --hours 6 --slices 10
```

## Slice Size Reference

| Budget | Hours | Interval | Slices | Slice Size |
|--------|-------|----------|--------|------------|
| $100 | 1h | 15m | 4 | $25.00 |
| $200 | 2h | 30m | 4 | $50.00 |
| $500 | 12h | 30m | 24 | $20.83 |
| $1000 | 24h | 30m | 48 | $20.83 |
| $2000 | 48h | 30m | 96 | $20.83 |
| $1000 | 4h | 15m | 16 | $62.50 |

## Minimum Slice Size

Most exchanges have a minimum order notional of ~$5-$10. Ensure your slice size is above this:
```
slice_usd = budget / slices
```
If `slice_usd < $10`, increase `--interval` or reduce `--hours` to reduce the number of slices.
