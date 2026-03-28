# Skill Trading CLI

Execute trading operations on TTC Box across 15+ exchanges.

Place orders, manage positions, scan markets, and control risk. Designed for AI agents and automated trading workflows.

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
git clone https://github.com/ttcbox/rust-cli-ttc-api.git
cd rust-cli-ttc-api

# Debug build (fast, for development)
make build

# Release build (optimized + copies binary to skills/skill-trading/scripts/)
make release

# Install to /usr/local/bin
make install
```

### Configure

Copy the example config and fill in your credentials:

```bash
cp config.example.toml config.toml
```

Or use a `.env` file in the working directory:

```env
TTC_AUTH_TOKEN=your_ttc_auth_token
TTC_PUBLIC_KEY=your_ttc_public_key
TTC_EXCHANGE=orderly

EXCHANGE_API_KEY=your_exchange_key
EXCHANGE_API_SECRET=your_exchange_secret
EXCHANGE_API_PASSPHRASE=your_passphrase   # required for Orderly, OKX, KuCoin, Bitget, BloFin
```

### Authenticate

```bash
# Register a new account (generates wallets, encrypts keys, writes .env)
skill-trading register

# Log in to an existing account (refreshes 24h session token in .env)
skill-trading login
```

Both commands prompt for email and passkey (hidden input). Session tokens expire after 24 hours — run `login` again to refresh.

> **Keep your passkey safe.** It is the only way to recover your encrypted wallet keys.

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
BTCUSDT / 4h — LONG HIGH  (strength 72/100)
Entry:     $65839.4000
Stop Loss: $65439.1599  (0.61% risk)
TP1:       $68258.7399  (+3.67%)
TP2:       $76717.4798  (+16.52%)
TP3:       $93634.9596  (+42.22%)
R/R:       6.04x
Note:      bull composite 72.4 (score 54, R/R 6.04) vs opposite 44.7
```

### Risk Management

```bash
# Set stop loss
skill-trading risk sl -e phemex -s BTCUSDT --stop-price 92000

# Set take profit
skill-trading risk tp -e phemex -s BTCUSDT --tp-price 100000

# Set trailing stop
skill-trading risk trail -e phemex -s BTCUSDT --distance 5
```

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

This project ships two [agentskills.io](https://agentskills.io) compatible skills:

### skill-trading

Teaches an AI agent how to safely use this CLI — pre-order checklists, order placement rules, balance validation, and all available commands.

### skill-shark

A signal-driven trade setup strategy. The agent scans a market using the TTC Scanner, checks that R/R ≥ 2.0, then sizes and places a bracketed order (entry + TP1 + TP2). If R/R is too low, it hunts for a better market via open interest or momentum filters.

The compiled binary at `skills/skill-trading/scripts/skill-trading` is kept up to date by `make release`. To distribute the skill, share the `skills/skill-trading/` folder — the binary is self-contained.

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
