---
name: tod-evidence
description: Evidence and modeling guidance for Scenario 3 (transit-oriented density): upzoning near transit, defining major transit stops and proximity thresholds, entitlement attrition, and null results. Use for questions about housing near transit, station-area upzoning, SB 79 or MBTA Communities-style laws.
---

# Transit-oriented density expert

**Scenario 3:** higher density near major transit stops, with a justified proximity threshold.

Start with `mcp__housing-ontology__evidence_brief` `{"domain": "tod", "query": "<question>"}`.

## Core evidence

| Study id | Type / outcome | Key point |
| --- | --- | --- |
| `anagol_ferreira_rexer_2026` | structural / stock | São Paulo corridor densification: **+1.6% stock, -0.4% prices** (published 2026). Don't use the older 1.9% working-paper figure. |
| `freemark_2019` | quasi-experimental / permits | Chicago: **no detectable** permitting increase over 5 years on upzoned transit parcels. |
| `kestelman_2025` | quasi-experimental / applications | LA TOC raised proposed units, not permitted units. Proposal-to-permit attrition is large. |
| `zhu_2021_la_toc` | pro forma | TOC packages can out-pencil market-rate in some neighborhoods. |
| `su_curran_groome_freemark_2026`, `liao_2023` | quasi-experimental / stock | U.S. upzoning responses vary by neighborhood. |
| `ben_moshe_genesove`, `adachi_2026` | structural | Scale economies near 5 floors; capital-land substitution at height. |
| `kulka_sood_chiumenti_2026` | boundary design | Relaxing one rule may not matter if density still binds; TOD counterfactuals there are modeled. |

## Modeling guidance

- Define the stop rule explicitly: modes, peak headway, distance (0.25 vs. 0.5 mile), and whether planned transit counts. Test sensitivity to it.
- A parcel near transit gains nothing if baseline zoning already allows the proposed density. Compute the **incremental** capacity.
- Follow the pipeline proposal → permit → completion. Don't count proposed or approved units as starts.

## Data gap (blocking)

**There is no clean-license stop-location source yet.** The BTS National Transit Map was rejected for unclear terms, and Cal-ITP GTFS is unverified. NTD has service levels but no locations. See `research/data/README.md`.
