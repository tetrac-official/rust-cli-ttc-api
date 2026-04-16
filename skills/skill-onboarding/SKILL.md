---
name: skill-onboarding
description: First-run setup and authentication for the skill-trading CLI. Walk the user from zero to a verified, ready-to-trade session. Use when the user is new, has a broken setup, or asks how to get started.
---

# skill-onboarding — Agent Setup & Authentication

This skill walks through the complete first-run setup for `skill-trading`. Follow the steps in order. Do not skip ahead — each step depends on the previous one succeeding.

For exchange-specific credential details, see `skill-trading/references/exchanges.md`.
For error diagnosis, see `skill-trading/references/troubleshooting.md`.

---

## Step 1 — Is the CLI installed?

```bash
skill-trading info
```

**Expected:** A version line like `skill-trading v0.1.2`.

**If "command not found":**
```bash
make release && make install
```

This builds the binary and copies it to `/usr/local/bin/skill-trading`. Verify with `skill-trading info` again before continuing.

---

## Step 2 — Does a .env file exist?

```bash
ls -la .env
```

If `.env` does not exist, check `.env.sample` for the template:

```bash
cat .env.sample
```

Create `.env` from the sample and fill in the required values. The minimum to proceed:

```env
TTC_EMAIL=user@example.com
TTC_PASSKEY=<64-char hex passkey>
TTC_EXCHANGE=orderly
```

If the user already has a TTC account, they will also have `TTC_AUTH_TOKEN` and `TTC_PUBLIC_KEY`. If not, Step 3 handles that.

**Never commit `.env` to git.**

---

## Step 3 — Authenticate with TTC Box

Two paths depending on whether the user has an account:

### New user — register

```bash
skill-trading register
```

This creates a TTC Box account, generates a local wallet (Ed25519 keypair encrypted with `TTC_PASSKEY`), and writes `TTC_AUTH_TOKEN`, `TTC_PUBLIC_KEY`, and `TTC_TOKEN_ISSUED_AT` to `.env`.

### Existing user — login

```bash
skill-trading login
```

Reads `TTC_EMAIL` and `TTC_PASSKEY` from `.env`. If both are set, completes silently. If either is missing, it will prompt. On success, `.env` is updated with a fresh `TTC_AUTH_TOKEN` and `TTC_TOKEN_ISSUED_AT`.

### Token lifecycle

- Tokens expire exactly **24 hours** after `TTC_TOKEN_ISSUED_AT` (Unix timestamp in `.env`).
- To check remaining validity:
  ```bash
  skill-trading status
  ```
- If token is expired or close to expiry (< 2 hours remaining), run `skill-trading login` before proceeding.

---

## Step 4 — Add exchange credentials

Exchange credentials are loaded from `.env` using the pattern:

```
{EXCHANGE}_API_KEY=...
{EXCHANGE}_API_SECRET=...
{EXCHANGE}_API_PASSPHRASE=...   # only for exchanges that require it
```

The exchange name must be **UPPERCASE** in the env var (e.g., `ORDERLY_API_KEY`).

Which exchanges need a passphrase and what to put there is documented in `skill-trading/references/exchanges.md`.

### Orderly-specific: ORDERLY_MAIN_WALLET_ADDRESS

Required for **email-registered** CLI users only. Web3 wallet users do not need this.

When you registered via email, TTC Box generated a random keypair for `TTC_PUBLIC_KEY`. But Orderly needs your real trading wallet address to locate your funds. Without this, balance will show `0.0000`.

```env
ORDERLY_MAIN_WALLET_ADDRESS=<base58 Solana address, ~44 chars>
```

The user can find this address in their Orderly Network dashboard under Account settings.

---

## Step 5 — Verify the full setup

Run these three checks in order. Each confirms a different layer.

### 5a — TTC Box connectivity (no exchange credentials needed)

```bash
skill-trading market hybrid-tickers --symbol BTCUSDT --market-type futures
```

**Expected:** A table of BTCUSDT price/volume/funding across exchanges.
**If it fails:** Token is likely expired. Run `skill-trading login` and retry.

### 5b — Exchange credentials

```bash
skill-trading account balance
```

**Expected:** USDT balance with total, locked, and available columns.
**If it fails:** Missing or incorrect exchange credentials. Go back to Step 4.
**If balance is 0.0000 on Orderly:** `ORDERLY_MAIN_WALLET_ADDRESS` is missing or wrong. See Step 4.

### 5c — Position and order state

```bash
skill-trading position get
skill-trading order open
```

**Expected:** Either a table of open positions/orders, or a message saying none are open. Both are correct.

### 5d — Full status check (all-in-one)

```bash
skill-trading status
```

**Expected output for a ready setup:**
```
TTC Box API          REACHABLE
Session token        VALID (Xh remaining)
Exchange credentials orderly configured
Status:              READY
```

If status shows `READY`, the setup is complete. The user is ready to trade.

---

## Step 6 — Confirm readiness

Once all steps pass, tell the user:

1. Their setup is verified and ready
2. Their default exchange is `$TTC_EXCHANGE`
3. Their token expires in X hours — remind them to run `skill-trading login` before it lapses
4. Point them to `skill-trading` for the trading protocol, or `skill-shark` for signal-driven setups

---

## Re-authentication flow

Use this when a previously working setup stops working (401 errors, expired tokens).

1. Run `skill-trading status` to diagnose which layer is broken
2. If session expired: `skill-trading login`
3. If exchange credentials missing: check `.env` for the correct `{EXCHANGE}_*` vars
4. If Orderly balance is zero: verify `ORDERLY_MAIN_WALLET_ADDRESS`
5. Run `skill-trading status` again to confirm `READY`

---

## Common first-run issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| `command not found` | Binary not installed | `make release && make install` |
| 401 Unauthorized | Token expired or missing | `skill-trading login` |
| `Missing credentials for exchange` | API key/secret not in `.env` | Add `{EXCHANGE}_API_KEY` / `{EXCHANGE}_API_SECRET` |
| Balance shows 0.0000 on Orderly | Missing wallet address | Set `ORDERLY_MAIN_WALLET_ADDRESS` in `.env` |
| Login prompts even though creds are set | `.env` not in working directory | `ls .env` to confirm, or check for typos in var names |
| `Configuration error` | Malformed `config.toml` | Delete it — `.env` alone is sufficient |

For detailed error reference, see `skill-trading/references/troubleshooting.md`.
