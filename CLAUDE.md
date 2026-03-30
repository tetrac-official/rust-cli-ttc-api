# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
make build      # debug build → target/debug/skill-trading
make release    # optimized build + copy binary to skills/skill-trading/scripts/
make install    # copy to /usr/local/bin (requires make release first)
make test       # run all tests
make clippy     # lint
make fmt        # format with rustfmt
make dist       # cross-platform builds (darwin-arm64, darwin-x64, linux-x64, windows-x64)
```

Run a single test:
```bash
cargo test <test_name>
cargo test --lib <module>::<test_name>
```

## Architecture

The binary (`skill-trading`) is a multi-exchange trading CLI proxied entirely through the TTC Box API (`https://ttc.box/api/v1`). All exchange operations route through TTC Box — the CLI never calls exchange APIs directly.

### Module Map

| Module | Purpose |
|--------|---------|
| `src/main.rs` | Entry point — config loading, logging, CLI dispatch |
| `src/cli.rs` | All clap command/argument definitions (derives) |
| `src/api/client.rs` | HTTP client, retry logic, all TTC Box API methods |
| `src/models.rs` | API request/response DTOs, enums (`OrderSide`, `PositionSide`, etc.) |
| `src/config.rs` | `AppConfig` struct, config file loading, priority resolution |
| `src/crypto.rs` | Wallet generation (Ed25519/secp256k1), PBKDF2, AES-256-CBC encryption |
| `src/commands/` | One file per top-level command group |
| `src/output/printer.rs` | Format-aware output (table/JSON/CSV/quiet) |
| `src/error.rs` | `TtcError` enum with retryable classification |

### Configuration Priority

CLI flags > environment variables / `.env` > `config.toml` > built-in defaults.

Config file is discovered in order: `--config` flag → `TTC_CONFIG` env var → `./config.toml` → platform default (`~/.config/skill-trading/config.toml`).

### Key Design Patterns

- **All async**: entire CLI runs on tokio; all API calls are async.
- **Retry logic**: up to 3 retries with exponential backoff in `client.rs` for network/rate-limit errors.
- **Output**: `Printer` dispatches to table/JSON/CSV/quiet based on `--output-format` flag or `TTC_OUTPUT` env var.
- **Crypto**: wallet keys are generated and encrypted locally (PBKDF2-SHA1 + AES-256-CBC) before transmission — private keys never sent in plaintext.
- **Dry-run**: `--dry-run` flag skips order execution throughout all commands.

### Commands Overview

| Command | Aliases | Purpose |
|---------|---------|---------|
| `order` | — | Limit, market, stop, take-profit placement; cancel; cancel-all; list open |
| `position` | `pos`, `positions` | Get, close, close-all |
| `account` | `acct` | Balance, leverage, margin mode, hedge mode |
| `orders` | `o` | Bulk order get/cancel-all |
| `market` | `m` | Tickers, funding rates, OI, volume snapshot, TTC scanner |
| `risk` | — | Stop-loss (`sl`), take-profit (`tp`), trailing stop (`trail`), polling trail watcher (`trail-watch`) |
| `config` | — | Init, show, path, set-default, add/rm exchange |
| `login` | `auth` | TTC Box login |
| `register` | — | TTC Box registration + local wallet generation |
| `twap` | — | Time-weighted average price position builder (polling loop, market orders) |

Market data commands (`hybrid-tickers`, `funding-rates`, `open-interest`, `volume-snapshot`, `scanner`) require no API key.

### Scanner output notes
- All prices display 4 decimal places throughout the CLI.
- Scanner includes a **Gann unit** line: `$X/bar (1x1) | Momentum: ±Y/bar (direction) | Avg range: $Z/bar`. Multiply the Gann unit by the fan ratio (2, 3, 4…) to get the slope of steeper fan lines.
- When signal is `NEUTRAL`, stop loss and TP levels are `null` from the API and omitted from output — this is expected, not a bug.
- Descending Gann fan lines from a high pivot can project below zero after many bars — mathematically valid, not an error.

### `risk trail-watch`
Polling trailing stop — activates once position enters profit, then trails stop at `peak × (1 - trail_pct%)` for longs, `peak × (1 + trail_pct%)` for shorts. Reads actual position side from the exchange and places a SELL stop for long positions, BUY stop for short positions, always `reduce_only`. Cancels and replaces stop only when the new level improves on the previous one. Stops automatically when position closes. Flags: `--trail-pct` (default 2.0%), `--interval` (default 30s).

## Skills

The `skills/` directory contains five AI agent instruction sets (agentskills.io format):

- **skill-trading** — Core safe-trading protocol: pre-order checklists, order placement rules, output interpretation
- **skill-shark** — Signal-driven bracketed trade setup (entry + TP1 + TP2), requires R/R ≥ 2.0
- **skill-market-overview** — BTC/ETH trend + funding sentiment + OI distribution briefing
- **skill-momentum** — Finds 10%+ movers with volume, scans for signals
- **skill-signal-patrol** — Scans a fixed watchlist for HIGH confidence R/R ≥ 3.0 setups

`make release` compiles the binary and copies it into `skills/skill-trading/scripts/` for distribution. Each skill folder is self-contained and shareable.

## Environment / Credentials

See `.env.sample` for all supported variables. Key ones:
- `TTC_AUTH_TOKEN` / `TTC_PUBLIC_KEY` — session credentials (24h expiry)
- `TTC_EXCHANGE` — default exchange
- `TTC_PASSKEY` — 64-char hex, encrypts wallet keys locally
- Per-exchange slots: `ORDERLY_API_KEY`, `BYBIT_API_KEY`, etc.

Never commit `.env`, `config.toml`, or any file with real credentials.
