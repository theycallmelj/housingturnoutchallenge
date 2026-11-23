---
name: parking-evidence
description: Evidence and modeling guidance for Scenario 4 (parking reduction): minimum parking repeal, voluntary developer parking, parking construction costs, and state parking preemption. Use for any question about parking minimums and housing.
---

# Parking expert

**Scenario 4:** reduce or eliminate minimum parking requirements, with explicit assumptions about developer-provided parking.

Start with `mcp__housing-ontology__evidence_brief` `{"domain": "parking", "query": "<question>"}`.

## Core evidence

The literature is strong on **parking supplied** and weak on **housing units**. Keep the two apart.

| Study id | Type / outcome | Key figure |
| --- | --- | --- |
| `gabbe_pierce_clowers_2020` | associational / parking | Seattle, 868 developments: 40% less parking than old standards required; about 70% without a minimum still built some. |
| `li_guo_2014` | quasi-experimental / parking | London: about 0.76 fewer spaces per dwelling, about 49% of the earlier level (minimums replaced with maximums). |
| `sohoni_lee_2024` | quasi-experimental / parking | 84% of 43 developments provided less parking than required before. |
| `sohoni_lee_2026` | quasi-experimental / parking | Response depends on demand and prior policy; little change in some low-density cities. |
| `hess_rehler_2021` | descriptive / parking | Buffalo, 36 projects: voluntary parking choices after repeal. |
| `andersson_2016` | structural / stock | Simulated: parking norms reduce stock by 1.2% (Stockholm suburb, long run). |
| `tanrisever_2025` | event study / permits | Poster reports supply up after reforms. **No coefficient yet**; get the full paper. |
| `lee_millard_ball_manville_2026` | descriptive | CA preemption is implemented unevenly (supportive vs. resistant cities). |
| `schwartz_2026_parking_costs`, `schwartz_manville_millard_ball_2026` | measurement; review | Parking costs across 17 cities; synthesis. |

## Modeling guidance

- Model parking as a **cost inside the pro forma**: developers choose parking by demand, and repeal lowers cost only where minimums bind.
- **Never** convert spaces removed into units added.
- Minneapolis (`pew_minneapolis_2024` vs. `minneapolis_fed_2025`) and Lower Hutt bundle parking with upzoning, so don't attribute their totals to parking alone.
