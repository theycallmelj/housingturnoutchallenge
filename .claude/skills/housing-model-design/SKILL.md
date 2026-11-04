---
name: housing-model-design
description: Model architecture for the housing challenge: pro forma + uptake + timing pipeline, policy interactions, uncertainty ranges, validation targets, and what not to add together. Use when designing or reviewing the simulation engine, combining scenarios, or setting low/central/high assumptions.
---

# Modeling expert

Start with `mcp__housing-ontology__evidence_brief` `{"domain": "modeling", "query": "<question>"}`.

## Recommended architecture (team memo + `terner_simulator`)

```
eligible parcels ─► incremental legal capacity ─► financially feasible projects ─► uptake over time ─► permits ─► completions ─► net stock
   (rules diff)        (vs. baseline zoning)        (pro forma: rents, costs,       (owner/developer     (SOC lags,     (minus demolitions,
                                                     fees, parking, financing)        probability)          attrition)      regional displacement)
```

1. **Define the request:** geography, policy settings, effective date, five-year window, outcome.
2. **Current-policy baseline** under the same economic assumptions.
3. **Measure the actual policy change** against existing rules. A reform that duplicates baseline rights adds nothing.
4. **Implementation, uptake, timing** as separate parameters. Don't discount the same delay twice (memo).
5. **Report** baseline and reform totals, the difference, annual timing, uncertainty, assumptions, sources, and data vintage.

## Interactions

- `kulka_sood_chiumenti_2026`: model the **binding constraint**; don't multiply independent effects.
- `lee_millard_ball_manville_2026`: local compliance varies, so vary implementation separately from market conditions ("slow compliance is different from expensive borrowing").
- `kholodilin_2026`: direct interaction evidence is sparse. Say so.

## Uncertainty and validation

- Present low/central/high with editable parameters, anchored to comparison cases (e.g. `stacy_2023` about +0.8% vs. the Auckland studies). These are comparison points, not an automatic confidence interval.
- Validation targets: pre-reform hold-outs; Portland, Minneapolis (with `minneapolis_fed_2025` as the demand-shock counterpoint), and California APR series.
- Don't double-count reviews and replies (`donovan_maltman_2024`, `stacy_freemark_lo_2026_reply`) as independent evidence.

## Simpler beats opaque

The handbook discourages opaque models. Prefer the pro forma + uptake pipeline over agent-based models (`furtado_policyspace2`, `burke_2024_abm`, `waddell_2002_urbansim`) unless validation justifies the extra complexity.
