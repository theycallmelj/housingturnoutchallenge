# Data sources

One provenance file per candidate data source. Raw data is **not committed**: it goes in `research/data/raw/` (gitignored) and must be fetched by a committed script. See [`../../AGENTS.md`](../../AGENTS.md) §2.

When a source is actually used, fill in its **version/vintage**, **access date**, and **cleaning steps**. For a new source, copy [`_TEMPLATE.md`](_TEMPLATE.md).

| Source | File | Main use | Access | Flag |
| --- | --- | --- | --- | --- |
| Census Building Permits Survey | [census-building-permits-survey.md](census-building-permits-survey.md) | **Baseline production (core)** | Bulk, public | ADUs reported inconsistently |
| Census PEP Housing Unit Estimates | [census-housing-unit-estimates.md](census-housing-unit-estimates.md) | Baseline stock | Bulk/API, public | Built from permits, so not independent |
| Census Survey of Construction | [census-survey-of-construction.md](census-survey-of-construction.md) | Permit→completion lags | Bulk, public | Region-level only |
| American Community Survey | [acs.md](acs.md) | Housing characteristics, eligible single-family stock | API, public | Margins of error |
| Census TIGER/Line | [census-tiger-line.md](census-tiger-line.md) | Geography and joins | Bulk, public | |
| BLS construction (QCEW/CES/PPI) | [bls-construction.md](bls-construction.md) | Capacity and costs | API, public | PPI national only |
| FHFA House Price Index | [fhfa-hpi.md](fhfa-hpi.md) | Market strength / pro forma | Bulk, public | Index, not levels; no new construction |
| National Transit Database | [national-transit-database.md](national-transit-database.md) | TOD service levels | Bulk/API, public | No stop locations |
| Transit stops (GTFS / NTAD) | [transit-stops-gtfs.md](transit-stops-gtfs.md) | **TOD buffers** | Bulk, mixed licenses | Per-agency licenses |
| USGS NLCD | [nlcd.md](nlcd.md) | Developed land screening | Bulk, public | 30 m; large |
| NASA land cover (MODIS) | [nasa-land-cover.md](nasa-land-cover.md) | Secondary land-cover check | Earthdata login (free) | 500 m; handbook product unspecified |
| Microsoft Building Footprints | [microsoft-building-footprints.md](microsoft-building-footprints.md) | ADU/missing middle physical feasibility | Bulk, public | **ODbL share-alike**; large |
| Google Open Buildings | [google-open-buildings.md](google-open-buildings.md) | (Not usable for US states) | Bulk, public | **No US mainland coverage** |
| HUD USPS Vacancy | [hud-usps-vacancy.md](hud-usps-vacancy.md) | Vacancy context | **Restricted login** | **Gov/non-profit only; likely ineligible as a required input** |
| Wharton WRLURI | [wharton-wrluri.md](wharton-wrluri.md) | State regulatory stringency | Download, academic | Terms unclear; 2018 snapshot |
| National Zoning Atlas | [national-zoning-atlas.md](national-zoning-atlas.md) | Baseline zoning rules | Varies | Partial state coverage |
| State/local open data portals | [state-local-open-data.md](state-local-open-data.md) | Parcels, zoning, fees | Varies | PII in owner fields; heterogeneous |

Sources marked "Not yet accessed" have been described from publisher documentation only. Verify their details when they are first downloaded.
