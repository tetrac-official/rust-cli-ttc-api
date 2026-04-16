---
name: skill-portfolio-manager
description: Portfolio health monitoring for skill-trading. Use when the user asks to check overall account health, review all positions at once, identify risk, or set up periodic monitoring. Aggregates balance + positions into a single health report with a HEALTHY / WATCH / DANGER status. Drives a clear decision tree for each risk level.
---

# skill-portfolio-manager

This skill governs how an AI assistant uses `portfolio summary` to monitor account health, interpret risk warnings, and drive appropriate responses.

Depends on `skill-trading` — all CLI commands below are from that binary.

## Reference Files

- `../skill-trading/SKILL.md` — core trading protocol (always follow this)

---

## WHEN TO USE THIS SKILL

- User asks "how is my account?" / "check my portfolio" / "what's my risk?"
- Before placing any order — pre-flight health check
- During a TWAP or DCA run — verify margin headroom after each slice
- Any position shows PnL loss > 5% — get full book context
- Setting up periodic monitoring with `/loop`

---

## PRIMARY COMMAND

```bash
skill-trading portfolio summary -e <exchange>
```

Runs **two API calls concurrently** (balance + positions) and aggregates into one health report. This is the only command needed for a complete picture.

### Output Sections

| Section | What to read |
|---------|-------------|
| ACCOUNT BALANCE | `Utilization %` — locked / total. High = less room for new trades |
| POSITIONS | Each: notional, PnL%, margin used, liq distance, inline warnings |
| TOTALS | Total notional exposure, net PnL, total margin committed |
| WARNINGS | All threshold breaches collected in one place |
| STATUS | **HEALTHY / WATCH / DANGER** — your primary action trigger |

### Example Output

```
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  PORTFOLIO SUMMARY — ORDERLY
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ACCOUNT BALANCE
  Total:        $153.44 USDT
  Available:    $15.88
  Locked:       $137.56
  Utilization:  89.7%  [WATCH — threshold 80%]

  POSITIONS (1 open)
  ───────────────────────────────────────────────────────

  NEARUSDT  BUY 10x
    Size:     1160    Notional: $1375.64
    PnL:      -33.15 USDT  (-4.56%)
    Margin:   $137.56    Liq dist: 8.88%  [DANGER — below 10%]

  TOTALS
  Notional:      $1375.64
  Total PnL:     -33.15 USDT
  Total Margin:  $137.56

  WARNINGS
  [!] Margin utilization 89.7% exceeds threshold (80.0%)
  [!] NEARUSDT liq distance 8.88% is below threshold (10.0%)

  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  STATUS: DANGER
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## HEALTH STATUS DECISION TREE

### STATUS: HEALTHY

No warnings. All thresholds clear.

**Action:**
- Confirm to user: "Portfolio healthy. Utilization: X%, no positions near liquidation."
- Safe to proceed with new trades if balance allows.
- If monitoring loop active: continue, no action needed.

### STATUS: WATCH

One or more of:
- Margin utilization > `max_margin_utilization` (default 80%)
- A position notional > `max_position_notional` (default $5,000)

**Action:**
1. Surface the specific warnings verbatim.
2. Suggest one of:
   - **Cancel open limit orders** to free locked margin: `skill-trading orders cancel-all -e <exchange>`
   - **Partially reduce the largest position**: `skill-trading order limit -e <exchange> -s <SYMBOL> --sell --reduce-only -q <qty> -p <price>`
   - **Do not open new positions** until utilization drops.
3. Confirm with user before executing.
4. Re-run `portfolio summary` after any action.

### STATUS: DANGER

Any position's liquidation price is fewer than `min_liq_distance_pct` away (default 10%).

**Treat as time-sensitive:**
1. Surface the at-risk position immediately: "NEARUSDT LONG is only 8.88% from liquidation."
2. Present options in priority order:
   - **a. Add a stop loss** (fastest protection, keeps position open):
     ```bash
     skill-trading risk sl -e <exchange> -s <SYMBOL> --stop-price <liq_price + 1%>
     ```
   - **b. Close the position** (eliminates risk entirely):
     ```bash
     skill-trading position close -e <exchange> -s <SYMBOL>
     ```
   - **c. Reduce size** (partial close, maintains direction):
     ```bash
     skill-trading order limit -e <exchange> -s <SYMBOL> --sell --reduce-only -q <partial_qty> -p <mark_price>
     ```
3. Do NOT suggest opening new positions while DANGER exists.
4. After action: re-run `portfolio summary` to confirm liq distance improved.

---

## RISK SCENARIO PLAYBOOK

### High margin utilization (WATCH)

```
Utilization: 89.7% [WATCH — threshold 80%]
```

Only 10.3¢ of every $1 is free. Losses will further erode available margin.

Actions:
```bash
# Cancel all open limit orders to free locked margin
skill-trading orders cancel-all -e <exchange>

# Or partially close the largest position
skill-trading order limit -e <exchange> -s NEARUSDT --sell --reduce-only -q <qty> -p <price>
```

### Position near liquidation (DANGER)

```
Liq dist: 8.88%  [DANGER — below 10%]
```

Actions in order:
```bash
# 1. Place stop just above liquidation (e.g. liq $1.08 → stop $1.10)
skill-trading risk sl -e <exchange> -s NEARUSDT --stop-price 1.10

# 2. Verify stop is in place
skill-trading orders get -e <exchange>

# 3. Or close entirely
skill-trading position close -e <exchange> -s NEARUSDT
```

### Negative PnL + high utilization (compounding WATCH/DANGER)

Losses are shrinking the account, so utilization % is rising even without new trades.

- Do not average down (add to losing positions).
- Close the worst PnL% position first — it is eroding margin fastest.
- Re-run `portfolio summary` after each close to track improvement.

---

## FIELD REFERENCE

| Field | Formula | Meaning |
|-------|---------|---------|
| `Utilization %` | locked / total × 100 | How much of the account is committed |
| `PnL%` | (mark − entry) / entry × 100, sign-flipped for short | Performance vs entry |
| `Margin used` | notional / leverage | USDT backing this position |
| `Liq dist %` | \|mark − liq\| / mark × 100 | Breathing room before forced close |
| `Total notional` | sum of position notionals | Gross exposure in USD |
| `Total PnL` | sum of unrealized PnL | Net profit/loss across book |

**Liq dist % interpretation:**
- > 20% — comfortable
- 10–20% — monitor, consider a stop loss
- < 10% — DANGER threshold, act now
- < 5% — critical, close or stop immediately

---

## PERIODIC MONITORING WITH /loop

```
/loop 5m skill-trading portfolio summary -e <exchange>
```

| Status | Agent action on each tick |
|--------|--------------------------|
| HEALTHY | Log status + utilization. Continue. |
| WATCH | Alert user with specific warnings. Await instruction. |
| DANGER | Alert immediately. Recommend stopping loop and taking manual action. |

**Recommended intervals:**
- Active session: `/loop 2m`
- Overnight hold: `/loop 15m`
- During TWAP: match to slice interval

**Stop loop when:**
- HEALTHY for two consecutive checks after a WATCH/DANGER
- All positions closed
- User cancels

---

## INTEGRATION WITH OTHER SKILLS

- **Before `skill-trading` orders**: run `portfolio summary` — confirm STATUS is not DANGER before adding exposure
- **During `skill-twap`**: run after each slice — verify utilization not rising dangerously
- **After `skill-momentum` or `skill-shark` signals**: check portfolio health before acting. Skip entry if DANGER.
- **After `skill-signal-patrol`**: portfolio health gates whether any signal is actionable

---

## CONFIGURATION

Thresholds live in `config.toml` under `[portfolio]`:

```toml
[portfolio]
max_margin_utilization = 80.0   # % — warn if locked/balance > this
min_liq_distance_pct   = 10.0   # % — warn if any liq is this close
max_position_notional  = 5000.0 # USD — warn if single position > this
```

The output shows the active threshold values inline in each `[WARN]` tag — always read those, not this document's defaults, as the user may have customized them.

---

## WHAT NOT TO DO

- Do not use `portfolio summary` when you only need balance — use `account balance` alone (one API call, not two)
- Do not ignore inline `[WARN]` tags on individual positions — a DANGER position triggers DANGER status regardless of the others
- Do not take closing action in DANGER without first confirming with the user which position and what action
- Do not open new positions while STATUS is DANGER
- Do not assume thresholds are at their defaults — read them from the actual output
