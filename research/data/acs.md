# American Community Survey (ACS)

| Field | Value |
| --- | --- |
| **Name** | American Community Survey 1-year and 5-year estimates |
| **Source URL** | https://www.census.gov/programs-surveys/acs · API: https://api.census.gov/data.html |
| **Publisher** | U.S. Census Bureau |
| **Access method** | Census Data API (a free key is recommended above about 500 requests/day; set it as `CENSUS_API_KEY`) or bulk summary files |
| **Version / vintage used** | TBD. 1-year estimates are released each September and 5-year each December. Pin the vintage in the fetch script. |
| **Access date** | Not yet accessed |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | Nation → state → county → place → tract → block group (5-year only). 1-year covers only geographies with 65,000+ people. |
| **Size** | Small if pulled by API for selected tables |
| **Used for (scenarios)** | Baseline (housing units, tenure, units-in-structure, vacancy, rents, values, household growth), ADU and missing middle (count of single-family detached owner-occupied homes) |

## Description

An annual household survey of about 3.5 million addresses. Key tables: B25001 (housing units), B25024 (units in structure), B25003 (tenure), B25004 (vacancy status), B25064 (median gross rent), B25077 (median value), B25034 (year structure built).

## Cleaning & transformation steps

None yet.

## Known limitations

- Estimates carry **margins of error**, which are large for small geographies and 5-year tract data. Propagate them rather than ignoring them.
- 5-year estimates are period averages (e.g. 2020–2024), not a point in time, which lags policy effects.
- The 2020 1-year release was replaced with experimental estimates because of COVID-era nonresponse.
- It counts *housing units* by structure type and cannot identify ADUs or legal versus illegal units.
- Geography boundaries change between vintages (2020 tract redraw), so time series need crosswalks.
