# U.S. Treasury daily par yield curve rates

| Field | Value |
| --- | --- |
| **Name** | Daily Treasury Par Yield Curve Rates (10-year constant maturity) |
| **Source URL** | https://home.treasury.gov/resource-center/data-chart-center/interest-rates/TextView?type=daily_treasury_yield_curve |
| **Publisher** | U.S. Department of the Treasury |
| **Access method** | One CSV per year from `https://home.treasury.gov/resource-center/data-chart-center/interest-rates/daily-treasury-rates.csv/{YEAR}/all?type=daily_treasury_yield_curve&field_tdr_date_value={YEAR}&page&_format=csv`, fetched by [`scripts/fetch_county_panel.py`](../../scripts/fetch_county_panel.py) |
| **Version / vintage used** | 2010-2025 as published on the access date |
| **Access date** | 2026-11-16 (checksums in `research/data/raw/CHECKSUMS-panel.txt`) |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | National; business days 2010-2025 |
| **Size** | About 17 KB per year |
| **Used for (scenarios)** | Baseline: financing conditions in the county permit model, and the "what if rates change" input (`rate_shift_pp`). Every scenario except ADUs scales with the baseline, so it reaches them too. |

## Description

Treasury's daily par yields by maturity, from on-the-run Treasury securities. The 10-year yield is a widely used benchmark for mortgage and long-term borrowing costs.

## Cleaning & transformation steps

1. Read the `10 Yr` column of each year's CSV and drop blank or `N/A` days.
2. Annual average of the daily values, written to `data/derived/national_series.csv` (`year,treasury_10y_pct,days`).
3. In `agents/src/forecast.rs`, the model uses the year's average and its change from the prior year. Forecast years hold the 2025 average (4.29%), plus any user shift.

## Known limitations

- **One national series**, so the model learns its effect from only 13 fitted years (2013-2025). Its coefficients are imprecise and partly confounded with other national swings, such as the 2021-2022 multifamily boom that happened while rates rose. Treat rate scenarios as illustrative.
- It is a Treasury yield, not a mortgage or construction-loan rate. Spreads to those rates vary over time.
