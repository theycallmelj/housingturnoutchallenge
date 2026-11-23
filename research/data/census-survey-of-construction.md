# Census Survey of Construction (SOC): New Residential Construction

| Field | Value |
| --- | --- |
| **Name** | New Residential Construction: Starts Rates, Length of Time (permit to start, start to completion), annual permits and completions |
| **Source URL** | https://www.census.gov/construction/nrc/ · starts rates: https://www.census.gov/construction/nrc/starts_rates.html · length of time: https://www.census.gov/construction/nrc/data/time.html and https://www.census.gov/construction/nrc/data/time/historic.html |
| **Publisher** | U.S. Census Bureau (with HUD co-sponsorship) |
| **Access method** | Fetched by [`scripts/fetch_construction_timing.py`](../../scripts/fetch_construction_timing.py): `xls/startsrates_YYYYMM.xls` (2015-2023) and `.xlsx` (2024-2026), `pdf/pct_start_to_comp_YYYY.pdf` (2015-2025), `xls/avg_starttocomp_cust.xlsx`, and `xls/permits_cust.xlsx` and `comps_cust.xlsx` for the back-test. Old `.xls` and the PDFs are read with the Python standard library ([`scripts/census_xl.py`](../../scripts/census_xl.py), zlib). |
| **Version / vintage used** | Starts-rate files 2015-01 to 2026-09 (Census lists no file for 2025-09, 2025-11 or 2026-02; the next month's revision fills those months). Start-to-completion tables 2015-2025. |
| **Access date** | 2026-11-16 (checksums in `research/data/raw/CHECKSUMS-timing.txt`) |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | 4 Census regions (starts rates also by division). Permit cohorts 2015-2024. |
| **Size** | About 15 MB raw; derived table 4 KB |
| **Used for (scenarios)** | Every scenario and the baseline: turning permits into construction starts and completed homes, year by year, by building type (1 unit, 2-4, 5+). |

## Description

A sample survey that follows permitted units through start and completion. Two products are used:

- **Starts Rates.** Each monthly file gives, for each permit-authorization month, the share of its units that started construction in the file's month, by region, for 1-unit and 2+ unit buildings. Three revisions are published (preliminary, first, second/final).
- **Length of Time.** For each year, the percent distribution of buildings completed by months from start (five buckets) by region and building size (1 unit; 2-4, 5-9, 10-19, 20+ units), plus the average months.

## Cleaning & transformation steps

Produces [`data/derived/construction_curves.csv`](../../data/derived/construction_curves.csv) (one row per region x building type x variant) and [`data/derived/construction_curves_backtest.csv`](../../data/derived/construction_curves_backtest.csv).

1. **Permit -> start.** For each month, take the most revised sheet available (final, else first revision, else preliminary). The sheet's month comes from its date cell, not the file name.
2. Follow each authorization month A of 2015-2024 across the files: lags -2 to 11 months are individual rows. Lags 12-17 and 18-65 come from the two pooled rows ("from 6 months before" and "from 48 months before"), assuming equal rates within the pool, so each month in the pool gets the pooled ratio. Lags 18-65 not yet observed for recent cohorts get the mean of the observed pooled values. An authorization month missing any individual lag is dropped.
3. The **start rate** is the sum over lags. For 1-unit buildings it slightly exceeds 1 (1.00-1.05), because SOC starts and BPS permits are estimated separately. The model caps it at 1 by rescaling the lags; the uncapped value is kept in `start_rate_uncapped`.
4. **Start -> completion.** Parse each year's PDF table (first occurrence of each region, so the relative-standard-error table is skipped). "(Z)" (less than 0.5%) counts as 0.25%. Suppressed cells fall back to the U.S. value. Months within the 3-, 6-, 9- and 12-month buckets are spread evenly. The open "13 months or more" bucket gets a geometric tail starting at 13 months whose mean makes the overall mean equal Census's published average months for that region, year and size. The tail mean is bounded to 13.5-60 months and cut at 120 months.
5. Building types: **1 unit** uses the 1-unit start rates and completion column. **2-4** uses the 2+ start rates and the 2-4 column. **5+** uses the 2+ start rates and the **20+ unit** column, an assumption: most units in 5+ buildings are in large buildings, so this is the slower, conservative choice.
6. **Convolution.** Permit -> completion months = start lag + build months. Permits are spread evenly over the 12 issue months. Starts and completions are binned into year offsets 1-6 after issue, plus "after".
7. One curve per cohort year 2015-2024, using the build-time table for the following year, when most of the cohort finishes. Curves are averaged with equal weight (`variant = all`). `excl_covid` drops the 2020 and 2021 cohorts. It differs by at most 1.5 points in five-year completion share; the model uses `all`.
8. **Back-test.** Applying the curves to actual regional permits (1-unit and 2+ from `permits_cust.xlsx`, with the 5+ curve for 2+) reproduces published completions for 2019-2025 at a ratio of 0.94-1.01 on average by region (range 0.81-1.16 in single years).

Note: the 2023 PDF's header reads "2022", but its values differ from the 2022 file, so it is treated as 2023 data.

## Known limitations

- **No state-level data.** Each state uses its Census region's curves.
- 2+ unit start rates aren't split by size, so 2-4 and 5+ share one start curve. Only their build times differ.
- The completion tables count **buildings**, not units. The 5+ curve uses the 20+ column, which may overstate build times for 5-19 unit buildings.
- Starts that never complete (abandoned after starting) aren't modeled. The never-built share comes entirely from permits that never start: 21-29% for 2+ units, near 0% for 1 unit.
- Averages hide cycles: build times rose in 2021-2023.
