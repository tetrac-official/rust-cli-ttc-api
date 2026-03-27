# BUG-002: --config Flag Causes config.toml to Override Session Token

**Date:** 2026-03-27
**Status:** Fixed (by avoiding --config flag)
**Severity:** High — auth silently fails, falls through to x402

---

## Summary

When `--config config.toml` was passed explicitly, the config file was loaded before CLI flags and env vars were applied. However, if `config.toml` was present in the current directory, it was also auto-discovered and loaded — causing stale or placeholder values in the file to shadow valid env var credentials.

In practice, running from inside the project directory (`~/Documents/rust-cli-ttc-api/`) meant `config.toml` was always auto-discovered and loaded, and its values were applied first. The CLI flag override logic then only applied values that were explicitly passed as flags — env vars set in the shell were being read by clap correctly but the underlying issue was that `--config config.toml` was explicitly passed in early test commands, causing confusion about priority ordering.

---

## Symptoms

- `TTC_AUTH_TOKEN` env var set correctly (verified via `echo`)
- CLI returned 402 (auth failed, fell through to x402 payment gate)
- Same command with `--api-key <token>` passed explicitly returned 400 (auth passed, exchange error)
- Removing `--config config.toml` from the command resolved the 402

---

## Root Cause

The config loading priority in `main.rs`:

```rust
let mut settings = AppConfig::load_from_file(&cli.config)?;  // 1. Load file first
if let Some(ref key) = cli.api_key { ... }                   // 2. Apply CLI/env overrides
```

When `--config config.toml` was passed explicitly, the file was loaded. Because `AppConfig` in TOML does not have a top-level `api_key` or `public_key` field populated (those are TTC session credentials, stored only in `.env`), the settings object had `api_key: None` after file load. The clap env binding (`env = "TTC_AUTH_TOKEN"`) then correctly populated `cli.api_key` — but only if the env var was actually present in the process environment.

The real failure mode: early test runs used `export $(grep ...)` in a parent shell, then ran subcommands — but `export` in one shell invocation does not persist across separate `Bash` tool calls. So `TTC_AUTH_TOKEN` was never actually in the environment when the binary ran, meaning `cli.api_key` stayed `None`, and `settings.api_key` stayed `None` → auth failure → 402.

---

## Fix

Run the CLI with env vars injected directly into the same process:

```bash
# CORRECT — env vars in same invocation
cd ~/Documents/rust-cli-ttc-api && \
  export $(grep -v '^#' .env | grep '=' | xargs) && \
  ./target/release/skill-trading account balance -e asterdex

# OR — inline
TTC_AUTH_TOKEN=<token> TTC_PUBLIC_KEY=<key> ./target/release/skill-trading account balance -e asterdex
```

Do NOT use `--config config.toml` explicitly unless you have intentionally populated TTC credentials inside `config.toml`. The auto-discovery of `./config.toml` is fine — just ensure `.env` is sourced in the same shell invocation.

---

## Notes

- `.env` is never auto-loaded by the binary itself — it must be sourced by the shell before running
- `config.toml` is auto-discovered if present in the current directory
- TTC session credentials (`TTC_AUTH_TOKEN`, `TTC_PUBLIC_KEY`) should live only in `.env`, never in `config.toml`
- Exchange API keys can live in either `config.toml` (under `[exchanges.xxx]`) or `.env`
