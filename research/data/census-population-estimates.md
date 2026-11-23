# Census Population Estimates: state and county totals (vintage 2025)

| Field | Value |
| --- | --- |
| **Name** | Annual Estimates of the Resident Population for the United States, Regions, States, DC and Puerto Rico (NST-EST2025-ALLDATA) |
| **Source URL** | https://www2.census.gov/programs-surveys/popest/datasets/2020-2025/state/totals/NST-EST2025-ALLDATA.csv · https://www2.census.gov/programs-surveys/popest/datasets/2020-2025/counties/totals/co-est2025-alldata.csv · https://www2.census.gov/programs-surveys/popest/datasets/2010-2020/counties/totals/co-est2020-alldata.csv |
| **Publisher** | U.S. Census Bureau, Population Estimates Program |
| **Access method** | Bulk CSV, fetched by [`scripts/fetch_state_baseline.py`](../../scripts/fetch_state_baseline.py) |
| **Version / vintage used** | Vintage 2025 (`POPESTIMATE2020`-`2025`, `NETMIG2021`-`2025`); vintage 2020 for 2010-2019 population and 2011-2020 net migration |
| **Access date** | 2026-11-16 (checksum in `research/data/raw/CHECKSUMS.txt`) |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | Nation, regions, states, counties, DC, Puerto Rico; July 1 estimates 2020-2025 |
| **Size** | About 2 MB (county file) |
| **Used for (scenarios)** | Converting per-capita rates from studies (e.g. ADU permits per 100,000 residents) to statewide counts (`state.scale_rate`); permits per 100,000 residents in the state profile |

## Description

The Census Bureau's annual estimates of resident population, built from the 2020 census base plus births, deaths and migration.

## Cleaning & transformation steps

1. Keep rows with `SUMLEV` = `040` (states and DC) whose FIPS code is in the 51 used for permits. This drops Puerto Rico, regions and the nation.
2. Keep `POPESTIMATE2025` as `population_2025` in `data/derived/state_baseline.csv`.
3. County file: keep `SUMLEV` = `050` rows in the 51 states, as `population_2025` in `data/derived/county_baseline.csv`. Counties absent from the permit files are added with `permit_issuing = 0`.
4. A test checks that each state's county populations sum to within 0.1% of the state estimate.
5. **County panel** ([`scripts/fetch_county_panel.py`](../../scripts/fetch_county_panel.py) -> `data/derived/county_panel.csv`): population 2010-2019 from vintage 2020 and 2020-2025 from vintage 2025; net migration (domestic + international, July-June) 2011-2020 from vintage 2020 and 2021-2025 from vintage 2025. `NETMIG2010` and vintage 2025's `NETMIG2020` cover only April-July, so they are dropped. The model uses net migration as a percent of the prior year's population.

## Known limitations

- These are estimates, and each new vintage revises earlier years. The panel joins two vintages (2010 and 2020 census bases) at 2020, so levels can shift by a few percent at the seam. Using rates and county effects limits the impact.
- A single year's population is applied to all five baseline years and to scaled rates, so population growth over the horizon is ignored.
- Applying another place's per-capita rate across a state treats rural and urban areas alike; the source rates often come from large cities. County weighting by housing mix partly offsets this. Residents of the 108 counties that issue no permits are left out of the ADU base, because their homes wouldn't show up as permits.
