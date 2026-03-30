---
name: skill-loop-trading
description: Agent-controlled trading loops using Claude Code's /loop scheduler. The agent places one atomic order per tick, retains full visibility and control between each fill, and can react to market conditions mid-run. Use this instead of the built-in twap or trail-watch commands when you want the agent to be in control — not a blind background process.
---

# skill-loop-trading — Agent-Controlled Loop Trading

This skill governs how to use Claude Code's `/loop` scheduler with `skill-trading` atomic commands to execute time-based strategies (TWAP, DCA, trailing stop checks) while keeping the agent fully in the loop on every tick.

## Reference Files

- `../skill-trading/SKILL.md` — core trading protocol (always follow this)
- `../skill-twap/SKILL.md` — TWAP concepts and calculations

---

## THE PROBLEM WITH BACKGROUND LOOPS

The built-in `twap` and `risk trail-watch` commands own their own loop — they sleep internally between actions. Once launched, the agent is completely blind for the entire duration. It cannot:

- See intermediate prices or fills
- React if price dumps mid-run
- Stop or adjust without killing the process
- Know if errors are occurring

This is fine for unattended automation. But for active agentic trading, the agent should own the loop.

---

## THE SOLUTION: `/loop` + ATOMIC COMMANDS

Claude Code's built-in `/loop` scheduler fires the agent on a recurring interval. The agent places one atomic slice per tick, sees the result, and decides whether to continue.

```
/loop every 5m: place one NEARUSDT slice and report status
```

On each tick, the agent:
1. Checks current position and balance
2. Places one `twap-slice` market order
3. Reads the result (price, qty, order ID, cost)
4. Decides: continue / pause / abort based on what it sees
5. Sleeps until next tick

**The agent is never blind.**

---

## `/loop` QUICK REFERENCE

```
/loop <interval> <task>

Intervals: 30s (rounds to 1m), 5m, 15m, 1h, 4h, 1d
Default:   10m if no interval specified
Max tasks: 50 simultaneous
Expires:   3 days (auto-cancels)
Stop:      "cancel the loop" or exit the session
```

The loop fires between your turns — never mid-response. Context accumulates across iterations so Claude remembers previous fills and prices.

---

## AGENT-CONTROLLED TWAP

### Setup (do this first)

```
# 1. Check balance
skill-trading account balance -e orderly

# 2. Calculate plan (agent computes this, not the CLI)
# Budget: $200, min_usd_entry: $15 → 13 slices
# Duration: 1 hour → interval: 60min / 13 = ~4m 36s

# 3. Dry-run one slice to confirm
skill-trading twap-slice -e orderly -s NEARUSDT --buy --amount 15 --decimals 0 --dry-run

# 4. Start the loop
/loop 5m place one NEARUSDT buy slice of $15 using twap-slice and report fill
```

### The `twap-slice` command

```bash
skill-trading twap-slice -e <exchange> -s <SYMBOL> --buy|--sell --amount <USD> [--decimals <N>] [--label <X/N>]
```

| Flag | Description |
|------|-------------|
| `--amount` | USD notional for this single slice |
| `--decimals` | Lot size precision (0 = integer like NEAR; 3 = BTC/ETH style) |
| `--label` | Optional "3/13" label for display |

**Output (one line):**
```
  SLICE [3/13]  NEARUSDT BUY  Price: $1.1997  Qty: 12  Cost: ~$15.38  Order: 20975495013
```

### Tracking progress

After each slice, ask the agent to maintain a running total:
```
Agent tracks: slices_done, total_deployed, total_qty, avg_price
Agent stops loop when: total_deployed >= budget OR slices_done >= target_slices
```

Or read the position directly:
```bash
skill-trading position get -e orderly -s NEARUSDT
```

---

## AGENT-CONTROLLED TRAIL WATCH

Instead of `risk trail-watch` (which runs blind), use `/loop` to check position status and manage stops:

```
/loop 30s: check NEARUSDT position on orderly — if in profit, calculate trail stop at 2% below peak and update if improved
```

On each tick, the agent:
1. Fetches position: `skill-trading position get -e orderly -s NEARUSDT`
2. If PnL > 0: calculates trail price = mark_price × (1 - trail_pct%)
3. If trail_price > previous_stop: cancel old stop, place new stop
4. If position gone: cancel loop

This is more verbose than `trail-watch` but the agent can react to anything — news events, margin warnings, signal changes.

---

## WHEN TO USE EACH APPROACH

| Scenario | Use |
|----------|-----|
| Unattended overnight TWAP | `twap` (built-in loop, crash recovery) |
| Active session, want visibility | `/loop` + `twap-slice` |
| Trail stop while you watch | `/loop` + `position get` + `risk sl` |
| Trail stop while you sleep | `risk trail-watch` (built-in loop) |
| DCA on a signal trigger | `/loop` + `market scanner` + `twap-slice` |

---

## AGENT RULES FOR LOOP TRADING

1. **Always verify balance before starting a loop** — `account balance` first
2. **Set a budget cap and track it** — agent must stop the loop when budget is exhausted, not just run until `/loop` expires
3. **Check position on the first tick** — confirm existing exposure before adding to it
4. **Log every fill** — keep a running summary in the conversation so any tick failure is visible
5. **Never start a loop without a stop plan** — define the exit condition before `/loop` fires
6. **If a slice errors, report it and continue** — one API error should not stop the entire run
7. **Use `--dry-run` on `twap-slice` to preview before the first live tick**

---

## EXAMPLE: $200 TWAP over 1 hour (agent-controlled)

```
Pre-flight:
  skill-trading account balance -e orderly
  → Available: $55 (enough for $200 notional at 10x leverage)

Plan:
  Budget: $200 notional | Min entry: $15 | Slices: 13 | Interval: ~5m

Start loop:
  /loop 5m: run twap-slice on NEARUSDT for $15, label the slice sequentially, and show running total

Agent on each tick:
  1. skill-trading twap-slice -e orderly -s NEARUSDT --buy --amount 15 --decimals 0 --label "N/13"
  2. Read result, add to total_deployed and total_qty
  3. If total_deployed >= 200: cancel loop and print summary
  4. Else: wait for next tick
```

---

## WHAT NOT TO DO

- Do not start a `/loop` without defining a budget cap — loops run for up to 3 days by default
- Do not run `/loop` TWAP and `twap` (built-in) on the same symbol simultaneously — they will race
- Do not use `/loop` intervals shorter than 1 minute — cron minimum is 1 minute
- Do not assume the loop will stop itself — the agent must track fills and cancel the loop when the budget is exhausted
- Do not use `/loop` for strategies that require sub-minute execution — the minimum interval is 1 minute
