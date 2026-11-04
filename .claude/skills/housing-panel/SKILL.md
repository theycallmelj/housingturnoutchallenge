---
name: housing-panel
description: Answer any question about state housing policy effects (ADUs, missing middle, transit-oriented density, parking, development fees), the evidence behind them, or which data the challenge submission may use. Routes to the domain expert skills and the housing-ontology MCP server. Use whenever a housing-policy research or modeling question comes up in this repo.
---

# Housing policy expert panel

This repo has a structured evidence base: [`ontology/housing-ontology.json`](../../../ontology/housing-ontology.json) holds 77 studies and 41 datasets. The `housing-ontology` MCP server (registered in `.mcp.json`) exposes it. Use the server rather than recalling studies from memory.

## Workflow

1. **Route.** Call `mcp__housing-ontology__route_query` with the question to see which domains it touches.
2. **Gather.** For each domain, load the matching skill below and call `mcp__housing-ontology__evidence_brief` with `{domain, query}`.
3. **Or ask the whole panel.** `mcp__housing-ontology__ask_experts` runs the condesate orchestrator, which spawns one sandboxed expert per relevant domain. From a shell: `cargo run --manifest-path agents/Cargo.toml --bin housing-ask -- "<question>"`.
4. **Synthesize** under the rules below.

| Domain id | Skill | Scenario |
| --- | --- | --- |
| `adu` | `adu-evidence` | 1: one ADU per single-family parcel, at least 600 sq ft, no added parking |
| `missing_middle` | `missing-middle-evidence` | 2: 2-6 units on single-family parcels, up to 3 stories |
| `tod` | `tod-evidence` | 3: higher density near major transit stops |
| `parking` | `parking-evidence` | 4: reduce or eliminate parking minimums |
| `fees` | `fees-evidence` | 5: 50% fee cut from an assumed $50,000/unit |
| `baseline_data` | `housing-data-provenance` | Baseline, datasets, licenses |
| `modeling` | `housing-model-design` | Architecture, interactions, uncertainty |

## Rules for every answer

- **Name the outcome with every number.** Permits, starts, completions, stock, feasible capacity, applications, and parking supply are different quantities. Most "starts" in this literature are permits or consents.
- **Cite the study id** (e.g. `terner_sb9_2021`) and its evidence type. `citation_only` and `lead` entries carry no finding, so never quote one.
- **Don't add effects** across scenarios, outcome types, or overlapping studies (e.g. the two Auckland estimates). Point to interactions instead.
- **Data:** only datasets with license status `open_verified` may be used in the submission. Name blocked sources as blocked (AGENTS.md §2).
- **Ranges, not points.** The challenge rewards direction, magnitude, uncertainty, and drivers.
