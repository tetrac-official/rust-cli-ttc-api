Looking at the existing tests, they cover CLI argument parsing, help text, and dry-run output well — but a lot of internal logic and failure paths aren't exercised. Here's what I'd add, ordered by hardening value.

Highest-value gaps
1. [x] Crypto round-trip (src/crypto.rs) — 21 tests added inline in src/crypto.rs (#[cfg(test)] mod tests). PBKDF2 determinism + email normalization, SHA-256 known vector, Ed25519 sign/verify round-trip, EVM address derivation, AES-256-CBC encrypt+decrypt round-trip (including empty / long / cross-block payloads), random-salt verification, wrong-key rejection.
2. [x] Config priority resolution (src/config.rs) — 11 unit tests in src/config.rs (full-config parse, optional sections default when omitted, missing required sections error, malformed TOML, explicit-path honored, CLI/env/config priority for credentials including partial-env fallthrough) + 5 CLI integration tests in tests/integration_test.rs (--config flag, TTC_CONFIG env, CLI flag beats env, TTC_EXCHANGE env beats config file, malformed config fails with clear error). Skipped "Secrets in config.toml rejected/warned" — that policy is documented in CLAUDE.md but isn't enforced in code; would require a feature change. Note: tests assert on api.base_url rather than the top-level `exchange` field because main.rs pre-loads the discovered config's `exchange` into TTC_EXCHANGE — `-e` and TTC_EXCHANGE both override it cleanly, so this isn't user-facing.
3. [x] Error classification (src/error.rs) — 11 unit tests inline in src/error.rs covering is_retryable() for every variant, Display strings (matching thiserror templates), and From conversions (io, serde_json, toml). Result alias smoke test included.

   **Original plan revised:** "5xx → retryable" is unsafe to apply blanket-style. POST /exchanges in next-ttc/src/app/api/v1/exchanges/route.ts handles non-idempotent operations (placeMarketOrder, placeLimitOrder, cancelOrder, closeAllPositions, createWithdrawal) without server-side clientOrderId deduplication, and route.ts uses `status = 500` as its catch-all (line 338) including for the case where the upstream exchange already accepted the order and a downstream Redis/KV write threw. Retrying that 500 would place a duplicate order. The test `api_5xx_is_not_retryable_today` locks in current behavior — keep it. Network/transport errors are still retried via the separate arm at client.rs:94, and 429 with retry-after is handled correctly. A future improvement could add per-method retry classification (retry 5xx only on idempotent reads — getTickers/getKlines/getBalance/getPositions/getOrders/getBestBidAsk) but that's a larger change than is_retryable() knows how to express today.

Recommendation
Keep is_retryable() as-is for now. The behavior is right for agents and humans — the difference between the two audiences is in how they recover, not in whether the CLI retries.

Three concrete follow-ups, in priority order:

Document the agent recovery protocol in skill-trading — small, high-leverage. I can draft this as a separate task after we finish the test list.
Add machine-readable error output — when --output-format=json, errors emit JSON to stderr with a structured shape. Medium-sized change to printer.rs + main.rs error handler. I'd want to /schedule this as its own follow-up.
Server-side idempotency keys — out of this repo, but the right long-term fix.



4. [x] HTTP client retry (src/api/client.rs) — 13 tests in tests/http_retry_test.rs using mockito 1.6 with `Server::new_async()`. Coverage: success-first-try (1 call), 429 then 200 retry success, persistent 429 caps at max_retries+1 attempts, max_retries=1 means 2 total attempts, 5xx (500/503) NOT retried (locks current behavior — see error classification gap in task 3), 4xx (400/401/403/422) NOT retried, retry backoff sleep is observable (250ms ≤ elapsed ≤ 5s for retry_delay_ms=100 with 2 retries), transport-layer connection refused IS retried via the separate arm at client.rs:94 (verified by pointing at 127.0.0.1:1 and asserting the call takes long enough to prove retries happened, then maps to TtcError::Request after exhaustion), and a sanity test confirming the request shape (ttc-auth-token / ttc-public-key headers + JSON body containing exchangeName/method).

   Note: the original plan included "succeeds on 2nd attempt after one 503" — but that's not how the code behaves today. Replaced with the 429 equivalent (which IS retryable) and added explicit 5xx-not-retried tests to lock in current behavior. The "times out cleanly" bullet is implicitly covered by the transport-error test (timeout=1s, 2 retries, retry_delay_ms=50 → call returns within seconds, never hangs).
5. Model serde round-trips (src/models.rs)
For each DTO, serde_json::from_str then to_string then from_str again equals the original. Catches breakage when TTC Box API changes shape:

OrderSide, PositionSide enum case sensitivity
Optional fields (stop_loss, tp may be null on NEUTRAL signals — already noted in CLAUDE.md)
Decimal precision (4-decimal display rule)
Medium-value
6. State-file handling

~/.trail-watch-{symbol}-{exchange}.json writes valid JSON, is removed on close
~/.twap-{symbol}-{exchange}.json survives crash mid-fill (read partial state, resume)
Concurrent writers don't corrupt the file (atomic rename pattern)
7. Output formatters (src/output/printer.rs)

JSON output is parseable JSON
CSV has correct number of columns, escaping for quoted fields
Quiet mode produces no stdout
--output-format env (TTC_OUTPUT) overrides default but loses to CLI flag
8. Status command exit codes
CLAUDE.md says status exits 1 if not ready. Add:

Exit 0 when session valid + exchange creds present
Exit 1 with no session
Exit 1 with expired token
9. Quantity/decimals math
Bugs here lose money. For twap-slice and market-maker:

--decimals 0 truncates correctly
USD amount → quantity conversion at given price is exact at boundaries
Spread math: entry × (1 ± spread_pct/100) rounds the right way for buy vs sell
Lower-value but cheap
10. Boundary validation

Negative --quantity rejected
Leverage > exchange max rejected (or at least passed through cleanly)
Empty symbol rejected
--rounds 0 for market-maker handled
11. Portfolio thresholds (src/commands/portfolio.rs)

HEALTHY/WATCH/DANGER classification at threshold boundaries (just over, just under)
Empty positions list returns HEALTHY
12. Cross-platform launcher
Test the scripts/skill-trading launcher against fake uname outputs (arm64 vs aarch64, x86_64, unsupported combos → useful error).

What I'd skip
Testing actual TTC Box API calls (flaky, requires creds, integration territory)
Testing /loop itself (it's Claude Code's harness, not yours)
UI snapshot tests on table output (brittle, low ROI)
The top 5 give you the most defense per line of test code. If you want, I can implement #1 (crypto) and #3 (error classification) as a starting batch — both are pure-function-heavy and won't need HTTP mocks.