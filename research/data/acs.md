# American Community Survey (ACS)

| Field | Value |
| --- | --- |
| **Name** | American Community Survey 1-year and 5-year estimates |
| **Source URL** | https://www.census.gov/programs-surveys/acs · API: https://api.census.gov/data.html |
| **Publisher** | U.S. Census Bureau |
| **Access method** | Bulk table-based summary file (no key needed): https://www2.census.gov/programs-surveys/acs/summary_file/2024/table-based-SF/data/5YRData/acsdt5y2024-b25024.dat, fetched by [`scripts/fetch_state_baseline.py`](../../scripts/fetch_state_baseline.py). The Census Data API now requires a key (`CENSUS_API_KEY`), so it isn't used. |
| **Version / vintage used** | 2020-2024 5-year estimates, table B25024 (units in structure) |
| **Access date** | 2026-11-16 (checksum in `research/data/raw/CHECKSUMS.txt`) |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | Nation → state → county → place → tract → block group (5-year only). 1-year covers only geographies with 65,000+ people. |
| **Size** | Small if pulled by API for selected tables |
| **Used for (scenarios)** | Baseline (housing units, tenure, units-in-structure, vacancy, rents, values, household growth), ADU and missing middle (count of single-family detached owner-occupied homes) |

## Description

An annual household survey of about 3.5 million addresses. Key tables: B25001 (housing units), B25024 (units in structure), B25003 (tenure), B25004 (vacancy status), B25064 (median gross rent), B25077 (median value), B25034 (year structure built).

## Cleaning & transformation steps

1. Keep county rows (`GEO_ID` starting `0500000US`) that match a county in the permit files or population estimates.
2. Estimates only (margins of error dropped): E001 total; E002 1-unit detached; E003 1-unit attached; E004 + E005 as 2-4 units; E006-E009 as 5+ units. Mobile homes and boats/RVs (E010, E011) count in the total only.
3. Written to `hu_*` columns of `data/derived/county_baseline.csv`; state rows sum their counties.
4. In `agents/src/engine.rs`, each county's share (single-family detached, or 5+ units) ÷ the U.S. share, bounded to 0.25-2.5, weights that county's contribution to a scenario. Counties without an ACS row use their state's share.

## Known limitations

- Estimates carry **margins of error**, which are large for small geographies and 5-year tract data. Propagate them rather than ignoring them.
- 5-year estimates are period averages (e.g. 2020–2024), not a point in time, which lags policy effects.
- The 2020 1-year release was replaced with experimental estimates because of COVID-era nonresponse.
- It counts *housing units* by structure type and cannot identify ADUs or legal versus illegal units.
- Housing mix is used as a proxy for exposure to a reform (single-family share for ADUs and missing middle, 5+-unit share for transit and parking). It isn't zoning or parcel eligibility.
- Geography boundaries change between vintages (2020 tract redraw), so time series need crosswalks.
