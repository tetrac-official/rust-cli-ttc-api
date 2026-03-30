# Skill Trading CLI

Execute trading operations on TTC Box across 15+ exchanges.

Place orders, manage positions, scan markets, and control risk. Designed for AI agents and automated trading workflows.

> **First time here?** Read [`GETTING_STARTED.md`](./GETTING_STARTED.md) — it walks through the full setup and first trade end-to-end.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Proprietary-blue.svg)]()
[![Version](https://img.shields.io/badge/version-1.0.0-green.svg)]()

---

## Features

- **Email Registration & Login** — Create and authenticate TTC Box accounts from the terminal
- **Client-Side Wallet Generation** — Solana, Orderly, and EVM wallets generated and encrypted locally; private keys never sent in plaintext
- **15+ Exchanges** — Orderly, Phemex, Bybit, Binance, OKX, Bitget, BloFin, KuCoin, Hyperliquid, AsterDEX, BingX, and more
- **Full Order Management** — Limit, market, stop-loss, take-profit orders
- **Position Control** — View, close, and manage positions
- **Account Operations** — Balance, leverage, margin mode, hedge mode
- **Market Data** — Cross-exchange tickers, funding rates, open interest, volume snapshots
- **TTC Scanner** — Technical analysis signal with entry, stop-loss, and take-profit levels
- **TWAP Builder** — Time-weighted average price execution: split a budget into equal slices over a time window
- **Atomic TWAP Slice** — Single-order building block for agent-controlled loops (use with Claude Code `/loop`)
- **Trailing Stop Watch** — Polling loop that activates a trailing stop once a position enters profit
- **Dry-Run Mode** — Preview all mutations without executing (`--dry-run`)
- **Rich Output** — Table, JSON, CSV, quiet formats
- **Agent Skills** — Packaged as installable skills for Claude Code and compatible AI agents

---

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustc --version`)
- Cargo (`cargo --version`)

### Build

```bash
# Clone the repository
git clone https://gitlab.com/tradingtoolcrypto/rust-cli-ttc-api.git
cd rust-cli-ttc-api

# Debug build (fast, for development)
make build

# Release build (optimized + copies binary to skills/skill-trading/scripts/)
make release

# Install to /usr/local/bin
make install
```

### Configure

Copy the example config and fill in your settings (Default is fine):

```bash
cp config.example.toml config.toml
```

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

# Close a position (market order)
skill-trading position close -e phemex -s BTCUSDT

# Close all positions
skill-trading position close-all -e phemex
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
```

#### Scanner Output

```
NEARUSDT / 4h — SHORT HIGH  (strength 86/100)
Entry:     $1.1720
Gann unit: $0.002344/bar (1x1)  |  Momentum: -0.003988/bar (down)  |  Avg range: $0.021650/bar
Stop Loss: $1.2033  (2.67% risk)
TP1:       $0.8874  (-24.27%)
TP2:       $0.5028  (-57.10%)
TP3:       $0.1182  (-89.91%)
R/R:       8.81x
Note:      bear composite 85.6 (score 76, R/R 8.81) vs opposite 33.1
```

The **Gann unit** line shows:
- `Gann unit` — price movement per bar at the 1x1 fan angle (the base unit for all fan lines)
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

# Add exchange credentials
skill-trading config add-exchange phemex --api-key KEY --api-secret SECRET

# Remove exchange credentials
skill-trading config rm-exchange phemex
```

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
| 1 | CLI flags | `--api-key abc123` |
| 2 | Environment variables / `.env` | `TTC_AUTH_TOKEN=abc123` |
| 3 | `config.toml` | `api_key = "abc123"` |
| 4 | Built-in defaults | |

For multi-exchange setups, use exchange-specific sections in `config.toml`:

```toml
[exchanges.phemex]
api_key = "YOUR_PHEMEX_KEY"
api_secret = "YOUR_PHEMEX_SECRET"

[exchanges.orderly]
api_key = "YOUR_ORDERLY_KEY"
api_secret = "YOUR_ORDERLY_SECRET"
passphrase = "what_exchange"   # broker ID

[exchanges.okx]
api_key = "YOUR_OKX_KEY"
api_secret = "YOUR_OKX_SECRET"
passphrase = "YOUR_OKX_PASSPHRASE"
```

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
| `orders` | `o` |
| `market` | `m` |
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
| `orders get` | `orders list`, `orders ls` |
| `orders cancel-all` | `orders cancel-all` |
| `info` | `version` |

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
├── skills/
│   ├── skill-trading/
│   │   ├── SKILL.md          # Agent instructions (agentskills.io format)
│   │   ├── scripts/
│   │   │   └── skill-trading # Compiled release binary
│   │   └── references/
│   │       └── exchanges.md  # Supported exchanges and credential guide
│   └── skill-shark/
│       ├── SKILL.md          # Signal-driven trade setup strategy
│       └── references/
│           └── setup-guide.md
├── Cargo.toml
├── Makefile
└── config.example.toml
```

---

## Skills

This project ships six [agentskills.io](https://agentskills.io) compatible skills:

| Skill | Purpose |
|-------|---------|
| **skill-trading** | Core safe-trading protocol: pre-order checklists, order placement rules, output interpretation |
| **skill-shark** | Signal-driven bracketed trade setup (entry + TP1 + TP2), requires R/R ≥ 2.0 |
| **skill-twap** | TWAP execution guide: calculations, checklist, output interpretation |
| **skill-loop-trading** | Agent-controlled loop trading via `/loop` + `twap-slice` — agent owns the loop |
| **skill-market-overview** | BTC/ETH trend + funding sentiment + OI distribution briefing |
| **skill-momentum** | Finds 10%+ movers with volume, scans for signals |
| **skill-signal-patrol** | Scans a fixed watchlist for HIGH confidence R/R ≥ 3.0 setups |

The compiled binary at `skills/skill-trading/scripts/skill-trading` is kept up to date by `make release`. Each skill folder is self-contained and shareable.

---

## Development

```bash
make build      # debug build
make release    # release build + copies binary to skills/skill-trading/scripts/
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

Proprietary — TTC Box

## Author

ttcbox
