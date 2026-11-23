# Census Population Estimates Program: Housing Unit Estimates

| Field | Value |
| --- | --- |
| **Name** | Annual Estimates of Housing Units (PEP) |
| **Source URL** | https://www.census.gov/programs-surveys/popest/data/tables.html |
| **Publisher** | U.S. Census Bureau |
| **Access method** | CSV download from PEP tables, or the Census API (`pep` endpoints) |
| **Version / vintage used** | TBD. Each "vintage" revises the whole post-census series. Pin it. |
| **Access date** | Not yet accessed |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | Nation, state, county (plus incorporated places), July 1 of each year since the last decennial census |
| **Size** | Small |
| **Used for (scenarios)** | Baseline **housing stock** level that permit-based flows are added to. The challenge allows outputs in production *or* aggregate stock. |

## Description

Annual housing-unit totals built from the decennial census base, plus permits (adjusted for unpermitted areas and completion lags), plus new mobile homes, minus estimated losses.

## Cleaning & transformation steps

None yet.

## Known limitations

- **Not independent of BPS.** The estimates are built from permits, so don't treat them as separate validation of permit data.
- Loss and demolition rates are modeled, not observed.
- Each vintage revises past years, so mixing vintages creates breaks.
- No breakdown by structure type.
