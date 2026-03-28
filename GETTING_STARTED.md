# Getting Started — Agent Onboarding Guide

This document tells you everything you need to know to go from zero to placing your first trade. Read it top to bottom before running any commands.

---

## What This Project Does

`skill-trading` is a command-line tool that lets you trade on 15+ crypto exchanges (Orderly, Binance, Bybit, Phemex, OKX, and more) through the TTC Box API. All exchange operations are proxied through `https://ttc.box` — you never call exchange APIs directly.

It ships two agent skills:

- **skill-trading** — the core tool. Place orders, check balance, view positions, get market data.
- **skill-shark** — a strategy layer. Scans a symbol for a signal, checks R/R ≥ 2.0, then sizes and places a bracketed order.

---

## Part 1 — Is the CLI Installed?

Run this first:

```
skill-trading info
```

Expected output: `skill-trading v1.0.0` (or similar version number).

If you get "command not found", the binary is not installed. Build and install it:

```
make release
make install
```

After install, `skill-trading` will be at `/usr/local/bin/skill-trading` and available in PATH.

---

## Part 2 — Do You Have a .env File?

The CLI loads a `.env` file from the current working directory automatically. Check if one exists:

```
ls -la .env
```

If it does not exist, you need to create one. The minimum required content is:

```env
TTC_AUTH_TOKEN=<your TTC Box auth token>
TTC_PUBLIC_KEY=<your TTC Box public key>
TTC_EXCHANGE=orderly
```

If you also have exchange credentials, add them now — they will not be overwritten by `register` or `login`:

```env
ORDERLY_API_KEY=<your key>
ORDERLY_API_SECRET=<your secret>
ORDERLY_API_PASSPHRASE=<your broker ID, e.g. what_exchange>
ORDERLY_MAIN_WALLET_ADDRESS=<your Orderly wallet address — see Part 5>
```

---

## Part 3 — Do You Have a Valid TTC Session?

A TTC session consists of two values: `TTC_AUTH_TOKEN` and `TTC_PUBLIC_KEY`. Sessions expire after exactly 24 hours.

To check if your session is current, look at `TTC_TOKEN_ISSUED_AT` in `.env`:

```
grep TTC_TOKEN_ISSUED_AT .env
```

It is a Unix timestamp (seconds since epoch). The token is valid for 86400 seconds (24 hours) from that value.

To calculate remaining validity:

```bash
issued=$(grep TTC_TOKEN_ISSUED_AT .env | cut -d= -f2)
now=$(date +%s)
age=$((now - issued))
remaining=$((86400 - age))
echo "Token age: ${age}s / 86400s. Remaining: ${remaining}s"
```

If the token is expired (remaining ≤ 0), or if you are unsure, run:

```
skill-trading login
```

This reads `TTC_EMAIL` and `TTC_PASSKEY` from `.env` automatically and refreshes the token. No prompts are needed if both are set.

If `TTC_EMAIL` or `TTC_PASSKEY` are missing from `.env`, it will prompt you to enter them. After a successful login, `.env` is updated with a new `TTC_AUTH_TOKEN` and `TTC_TOKEN_ISSUED_AT`.

---

## Part 4 — Verify the Full Setup

Run these three commands in order. Each one confirms a different layer of the setup.

**Step 1 — Test TTC Box connectivity (no exchange credentials needed):**

```
skill-trading market hybrid-tickers --symbol BTCUSDT --market-type futures
```

Expected: A table showing BTCUSDT price, volume, and funding across exchanges. If this fails, your `TTC_AUTH_TOKEN` is likely expired or missing. Run `skill-trading login` and try again.

**Step 2 — Test exchange credentials:**

```
skill-trading account balance
```

Expected: A table showing your USDT balance with columns for total, locked, and available. If this returns an error, go to Part 5 (credential setup per exchange).

**Step 3 — Test position and order state:**

```
skill-trading position get
skill-trading order open
```

Expected: Either a table of open positions/orders, or a message saying none are open. Both are correct.

Once all three steps succeed, your setup is complete. Jump to Part 6 to place your first trade.

---

## Part 5 — Exchange Credential Setup

Credentials are loaded from `.env` using the pattern `{EXCHANGE}_API_KEY`, `{EXCHANGE}_API_SECRET`, and optionally `{EXCHANGE}_API_PASSPHRASE`. The exchange name in the env var must be uppercase.

### Which exchanges need a passphrase?

| Exchange | Needs Passphrase? | What to put in PASSPHRASE |
|----------|------------------|--------------------------|
| orderly | Yes | Broker ID — e.g. `what_exchange`, `woofi_pro`, `ttc` |
| okx | Yes | Trading password (set when creating the API key on OKX) |
| kucoin | Yes | API passphrase (set when creating the API key on KuCoin) |
| bitget | Yes | API passphrase (set when creating the API key on Bitget) |
| blofin | Yes | API passphrase (set when creating the API key on BloFin) |
| binance | No | — |
| bybit | No | — |
| phemex | No | — |
| hyperliquid | No | — |
| bingx | No | — |
| asterdex | No | — |

### Orderly: ORDERLY_MAIN_WALLET_ADDRESS

This is required for accounts created via the CLI `register` command (email-registered users).

**Why it is needed:** When you registered via email, TTC Box generated a random keypair for your `TTC_PUBLIC_KEY`. But Orderly needs to know your *actual* trading wallet address to look up your funds. This setting tells it where to find your account.

**Web3 users do not need this.** If you created your account through the TTC Box web interface using a wallet (MetaMask, Phantom, etc.), your `TTC_PUBLIC_KEY` is already your Orderly wallet.

**How to find your Orderly wallet address:**
1. Log into the TTC Box web interface or the Orderly Network app
2. Go to Account settings
3. Copy the main wallet public key (it is a base58-encoded Solana address, starts with a capital letter, around 44 characters)
4. Add to `.env`: `ORDERLY_MAIN_WALLET_ADDRESS=<that address>`

**How to verify it is correct:**

```
skill-trading account balance
```

If you see your real Orderly balance (not zero and not negative), the address is correct.

### Example .env for Orderly

```env
TTC_AUTH_TOKEN=eyJ...
TTC_PUBLIC_KEY=5abc...
TTC_EMAIL=user@example.com
TTC_PASSKEY=a1b2c3d4...
TTC_TOKEN_ISSUED_AT=1711234567
TTC_EXCHANGE=orderly

ORDERLY_API_KEY=ed25519:abc...
ORDERLY_API_SECRET=abc123...
ORDERLY_API_PASSPHRASE=what_exchange
ORDERLY_MAIN_WALLET_ADDRESS=GgUWyS5rsH4...
```

---

## Part 6 — Placing Your First Trade (Safe Sequence)

Never place an order without running the pre-order checklist. Here is the full safe sequence:

### 1. Check balance

```
skill-trading account balance
```

Look at the `Available` value. If it is zero or negative, do not place an order. Close a position or add funds first.

### 2. Check existing positions

```
skill-trading position get
```

Note any open positions. If you are about to open in the same direction on the same symbol as an existing position, confirm with the user first.

### 3. Check open orders

```
skill-trading order open
```

Look for duplicate or conflicting orders on the same symbol.

### 4. Get the current price

```
skill-trading market best-bid-ask -e <exchange> --symbol <SYMBOL>
```

Read the bid price. Use this for your limit order price (not a guessed number).

### 5. Calculate order size

```
required_margin = (quantity × price) / leverage
```

Make sure `required_margin ≤ available balance`. If not, reduce quantity.

### 6. Show the order summary before placing

Always print what you are about to do before running the order command:

```
Symbol:    NEARUSDT
Side:      BUY
Type:      LIMIT
Price:     $1.1710
Quantity:  850
Value:     ~$995.35
Exchange:  orderly
```

Ask the user to confirm.

### 7. Place the order

```
skill-trading order limit -e orderly --symbol NEARUSDT --buy --quantity 850 --price 1.1710
```

If you want to test first without executing:

```
skill-trading --dry-run order limit -e orderly --symbol NEARUSDT --buy --quantity 850 --price 1.1710
```

---

## Part 7 — Using the Scanner

The scanner runs technical analysis on a symbol and returns a signal with a recommended entry, stop-loss, and three take-profit targets.

```
skill-trading market scanner --symbol NEARUSDT --timeframe 1h
```

### How to read the output

```
NEARUSDT / 1h — LONG HIGH  (strength 84/100)
Entry:     $1.1710
Stop Loss: $1.1603  (0.92% risk)
TP1:       $1.3695  (+16.95%)
TP2:       $1.7880  (+52.69%)
TP3:       $2.1700  (+85.31%)
R/R:       18.47x
Note:      bull composite 84.2 (score 72, R/R 18.47) vs opposite 31.4
```

**Direction** — LONG means the signal expects price to go up. SHORT means down.

**Confidence** — HIGH (strength ≥ 70), MEDIUM (50–70), LOW (< 50). Only act on MEDIUM or HIGH.

**Entry** — Place your limit order at this price.

**Stop Loss** — This is your exit if the trade goes wrong. Note the % risk — it tells you how much you lose per unit if stopped out.

**TP1 / TP2 / TP3** — Take profit targets. A typical approach is to exit 50% at TP1 and 50% at TP2.

**R/R (Risk/Reward Ratio)** — How many dollars you make for every dollar you risk. R/R of 18.47x means you risk $1 to make $18.47. The skill-shark strategy requires R/R ≥ 2.0 to proceed.

**When not to use the signal:**
- Confidence is LOW
- R/R is below 2.0
- There is already an open position in the opposite direction on the same symbol

### Standard timeframes

- `1h` — default, good for swing trades lasting hours to days
- `4h` — longer-term swing trades, more reliable signals but less frequent
- `1d` — position trades lasting days to weeks
- `15m` — short-term, more noise

Try `1h` first. If the signal is weak, check `4h` as a second opinion.

---

## Part 8 — Session Expiry and Re-Login

Token expiry is the most common cause of `401` errors. Tokens expire 24 hours after `TTC_TOKEN_ISSUED_AT`.

If any command fails with an auth error, run:

```
skill-trading login
```

This is safe to run at any time. If `TTC_EMAIL` and `TTC_PASSKEY` are in `.env`, it completes silently and updates the token. If they are missing, it will prompt.

For long-running automated workflows, check the token age before starting each session and refresh proactively if it is more than 20 hours old.

---

## Part 9 — Common Errors and Fixes

**`Missing credentials for exchange: orderly`**
Your `ORDERLY_API_KEY` or `ORDERLY_API_SECRET` is not in `.env`. Add them and re-run.

**`API error [401]: Unauthorized`**
Your `TTC_AUTH_TOKEN` is expired or invalid. Run `skill-trading login`.

**`API error [400]: ...tick size...` or `...price precision...`**
Your order price does not match the exchange's required tick size. Run `skill-trading market best-bid-ask -e <exchange> --symbol <SYMBOL>`, observe the precision of the bid/ask prices, and round your order price to match.

**`API error [400]: ...quantity...` or `...lot size...`**
Same as above but for quantity. Round your quantity to the precision shown in the best-bid-ask output.

**`Configuration error: ...`**
Something in `config.toml` is malformed, or the file path is wrong. Check your `--config` flag or `TTC_CONFIG` env var.

**Balance shows `0.0000` or `-0.0000` for Orderly**
Your `ORDERLY_MAIN_WALLET_ADDRESS` is missing or wrong. See Part 5. This is the most common Orderly issue for email-registered users.

**`Position not found: BTCUSDT`**
You tried to close or modify a position that does not exist on this exchange under this symbol. Run `skill-trading position get` first to confirm the exact symbol and side.

**Rate limited**
The CLI automatically retries up to 3 times with increasing delays. If it still fails, wait 30 seconds and retry manually.

---

## Part 10 — Key Rules to Never Break

1. **Always check balance before placing an order.** A negative available balance means no new orders.
2. **Never guess the current price.** Always fetch it with `market best-bid-ask`.
3. **Never place a market order unless the user explicitly asks for it.** Limit orders only.
4. **Always confirm with the user before placing any order** — show the full order summary first.
5. **Stop loss is informational.** Do not place a stop order automatically unless the user asks.
6. **Always use `--reduce-only` on take-profit orders** to prevent accidentally opening a new position in the wrong direction.
7. **If the scanner R/R is below 2.0, do not place the order.** Hunt for a better setup instead (see skill-shark).

---

## Quick Reference

```bash
# Check token validity
grep TTC_TOKEN_ISSUED_AT .env

# Refresh token
skill-trading login

# Full setup verification
skill-trading market hybrid-tickers --symbol BTCUSDT --market-type futures
skill-trading account balance
skill-trading position get

# Pre-order checklist
skill-trading account balance
skill-trading position get
skill-trading order open
skill-trading market best-bid-ask -e <exchange> --symbol <SYMBOL>

# Scanner
skill-trading market scanner --symbol <SYMBOL> --timeframe 1h

# Place order (always confirm first)
skill-trading order limit -e <exchange> --symbol <SYMBOL> --buy --quantity <QTY> --price <PRICE>

# Dry-run any command
skill-trading --dry-run order limit -e <exchange> --symbol <SYMBOL> --buy --quantity <QTY> --price <PRICE>
```
