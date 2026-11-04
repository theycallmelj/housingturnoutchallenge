# Housing policy evidence ontology

[`housing-ontology.json`](housing-ontology.json) is a typed graph of everything the team has reviewed: 77 studies, 41 datasets, the six scenarios (baseline plus the five benchmarks), seven expert domains, outcome measures, and causal mechanisms. The agents in [`agents/`](../agents/), the MCP server, and the skills in [`skills/`](../skills/) all read from this one file.

Sources: the team research memo ("Literature & data review, + thoughts on project"), the combined research workbook (Studies sheet), [`research/papers/README.md`](../research/papers/README.md), and the provenance files in [`research/data/`](../research/data/).

## Classes

| Class | Key fields | Links to |
| --- | --- | --- |
| `scenarios` | `benchmark_definition`, `key_mechanisms` | domains, mechanisms |
| `domains` | `expert`, `keywords` (used for routing) | scenarios |
| `outcomes` | `note` (what the outcome is *not*) | |
| `mechanisms` | `description` | |
| `datasets` | `license_status`, `provenance_file`, `role`, `limitations` | domains, scenarios |
| `studies` | `evidence_type`, `outcome`, `finding`, `limitation`, `relevance`, `verification`, `url` | domains, scenarios, datasets, outcomes |
| `pilot_states` | `rationale` | datasets |

The controlled vocabularies for `license_status`, `evidence_type`, and `verification` are defined under `enums`.

## Rules encoded in the data

- **Only `open_verified` datasets are usable** in the submission. Every other status (unverified, proprietary, restricted, rejected) is kept so we know what each study relied on, but the agents flag those as unusable. This follows [`AGENTS.md`](../AGENTS.md) §2.
- **Outcomes aren't interchangeable.** Permits, starts, completions, stock, feasible capacity, and parking supply are separate outcome nodes. The agents report a study's outcome alongside its finding so they don't get mixed.
- **Versions matter.** Where the workbook flags a newer publication (e.g. Anagol et al. 2026: +1.6% stock, which replaces the working paper's 1.9%), the ontology uses the newer figure and says so in `limitation`.
- **Findings are only as strong as their `verification`.** `citation_only` and `lead` entries carry no finding.

## Editing

Add new studies or datasets directly to the JSON, then run the integrity tests:

```bash
cargo test --manifest-path agents/Cargo.toml ontology
```

They check that IDs are unique, every cross-reference resolves, enum values are valid, and every `open_verified` dataset has a provenance file that exists.
