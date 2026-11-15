# Data sources

One provenance file per candidate data source. Raw data is **not committed**: it goes in `research/data/raw/` (gitignored) and must be fetched by a committed script. See [`../../AGENTS.md`](../../AGENTS.md) §2.

**License rule:** only sources that are **public domain / U.S. government works** or under an **explicit, unrestricted open license** are listed. Restricted-access, share-alike (ODbL), noncommercial, and unclear-license sources were removed to satisfy Official Rules §7.b.ii, §7.b.iii, §10.4, and §10.5.

When a source is actually used, fill in its **version/vintage**, **access date**, and **cleaning steps**. For a new source, copy [`_TEMPLATE.md`](_TEMPLATE.md).

| Source | File | Main use | License | Flag |
| --- | --- | --- | --- | --- |
| Census Building Permits Survey | [census-building-permits-survey.md](census-building-permits-survey.md) | **Baseline production (core)** | Public domain | **In use** (2021-2025 state files); ADUs reported inconsistently |
| Census Population Estimates (state totals) | [census-population-estimates.md](census-population-estimates.md) | Per-capita scaling, state profile | Public domain | **In use**; one year applied to the whole horizon |
| Census PEP Housing Unit Estimates | [census-housing-unit-estimates.md](census-housing-unit-estimates.md) | Baseline stock | Public domain | Built from permits, so not independent |
| Census Survey of Construction | [census-survey-of-construction.md](census-survey-of-construction.md) | Permit→completion lags | Public domain | Region-level only |
| American Community Survey | [acs.md](acs.md) | Housing characteristics, eligible single-family stock, vacancy | Public domain | Margins of error |
| Census TIGER/Line | [census-tiger-line.md](census-tiger-line.md) | Geography and joins | Public domain | |
| BLS construction (QCEW/CES/PPI) | [bls-construction.md](bls-construction.md) | Capacity and costs | Public domain | PPI national only |
| FHFA House Price Index | [fhfa-hpi.md](fhfa-hpi.md) | Market strength / pro forma | Public domain | Index, not levels; no new construction |
| National Transit Database | [national-transit-database.md](national-transit-database.md) | TOD service levels | Public domain | No stop locations |
| USGS NLCD | [nlcd.md](nlcd.md) | Developed land screening | Public domain | 30 m; large |
| NASA land cover (MODIS) | [nasa-land-cover.md](nasa-land-cover.md) | Secondary land-cover check | NASA open data, no restrictions | 500 m; free Earthdata login |
| State/local open data portals | [state-local-open-data.md](state-local-open-data.md) | Parcels, zoning, fees | **Per portal; must pass the license rule** | PII in owner fields |

Sources marked "Not yet accessed" have been described from publisher documentation only. Verify their details when they are first downloaded.

## Removed sources and open gaps

These sources were evaluated and **removed** on 2026-10-28. Don't add them back without written clarification from the organizers or the publisher.

| Source | Reason removed | Gap it leaves / candidate replacement |
| --- | --- | --- |
| HUD USPS Vacancy data | **Restricted access** (governmental entities and registered non-profits only) | Vacancy: use ACS B25004 |
| Microsoft US Building Footprints | **ODbL share-alike** (copyleft-style data license) | Building footprints for ADU/missing-middle feasibility. FEMA USA Structures is a candidate, but catalogs disagree on its license (public domain vs. CC BY 4.0), so confirm with FEMA (FEMA-RGO@fema.dhs.gov) first. |
| Google Open Buildings | CC BY 4.0 / ODbL, and **no US mainland coverage** | None needed |
| Wharton WRLURI | **License/terms unclear** | State regulatory stringency covariate: build our own from public statutes, or confirm terms with the authors |
| National Zoning Atlas | **Terms vary by state atlas and may be noncommercial** | Baseline zoning rules: use local zoning data from portals that pass the license rule, or state statute text |
| Transit stops (BTS National Transit Map / agency GTFS) | **License unclear**: data.gov records variously show public domain, "no license provided", and CC BY 3.0 inherited from agency feeds | **Stop locations for TOD buffers (needed for Scenario 3).** Options: a single agency's GTFS feed with an explicit public-domain/CC0 license, or ask the organizers whether NTAD is acceptable. |
