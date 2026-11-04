---
name: adu-evidence
description: Evidence and modeling guidance for Scenario 1 (accessory dwelling units): ADU uptake, permitting predictors, financing, state preemption, and permitted vs. occupied ADUs. Use for any question about ADU laws or ADU production estimates.
---

# ADU expert

**Scenario 1:** allow one ADU on parcels with single-family homes, at least 600 sq ft, with no added parking requirement.

Start with `mcp__housing-ontology__evidence_brief` `{"domain": "adu", "query": "<question>"}`. Use `mcp__housing-ontology__get_entity` for any study id below.

## Core evidence

| Study id | Type / outcome | What it gives the model |
| --- | --- | --- |
| `marantz_elmendorf_kim_2023` | associational / permits | Highest-priority input. Larger parcels and multiple structures predict permitting (CA HCD APR, 14 counties). Not a causal statewide effect. |
| `bhatt_2023` | quasi-experimental / permits | Most direct ADU policy-effect design (DiD across zones). Don't turn the zone coefficient into a parcel uptake rate. |
| `pew_2026_adu` | descriptive / permits | 81 ADU permits per 100k residents (2025) under permissive laws, against 26 outside California. Report CA separately. |
| `rose_institute_2024` | descriptive / permits | ADU reforms raised construction substantially; SB 9 produced few units. Use different take-up rates for ADUs and missing middle. |
| `portland_rip_2025` | descriptive / permits | Over 1,400 ADU and middle units (Aug 2021 to Jun 2024). |
| `chapple_2017`, `wang_2026_seattle_adu` | survey; lead | Financing and owner capacity limit uptake. |
| `crane_2020` | survey | A permitted ADU is not necessarily occupied long-term housing. |
| `wielga_2023_cityscape`, `alameldin_underriner_2023` | measurement; descriptive | Preemption details and local incentives vary. |

## Modeling guidance

- Pipeline: eligible single-family parcels (ACS, plus parcels from a portal that passes the license rule), then owner uptake by parcel traits, then permit, then completion (Survey of Construction lags).
- **Separate legalizations from new construction.** ADU permits can legalize existing units.
- Census BPS counts ADUs inconsistently, so state APR-type reports are needed to validate.
- Vary uptake low/central/high around financing and implementation, not only market strength.

## Data cautions

Microsoft footprints (ODbL), CoreLogic, and Boundary Solutions were used by Marantz et al. but are **not usable**. There is no clean-license footprint source yet (see `research/data/README.md`).
