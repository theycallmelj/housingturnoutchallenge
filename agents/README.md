# housing-agents

An orchestrator with one expert agent per policy domain, answering questions from the [housing policy evidence ontology](../ontology/README.md). Built on [`condesate`](https://crates.io/crates/condesate) (agent loop, tools, and permission kernel). It also ships an MCP server so any MCP client can query the same evidence.

```
question ─► orchestrator ──consult.<domain>──► expert.<domain> ──<domain>.* tools──► ontology
             Privileged      spawn_child          Sandboxed
```

| Expert | Domain | Tools it may call |
| --- | --- | --- |
| `expert.adu` | Scenario 1, ADUs | `adu.*`, `ontology.get` |
| `expert.missing_middle` | Scenario 2, 2-6 units | `missing_middle.*`, `ontology.get` |
| `expert.tod` | Scenario 3, transit density | `tod.*`, `ontology.get` |
| `expert.parking` | Scenario 4, parking | `parking.*`, `ontology.get` |
| `expert.fees` | Scenario 5, fees | `fees.*`, `ontology.get` |
| `expert.baseline_data` | Baseline, data, licenses | `baseline_data.*`, `ontology.get` |
| `expert.modeling` | Architecture, interactions, uncertainty | `modeling.*`, `ontology.get` |

Each domain has three tools: `evidence_brief`, `find_studies`, and `find_datasets`.

## How the permissions work

- The orchestrator is admitted with a spawn grant for `expert.*`. Each `consult.<domain>` tool spawns its expert through `GuardedServices::spawn_child`, so the expert's authority is **attenuated** from the orchestrator's.
- An expert may invoke only its own `<domain>.*` tools plus `ontology.get`, write only under `findings/<domain>/`, and cannot spawn agents. The kernel refuses anything else before the tool runs.
- Every decision is audited. `housing-ask --audit` prints the trail.

Tests in `src/panel.rs` prove the boundary holds: the ADU expert is denied `fees.evidence_brief` and `findings/fees/`, and can't spawn further agents.

## Run

```bash
# Ask the panel (offline, deterministic)
cargo run --manifest-path agents/Cargo.toml --bin housing-ask -- "Would eliminating parking minimums near transit add housing?"
cargo run --manifest-path agents/Cargo.toml --bin housing-ask -- --audit "What do impact fee cuts do?"

# Tests: ontology integrity, skills ↔ ontology consistency, routing, permission scoping, MCP protocol
cargo test --manifest-path agents/Cargo.toml
```

**MCP server.** `.mcp.json` at the repo root registers `housing-ontology` for Claude Code. Its tools are `route_query`, `evidence_brief`, `find_studies`, `find_datasets`, `get_entity`, and `ask_experts`. To run it by hand:

```bash
cargo run --release --manifest-path agents/Cargo.toml --bin housing-mcp
```

The skills in [`.claude/skills/`](../.claude/skills/) tell agents how to use these tools for each domain.

## Models: Apache-2.0 only, leader stronger than experts

| Backend | When | Behavior |
| --- | --- | --- |
| Offline (default) | Always available | Deterministic. The orchestrator routes by ontology keywords; each expert pulls its evidence brief and condenses it. It answers only from tool results, needs no network, and is fully reproducible. |
| OpenRouter | `--features llm`, `HOUSING_LLM_HOST=openrouter`, and an API key | Hosted. No GPU or server to run. Recommended for demos and judging. |
| Self-hosted | `--features llm` and `HOUSING_LLM_BASE_URL` | Your own OpenAI-compatible server: Ollama (default naming) or vLLM (`HOUSING_LLM_HOST=vllm`). |

With either model backend, each agent calls its own model. Tool calls use condesate's `PromptedToolModel`, and `<think>` reasoning blocks are stripped.

[`model-registry.json`](model-registry.json) is the allow-list. Every model in it is **Apache-2.0**, verified from its Hugging Face card. When the panel starts, `ModelPlan::resolve` checks these rules and refuses to run otherwise:

1. Every configured model is in the registry with license Apache-2.0. Llama, Gemma, and Claude are listed as rejected, with the reason.
2. The **leader (orchestrator) outranks every expert** by `capability_rank`. The rank orders models by size and generation; it isn't a benchmark score.
3. Every chosen model is **served by the chosen host**. Each host names models its own way: Ollama `qwen3:32b`, vLLM `Qwen/Qwen3-32B`, OpenRouter `qwen/qwen3-32b`. OpenRouter also requires a key.

| Profile | Leader | Experts | Where |
| --- | --- | --- | --- |
| `openrouter` (default when the host is OpenRouter) | `gpt-oss-120b` | `gpt-oss-20b` | OpenRouter: $0.15 / $0.60 and $0.03 / $0.13 per 1M tokens (in / out) on 2026-11-15 |
| `laptop` (default otherwise) | `qwen3-4b` | `qwen3-1.7b` | 8 GB Apple Silicon with Ollama |
| `workstation` | `qwen3-32b` | `qwen3-8b` | 32 GB+ or a 24 GB GPU |
| `server` | `gpt-oss-120b` | `gpt-oss-20b` | 80 GB GPU for the leader |

### OpenRouter (hosted)

```bash
cp agents/.env.example agents/.env      # then put your key in agents/.env (gitignored)
set -a; source agents/.env; set +a
cargo run --features llm --manifest-path agents/Cargo.toml --bin housing-ask -- --audit "How many ADUs could a state law add?"
```

Buy paid credits rather than relying on `:free` models: free models are capped at 20 requests a minute and 50 a day, and one panel question makes several calls. OpenRouter adds a 5.5% fee when you buy credits; token prices are the hosts' list prices. The question and the ontology excerpts are sent to OpenRouter and its upstream hosts, so never put secrets or personal data in a question.

### Self-hosted (Ollama shown)

```bash
ollama pull qwen3:4b && ollama pull qwen3:1.7b
export HOUSING_LLM_BASE_URL=http://localhost:11434/v1
cargo run --features llm --manifest-path agents/Cargo.toml --bin housing-ask -- --audit "..."
```

| Variable | Effect |
| --- | --- |
| `HOUSING_LLM_HOST` | `ollama` (default), `vllm`, or `openrouter` |
| `HOUSING_LLM_BASE_URL` | OpenAI-compatible base URL. Defaults to `https://openrouter.ai/api/v1` for OpenRouter. Unset on other hosts means offline. |
| `HOUSING_LLM_API_KEY` (or `OPENROUTER_API_KEY`) | Bearer token. Required for OpenRouter; optional for self-hosted. |
| `HOUSING_MODEL_PROFILE` | `openrouter`, `laptop`, `workstation`, or `server` |
| `HOUSING_LEADER_MODEL` | Registry id overriding the leader |
| `HOUSING_EXPERT_MODEL` | Registry id overriding every expert |
| `HOUSING_EXPERT_MODEL_<DOMAIN>` | Per-expert override, e.g. `HOUSING_EXPERT_MODEL_ADU=qwen3-14b` |

Each agent's manifest records the host and the model it runs (e.g. `openrouter` / `openai/gpt-oss-120b` for the orchestrator), so the `--audit` trail shows which model made each call. The MCP server reads the same variables; add them under `env` in `.mcp.json` to use open models there.

## SBOM

[`sbom.cdx.json`](sbom.cdx.json) is CycloneDX 1.5 covering all features. Every component is permissive; see [`DEPENDENCIES.md`](DEPENDENCIES.md). To regenerate after any dependency change:

```bash
cargo install cargo-cyclonedx --locked   # once
cd agents && cargo cyclonedx -f json --all-features --spec-version 1.5 --override-filename sbom.cdx
```
