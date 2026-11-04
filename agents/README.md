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

## Models

| Backend | When | Behavior |
| --- | --- | --- |
| Offline (default) | Always available | Deterministic. The orchestrator routes by ontology keywords; each expert pulls its evidence brief and condenses it. It answers only from tool results, needs no network, and is fully reproducible. |
| Anthropic | Built with `--features llm` **and** `ANTHROPIC_API_KEY` set | Real Claude calls through condesate's `AnthropicModel` and `PromptedToolModel`. The model is chosen by `HOUSING_AGENT_MODEL` (default `claude-sonnet-5-5`). This is a paid API, disclosed in [`AI_DISCLOSURE.md`](../AI_DISCLOSURE.md). |

```bash
ANTHROPIC_API_KEY=... cargo run --features llm --manifest-path agents/Cargo.toml --bin housing-ask -- "..."
```

**License flag:** the `llm` feature pulls in `webpki-roots` (MPL-2.0) through `condesate/remote` and `reqwest` 0.11. See [`DEPENDENCIES.md`](DEPENDENCIES.md). The default build does not include it.

## SBOM

[`sbom.cdx.json`](sbom.cdx.json) is CycloneDX 1.5 for the default build. To regenerate after any dependency change:

```bash
cargo install cargo-cyclonedx --locked   # once
cd agents && cargo cyclonedx -f json --spec-version 1.5 --override-filename sbom.cdx
```
