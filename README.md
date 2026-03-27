# Skill Trading CLI

Execute trading operations on TTC Box across 15+ exchanges.

Place orders, manage positions, set leverage, and control risk. Designed for AI agents and automated trading workflows.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Proprietary-blue.svg)]()
[![Version](https://img.shields.io/badge/version-1.0.0-green.svg)]()

---

## Features

- **Email Registration & Login** — Create and authenticate TTC Box accounts from the terminal
- **Client-Side Wallet Generation** — Solana, Orderly, and EVM wallets generated and encrypted locally; private keys never sent in plaintext
- **15+ Exchanges** — Phemex, Bybit, Binance, OKX, and more
- **Full Order Management** — Limit, market, stop-loss, take-profit orders
- **Position Control** — View, close, and manage positions
- **Account Operations** — Balance, leverage, margin mode
- **Risk Management** — Stop losses, take profits, trailing stops
- **Dry-Run Mode** — Preview all mutations without executing (`--dry-run`)
- **Rich Output** — Table, JSON, CSV (with headers), quiet formats
- **AI Compatible** — Designed for Claude Code and AI agents

---

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustc --version`)
- Cargo (`cargo --version`)

### Build & Run

```bash
# Clone the repository
git clone https://github.com/ttcbox/rust-cli-ttc-api.git
cd rust-cli-ttc-api

# Build
cargo build --release

# Run
./target/release/skill-trading --help
```

### Configure

```bash
# 1. Copy example config
cp config.example.toml config.toml

# 2. Edit with your credentials
nano config.toml

# 3. Or use environment variables
export TTC_AUTH_TOKEN="your-ttc-auth-token"
export TTC_PUBLIC_KEY="your-ttc-public-key"
export EXCHANGE_API_KEY="your-exchange-key"
export EXCHANGE_API_SECRET="your-exchange-secret"
```

### Authenticate

The quickest way to get started is to register or log in directly from the terminal.

```bash
# Register a new account (generates wallets, encrypts keys, writes .env)
cd ~/Documents/rust-cli-ttc-api
./target/release/skill-trading register

# Log in to an existing account (writes fresh session token to .env)
./target/release/skill-trading login
```

Both commands prompt for email and passkey (hidden input), then write `TTC_AUTH_TOKEN` and `TTC_PUBLIC_KEY` to `.env`. Session tokens expire after 24 hours — just run `login` again to refresh.

> **Keep your passkey safe.** It is the only way to recover your encrypted wallet keys.

### Place Your First Order

```bash
# Dry run first (no real execution)
skill-trading --dry-run order market \
  --exchange phemex \
  --symbol BTCUSDT \
  --buy \
  --quantity 0.001

# Execute for real
skill-trading order market \
  --exchange phemex \
  --symbol BTCUSDT \
  --buy \
  --quantity 0.001
```

---

## Usage

> **Important:** All order commands require exactly one of `--buy` or `--sell`. Omitting both will return an error.

### Orders

```bash
# Place limit order
skill-trading order limit -e phemex -s BTCUSDT --buy -q 0.001 -p 95000

# Place market order
skill-trading order market -e bybit -s ETHUSDT --sell -q 0.1

# Place stop-loss order
skill-trading order stop -e binance -s BTCUSDT --sell -q 0.001 -s 90000

# Place take-profit order
skill-trading order take-profit -e phemex -s BTCUSDT --sell -q 0.001 -t 100000

# List open orders
skill-trading order open -e phemex

# Cancel order
skill-trading order cxl -e phemex -s BTCUSDT --order-id 123456

# Cancel all orders
skill-trading order cxl-all -e phemex
```

### Positions

```bash
# List positions
skill-trading position get -e phemex

# List positions for specific symbol
skill-trading position get -e phemex -s BTCUSDT

# Close position
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

# Enable hedge mode
skill-trading account hedge -e phemex --enable
```

### Risk Management

```bash
# Set stop loss (auto-detects position side and size)
skill-trading risk sl -e phemex -s BTCUSDT --stop-price 92000

# Set take profit
skill-trading risk tp -e phemex -s BTCUSDT --tp-price 100000

# Set trailing stop (percentage of mark price)
# Example: 5% trailing distance
skill-trading risk trail -e phemex -s BTCUSDT --distance 5
```

### Configuration

```bash
# Initialize config
skill-trading config init

# Show current config (credentials are masked)
skill-trading config show

# Show config path
skill-trading config path

# Set default exchange
skill-trading config set-default phemex

# Add exchange credentials
skill-trading config add-exchange phemex --auth-token KEY --api-secret SECRET
```

### Authentication

```bash
# Register a new TTC Box account
# Generates Solana, Orderly, and EVM wallets client-side,
# encrypts all private keys with your passkey, and posts to the server.
skill-trading register
skill-trading register --email you@example.com

# Log in to an existing account
# Verifies credentials and refreshes the 24h session token in .env
skill-trading login
skill-trading login --email you@example.com
```

### Info

```bash
# Show version
skill-trading info
```

---

## Global Flags

| Flag | Env Variable | Description |
|------|-------------|-------------|
| `--config <path>` | `TTC_CONFIG` | Path to config file |
| `--auth-token <key>` | `TTC_AUTH_TOKEN` | TTC Box API key |
| `--public-key <key>` | `TTC_PUBLIC_KEY` | TTC Box public key |
| `--exchange-api-key <key>` | `EXCHANGE_API_KEY` | Default exchange API key |
| `--exchange-api-secret <secret>` | `EXCHANGE_API_SECRET` | Default exchange API secret |
| `--exchange-api-passphrase <pp>` | `EXCHANGE_API_PASSPHRASE` | Exchange API passphrase (OKX, KuCoin, Orderly, Bitget, BloFin) |
| `-e, --exchange <name>` | `TTC_EXCHANGE` | Default exchange name |
| `--output-format <fmt>` | `TTC_OUTPUT` | Output format: table, json, csv, quiet |
| `--dry-run` | | Preview without executing |
| `-v, --verbose` | | Enable debug logging |
| `--no-color` | `NO_COLOR` | Disable colored output |

---

## Output Formats

```bash
# Table (default) — human-readable
skill-trading position get -e phemex --output-format table

# JSON — for scripting and piping
skill-trading position get -e phemex --output-format json

# CSV — with headers, for spreadsheets and analysis
skill-trading position get -e phemex --output-format csv

# Quiet — minimal, IDs only
skill-trading position get -e phemex --output-format quiet
```

---

## Dry-Run Mode

All mutation commands (order placement, cancellation, position close, leverage/margin/hedge changes) support `--dry-run`. This previews what would happen without making any API calls.

```bash
skill-trading --dry-run order limit -e phemex -s BTCUSDT --buy -q 0.001 -p 95000
# Output: DRY-RUN Would place limit order: buy 0.001 BTCUSDT @ 95000
```

---

## Configuration Priority

All settings follow this override order (highest priority wins):

| Priority | Source | Example |
|----------|--------|---------|
| 1 | **CLI flags** | `--auth-token abc123` |
| 2 | **Environment variables** (`.env`) | `TTC_AUTH_TOKEN=abc123` |
| 3 | **config.toml** | `api_key = "abc123"` |
| 4 | **Built-in defaults** | `AppConfig::default()` |

CLI flags and environment variables are resolved together by clap — if a flag is provided, it takes precedence over the corresponding env var. The result then overwrites whatever was loaded from `config.toml`. If none are set, built-in defaults apply.

### Credential Resolution

For **exchange credentials** specifically, there is an additional layer within `config.toml`:

1. **CLI flags** — `--exchange-api-key`, `--exchange-api-secret`, and `--exchange-api-passphrase`
2. **Exchange-specific config** — `[exchanges.phemex]` section in `config.toml`
3. **Global config/env** — `exchange_api_key` in `config.toml` or `EXCHANGE_API_KEY` / `EXCHANGE_API_SECRET` / `EXCHANGE_API_PASSPHRASE` env vars

Some exchanges require a **passphrase** as a third credential (OKX, KuCoin, Orderly, Bitget, BloFin). Pass it via `--passphrase`, `EXCHANGE_API_PASSPHRASE` env var, or per-exchange config.

For multi-exchange setups, use exchange-specific sections in `config.toml`:

```toml
[exchanges.phemex]
api_key = "YOUR_PHEMEX_API_KEY"
api_secret = "YOUR_PHEMEX_API_SECRET"

[exchanges.bybit]
api_key = "YOUR_BYBIT_API_KEY"
api_secret = "YOUR_BYBIT_API_SECRET"

# Exchanges requiring passphrase (OKX, KuCoin, Orderly, Bitget, BloFin)
[exchanges.okx]
api_key = "YOUR_OKX_API_KEY"
api_secret = "YOUR_OKX_API_SECRET"
passphrase = "YOUR_OKX_PASSPHRASE"
```

### Config File Discovery

The CLI looks for `config.toml` in this order:

1. Explicit `--config <path>` flag or `TTC_CONFIG` env var
2. `./config.toml` in the current directory
3. User-level config at `~/Library/Application Support/com.ttcbox.skill-trading/config.toml` (macOS)

---

## Command Aliases

| Full Command | Aliases |
|-------------|---------|
| `position` | `positions`, `pos` |
| `account` | `acct` |
| `orders` | `o` |
| `market` | `m` |
| `order open` | `order list`, `order ls` |
| `order cxl` | `order cancel` |
| `order cxl-all` | `order cancelall` |
| `position get` | `position list`, `position ls` |
| `position close` | `position exit` |
| `account balance` | `account bal` |
| `account leverage` | `account lev` |
| `market tickers` | `market t` |
| `market best-bid-ask` | `market bb`, `market book` |
| `orders get` | `orders list`, `orders ls` |
| `orders cancel-all` | `orders cancel-all` |
| `info` | `version` |

---

## Project Structure

```
src/
├── main.rs               # Entry point, CLI parsing, command dispatch
├── lib.rs                # Library re-exports
├── cli.rs                # Clap subcommand and argument definitions
├── config.rs             # Configuration loading and credential resolution
├── crypto.rs             # Wallet generation, PBKDF2 key derivation, AES encryption
├── error.rs              # Error types (TtcError)
├── models.rs             # API request/response data models
├── api/
│   ├── mod.rs            # API module exports
│   └── client.rs         # HTTP client with retry logic
├── commands/
│   ├── mod.rs            # Command module exports
│   ├── common.rs         # Shared helpers (get_credentials, convert_position_side)
│   ├── login.rs          # Login with email + passkey, writes .env
│   ├── register.rs       # Register new account, generates wallets, writes .env
│   ├── order.rs          # Order placement and cancellation
│   ├── orders.rs         # Order listing and bulk cancel
│   ├── position.rs       # Position viewing and closing
│   ├── account.rs        # Balance, leverage, margin, hedge mode
│   ├── risk.rs           # Stop loss, take profit, trailing stop
│   ├── config.rs         # Config init, show, path
│   └── market.rs         # Tickers and best bid/ask
└── output/
    ├── mod.rs            # Output format enum
    └── printer.rs        # Formatted output (table, JSON, CSV, quiet)
```

---

## Development

```bash
# Debug build
cargo build

# Run with arguments
cargo run -- order open -e phemex

# Run with local config
cargo run -- --config config.toml config show

# Run tests
cargo test

# Lint
cargo clippy

# Format
cargo fmt

# Release build (optimized, stripped)
cargo build --release
```

---

## API Reference

### TTC Box API Endpoint

```
POST https://ttc.box/api/v1/exchanges
```

Headers:
- `ttc-auth-token`: Your TTC Auth Token
- `ttc-public-key`: Your TTC public key
- `Content-Type`: application/json

---

## Supported Exchanges

Phemex, Bybit, Binance, OKX, Bitget, KuCoin, Gate.io, MEXC, HTX (Huobi), Kraken, Coinbase, and more.

---

## License

Proprietary - TTC Box

## Author

ttcbox
