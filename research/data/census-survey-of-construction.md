# Census Survey of Construction (SOC): New Residential Construction

| Field | Value |
| --- | --- |
| **Name** | New Residential Construction (starts, under construction, completions), Characteristics of New Housing |
| **Source URL** | https://www.census.gov/construction/nrc/ · https://www.census.gov/construction/chars/ |
| **Publisher** | U.S. Census Bureau (with HUD co-sponsorship) |
| **Access method** | Excel time series at https://www.census.gov/construction/nrc/xls/ (`permits_cust.xlsx`, `comps_cust.xlsx`, `avg_authtostart_cust.xlsx`, `avg_starttocomp_cust.xlsx`), fetched by [`scripts/fetch_construction_timing.py`](../../scripts/fetch_construction_timing.py) |
| **Version / vintage used** | Annual series as published on the access date (completions 2016-2025, permits 2015-2024, timing 2021-2025) |
| **Access date** | 2026-11-16 (checksums in `research/data/raw/CHECKSUMS-timing.txt`) |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | National and 4 Census regions only, monthly and annual |
| **Size** | Small, with moderate-size microdata |
| **Used for (scenarios)** | Converting permits to completions within the 5-year window: authorization-to-start and start-to-completion lag times, and the share of units authorized but never started. Also square footage and cost per square foot for pro forma inputs. |

## Description

A sample survey that follows permitted units through start and completion. It reports time lags and the physical and price characteristics of new single-family and multifamily units.

## Cleaning & transformation steps

Produces [`data/derived/completion_factors.csv`](../../data/derived/completion_factors.csv), one row per region plus the U.S.:

1. Read the `Annual` sheets of the permits and completions workbooks (thousands of units; total columns for the U.S. and each region). The xlsx files are parsed with the Python standard library (zipfile + xml).
2. `completion_ratio` = Σ completions 2016-2025 ÷ Σ permits 2015-2024 (one-year lag). The model caps it at 1.
3. `months_permit_to_completion_1unit` / `_2plus` = average months from authorization to start plus start to completion, averaged over 2021-2025, from the region's sheet of each timing workbook (1-unit and 2+-unit columns).
4. In `agents/src/state.rs`, each state uses its Census region's row (the U.S. row if missing).

## Known limitations

- **No state-level data.** Each state uses its region's ratio and lags.
- The completion ratio includes completions in places that don't issue permits, so it can overstate how many permits get built. The model caps it at 1.
- Average months hide a long tail: large multifamily projects can take far longer than the 2+ average.
- Small-unit types (ADUs, 2–4 unit buildings) have small samples.
- The characteristics data describe new construction overall, not reform-induced projects specifically.
