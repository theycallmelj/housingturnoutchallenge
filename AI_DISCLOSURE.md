# AI Disclosure

Material use of AI tools, models, and APIs in this project, as required by the challenge handbook. See [AGENTS.md](AGENTS.md) §3 for the rules.

## Development-time AI assistance

| Date | Tool | Model | What it produced | Human verification |
| --- | --- | --- | --- | --- |
| (before 2026-10-28) | OpenAI Codex | not recorded | `prototypes/state-housing-policy-lab/`, the UI prototype with fictional fixtures (inferred from the Codex paths in its README and `.openai/hosting.json`) | Contributor to fill in |
| 2026-10-28 | Claude Code | `claude-opus-5-5` | `AGENTS.md`, `CLAUDE.md`, this file, `research/` (paper search, `papers.tsv`, `fetch_papers.sh`, literature index, data-source provenance files) | Pending review by @theycallmelj; data-source facts should be spot-checked against the linked publisher pages |
| 2026-10-28 | Claude Code | `claude-opus-5-5` | Apache-2.0 `LICENSE` (official text from apache.org), root `README.md`, license audit against Official Rules §7/§10: removed committed PDFs and restricted, share-alike, or unclear-license data sources | Pending review by @theycallmelj |

| 2026-11-03 | Claude Code | `claude-opus-5-5` | `ontology/` (built from the team research memo and combined workbook), `agents/` (condesate expert panel, MCP server, tests, SBOM), `.claude/skills/`, `.mcp.json` | 22 automated tests (`cargo test --manifest-path agents/Cargo.toml`), including ontology integrity, skills-to-ontology consistency, and permission-boundary tests. Ontology figures were transcribed from the team workbook; spot-check against the source documents is pending review by @theycallmelj |

## Runtime AI / external APIs

The default build makes **no** AI or network calls. The prototype's "assistant" is scripted (`src/demoAssistant.ts`), and the `agents/` panel defaults to deterministic offline models.

| Service | Purpose | Cost / access terms | Called from |
| --- | --- | --- | --- |
| Anthropic Messages API (model from `HOUSING_AGENT_MODEL`, default `claude-sonnet-5-5`) | Optional LLM backend for the orchestrator and expert agents | Paid API; needs the user's own `ANTHROPIC_API_KEY`. **Off by default**: only with `--features llm` and the key set. The offline backend gives free access to the same evidence. | `agents/src/models.rs` via condesate `AnthropicModel` |
