# ClawHub Research

Goal: publish `skill-trading` (and the family of sub-skills in `.claude/skills/`) to ClawHub so other OpenClaw agents can install them. Decide whether to ship as a **skill** or a **plugin** given that our CLI is a Rust binary, not an npm package.

Reference example: <https://clawhub.ai/chrisling-dev/hyperliquid-cli>

---

## TL;DR — what to build

- **Ship as a Skill, not a Plugin.** Plugins are JS/TS code that extends the OpenClaw *gateway* runtime (channels, storage, routing). Skills are Markdown instruction bundles that ride alongside an external CLI. Our project is the second pattern.
- **We do not need an npm package.** ClawHub's skill registry is language-agnostic: `SKILL.md` + supporting files. The `hyperliquid-cli` example *is* published as an npm package, but that is one author's choice, not a registry requirement. Skill metadata supports `requires.bins` so we can declare external binary dependencies, and skills can ship scripts inside the bundle.
- **Bundle the prebuilt Rust binaries inside the skill folder.** We already do this in `.claude/skills/skill-trading/scripts/skill-trading-{darwin-arm64,linux-x64}` plus the POSIX launcher. That layout is portable and works as-is when published.
- **Publish each sub-skill separately**, with `skill-trading` as the core dependency. Mirrors the sub-skill structure already in the repo and lets other agents pick only what they need.

---

## Skill vs Plugin — which OpenClaw expects

Source: docs.openclaw.ai, openclawplaybook.ai/guides/openclaw-skills-vs-plugins-explained.

| | Skill | Plugin |
|---|---|---|
| What it is | `SKILL.md` + scripts/refs, installed into `~/.openclaw/workspace/skills/` | npm package with `openclaw` block in `package.json`, loaded by the gateway at startup |
| Who runs it | The agent reads `SKILL.md` on demand | The gateway daemon executes JS/TS code |
| Language | Markdown + any binary the skill calls | JavaScript / TypeScript only |
| Right for | "What the agent knows how to do" — workflows, expertise, CLI wrappers | "What the gateway can do" — channels, storage, routing, auth providers |
| Our project | ✅ matches | ❌ wrong layer |

The OpenClaw CLI explicitly redirects `clawhub install` to `openclaw skills install` when a thing is a skill — they treat the two namespaces distinctly. We never need to touch the plugin path.

---

## Anatomy of the reference: `chrisling-dev/hyperliquid-cli`

Pulled from the public skill page. Useful as a template because it is also a CLI-wrapping skill.

- **Distribution:** published as an npm global (`npm install -g hyperliquid-cli`), discoverable via `openclaw skills install hyperliquid-cli` and `npx clawhub@latest install hyperliquid-cli`. The npm package is a delivery vehicle for the CLI binary `hl`.
- **Required external deps** declared in `SKILL.md`:
  - binary `hl`
  - env `HYPERLIQUID_PRIVATE_KEY`
  - state directory `~/.hyperliquid`
- **Files included:** `SKILL.md`, `README.md`, `reference.md`, `examples.md`.
- **Public metadata shown by ClawHub:** version, downloads, stars, license, author handle + GitHub avatar, last-updated date, and an OpenClaw security scan badge.
- **Watch-out:** the registry record disagreed with the SKILL.md (`required env vars: none` in registry vs `HYPERLIQUID_PRIVATE_KEY` in the file). Lesson: keep the frontmatter `metadata.openclaw.requires` block accurate — that's what the registry indexes.

ClawHub flagged the example "Suspicious (medium confidence)" via its scanner. Crypto skills get extra scrutiny because of the **ClawHavoc** incident (1,184+ malicious skills disguised as crypto trading tools were taken down). We need to clear that bar — see "Risks" below.

---

## SKILL.md schema we should use

Combining the frontmatter conventions from the OpenClaw docs and what the registry actually indexes:

```yaml
---
name: skill-trading
version: 0.1.5
description: >
  Multi-exchange trading CLI for 15+ perp venues via the TTC Box API.
  Place orders, manage positions, run TWAP/DCA ladders, scan signals,
  build trailing-stop loops. Bundles a prebuilt Rust binary (darwin-arm64,
  linux-x64) — no Rust toolchain needed on the host.
tags:
  - trading
  - perpetuals
  - crypto
  - cli
  - rust
metadata:
  openclaw:
    requires:
      env:
        - TTC_AUTH_TOKEN
        - TTC_PASSKEY
      bins: []   # binary is bundled inside scripts/, not required from PATH
    primaryEnv: TTC_AUTH_TOKEN
---
```

Key points:
- `bins: []` because we ship the binary **inside** the skill bundle. The hyperliquid example uses `bins: [hl]` because they expect the user to install `hl` separately. Our pattern is stronger — the agent does not need a side install step.
- `primaryEnv` lets the registry surface "this skill needs a token" cleanly.
- `tags` is the only discoverability lever — search is heavily tag-driven (see "Discoverability" below).

---

## Publishing flow

Source: docs.openclaw.ai/tools/clawhub.

```bash
# one-time
clawhub login                  # GitHub OAuth; account must be ≥ 14 days old (docs say 1 week, registry actually enforces 14 days)
clawhub whoami                 # verify

# publish a single skill
clawhub skill publish ./.claude/skills/skill-trading \
  --slug skill-trading \
  --name "TTC skill-trading" \
  --version 0.1.5 \
  --changelog "Initial public release. Multi-exchange perps, TWAP, DCA, trail-watch, scanner." \
  --tags trading,perp,crypto,cli,rust

# bulk re-publish all sub-skills with auto bump
clawhub sync --all --bump patch --dry-run    # preview
clawhub sync --all --bump patch              # commit
```

Telemetry sends an install-count snapshot on `sync` — opt-out with `CLAWHUB_DISABLE_TELEMETRY=1` if we want.

Useful sister commands when iterating: `clawhub package inspect <slug>`, `clawhub list`, `clawhub update <slug>`, `clawhub delete <slug> --yes` (soft-delete, recoverable via `undelete`).

---

## Binary distribution — the actual hard part

The Rust binary is the thing that doesn't fit the npm-shaped hole most ClawHub skills sit in. Three viable approaches:

### Option A — bundle binaries inside the skill (recommended)

What we already do in-repo: `.claude/skills/skill-trading/scripts/` contains `skill-trading` (POSIX launcher) + `skill-trading-darwin-arm64` + `skill-trading-linux-x64`. The launcher detects `uname -s`/`-m` and execs the right binary.

Pros: zero-step install, no PATH mutation, matches our existing `make release-all` workflow.
Cons: each skill bump ships ~10–20 MB of binary. ClawHub doesn't publish package-size limits, but worth checking before the first `publish`.

### Option B — declare `bins: [skill-trading]` and let the user install separately

Mirrors the hyperliquid-cli pattern. We'd need a separate distribution channel — a Homebrew tap and a `curl | sh` installer pointing at GitHub Releases.

Pros: tiny skill bundle, single source of truth for the binary.
Cons: side-install friction is exactly the thing we removed in v0.1.5 (the `install.sh` flow). Users / agents have to do an extra step.

### Option C — wrap in an npm package like the reference does

Pure delivery hack: an `npm install -g` that drops the prebuilt Rust binary into the user's PATH. Several Rust CLIs do this (esbuild, swc).

Pros: matches what ClawHub's install command expects (`npx clawhub install ...`), gets us into the npm-driven install path.
Cons: adds an npm publish workflow we don't have today; doesn't actually buy us anything Option A doesn't.

**Recommendation: Option A.** Validate bundle size on first publish; fall back to Option C only if the registry rejects the size.

---

## Discoverability findings

I searched ClawHub directly via WebFetch for `trade on hyperliquid`, `perp`, `trade`, `binance`. **All four returned "no results."** The hyperliquid-cli skill exists at a deep-link URL but doesn't appear in those searches, and the public Plugins page lists 50 plugins with only one finance entry (an A-share / China stock data plugin).

Two things this tells us:
1. Search relevance on ClawHub is weak / tag-heavy. Tag generously: `trading, perp, perpetuals, futures, crypto, defi, orderly, hyperliquid, bybit, binance, dydx, twap, dca, market-making, signals` etc. Include each exchange we support as a tag.
2. The trading vertical on ClawHub is wide open. There is no incumbent skill for general multi-exchange perps trading. First-mover advantage is real if we ship soon.

(WebFetch may have missed JS-rendered results, but the deep-link page rendered fine, so the search index itself is the likely gap, not the rendering.)

---

## What to publish — a staging plan

The repo already has 12 skills in `.claude/skills/`. Map them to ClawHub like this:

| Order | Slug | Why publish | Depends on |
|---|---|---|---|
| 1 | `skill-trading` | Core CLI wrapper. Everything else assumes it. | — |
| 2 | `skill-onboarding` | First-run setup; lowers the trial barrier for new users. | skill-trading |
| 3 | `skill-portfolio-manager` | Portfolio health; broad appeal even for non-traders. | skill-trading |
| 4 | `skill-shark` | Signal-driven bracketed entries — the headline workflow. | skill-trading |
| 5 | `skill-twap` + `skill-loop-trading` | Differentiated agentic loops; ClawHub has nothing like them. | skill-trading |
| 6 | `skill-market-overview`, `skill-momentum`, `skill-signal-patrol` | Research / scanner skills. Bundle later. | skill-trading |
| 7 | `skill-market-maker`, `skill-dca` | Niche — ship after the headline skills land. | skill-trading |

Each `SKILL.md` should declare its dependency on `skill-trading` in plain prose under "Requirements" — there's no native dependency field in the registry frontmatter.

---

## Should we also build a plugin?

Probably not, but two scenarios where it would make sense:

- **TTC Box gateway plugin.** If we wanted OpenClaw agents to talk to TTC Box natively (without invoking our CLI), a code plugin could expose TTC Box as a first-class connection in the gateway. This is a much larger effort: TS/JS port of the auth flow, the encrypted-wallet logic in `src/crypto.rs`, and the API client in `src/api/client.rs`. Only worth it if we see real demand from agent builders who don't want to shell out to a CLI.
- **Telegram-style channel plugin** for trade alerts. Out of scope for our current repo, but a plausible follow-on.

For the immediate goal — letting other agents use this Rust CLI — the skill path is correct and sufficient.

---

## Risks

- **ClawHavoc taint on crypto skills.** ClawHub aggressively flags crypto trading skills. Mitigations: clear authorship (link to the public GitHub repo), MIT license, signed binaries if feasible, point the OpenClaw scanner at the source, respond fast to any "Suspicious" badge by submitting a re-scan request.
- **Manifest drift.** The hyperliquid-cli skill had its frontmatter disagree with its registry record. Whatever we put in `metadata.openclaw.requires` is what users will see — diff it before each publish.
- **Bundle size.** ~10 MB per binary × 2 platforms × every version bump. Watch for any registry size cap on the first publish; if hit, switch to Option B or C.
- **Auto-hide on reports.** 3+ unique reports auto-hide a skill pending review. Make sure the README and SKILL.md spell out the security model (encrypted wallets, no key transmission in plaintext) so reviewers don't misread the crypto-handling code as malicious.

---

## Open questions to resolve before publishing

1. Does `clawhub skill publish` enforce a max bundle size? (Test with `--dry-run` on `skill-trading`.)
2. Does the registry preserve the executable bit on bundled binaries when delivered to other users? If not, the launcher needs to `chmod +x` on first run.
3. What does the OpenClaw scanner look at? Source code in the repo, or just the published bundle? (Affects whether having the Rust source on GitHub helps the scan score.)
4. Is there a way to mark a skill family (`skill-trading` + sub-skills) so users see them grouped, or do we just rely on naming convention?

---

## Sources

- <https://clawhub.ai/chrisling-dev/hyperliquid-cli>
- <https://docs.openclaw.ai/tools/clawhub>
- <https://github.com/openclaw/clawhub>
- <https://www.openclawplaybook.ai/guides/openclaw-skills-vs-plugins-explained/>
- <https://www.openclawplaybook.ai/blog/clawhub-skill-registry-openclaw-agents/>
- <https://help.apiyi.com/en/clawhub-ai-openclaw-skills-registry-guide-en.html>
- <https://clawhub.ai/plugins>

# PUBLISHING                                                                                          
  1. Init the GitHub repo (in /Users/mac/Documents/clawhub/)                                                   
  cd /Users/mac/Documents/clawhub                                                                              
  git init                                                                                                     
  git add .                                                                                                    
  git commit -m "Initial publish-ready bundle for skill-trading v0.1.5"                                       
  gh repo create ttcbox/skill-trading --public --source=. --push        
  Then update the homepage: line in SKILL.md frontmatter to the real URL — currently it's a placeholder        
  (https://github.com/ttcbox/skill-trading).                                                                   
                                                                                                               
  2. Install + log into ClawHub                                                                                
  npm i -g clawhub@latest                                                                                      
  clawhub login                # GitHub OAuth; your GH account must be ≥ 1 week old                            
  clawhub whoami               # verify                                            
                                                                                                               
  3. Dry-run the publish                                    
  ./publish.sh                 # validates + runs `clawhub skill publish ... --dry-run`                        
  This is where the open questions from clawhub_research.md get answered: bundle-size cap, executable-bit
  preservation, scanner verdict. Read the dry-run output carefully before going live.                          
                                                                                                               
  4. Live publish                                                                                              
  ./publish.sh --live          # prompts for confirmation, then publishes for real                             
  Skill goes live at https://clawhub.ai/<your-handle>/skill-trading.              
                                                                                                               
  5. Post-publish smoke test                                                                                   
  - From a clean machine (or fresh shell with no ~/.openclaw/workspace/skills/skill-trading), run:
  openclaw skills install skill-trading                                                                        
  - Verify the binary is executable after install (this is open question #2 — if not, we'll need a postinstall
  chmod +x or an in-launcher self-chmod).                                                                      
  - Run skill-trading status to confirm it works end-to-end.                                                   
                                                            
  6. Watch for the OpenClaw scanner badge                                                                      
  Crypto skills often get flagged "Suspicious (medium confidence)" — same as the hyperliquid example. If it    
  happens, request a re-scan and link the public source repo. License is already MIT in the manifest, which    
  helps.                                                                                                       
                                                                                                               
  7. (Once #1–6 are clean) start the sub-skill rollout                                                         
  Per the staging table in clawhub_research.md: skill-onboarding → skill-portfolio-manager → skill-shark →
  skill-twap + skill-loop-trading → research/scanner skills → market-maker/dca. Each one will be a fresh       
  clawhub bundle that depends on skill-trading being installed first; you can copy clawhub/publish.sh and adapt
   the slug.                                                                                                   
                                                            
  Likely first snags to expect: bundle size rejection (fallback: switch to Option C, npm wrapper) and          
  executable-bit loss on install (fallback: launcher self-chmods on first run). Both are in the research doc's
  open-questions list — neither is a blocker for the dry-run.