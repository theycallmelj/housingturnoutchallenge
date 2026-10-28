# AI Disclosure

Material use of AI tools, models, and APIs in this project, as required by the challenge handbook. See [AGENTS.md](AGENTS.md) §3 for the rules.

## Development-time AI assistance

| Date | Tool | Model | What it produced | Human verification |
| --- | --- | --- | --- | --- |
| (before 2026-10-28) | OpenAI Codex | not recorded | `prototypes/state-housing-policy-lab/`, the UI prototype with fictional fixtures (inferred from the Codex paths in its README and `.openai/hosting.json`) | Contributor to fill in |
| 2026-10-28 | Claude Code | `claude-opus-5-5` | `AGENTS.md`, `CLAUDE.md`, this file, `research/` (paper search, `papers.tsv`, `fetch_papers.sh`, literature index, data-source provenance files) | Pending review by @theycallmelj; data-source facts should be spot-checked against the linked publisher pages |
| 2026-10-28 | Claude Code | `claude-opus-5-5` | Apache-2.0 `LICENSE` (official text from apache.org), root `README.md`, license audit against Official Rules §7/§10: removed committed PDFs and restricted, share-alike, or unclear-license data sources | Pending review by @theycallmelj |

## Runtime AI / external APIs

None at this time. The prototype's "assistant" is scripted (`src/demoAssistant.ts`) and makes no network calls.

| Service | Purpose | Cost / access terms | Called from |
| --- | --- | --- | --- |
| None | | | |
