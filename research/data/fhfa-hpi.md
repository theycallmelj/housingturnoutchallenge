# FHFA House Price Index (HPI)

| Field | Value |
| --- | --- |
| **Name** | FHFA House Price Index (purchase-only and all-transactions; expanded-data; annual county/ZIP/tract developmental indexes) |
| **Source URL** | https://www.fhfa.gov/data/hpi |
| **Publisher** | Federal Housing Finance Agency |
| **Access method** | XLSX download of the annual county index: https://www.fhfa.gov/hpi/download/annual/hpi_at_county.xlsx, fetched by [`scripts/fetch_county_panel.py`](../../scripts/fetch_county_panel.py) |
| **Version / vintage used** | Annual county all-transactions index (developmental), "last updated March 31, 2026"; years 2010-2025 |
| **Access date** | 2026-11-16 (checksum in `research/data/raw/CHECKSUMS-panel.txt`) |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | National, Census division, state, metro (quarterly/monthly), with annual county, ZIP5, and tract indexes. Since 1975 for some series. |
| **Size** | Small |
| **Used for (scenarios)** | Market strength by area, as a pro forma revenue input. Where prices are high relative to construction costs, reforms are more likely to produce units. Also used for scenario sensitivity. |

## Description

A weighted repeat-sales index of single-family home prices based on mortgages purchased or guaranteed by Fannie Mae and Freddie Mac.

## Cleaning & transformation steps

1. Read the county sheet. FIPS codes are stored as text in some rows and as numbers in others, so they are normalized to 5 digits.
2. Keep column E, the annual change (%), for 2010-2025, joined to the county panel (`hpi_change_pct` in `data/derived/county_panel.csv`). 43,719 county-years match; 2,600-2,790 counties have a value each year.
3. In `agents/src/forecast.rs`, missing county-years are filled with the state-year mean of reported counties, else the national-year mean. The model uses the prior year's change (per 10 points). Forecasts hold each county's 2023-2025 average, plus any user shift (`price_shift_pp`).

## Known limitations

- It is an **index, not a price level**. Pair it with an ACS median value or other benchmark to get dollars.
- **Conforming mortgages only**: it excludes jumbo loans, many cash sales, and FHA/VA in the base index. That biases it toward the middle of the market.
- Repeat sales means **new construction is mostly excluded**, which matters when modeling new-unit prices.
- Single-family only. There is no rent or multifamily index, so rents need another source (e.g. ACS or HUD Fair Market Rents).
- The county index is **developmental**, and small counties often have no value. Imputed state means stand in for them.
- Connecticut's planning regions (2022+) have no FHFA series, so they use the state mean.
