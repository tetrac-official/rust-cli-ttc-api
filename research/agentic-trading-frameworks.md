# Agentic Trading Frameworks — Research

> Research date: March 2026

---

## The Core Problem

When an agent launches a long-running CLI subprocess (TWAP, trailing stop), it loses all visibility. The subprocess owns the loop — the agent is blind for the entire duration. This is the fundamental tension between CLI automation and agentic control.

---

## Notable Frameworks

### TradingAgents (TauricResearch, ICML 2025)
Seven specialized agents modelling a real trading firm: Fundamentals Analyst, Sentiment Analyst, News Analyst, Technical Analyst, Bull/Bear Researcher, Risk Manager, Fund Manager. Built on LangGraph. Key design: the graph owns the loop — no subprocesses. Each agent is a node; execution flows through the graph. Supports GPT-5.x, Gemini 3.x, Claude 4.x, Grok 4.x. Operates at daily bar cadence, not sub-minute.

### FLAG-Trader (ACL 2025)
Fuses LLM reasoning with reinforcement learning. LLM acts as policy network — frozen base layers + trainable top layers fine-tuned on financial data. True closed loop: LLM reasons → action executes → reward returned → policy updates. Agent learns from trade outcomes.

### CryptoTrade (EMNLP 2024)
Reflective LLM agent for zero-shot crypto trading. Combines on-chain (transactions, gas) with off-chain (news, sentiment). After each daily decision, reviews prior outcomes and incorporates reflection into next step. Daily cadence.

### NeurIPS 2025 Financial Agents Framework
Full pipeline: planner → orchestrator → alpha agents → risk agents → portfolio agents → execution agents → audit agents → memory agent. Uses MCP for control messages and A2A protocol for inter-agent communication. A memory agent records all states, prompts, tool calls, and decisions. Result: 20.42% return, 2.63 Sharpe, -3.59% max drawdown on hourly stock data vs S&P 500 at 15.97%.

### NautilusTrader
Most mature production-grade event-driven trading engine. Rust core, Python bindings. Nanosecond resolution. No built-in LLM integration by design — the best foundation to build LLM strategies on top of.

### OctoBot
Open source (4k+ stars). Pluggable "tentacles" architecture. ChatGPT trading mode feeds market context to an LLM and trades on its output. Supports local models via Ollama.

---

## Who Owns the Loop

In all academic frameworks: **the agent framework owns the loop**. No subprocesses. The LLM is called per iteration; the graph/workflow drives timing.

In production systems:
- **Temporal-based**: Temporal Workflows own the loop. The LLM agent is triggered by Signals or Schedules (nudge every N seconds). Agent re-reads state on each wake — never goes blind.
- **Asyncio harness**: Python event loop owns the scheduling. LLM called as a function per cycle. The harness drives; the LLM decides.

**Consensus**: harness owns the loop, agent is called per iteration. The agent should not launch a subprocess that runs for an hour.

---

## General Agentic Frameworks

### LangGraph
Leading framework for stateful long-running agents. Durable state via checkpointers — survives restarts and context resets. Human-in-the-loop: pauses, saves state, waits, resumes. **Native cron via `SyncCronClient`** — fires a new thread per schedule, configurable retention. Real-time streaming from background workers. TradingAgents was built on this.

### AutoGen v0.4 (Microsoft, Jan 2025)
Full async redesign. Event-driven architecture. Mid-execution control: pause, redirect, adjust, resume. OpenTelemetry integration. No native cron — requires external orchestration.

### CrewAI (with Flows)
Role-based multi-agent collaboration. "Flows" adds event-driven orchestration with state management. Production triggers via webhooks (CI/CD, cron, Zapier). No native cron primitive.

### OpenAI Agents SDK (March 2025)
Agents as objects with tools, handoffs, memory. Webhook support for batch/background completion. MCP support. No native cron — scheduling is left to the developer.

### Agno (formerly Phidata)
Claims ~10,000x faster than LangGraph. Durable execution — state saved at each step, resume from interruption. Persistent session storage. No native cron.

### Composio
Tool/action layer, not an agent runtime. 1,000+ toolkits with authentication. Explicit guidance: **one tool = one atomic action**. Supports call-now/callback-later for long-running tools. 100,000+ developers.

---

## Scheduling Primitives

| Mechanism | Native? | Best For |
|-----------|---------|----------|
| LangGraph cron (`SyncCronClient`) | Yes | Periodic agent runs (daily analysis, cache refresh) |
| Temporal Schedules + Signals | External | Production trading loops, sub-minute triggers |
| Claude Code `/loop` | Yes (Claude Code built-in) | Agent-owned recurring tasks at human-interactive cadence |
| APScheduler / cron daemon | External | Simple interval triggers for any Python process |
| Webhooks (CrewAI, AutoGen) | Via integration | Event-driven triggers from external systems |

### Temporal (Best for Production Trading)
- **Schedules**: trigger Workflows at regular intervals (seconds to days), manages drift and overlap
- **Signals**: send typed events to a running Workflow mid-execution (nudge every N seconds)
- **Timers**: sleep within a Workflow for precise durations, survives restarts
- **Queries**: read Workflow state without interrupting it
- **Retry**: Activities retry automatically with configurable backoff

Pattern: Temporal Schedule fires nudge → Workflow sends Signal to execution agent → agent wakes, fetches market data, places order, sleeps → repeat. Agent never goes blind — all state in Temporal history.

---

## The Blindness Problem

Well-recognized and actively researched:

- Application-level instrumentation (LangChain, AutoGen) captures reasoning and tool selection, but a spawned shell command escapes their view entirely.
- **AgentSight** (2025) proposes eBPF-level system monitoring to observe subprocesses, file access, and network calls independent of the agent framework.
- Agents degrade over long runs: sharp for first 10-20 tool calls, then repeat/forget/cycle around the 30-minute or 50-tool-call mark.
- Anthropic's long-running agent harness design uses external state files (`progress.json`) and git history as cross-session memory. Agent reads on startup — the visibility mechanism.
- MCP November 2025 spec added **Tasks** — upgrades MCP from synchronous to call-now/fetch-later. Tool call returns a durable handle immediately; real work runs in background; agent polls or subscribes.

### Solutions in Practice
1. **Status command** — agent calls a separate `status` subcommand to poll the running process
2. **Progress files** — subprocess writes JSON to a predictable path on each tick; agent reads with a read tool
3. **Atomic commands** — subprocess only does one operation; harness calls agent on each tick
4. **MCP background-job pattern** — `start`/`status`/`cancel` lifecycle for long-running commands

---

## Tool Design: Atomic vs Compound

**Industry consensus: one tool = one atomic action.**

- Compound tools with multiple action types cause LLM confusion about which parameters apply to which action
- Atomic tools produce cleaner logs (each call = one action)
- Easier to debug, grant/revoke permissions granularly

**Exception: transactional bracket orders** (entry + TP + SL simultaneously). Splitting creates a race condition — compound is correct here. This is what `skill-shark` implements.

The existing `skill-trading` split (`order limit`, `order market`, `order cancel`, `position close`) is the correct atomic design.

---

## CLI vs MCP vs Function Calling in Agentic Context

| Pattern | Agent Visibility | Loop Ownership | Token Cost | Best For |
|---------|-----------------|----------------|------------|----------|
| CLI atomic (one op) | Full — sees each result | Agent | Low | Order placement, balance check, position fetch |
| CLI long-running loop | None — blind for duration | Subprocess | Low | Unreliable for agentic use |
| MCP with Tasks (call+poll) | Partial — polls status | Subprocess | Medium | Long-running ops with progress reporting |
| Function calling (native) | Full | Agent | Medium | Any atomic operation |
| Agent owns loop via `/loop` | Full — sees each iteration | Agent/Scheduler | Low per call | TWAP, DCA, monitoring |

---

## What This Means for skill-trading

### Current architecture problems
- `twap` runs internally for up to hours — agent blind for entire duration
- `risk trail-watch` runs indefinitely — agent blind for entire duration
- Agent cannot react to mid-run events (price dump, news, margin call)

### Recommended redesign

**Option A: Agent-owned loop via `/loop`**
Expose atomic `twap slice` command (one market order, one call). Agent uses `/loop` to call it every N minutes. Agent sees every price, every fill, every error. Can stop, adjust, or react mid-run.

```
/loop every 4m: skill-trading twap-slice -e orderly -s NEARUSDT --budget 15 --decimals 0
```

**Option B: Status file + polling**
Keep the internal loop but write JSON progress to `~/.twap-{symbol}.json` after each slice (already implemented for crash recovery). Agent reads the file on demand. Partial visibility — agent sees state when asked.

**Option C: Temporal Workflow**
Full production solution. Temporal owns the loop, signals the agent on each tick, stores all state durably. Overkill for current scale but the right architecture for a multi-user platform.

### Immediate improvements
1. `twap-slice` atomic subcommand — one slice, one call, returns JSON result ✓ enables `/loop` pattern
2. `trail-watch` progress file — writes JSON status on each tick, readable by agent
3. Notification hooks — CLI POSTs to webhook (Telegram bot, local server) on each fill

---

## Production Reliability Notes

- LLM inference latency (500ms–3s) makes LLMs unsuitable for HFT or market-making — all production LLM trading operates at per-minute cadence at fastest
- Partial fills: known unsolved problem at the LLM layer in all published frameworks
- Risk layer should be deterministic and separate from LLM decision layer — LLM proposes, risk engine approves/rejects before order submission
- Fallback model chains for reliability (primary → fallback → fallback)
- Most production LLM trading systems separate the alpha/decision layer (LLM) from the execution layer (deterministic)

---

## Sources

- TradingAgents: arxiv.org/abs/2412.20138
- FLAG-Trader: arxiv.org/abs/2502.11433
- NeurIPS 2025 Financial Agents: arxiv.org/abs/2512.02227
- LangGraph Platform: blog.langchain.com/langgraph-platform-ga
- AutoGen v0.4: devblogs.microsoft.com/autogen/autogen-reimagined-launching-autogen-0-4
- Temporal for AI: temporal.io/solutions/ai
- MCP November 2025 spec: modelcontextprotocol.io/specification/2025-11-25
- Anthropic Building Effective Agents: anthropic.com/research/building-effective-agents
- AgentSight eBPF: arxiv.org/html/2508.02736v1
- Composio Tool Design: composio.dev/blog/how-to-build-tools-for-ai-agents-a-field-guide
- NautilusTrader: nautilustrader.io
- OctoBot: github.com/Drakkar-Software/OctoBot
