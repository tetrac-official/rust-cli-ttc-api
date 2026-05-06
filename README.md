# Skill Trading CLI

Execute trading operations on TTC Box across 15+ exchanges.

Place orders, manage positions, scan markets, and control risk. Designed for AI agents and automated trading workflows.

> **First time here?** See [`.claude/skills/skill-onboarding/SKILL.md`](./.claude/skills/skill-onboarding/SKILL.md) — it walks an agent through the full setup from install to first trade.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE.txt)
[![Version](https://img.shields.io/badge/version-0.1.5-green.svg)]()

---

## Features

- **Email Registration & Login** — Create and authenticate TTC Box accounts from the terminal
- **Client-Side Wallet Generation** — Solana, Orderly, and EVM wallets generated and encrypted locally; private keys never sent in plaintext
- **15+ Exchanges** — Orderly, Phemex, Bybit, Binance, OKX, Bitget, BloFin, KuCoin, Hyperliquid, AsterDEX, BingX, and more
- **Full Order Management** — Limit, market, stop-loss, take-profit orders
- **DCA Ladder** — Place a stepped grid of limit orders from current price, auto-sized from budget and min entry
- **Position Control** — View, close, and manage positions
- **Position PnL Breakdown** — Entry, mark price, % move, margin used, liquidation distance
- **Portfolio Health Report** — Balance + all positions in one view: HEALTHY / WATCH / DANGER status with configurable risk thresholds
- **Account Operations** — Balance, leverage, margin mode, hedge mode
- **Market Data** — Cross-exchange tickers, funding rates, open interest, volume snapshots
- **TTC Scanner** — Technical analysis signal with entry, stop-loss, and take-profit levels
- **TWAP Builder** — Time-weighted average price execution: split a budget into equal slices over a time window
- **Atomic TWAP Slice** — Single-order building block for agent-controlled loops (use with Claude Code `/loop`)
- **Trailing Stop Watch** — Polling loop that activates a trailing stop once a position enters profit
- **Dry-Run Mode** — Preview all mutations without executing (`--dry-run`)
- **Rich Output** — Table, JSON, CSV, quiet formats
- **Session Status Check** — Ping API + verify session token expiry + confirm exchange credentials → READY / NOT READY before starting any loop
- **Agent Skills** — Packaged as installable skills for Claude Code and compatible AI agents

---

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustc --version`)
- Cargo (`cargo --version`)

### Build : Development

AI agents do not need to proceed with the below commands. Point your agent at `.claude/skills/skill-trading/scripts/skill-trading`

```bash
# Clone the repository
git clone https://gitlab.com/tradingtoolcrypto/rust-cli-ttc-api.git
cd rust-cli-ttc-api

# Debug build (fast, for development)
make build

# Release build (optimized + copies binary to .claude/skills/skill-trading/scripts/)
make release

# Install to /usr/local/bin
make install
```

### Configure

Copy the example config and fill in your settings (Default is fine):

```bash
cp config.example.toml config.toml
cp .env.sample .env
```

> ⚠️ **Never put API keys in `config.toml`.** All secrets — exchange credentials (`{EXCHANGE}_API_KEY`, `{EXCHANGE}_API_SECRET`, `{EXCHANGE}_API_PASSPHRASE`) and TTC Box session tokens (`TTC_AUTH_TOKEN`, `TTC_PASSKEY`) — belong in `.env` only. `config.toml` is for non-secret preferences (default exchange, output format, watchlist, portfolio thresholds, market-maker tunables).

Then use a `.env` file in the working directory for all your exchange account api keys:

```env
# TTC Box session (written automatically by `register` and `login`)
TTC_AUTH_TOKEN=your_ttc_auth_token
TTC_PUBLIC_KEY=your_ttc_public_key
TTC_EMAIL=your_ttc_email
TTC_PASSKEY=your_generated_passkey
TTC_TOKEN_ISSUED_AT=unix_timestamp
TTC_EXCHANGE=orderly

# Orderly trading credentials
ORDERLY_API_KEY=your_orderly_api_key
ORDERLY_API_SECRET=your_orderly_api_secret
ORDERLY_API_PASSPHRASE=your_broker_id    # e.g. what_exchange, woofi_pro, ttc

# Required for email-registered CLI users only (not needed for Web3 users)
ORDERLY_MAIN_WALLET_ADDRESS=your_orderly_wallet_public_key
```

> **Email vs Web3 users:** If you registered via Web3, your `TTC_PUBLIC_KEY` is already your Orderly wallet — `ORDERLY_MAIN_WALLET_ADDRESS` is not needed. If you registered via email (CLI `register` command), set `ORDERLY_MAIN_WALLET_ADDRESS` to your real Orderly trading wallet public key so the server routes requests to the correct account.

### Authenticate

```bash
# Register a new account (generates wallets, encrypts keys, writes .env)
skill-trading register

# Log in to an existing account (refreshes 24h session token in .env)
skill-trading login
```

**`register`** — auto-generates a random 64-char hex passkey and a random email if none are provided. No prompts. Saves `TTC_EMAIL`, `TTC_PASSKEY`, `TTC_AUTH_TOKEN`, `TTC_PUBLIC_KEY`, and `TTC_TOKEN_ISSUED_AT` to `.env`. Never overwrites exchange API keys.

**`login`** — reads `TTC_EMAIL` and `TTC_PASSKEY` from `.env` automatically; prompts only if either is missing. Refreshes `TTC_AUTH_TOKEN` and `TTC_TOKEN_ISSUED_AT`. Session tokens expire after 24 hours.

> **Keep your passkey safe.** It encrypts your generated wallet keys — losing it means losing access to those wallets.

## Generated Wallets

During `register`, four wallets are generated client-side (Solana, Orderly, EVM main, EVM signing). Private keys are encrypted with a key derived from your passkey and email, then sent to TTC Box — they are never stored unencrypted locally.

CLI trading operations use exchange API keys, not these wallets. The wallets are used by the TTC Box web interface for Web3 authentication.

---

## Usage

> **Important:** All order commands require exactly one of `--buy` or `--sell`.

### Orders

```bash
# Place limit order
skill-trading order limit -e phemex -s BTCUSDT --buy -q 0.001 -p 95000

# Place market order
skill-trading order market -e bybit -s ETHUSDT --sell -q 0.1

# Place stop-loss order
skill-trading order stop -e binance -s BTCUSDT --sell -q 0.001 --stop-price 90000

# Place take-profit order
skill-trading order take-profit -e phemex -s BTCUSDT --sell -q 0.001 --tp-price 100000

# List open orders
skill-trading order open -e phemex

# Cancel a specific order
skill-trading order cancel -e phemex -s BTCUSDT --order-id 123456

# Cancel all orders
skill-trading order cancel-all -e phemex
```

### Positions

```bash
# List all positions
skill-trading position get -e phemex

# Filter by symbol
skill-trading position get -e phemex -s BTCUSDT

# Detailed PnL breakdown — entry, mark, % move, margin used, liquidation distance
skill-trading position pnl -e orderly -s NEARUSDT

# Close a position (market order)
skill-trading position close -e phemex -s BTCUSDT

# Close all positions
skill-trading position close-all -e phemex
```

#### Position PnL Output

```
── NEARUSDT BUY 10x ─────────────────────────────────
Size:           1160 units  ($1356.62 notional)
Entry price:    $1.2425
Mark price:     $1.1695  (-5.88% from entry)
Unrealized PnL: -52.17 USDT  (-5.88%)
Margin used:    $135.66  (10x leverage, cross mode)
Liquidation:    $1.0816  (7.52% away)
```

### Account

```bash
# Get balance
skill-trading account balance -e phemex

# Set leverage
skill-trading account leverage -e phemex -s BTCUSDT -l 20

# Set margin mode
skill-trading account margin -e phemex --mode cross -s BTCUSDT

# Enable/disable hedge mode
skill-trading account hedge -e phemex --enable
skill-trading account hedge -e phemex --disable
```

### Market Data

Market data commands hit TTC Box aggregation endpoints directly — no exchange credentials required.

```bash
# Cross-exchange tickers (spot + futures with OI and funding)
skill-trading market hybrid-tickers
skill-trading market hybrid-tickers --market-type futures
skill-trading market hybrid-tickers --symbol NEARUSDT
skill-trading market hybrid-tickers --up 5 --min-volume 1000000      # markets up 5%+ with $1M+ volume
skill-trading market hybrid-tickers --source binance                  # filter by exchange

# Funding rates across all exchanges
skill-trading market funding-rates
skill-trading market funding-rates --symbol BTCUSDT

# Open interest across all exchanges
skill-trading market open-interest
skill-trading market open-interest --symbol ETHUSDT

# 24h volume snapshot per exchange (CEX + DEX)
skill-trading market volume-snapshot

# Technical analysis scanner — entry, stop-loss, and take-profit levels
skill-trading market scanner --symbol BTCUSDT --timeframe 1h
skill-trading market scanner --symbol NEARUSDT --timeframe 4h

# Exchange-specific ticker (requires -e)
skill-trading market tickers -e phemex -s BTCUSDT

# Best bid/ask for a symbol (requires -e)
skill-trading market best-bid-ask -e phemex -s BTCUSDT

# Price alert — polling watch, fires when price crosses upper/lower
skill-trading market alert --symbol BTCUSDT --upper 100000 --lower 90000
skill-trading market alert --symbol NEARUSDT --upper 1.25 --interval 60 --continuous
```

#### Scanner Output

```
NEARUSDT / 4h — SHORT HIGH  (strength 86/100)
Entry:     $1.1720
Vola unit: $0.002344/bar (1x1)  |  Momentum: -0.003988/bar (down)  |  Avg range: $0.021650/bar
Stop Loss: $1.2033  (2.67% risk)
TP1:       $0.8874  (-24.27%)
TP2:       $0.5028  (-57.10%)
TP3:       $0.1182  (-89.91%)
R/R:       8.81x
Note:      bear composite 85.6 (score 76, R/R 8.81) vs opposite 33.1
```

The **Vola unit** line shows:
- `Vola unit` — price movement per bar at the 1x1 fan angle (the base unit for all fan lines)
- `Momentum` — actual average price change per bar over the last 20 bars (negative = downtrend)
- `Avg range` — average bar range (high - low) over 20 bars

When a signal is `NEUTRAL`, stop loss and TP levels are omitted — the API returns no levels for neutral signals.

### Risk Management

```bash
# Set stop loss
skill-trading risk sl -e phemex -s BTCUSDT --stop-price 92000

# Set take profit
skill-trading risk tp -e phemex -s BTCUSDT --tp-price 100000

# Set trailing stop (one-shot, places stop at distance% from mark price)
skill-trading risk trail -e phemex -s BTCUSDT --distance 5

# Trail watch — polling loop that activates once position enters profit,
# then trails a stop dynamically as price moves in your favour
skill-trading risk trail-watch -e orderly -s NEARUSDT --trail-pct 2 --interval 120
```

#### Trail Watch

`trail-watch` runs a foreground polling loop — no websocket required:

1. **Waiting** — polls every `--interval` seconds. Prints mark price vs entry until PnL turns positive.
2. **Activation** — once position enters profit, records peak price and places first stop at `peak × (1 - trail_pct%)`.
3. **Trailing** — each poll: if price sets a new peak, cancels old stop and places a new one at the updated trail level. Stop only moves in your favour — never backwards.
4. **Exit** — loop stops automatically when the position closes (filled stop, manual close, liquidation).

| Flag | Default | Description |
|------|---------|-------------|
| `--trail-pct` | `2.0` | Trail distance as % of peak price |
| `--interval` | `30` | Poll interval in seconds |
| `--position-side` | auto | Filter: `long`, `short`, `both` |

Press `Ctrl+C` to stop watching.

### Configuration

```bash
# Initialize config file
skill-trading config init

# Show current config (credentials masked)
skill-trading config show

# Show config file path
skill-trading config path

# Set default exchange
skill-trading config set-default phemex
```

> Exchange credentials live in `.env` (`{EXCHANGE}_API_KEY` / `{EXCHANGE}_API_SECRET` / `{EXCHANGE}_API_PASSPHRASE`) — never in `config.toml`.

### Portfolio Health

Aggregates balance and all positions into a single health report. Checks configurable risk thresholds and emits a `HEALTHY`, `WATCH`, or `DANGER` status.

```bash
skill-trading portfolio summary -e orderly

# Aliases
skill-trading port summary -e orderly
skill-trading pf summary -e orderly
```

#### Output

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

  WARNINGS
  [!] Margin utilization 89.7% exceeds threshold (80.0%)
  [!] NEARUSDT liq distance 8.88% is below threshold (10.0%)

  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  STATUS: DANGER
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

#### Risk Thresholds (config.toml)

```toml
[portfolio]
max_margin_utilization = 80.0   # % — warn if locked/balance > this
min_liq_distance_pct   = 10.0   # % — warn if any liq is this close
max_position_notional  = 5000.0 # USD — warn if single position > this
```

| Status | Condition |
|--------|-----------|
| HEALTHY | All thresholds clear |
| WATCH | Margin utilization or position size over limit |
| DANGER | Any position within `min_liq_distance_pct` of liquidation |

---

### DCA Ladder

Places multiple limit orders stepping away from the current price. Number of levels is auto-calculated from `amount / min_usd_entry` in `config.toml` (default $15).

```bash
# $150 across 10 levels, 1% apart stepping down from current price
skill-trading order dca -e orderly -s NEARUSDT --buy --amount 150 -d 1

# $300, tighter 0.5% steps
skill-trading order dca -e orderly -s BTCUSDT --buy --amount 300 -d 0.5

# Custom start price (instead of fetching current price)
skill-trading order dca -e orderly -s NEARUSDT --buy --amount 150 -d 1 --start-price 1.15

# Always dry-run first
skill-trading order dca -e orderly -s NEARUSDT --buy --amount 150 -d 1 --dry-run
```

#### DCA Ladder Output

```
  DCA Ladder — NEARUSDT BUY on orderly
  Amount:  $150.00 total  |  Levels: 10  |  Per level: $15.00
  Base:    $1.1667  |  Step: 1.00% per level  |  Min entry: $15.00
  ─────────────────────────────────────────────────────

  [1/10]   Price: $1.1667  Qty: 12  Cost: ~$15.00  Order: 20975514385
  [2/10]   Price: $1.1550  Qty: 12  Cost: ~$15.00  Order: 20975514386
  ...
  [10/10]  Price: $1.0658  Qty: 14  Cost: ~$15.00  Order: 20975514394

  ─────────────────────────────────────────────────────
  DCA complete — 10/10 levels placed
  Total allocated: $150.00  |  Total qty: 131  |  Avg price: $1.1450
```

The `min_usd_entry` setting in `config.toml` controls the granularity — lower values produce more levels for the same budget.

---

### TWAP

```bash
# Buy $1000 of NEAR over 24 hours (48 slices × $20.83, every 30 min)
skill-trading twap -e orderly -s NEARUSDT --buy --budget 1000 --hours 24

# Sell $500 of SOL over 4 hours, every 15 minutes
skill-trading twap -e orderly -s SOLUSDT --sell --budget 500 --hours 4 --interval 15

# 10 fixed slices over 6 hours (interval auto-calculated to 36 min)
skill-trading twap -e orderly -s BTCUSDT --buy --budget 1000 --hours 6 --slices 10

# Always dry-run first
skill-trading twap -e orderly -s NEARUSDT --buy --budget 1000 --hours 24 --dry-run
```

TWAP fetches the current price each interval and places a market order for `slice_usd / price` units. Skipped slices (API errors, zero qty) are logged and execution continues. Summary printed at completion showing total deployed, total qty, and average price.

### Agentic Loop Trading

The CLI supports two modes for time-based strategies:

#### Mode 1 — Unattended (CLI owns the loop)

The `twap` command runs a full internal loop — it places all slices, sleeps between them, and finishes. Set it and walk away. Crash recovery state is written after every fill.

```bash
skill-trading twap -e orderly -s NEARUSDT --buy --budget 200 --hours 1
# Resumes from last completed slice if interrupted:
skill-trading twap -e orderly -s NEARUSDT --buy --budget 200 --hours 1 --resume
```

#### Mode 2 — Agentic (Agent owns the loop)

Use `twap-slice` — a single atomic market order for a fixed USD amount — combined with Claude Code's built-in `/loop` scheduler. The agent places one slice per tick, reads every result, and can react between fills.

```bash
# One slice, one call:
skill-trading twap-slice -e orderly -s NEARUSDT --buy --amount 15 --decimals 0 --label "1/13"
# Output: SLICE [1/13]  NEARUSDT BUY  Price: $1.1997  Qty: 12  Cost: ~$15.00  Order: 20975495013
```

With Claude Code's `/loop`:
```
/loop 5m: place one NEARUSDT buy slice of $15 using twap-slice, track total deployed, stop at $200
```

On each tick Claude places one slice, sees the price and fill, updates its running total, and cancels the loop when the budget is exhausted. The agent is never blind.

**When to use which:**

| Scenario | Use |
|----------|-----|
| Overnight unattended TWAP | `twap` (built-in loop) |
| Active session, want visibility | `/loop` + `twap-slice` |
| Trailing stop while you watch | `/loop` + `position get` + `risk sl` |
| Trailing stop while you sleep | `risk trail-watch` |

**`/loop` reference:**
- Built into Claude Code — minimum 1 minute interval, default 10 minutes
- Up to 50 simultaneous loops per session, auto-expires after 3 days
- Stop with: "cancel the loop" or exit the session

See `skills/skill-loop-trading/SKILL.md` for the full agent loop protocol.

---

### Session Status Check

Run before any loop or automated workflow to verify the connection and session are healthy:

```bash
skill-trading status
```

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  SKILL-TRADING STATUS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ✓  TTC Box API
  ✓  Session token               VALID  23h 43m remaining
  ✓  Exchange credentials        orderly configured

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  STATUS: READY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

Three checks run concurrently:
1. **TTC Box API** — raw HTTP ping to a public market endpoint; verifies network reachability
2. **Session token** — confirms `TTC_AUTH_TOKEN` is set and `TTC_TOKEN_ISSUED_AT` is within the 24h window; shows exact time remaining
3. **Exchange credentials** — scans env vars, config.toml, and per-exchange vars for at least one usable API key + secret pair

Exit code is `0` (READY) or `1` (NOT READY) — use as a gate in shell scripts or agent pre-checks.

### Morning Brief

Single-command pre-session briefing: session check + watchlist prices + scanner signals + portfolio health + open orders.

```bash
skill-trading brief -e orderly
skill-trading brief -e orderly --watchlist NEARUSDT,BTCUSDT,SOLUSDT --timeframe 4h

# Aliases
skill-trading morning -e orderly
skill-trading mb -e orderly
```

### Market Maker

Limit-order spread capture loop. Enters at the best bid (or ask) and exits at entry ± spread. Designed for zero-fee exchanges where round-trip commission doesn't eat the spread.

```bash
# Enter long at best bid, exit at bid + 0.1% — default behaviour
skill-trading market-maker -e orderly -s NEARUSDT --buy -q 12 --spread-pct 0.1

# Alias
skill-trading mm -e orderly -s NEARUSDT --buy -q 12
```

Commission and other tunables live under `[market-maker]` in `config.toml`.

### Authentication

```bash
# Register a new TTC Box account
skill-trading register
skill-trading register --email you@example.com

# Log in to an existing account
skill-trading login
skill-trading login --email you@example.com

# Show version
skill-trading info
```

---

## Global Flags

| Flag | Env Variable | Description |
|------|-------------|-------------|
| `--config <path>` | `TTC_CONFIG` | Path to config file |
| `--api-key <key>` | `TTC_AUTH_TOKEN` | TTC Box auth token |
| `--public-key <key>` | `TTC_PUBLIC_KEY` | TTC Box public key |
| `--exchange-api-key <key>` | `EXCHANGE_API_KEY` | Exchange API key |
| `--exchange-api-secret <secret>` | `EXCHANGE_API_SECRET` | Exchange API secret |
| `--exchange-api-passphrase <pp>` | `EXCHANGE_API_PASSPHRASE` | Exchange passphrase (Orderly, OKX, KuCoin, Bitget, BloFin) |
| `-e, --exchange <name>` | `TTC_EXCHANGE` | Default exchange name |
| `--output-format <fmt>` | `TTC_OUTPUT` | Output format: table, json, csv, quiet |
| `--dry-run` | | Preview without executing |
| `-v, --verbose` | | Enable debug logging |
| `--no-color` | `NO_COLOR` | Disable colored output |

---

## Output Formats

```bash
skill-trading position get -e phemex --output-format table    # default, human-readable
skill-trading position get -e phemex --output-format json     # for scripting
skill-trading position get -e phemex --output-format csv      # with headers, for spreadsheets
skill-trading position get -e phemex --output-format quiet    # minimal output
```

---

## Dry-Run Mode

All mutation commands support `--dry-run`. No API calls are made — the CLI shows what would happen.

```bash
skill-trading --dry-run order limit -e phemex -s BTCUSDT --buy -q 0.001 -p 95000
# DRY-RUN Would place limit order: buy 0.001 BTCUSDT @ 95000
```

---

## Configuration Priority

| Priority | Source | Example |
|----------|--------|---------|
| 1 | CLI flags | `--exchange-api-key abc123` |
| 2 | Environment variables / `.env` | `ORDERLY_API_KEY=abc123` |
| 3 | `config.toml` (non-secret preferences only) | `default_leverage = 10` |
| 4 | Built-in defaults | |

> ⚠️ **API keys never go in `config.toml`.** For multi-exchange setups, put each exchange's credentials in `.env` using the `{EXCHANGE}_API_KEY` / `{EXCHANGE}_API_SECRET` / `{EXCHANGE}_API_PASSPHRASE` pattern:
>
> ```env
> # .env
> PHEMEX_API_KEY=YOUR_PHEMEX_KEY
> PHEMEX_API_SECRET=YOUR_PHEMEX_SECRET
>
> ORDERLY_API_KEY=YOUR_ORDERLY_KEY
> ORDERLY_API_SECRET=YOUR_ORDERLY_SECRET
> ORDERLY_API_PASSPHRASE=what_exchange     # broker ID
>
> OKX_API_KEY=YOUR_OKX_KEY
> OKX_API_SECRET=YOUR_OKX_SECRET
> OKX_API_PASSPHRASE=YOUR_OKX_PASSPHRASE
> ```
>
> Switch the active exchange with `-e <name>` per command or `TTC_EXCHANGE=<name>` in `.env`.

### Config File Discovery

1. `--config <path>` flag or `TTC_CONFIG` env var
2. `./config.toml` in the current directory
3. `~/Library/Application Support/com.ttcbox.skill-trading/config.toml` (macOS)

---

## Command Aliases

| Full Command | Aliases |
|-------------|---------|
| `position` | `positions`, `pos` |
| `account` | `acct` |
| `market` | `m` |
| `login` | `auth` |
| `portfolio` | `port`, `pf` |
| `brief` | `morning`, `mb` |
| `market-maker` | `mm` |
| `info` | `version` |
| `order open` | `order list`, `order ls` |
| `order cancel` | `order cxl` |
| `order cancel-all` | `order cxl-all` |
| `position get` | `position list`, `position ls` |
| `position close` | `position exit` |
| `account balance` | `account bal` |
| `account leverage` | `account lev` |
| `market tickers` | `market t` |
| `market best-bid-ask` | `market bb`, `market book` |
| `market hybrid-tickers` | `market ht`, `market agg` |
| `market funding-rates` | `market fr`, `market funding` |
| `market open-interest` | `market oi` |
| `market volume-snapshot` | `market vol`, `market vs` |
| `market scanner` | `market scan` |
| `market alert` | `market watch`, `market al` |

---

## Project Structure

```
rust-cli-ttc-api/
├── src/
│   ├── main.rs               # Entry point, CLI parsing, command dispatch
│   ├── cli.rs                # Clap subcommand and argument definitions
│   ├── config.rs             # Configuration loading and credential resolution
│   ├── crypto.rs             # Wallet generation, PBKDF2 key derivation, AES encryption
│   ├── error.rs              # Error types (TtcError)
│   ├── models.rs             # API request/response data models
│   ├── api/
│   │   └── client.rs         # HTTP client, retry logic, all API methods
│   ├── commands/
│   │   ├── common.rs         # Shared helpers (get_credentials)
│   │   ├── login.rs          # Login flow
│   │   ├── register.rs       # Registration + wallet generation
│   │   ├── order.rs          # Order placement and cancellation
│   │   ├── orders.rs         # Order listing and bulk cancel
│   │   ├── position.rs       # Position viewing and closing
│   │   ├── account.rs        # Balance, leverage, margin, hedge mode
│   │   ├── risk.rs           # Stop loss, take profit, trailing stop
│   │   ├── config.rs         # Config management
│   │   └── market.rs         # All market data commands
│   └── output/
│       └── printer.rs        # Formatted output (table, JSON, CSV, quiet)
├── .claude/skills/
│   ├── skill-trading/
│   │   ├── SKILL.md          # Agent instructions (agentskills.io format)
│   │   ├── scripts/
│   │   │   ├── skill-trading              # POSIX launcher — execs the right platform binary
│   │   │   ├── skill-trading-darwin-arm64 # Mach-O, Apple Silicon (dev machine)
│   │   │   └── skill-trading-linux-x64    # ELF, Linux x86_64 (VPS/Railway/Docker)
│   │   └── references/
│   │       └── exchanges.md  # Supported exchanges and credential guide
│   └── skill-*/                # One folder per agent skill (see Skills table below)
├── Cargo.toml
├── Makefile
└── config.example.toml
```

---

## Skills

This project ships eleven [agentskills.io](https://agentskills.io) compatible skills:

| Skill | Purpose |
|-------|---------|
| **skill-onboarding** | First-run setup and authentication: install check → .env → login/register → verify READY |
| **skill-trading** | Core safe-trading protocol: pre-order checklists, order placement rules, output interpretation |
| **skill-shark** | Signal-driven bracketed trade setup (entry + TP1 + TP2), requires R/R ≥ 2.0 |
| **skill-twap** | TWAP execution guide: calculations, checklist, output interpretation |
| **skill-dca** | DCA ladder builder — stepped limit orders to scale into longs or shorts at better prices |
| **skill-loop-trading** | Agent-controlled loop trading via `/loop` + `twap-slice` — agent owns the loop |
| **skill-market-overview** | BTC/ETH trend + funding sentiment + OI distribution briefing |
| **skill-momentum** | Finds 10%+ movers with volume, scans for signals |
| **skill-signal-patrol** | Scans a fixed watchlist for HIGH confidence R/R ≥ 3.0 setups |
| **skill-portfolio-manager** | Portfolio health monitoring: balance + positions → HEALTHY/WATCH/DANGER status with risk thresholds |
| **skill-market-maker** | Automated limit-order market-making loop: enter at best bid/ask, exit at entry ± spread |

The compiled binaries at `.claude/skills/skill-trading/scripts/` are kept up to date by `make release-all` — run it after **every** change under `src/` so both the macOS arm64 binary (for the dev machine) and the linux-x64 binary (for the VPS) ship together. Each skill folder is self-contained and shareable.

---

## Development

```bash
make build      # debug build
make release    # host-native release → .claude/skills/skill-trading/scripts/skill-trading-<host-suffix>
make release-linux  # cross-compile linux-x64 via `cross` (requires Docker Desktop running)
make release-all    # release + release-linux — run this after EVERY source change, before committing
make install    # release + install to /usr/local/bin
make test       # run tests
make clippy     # lint
make fmt        # format
make clean      # remove build artifacts
make help       # show all targets
```

---

## API Reference

All exchange operations are proxied through TTC Box:

```
POST https://ttc.box/api/v1/exchanges
```

Market data endpoints are called directly:

```
GET  https://ttc.box/api/v1/markets/hybrid-tickers
GET  https://ttc.box/api/v1/markets/funding-rates
POST https://ttc.box/api/v1/markets/open-interest
GET  https://ttc.box/api/v1/markets/volume-snapshot
GET  https://ttc.box/api/v1/markets/ttc-scanner
```

Headers required for all requests:
- `ttc-auth-token` — your TTC auth token
- `ttc-public-key` — your TTC public key

---

## License

Apache License 2.0 — see [LICENSE.txt](LICENSE.txt). © TTC Box

## Author

ttc.box: Tetrac
