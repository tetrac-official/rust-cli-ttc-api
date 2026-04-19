# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Point your agent at `.claude/skills/skill-trading/scripts/skill-trading` and run
```bash
skill-trading info
```

## Which binary to run

`.claude/skills/skill-trading/scripts/` ships three files:

| File | Purpose |
|------|---------|
| `skill-trading` | POSIX launcher — detects `uname -s`/`-m` and execs the right binary. **Always call this one.** |
| `skill-trading-darwin-arm64` | Mach-O, Apple Silicon (M1/M2/M3). Used on the local dev machine. |
| `skill-trading-linux-x64` | ELF, Linux x86_64. Used on VPS / Railway / Docker. |

**Rules for the agent:**
- Always invoke `skill-trading` (the launcher), never a platform-suffixed binary directly — keeps commands portable across dev machine and VPS.
- Never run `cargo build`, `make release`, or `make release-linux` on a VPS. The VPS has no Rust toolchain and no Docker. It runs the prebuilt binary that was committed to the repo.
- If the launcher errors with `binary for <OS>-<ARCH> not bundled`, the repo is missing that platform's build — rebuild locally with `make release-all` and commit, don't try to build on the VPS.
- `uname -m` returns `arm64` on macOS and `aarch64` on Linux — the launcher already handles both; don't special-case in callers.

## Architecture

The binary (`skill-trading`) is a multi-exchange trading CLI proxied entirely through the TTC Box API (`https://ttc.box/api/v1`). All exchange operations route through TTC Box — the CLI never calls exchange APIs directly.


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
| `order` | — | Limit, market, stop, take-profit placement; `cancel`, `cancel-all`, `open` (list); **DCA ladder** |
| `position` | `pos`, `positions` | Get, close, close-all; **PnL breakdown** (`position pnl`) |
| `account` | `acct` | Balance, leverage, margin mode, hedge mode |
| `market` | `m` | Tickers, funding rates, OI, volume snapshot, TTC scanner, **price alerts** (`market alert`) |
| `risk` | — | Stop-loss (`sl`), take-profit (`tp`), trailing stop (`trail`), polling trail watcher (`trail-watch`) |
| `config` | — | Init, show, path, set-default, add/rm exchange |
| `login` | `auth` | TTC Box login |
| `register` | — | TTC Box registration + local wallet generation |
| `portfolio` | `port`, `pf` | Health report: balance + positions → HEALTHY/WATCH/DANGER status with risk warnings |
| `twap` | — | Time-weighted average price position builder (polling loop, market orders, crash recovery) |
| `twap-slice` | — | **Atomic single slice** — one market order for a fixed USD amount. Designed for `/loop` agent-controlled runs |
| `status` | — | Ping TTC Box API + verify session token + check exchange credentials → READY / NOT READY. Exits 1 if not ready. |
| `brief` | `morning`, `mb` | Morning market brief: session check + watchlist prices + signals + portfolio + open orders |
| `market-maker` | `mm` | Limit-order spread capture loop: enter at best bid/ask, exit at entry ± spread. See `[market-maker]` config for commission. |
| `info` | `version` | Show binary version and build info |

Market data commands (`hybrid-tickers`, `funding-rates`, `open-interest`, `volume-snapshot`, `scanner`) require no API key.

### Scanner output notes
- All prices display 4 decimal places throughout the CLI.
- Scanner includes a **Vola unit** line: `$X/bar (1x1) | Momentum: ±Y/bar (direction) | Avg range: $Z/bar`. Multiply the Vola unit by the fan ratio (2, 3, 4…) to get the slope of steeper fan lines.
- When signal is `NEUTRAL`, stop loss and TP levels are `null` from the API and omitted from output — this is expected, not a bug.
- Descending Vola fan lines from a high pivot can project below zero after many bars — mathematically valid, not an error.

### `risk trail-watch`
Polling trailing stop — activates once position enters profit, then trails stop at `peak × (1 - trail_pct%)` for longs, `peak × (1 + trail_pct%)` for shorts. Reads actual position side from the exchange and places a SELL stop for long positions, BUY stop for short positions, always `reduce_only`. Cancels and replaces stop only when the new level improves on the previous one. Stops automatically when position closes. Flags: `--trail-pct` (default 2.0%), `--interval` (default 30s).

**Progress file:** writes JSON state to `~/.trail-watch-{symbol}-{exchange}.json` on every tick. Contains `active`, `mark_price`, `peak_price`, `current_stop`, `stop_order_id`, `unrealized_pnl`, `position_size`, and `updated_at`. File is removed when the position closes. An agent can read this file on demand to monitor trail-watch without interrupting the loop.

### Agentic Loop Trading

Two modes of operation exist for time-based strategies:

**Unattended mode** (`twap`, `risk trail-watch`) — CLI owns the loop internally. Good for set-and-forget overnight runs. Both write JSON progress files that an agent can read on demand: `twap` writes crash recovery state to `~/.twap-{symbol}-{exchange}.json` after every fill; `trail-watch` writes status to `~/.trail-watch-{symbol}-{exchange}.json` on every tick.

**Agentic mode** (`twap-slice` + `/loop`) — Agent owns the loop via Claude Code's built-in `/loop` scheduler. Agent calls `twap-slice` once per tick, sees every fill, and can react between ticks. The agent must track budget/slice count and cancel the loop when done.

```
/loop 5m: skill-trading twap-slice -e orderly -s NEARUSDT --buy --amount 15 --decimals 0
```

Key `/loop` facts:
- Built into Claude Code — uses POSIX cron under the hood (`CronCreate` tool)
- Minimum interval: 1 minute (seconds round up)
- Default interval: 10 minutes
- Max 50 simultaneous loops per session; auto-expires after 3 days
- Context accumulates across ticks — agent remembers previous fills
- Stop with: "cancel the loop" or exit the session

See `skills/skill-loop-trading/SKILL.md` for the full agentic loop protocol.

## Skills

The `skills/` directory contains AI agent instruction sets (agentskills.io format):

- **skill-onboarding** — First-run setup and authentication: install check → .env → login/register → exchange credentials → verify READY status
- **skill-trading** — Core safe-trading protocol: pre-order checklists, order placement rules, output interpretation
- **skill-shark** — Signal-driven bracketed trade setup (entry + TP1 + TP2), requires R/R ≥ 2.0
- **skill-market-overview** — BTC/ETH trend + funding sentiment + OI distribution briefing
- **skill-momentum** — Finds 10%+ movers with volume, scans for signals
- **skill-signal-patrol** — Scans a fixed watchlist for HIGH confidence R/R ≥ 3.0 setups
- **skill-loop-trading** — Agent-controlled loop trading via `/loop` + `twap-slice`; agent owns the loop, retains full visibility
- **skill-twap** — TWAP position builder: splits USD budget into equal slices over time to average entry price and reduce market impact
- **skill-portfolio-manager** — Portfolio health report (`portfolio summary`): HEALTHY/WATCH/DANGER status, margin utilization, liq distance, position risk thresholds from `[portfolio]` config
- **skill-market-maker** — Limit-order spread capture loop (`market-maker` / `mm`); enters at best bid/ask, exits at entry ± spread; designed for zero-fee exchanges



## Environment / Credentials

See `.env.sample` for all supported variables. Key ones:
- `TTC_AUTH_TOKEN` / `TTC_PUBLIC_KEY` — session credentials (24h expiry)
- `TTC_EXCHANGE` — default exchange
- `TTC_PASSKEY` — 64-char hex, encrypts wallet keys locally
- Per-exchange slots: `ORDERLY_API_KEY`, `BYBIT_API_KEY`, etc.

Never commit `.env`, `config.toml`, or any file with real credentials.

## Rebuild rule — every source change

**Any edit under `src/` (or `Cargo.toml` / `Cargo.lock`) requires `make release-all` before committing.** Not the first time only — every time. Stage the source change **and** both updated binaries in the same commit — shipping source without the refreshed `skill-trading-linux-x64` leaves the VPS on stale code.

## Commands

```bash
make build          # debug build → target/debug/skill-trading
make release        # host-native release → .claude/skills/skill-trading/scripts/skill-trading-<host-suffix>
make release-linux  # cross-compile linux-x64 via `cross` (requires Docker Desktop running)
make release-all    # release + release-linux — run this after EVERY source change, before committing
make install        # copy host binary to /usr/local/bin (requires make release first)
make test           # run all tests
make clippy         # lint
make fmt            # format with rustfmt
make dist           # cross-platform builds (darwin-arm64, darwin-x64, linux-x64, windows-x64)
```

One-time cross-compile toolchain setup (local dev machine only — run once per machine, not per source change):
```bash
cargo install cross --git https://github.com/cross-rs/cross
```

### Docker (required for `make release-linux` / `make release-all`)

`cross` runs the linux-x64 build inside a container, so the Docker daemon must be up **every** time you cross-compile.

```bash
docker info >/dev/null 2>&1 && echo ready || echo not-ready   # check status
open -a Docker                                                 # start Docker Desktop (macOS)
until docker info >/dev/null 2>&1; do sleep 2; done            # wait until daemon is ready
```

If `make release-linux` fails with `Docker daemon not running`:
1. `open -a Docker` to launch Docker Desktop.
2. Wait for the whale icon in the menu bar to stop animating (~10–30s).
3. Re-run `make release-linux` (no need to redo `make release` — the darwin binary is unaffected).

Docker Desktop is **only** needed on the local dev machine for cross-compiling. The VPS never builds, so it does not need Docker.

Run a single test:
```bash
cargo test <test_name>
cargo test --lib <module>::<test_name>
```


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

