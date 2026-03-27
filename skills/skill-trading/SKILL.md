# skill-trading — Order Management Skill

This skill governs how an AI assistant should interact with the `skill-trading` CLI.
It exists to prevent hallucination and unsafe order execution.

---

## MANDATORY PRE-ORDER CHECKLIST

Before placing **any** order (`limit`, `market`, `stop`), you MUST run these checks in order.
Do not skip steps. Do not place an order if any check fails.

### Step 1 — Check available balance

```
skill-trading account balance
```

Parse the `Available` value from the output.

- If `Available` is **negative** or **zero** → **STOP. Do not place the order.**
  - Tell the user: "Available balance is [amount]. Cannot place order."
  - Suggest: close a position, reduce size, or add funds.

- If `Available` is **positive** → continue to Step 2.

### Step 2 — Check existing positions

```
skill-trading position get
```

Review open positions:
- Note symbols, sizes, side (long/short), PnL, and liquidation prices.
- If the user wants to open in the same direction as an existing position, flag it.
- If the user wants to reduce/close, use `position close` not a new order.

### Step 3 — Validate order size vs available margin

Estimate required margin:
```
required_margin = (quantity × price) / leverage
```

- If `required_margin > available` → **STOP. Reduce quantity or skip.**
- Always confirm with the user before placing if margin is tight (< 20% headroom).

### Step 4 — Check open orders

```
skill-trading order open
```

- If there are existing orders on the same symbol/side, flag potential duplicates.
- Ask the user to confirm before adding another order.

---

## ORDER PLACEMENT RULES

- **Never place a real order without user confirmation** unless explicitly told to automate.
- **Always show the order summary** before executing:
  ```
  Symbol:    NEARUSDT
  Side:      BUY
  Type:      LIMIT
  Price:     $1.168
  Quantity:  17
  Value:     ~$19.86
  Exchange:  orderly
  ```
- Use `--dry-run` first when testing a new order type or exchange.
- If the user says "place an order at 1% under price", always fetch the current price first via:
  ```
  skill-trading market best-bid-ask --symbol <SYMBOL>
  ```
  Then calculate: `price = bid × 0.99`, `quantity = budget / price`.

---

## INTERPRETING BALANCE OUTPUT

| Field       | Meaning                                              |
|-------------|------------------------------------------------------|
| `balance`   | Total USDT in account                                |
| `locked`    | Margin currently committed to open positions/orders  |
| `available` | Free margin = balance − locked. **This is what matters for new orders.** |

A **negative available** means the account is over-committed. No new orders can be placed until a position is closed or margin is freed.

---

## INTERPRETING POSITION OUTPUT

| Field        | Meaning                                         |
|--------------|-------------------------------------------------|
| `size`       | Position size in base asset                     |
| `entry_price`| Average entry price                             |
| `mark_price` | Current market price                            |
| `pnl`        | Unrealized profit/loss                          |
| `liq`        | Liquidation price — dangerous if approached     |
| `leverage`   | Current leverage multiplier                     |

If `mark_price` is approaching `liq`, warn the user immediately.

---

## COMMON WORKFLOWS

### Open a new position
1. `account balance` → verify available > 0
2. `position get` → check no conflicting positions
3. `market best-bid-ask --symbol <SYM>` → get current price
4. Calculate price and quantity
5. Show order summary → confirm with user
6. `order limit` or `order market`

### Close a position
1. `position get` → get symbol and size
2. `position close --symbol <SYM>` (uses market order)
   OR `order limit --sell --reduce-only` for a limit close

### Check account health
```
skill-trading account balance
skill-trading position get
```
Together these give a full picture of margin usage and risk.

---

## EXCHANGE DEFAULTS

- Default exchange is set in `config.toml` via `skill-trading config set-default <exchange>`
- Override per-command with `-e <exchange>`
- Credentials are loaded from `.env` using `{EXCHANGE}_API_KEY` / `{EXCHANGE}_API_SECRET` / `{EXCHANGE}_API_PASSPHRASE`
- For Orderly: passphrase = broker ID (e.g. `what_exchange`, `woofi_pro`, `ttc`)

---

## WHAT NOT TO DO

- Do not place an order immediately after being asked — always run the checklist first.
- Do not assume the account has funds — always verify.
- Do not guess the current price — always fetch it.
- Do not place duplicate orders without confirming with the user.
- Do not use market orders unless the user explicitly requests it — prefer limit orders.
