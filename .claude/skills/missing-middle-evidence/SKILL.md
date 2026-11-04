---
name: missing-middle-evidence
description: Evidence and modeling guidance for Scenario 2 (missing middle, 2-6 units on single-family lots), including SB 9 lot splits and broad upzoning studies (Auckland, Lower Hutt, NYC, Philadelphia). Use for duplex/triplex/fourplex legalization or upzoning questions.
---

# Missing-middle expert

**Scenario 2:** allow 2-6 units on single-family parcels, up to three stories.

Start with `mcp__housing-ontology__evidence_brief` `{"domain": "missing_middle", "query": "<question>"}`.

## Core evidence

| Study id | Type / outcome | Key figure |
| --- | --- | --- |
| `terner_sb9_2021` | pro forma / feasible capacity | SB 9 makes new development feasible on 5.4% of single-family parcels, about 700,000 units of capacity. Not a forecast. |
| `garcia_alameldin_2023` | descriptive / applications | Year one: LA received 211 new-unit applications, San Diego 7. Uptake is slow. |
| `greenaway_mcgrevy_2025_wp017` | synthetic control / permits | Auckland roughly doubled permits per capita in 5 years, about 52,200 extra over 7 years. Upper bound for broad by-right reform. |
| `greenaway_mcgrevy_phillips_2023` | quasi-experimental / permits | 21,808 extra permits over 5 years (spillover-adjusted). **Compare with WP017; don't add.** |
| `maltman_greenaway_mcgrevy_2024` | synthetic control / permits | Lower Hutt consents rose 3-4x; **net regional +12-17%** after displacement. |
| `stacy_2023` (+ `stacy_freemark_lo_2026_reply`) | associational / stock | About +0.8% supply 3-9 years after a typical U.S. loosening reform. The low anchor. |
| `su_curran_groome_freemark_2026` | quasi-experimental / stock + permits | NYC over 4,000 extra units in 4 years; Philadelphia effects vary. |
| `murray_limb_2022` | associational | Brisbane: lots of unused upzoned capacity. |
| `kulka_sood_chiumenti_2026` | boundary design / stock | Density limits often bind even when other rules are relaxed. |

## Modeling guidance

- Use feasible capacity × a time-varying uptake rate, then permit-to-completion lags. **Large feasible capacity can coexist with low uptake.**
- Count net new units against existing homes (Terner found about 97% of homes are retained).
- Statewide totals must be net of regional displacement (Lower Hutt).
- Anchor the range between U.S. averages (`stacy_2023`) and broad by-right reform (Auckland); explain where a given state sits.
