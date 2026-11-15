# Census Population Estimates: state totals (vintage 2025)

| Field | Value |
| --- | --- |
| **Name** | Annual Estimates of the Resident Population for the United States, Regions, States, DC and Puerto Rico (NST-EST2025-ALLDATA) |
| **Source URL** | https://www2.census.gov/programs-surveys/popest/datasets/2020-2025/state/totals/NST-EST2025-ALLDATA.csv |
| **Publisher** | U.S. Census Bureau, Population Estimates Program |
| **Access method** | Bulk CSV, fetched by [`scripts/fetch_state_baseline.py`](../../scripts/fetch_state_baseline.py) |
| **Version / vintage used** | Vintage 2025, column `POPESTIMATE2025` |
| **Access date** | 2026-11-15 (checksum in `research/data/raw/CHECKSUMS.txt`) |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | Nation, regions, states, DC, Puerto Rico; July 1 estimates 2020-2025 |
| **Size** | Under 100 KB |
| **Used for (scenarios)** | Converting per-capita rates from studies (e.g. ADU permits per 100,000 residents) to statewide counts (`state.scale_rate`); permits per 100,000 residents in the state profile |

## Description

The Census Bureau's annual estimates of resident population, built from the 2020 census base plus births, deaths and migration.

## Cleaning & transformation steps

1. Keep rows with `SUMLEV` = `040` (states and DC) whose FIPS code is in the 51 used for permits. This drops Puerto Rico, regions and the nation.
2. Keep `POPESTIMATE2025` as `population_2025` in `data/derived/state_baseline.csv`.

## Known limitations

- These are estimates, and each new vintage revises earlier years.
- A single year's population is applied to all five baseline years and to scaled rates, so population growth over the horizon is ignored.
- Applying another place's per-capita rate to a whole state treats rural and urban areas alike; the source rates often come from large cities.
