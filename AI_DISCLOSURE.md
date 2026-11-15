# AI Disclosure

Material use of AI tools, models, and APIs in this project, as required by the challenge handbook. See [AGENTS.md](AGENTS.md) §3 for the rules.

## Development-time AI assistance

| Date | Tool | Model | What it produced | Human verification |
| --- | --- | --- | --- | --- |
| (before 2026-10-28) | OpenAI Codex | not recorded | `prototypes/state-housing-policy-lab/`, the UI prototype with fictional fixtures (inferred from the Codex paths in its README and `.openai/hosting.json`) | Contributor to fill in |
| 2026-10-28 | Claude Code | `claude-opus-5-5` | `AGENTS.md`, `CLAUDE.md`, this file, `research/` (paper search, `papers.tsv`, `fetch_papers.sh`, literature index, data-source provenance files) | Pending review by @theycallmelj; data-source facts should be spot-checked against the linked publisher pages |
| 2026-10-28 | Claude Code | `claude-opus-5-5` | Apache-2.0 `LICENSE` (official text from apache.org), root `README.md`, license audit against Official Rules §7/§10: removed committed PDFs and restricted, share-alike, or unclear-license data sources | Pending review by @theycallmelj |
| 2026-11-03 | Claude Code | `claude-opus-5-5` | `ontology/` (built from the team research memo and combined workbook), `agents/` (condesate expert panel, MCP server, tests, SBOM), `.claude/skills/`, `.mcp.json` | 22 automated tests (`cargo test --manifest-path agents/Cargo.toml`), including ontology integrity, skills-to-ontology consistency, and permission-boundary tests. Ontology figures were transcribed from the team workbook; spot-check against the source documents is pending review by @theycallmelj |
| 2026-11-03 | Claude Code | `claude-opus-5-5` | Apache-2.0 model registry, leader/expert model plans, and OpenAI-compatible provider in `agents/` (replaces the Anthropic backend) | 29 tests with `--features llm`, including license refusal, leader-outranks-experts, per-expert models, and a mock HTTP server; licenses verified from Hugging Face card metadata |
| 2026-11-15 | Claude Code | `claude-opus-5-5` | OpenRouter and vLLM hosts in `agents/` (`Host`, per-host model ids, `openrouter` profile, `.env.example`); native function tool calling with text fallback; stop sequence and tool-call cleanup; generation-failure retries that skip failed hosts; grounding guards; automatic citation check against the ontology | 39 tests with `--features llm` (30 without), all passing, including regression tests built from real gpt-oss failures (repeated tool calls, invented tool results, invented study ids and figures). Live runs on OpenRouter showed the leader consulting the ADU expert and the expert calling only its own tools. OpenRouter ids checked against its live models API. |

## Runtime AI / external APIs

The default build makes **no** AI or network calls. The prototype's "assistant" is scripted (`src/demoAssistant.ts`), and the `agents/` panel defaults to deterministic offline models.

| Service | Purpose | Cost / access terms | Called from |
| --- | --- | --- | --- |
| Open-weight models from `agents/model-registry.json` (Apache-2.0 only: Qwen3, gpt-oss, Mistral Small 3.2, Granite 3.3) | Optional LLM backend. A stronger leader model orchestrates smaller expert models. | Free to download and run; the user serves them on their own OpenAI-compatible server (e.g. Ollama). **Off by default**: only with `--features llm` and `HOUSING_LLM_BASE_URL` set. The offline backend gives the same evidence with no model at all. | `agents/src/openai_compat.rs`, configured in `agents/src/models.rs` |
| OpenRouter (`https://openrouter.ai/api/v1`) | Optional hosted endpoint for the same Apache-2.0 models: gpt-oss-120b leader, gpt-oss-20b experts | Paid per token at the upstream hosts' list prices ($0.15/$0.60 and $0.03/$0.13 per 1M tokens on 2026-11-15) plus a 5.5% credit fee; needs the user's own `OPENROUTER_API_KEY`. **Off by default**: only with `--features llm` and `HOUSING_LLM_HOST=openrouter`. Free alternatives: self-host the same Apache-2.0 weights, or use the offline backend. Questions and ontology excerpts are sent to OpenRouter and its upstream hosts. | `agents/src/openai_compat.rs`, configured in `agents/src/models.rs` |
