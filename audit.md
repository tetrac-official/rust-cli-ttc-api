# Security Audit Status

Run `cargo audit` to refresh.

**Current state (branch `v0.1.6`):** 0 vulnerabilities, 2 warnings — both transitive, both blocked on upstream releases. No action required from us until upstream ships fixed versions.

---

## Warning 1 — `number_prefix 0.4.0` unmaintained

- **Advisory:** [RUSTSEC-2025-0119](https://rustsec.org/advisories/RUSTSEC-2025-0119)
- **Severity:** unmaintained (no known exploit)
- **Path:**
  ```
  number_prefix 0.4.0
    └── indicatif 0.17.11
        └── skill-trading
  ```
- **Why we can't fix it locally:** `indicatif` is the source of this dep. We don't depend on `number_prefix` ourselves. Until the `indicatif` maintainers drop or replace it, we inherit the warning.
- **What to do:** Wait. Recheck on every `cargo update` or new `indicatif` release. Track the upstream issue: <https://github.com/console-rs/indicatif>.
- **Resolution trigger:** an `indicatif` release that drops `number_prefix` (likely a bump to `0.18` or `0.19`).

## Warning 2 — `rand 0.9.2` unsound

- **Advisory:** [RUSTSEC-2026-0097](https://rustsec.org/advisories/RUSTSEC-2026-0097) — *Rand is unsound with a custom logger using `rand::rng()`*
- **Severity:** unsound (only triggerable by an unusual logger configuration; not exploited via our code)
- **Paths:**
  ```
  rand 0.9.2
    ├── quinn-proto 0.11.14
    │     └── quinn 0.11.9
    │           └── reqwest 0.12.28
    │                 └── skill-trading
    └── mockito 1.7.2          [dev-dependency]
          └── skill-trading
  ```
- **Why we can't fix it locally:**
  - `reqwest` (production path) decides which `rand` version `quinn-proto` resolves to.
  - `mockito` is dev-only; it never ships in the binary, but `cargo audit` still reports it.
  - We already removed our direct `rand` dep (replaced with `rand_core 0.6`) — these are the only paths left.
- **What to do:** Wait. Recheck on every `cargo update` or new `reqwest`/`mockito` release.
- **Resolution trigger:** a `quinn-proto` release pinning `rand >= 0.9.3` once upstream cuts a fix, then a matching `reqwest` bump. The `mockito` dev path resolves automatically once the broader ecosystem moves.

---

## Re-running the audit

```bash
cargo audit
```

If new vulnerabilities appear (severity > unmaintained/unsound), treat them as release-blockers and patch immediately.

## What we've already fixed on this branch

- Bumped `quinn-proto 0.11.13 → 0.11.14` (DoS, RUSTSEC-2026-0037)
- Bumped `rustls-webpki 0.103.9 → 0.103.13` (4 advisories: panic + name-constraint bypasses)
- Removed unused `backoff 0.4` direct dep (cleared the `backoff` + transitive `instant` warnings)
- Replaced our `rand 0.8` direct dep with `rand_core 0.6` (cleared the direct-`rand` unsoundness warning)
