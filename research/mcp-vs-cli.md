# MCP vs CLI — Agent Tool Integration Research

> Research date: March 2026

---

## What Is MCP

**Model Context Protocol (MCP)** is an open standard introduced by Anthropic in November 2024, now maintained by the Agentic AI Foundation (Linux Foundation). It standardizes how LLM applications connect to external tools, data sources, and services — directly inspired by the Language Server Protocol (LSP).

**Architecture — three roles:**
- **Host** — the LLM app (Claude Desktop, Cursor, Claude Code, VS Code)
- **Client** — protocol connector embedded in the host; maintains a 1:1 stateful connection per server
- **Server** — lightweight service exposing capabilities to the AI

**MCP servers expose three primitives:**
- **Tools** — functions the AI can call (`create_issue`, `query_database`)
- **Resources** — data/context the AI can read (files, API responses)
- **Prompts** — pre-built templated workflows

**Session lifecycle:** Stateful. On connection, the server sends its full capability manifest (tool schemas) to the client. State persists across the session — sequential calls share context.

---

## MCP Transport Types

| Transport | Latency | Concurrency | Best For | Status |
|-----------|---------|-------------|----------|--------|
| **stdio** | <1ms | Single client | Local tools, desktop apps, Claude Code plugins | Recommended for local |
| **Streamable HTTP** | 10–50ms | Unlimited | Web apps, cloud, remote services, multi-user SaaS | Recommended for remote |
| **SSE / HTTP+SSE** | 10–50ms | Moderate | Legacy remote deployments | Deprecated (March 2025) |

**Decision rule:** stdio for same-machine; Streamable HTTP for anything remote or multi-tenant.

---

## How CLI Tools Work as Agent Tools

The agent generates a shell command string and executes it via subprocess call (e.g., Claude Code's `Bash` tool). The process runs to completion; the agent reads stdout/stderr when it exits.

**No schema negotiation** — each invocation is a fresh process. The agent interprets raw text output (or structured JSON if the tool offers `--output json`). Tool discovery is implicit — the model knows common tools from training data, and can run `--help` for unfamiliar ones.

**State handling:** Inherently stateless. Multi-step workflows compose via pipes, shell variables, and the filesystem. Unix philosophy: many small, composable tools.

---

## Key Differences

| Dimension | CLI | MCP |
|-----------|-----|-----|
| Invocation | Subprocess + shell command string | JSON-RPC method call over stdio or HTTP |
| Output format | Unstructured text (opt-in JSON) | Structured JSON-RPC always |
| Tool discovery | Implicit (training data) + `--help` on demand | Explicit schema manifest at session start |
| State/session | Stateless; compose via pipes and files | Stateful; context persists across calls |
| Auth handling | dotfiles, env vars, credential helpers | Centralized at server; OAuth 2.1 |
| Context window cost | Low — command + output only | High — full schema manifest upfront |
| Latency | Subprocess spawn only (<10ms local) | stdio: <1ms; HTTP: 10–50ms |
| Reliability | ~100% (local process) | 72% observed (TCP timeouts to remote servers) |
| Audit trail | OS process logs; no structured record | Structured JSON-RPC log per call |
| Multi-tenant isolation | None at protocol level | Per-user OAuth; tenant isolation built-in |
| Composability | High — pipes, xargs, shell scripting | Low — sequential round-trips; no native chaining |
| Implementation cost | Zero — tool already exists | Must build and maintain a server |

---

## Token Cost (Real Benchmark Data)

| Task | CLI tokens | MCP tokens | Multiplier |
|------|-----------|-----------|-----------|
| Repo language query | 1,365 | 44,026 | 32× |
| PR details | 1,648 | 32,279 | 20× |
| Median across tasks | ~1,500 | ~38,000 | ~25× |

**Monthly cost at 10,000 ops/month (Claude Sonnet pricing):**
- CLI: ~$3.20
- MCP direct: ~$55.20 (17× multiplier)
- MCP via gateway with schema filtering: ~$5.00 (~90% reduction)

**Reliability:** CLI 100% (25/25 runs) vs MCP 72% (18/25 runs) — failures were TCP-level timeouts to GitHub's Copilot MCP server.

**Key finding:** Adding an 800-token skill document describing a CLI tool's usage reduced tool calls and latency by ~33% — more impact than switching protocols.

---

## Which Frameworks Use Which

| Framework | Primary Pattern |
|-----------|----------------|
| **Claude Code** | CLI-first (`Bash` tool); MCP supported as supplement |
| **Claude Desktop** | MCP-native; stdio servers configured in JSON config |
| **Cursor** | MCP as plugin system; 40-tool hard cap; one-click setup |
| **VS Code (Copilot)** | MCP via `mcp.servers` config |
| **LangChain / LlamaIndex** | Both — CLI wrapped as `Tool` objects, MCP via client libs |
| **OpenAI Agents SDK** | Function calling (structured JSON schemas — conceptually closer to MCP) |

---

## When CLI Is Better

1. **Tool has a mature CLI** — git, gh, docker, kubectl, aws, cargo, npm, terraform (model has strong training familiarity, zero schema overhead)
2. **Token budget is tight** — 25–32× cost difference is decisive
3. **Local / developer workflow** — inner loop: edit-compile-test
4. **Composability matters** — `cargo test 2>&1 | grep FAILED | head -20` is one line
5. **Single user, no multi-tenancy** — no need for per-user OAuth or audit trails
6. **Speed of implementation** — use `gh` instead of building a GitHub MCP server
7. **Model already knows the tool** — zero-shot usage without schema injection

## When MCP Is Better

1. **No CLI exists for the service** — proprietary internal APIs, SaaS systems without CLIs
2. **Multi-tenant / enterprise compliance** — per-user OAuth, structured audit trail, tenant isolation
3. **Stateful multi-step workflows** — context must persist across calls without file intermediates
4. **Dynamic capability discovery** — available operations change at runtime
5. **Cross-organization tool sharing** — MCP servers can be published to registries
6. **Rich structured data return** — database queries, CI logs, deployment status
7. **Non-developer users** — customer service, PMs querying systems without shell access

---

## Production Examples

### CLI in production
- **Claude Code** — bash/subprocess for all local ops (file editing, testing, git, builds)
- **skill-trading** (this repo) — Rust binary invoked via bash; wraps TTC Box API through subprocess
- `gh pr list`, `kubectl apply`, `aws ec2 describe-instances --output json`
- **Perplexity (2026)** — built MCP, ran it in production, reversed to REST API + CLI calls within one quarter for cost and latency reasons

### MCP servers in production
- **GitHub MCP** — 43 tools; 55,000 tokens of schema overhead
- **Supabase MCP** — 20+ tools; table design, migrations, SQL, TypeScript type generation
- **Slack MCP** — channel summaries, message drafting, thread access
- **Razorpay Blade MCP** — Figma-to-code: 75% first-gen accuracy, 70% of frontend engineers using it, 3× faster shipping
- **Internal data warehouse MCPs** — dominant real-world pattern: non-developer access to internal APIs (Pragmatic Engineer: "massive long tail of public servers with near-zero users")

---

## Using Both Together

This is the dominant pattern in mature production systems.

### Pattern 1: CLI-first, MCP as fallback
Use CLI when a mature tool exists; use MCP only when no CLI alternative exists. Perplexity's current direction.

### Pattern 2: Skills abstraction layer
Skills wrap either transport. The agent calls one stable interface regardless of which wire protocol fires underneath. **This is what this repo does** — `skill-trading` is a CLI binary; the agent calls it the same way regardless.

### Pattern 3: MCP Gateway with CLI backends
A gateway MCP server accepts JSON-RPC from the agent, filters schemas (reducing context ~90%), handles OAuth centrally, then executes CLI commands on the backend. Cost: ~$5/10k ops vs $55 for direct MCP.

### Pattern 4: Scope-based routing
CLI for the inner dev loop (local, fast, cheap). MCP for the outer orchestration loop (CI/CD, deployment, cross-system monitoring).

---

## Why This Repo Uses CLI

`skill-trading` is a compiled Rust binary invoked via subprocess. This is a deliberate choice:

- **Token efficiency** — no schema manifest injected into context on every call
- **Zero server infra** — binary runs anywhere; no HTTP server to deploy or maintain
- **Composability** — agents can pipe output, combine with shell tools, pass flags dynamically
- **Model familiarity** — skill documents (SKILL.md) teach the agent the interface for ~800 tokens, not 30,000+
- **Reliability** — local process, no TCP timeouts
- **Distribution** — host + linux-x64 binaries copied to `.claude/skills/skill-trading/scripts/` via `make release-all`

The tradeoff accepted: no built-in OAuth/multi-tenant, no structured audit trail. Acceptable for a single-user trading CLI.

---

## Decision Matrix

| Use Case | Recommended |
|----------|-------------|
| Solo dev, local tools, cost-sensitive | CLI |
| Tool has mature vendor CLI (gh, kubectl, aws) | CLI |
| Token budget constrained | CLI |
| Fast inner-loop dev (edit/test/build) | CLI |
| Multi-tenant SaaS product | MCP |
| Enterprise compliance / audit trails | MCP |
| No CLI exists for the service | MCP |
| Non-developer users | MCP |
| Cross-team shared tool registry | MCP |
| Production system at scale | CLI + MCP gateway |
| Complex stateful multi-step workflows | MCP |

---

## Protocol Timeline

| Date | Event |
|------|-------|
| Nov 2024 | MCP launched by Anthropic; stdio + SSE transports |
| Mar 2025 | Streamable HTTP introduced; SSE deprecated |
| Jun 2025 | OAuth 2.1, structured output types, user elicitation added |
| Nov 2025 | Async Tasks, server identity, community registry, statelessness improvements |
| Dec 2025 | Anthropic donates MCP to Agentic AI Foundation (Linux Foundation) |
| Early 2026 | Perplexity reverses from MCP to CLI/API; CLI-vs-MCP becomes a recognized architectural decision |

**Adoption:** OpenAI, Google DeepMind, Microsoft, Cloudflare all support MCP. It is the de facto industry standard for tool integration — but CLI remains dominant for local developer tooling.
