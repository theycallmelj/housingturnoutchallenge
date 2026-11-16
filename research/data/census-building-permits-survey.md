# Census Building Permits Survey (BPS)

| Field | Value |
| --- | --- |
| **Name** | Building Permits Survey: new privately-owned residential units authorized |
| **Source URL** | https://www.census.gov/construction/bps/ · bulk files: https://www2.census.gov/econ/bps/ |
| **Publisher** | U.S. Census Bureau |
| **Access method** | Bulk text/CSV files by state, county, metro, and place, monthly and annual |
| **Version / vintage used** | Annual county files `co2021a.txt` to `co2025a.txt` (imputed totals); annual state files `st2021a.txt` to `st2025a.txt` for reconciliation |
| **Access date** | 2026-11-16 (checksums in `research/data/raw/CHECKSUMS.txt`, regenerated on each fetch) |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | National, state, metro, county, and permit-issuing place (about 20,000 jurisdictions). Annual place-level data go back decades; monthly data exist for a sample of places, with the rest imputed. |
| **Size** | A few hundred MB for the full history |
| **Used for (scenarios)** | **Core baseline outcome**: the five-year permit trend by state. It is also the outcome variable for any before/after evaluation of reforms. |

## Description

Counts of housing units authorized by building permits and their construction value, split into 1-unit, 2-unit, 3–4 unit, and 5+ unit structures. These are the most direct, consistent statewide measure of new housing production.

## Cleaning & transformation steps

Done by [`scripts/fetch_state_baseline.py`](../../scripts/fetch_state_baseline.py), producing [`data/derived/county_baseline.csv`](../../data/derived/county_baseline.csv) and [`data/derived/state_baseline.csv`](../../data/derived/state_baseline.csv):

1. Download `https://www2.census.gov/econ/bps/County/co{YEAR}a.txt` and `https://www2.census.gov/econ/bps/State/st{YEAR}a.txt` for 2021-2025 into `research/data/raw/` and record SHA-256 checksums.
2. Keep counties in the 50 states and DC (51 FIPS codes); drop territories.
3. Use the **imputed estimate** columns (reported plus imputed for non-reporting places), not the "rep" reported-only columns.
4. Total units authorized per county per year = 1-unit + 2-unit + 3-4-unit + 5+-unit **units** (not buildings or value). The 2025 split by building size is kept too.
5. **Reconcile:** for every state and year, the sum of its counties must equal the state file's total exactly, or the script fails. (On 2026-11-16 the largest difference was 0 units.)
6. Add the 108 counties that appear in the population estimates but issue no permits, with zero permits and `permit_issuing = 0`.
7. State rows = the sum of their counties.
8. In `agents/src/state.rs`: five-year baseline = 2021-2025 average annual units × 5; low/high = slowest and fastest year × 5.

## Known limitations

- **Permits are not completions.** Some authorized units are never started or are delayed. The model converts them with the Survey of Construction's regional ratios and lags (`census-survey-of-construction.md`).
- **No tract detail.** The finest published geography is the permit-issuing place, which doesn't nest in tracts. The model works at the county level.
- 14 county records in the permit files have no matching population or ACS row (retired geographies such as Connecticut's former counties, and balance-of-state records). Their permits count toward the state total; for weighting they use the state's housing mix.
- **ADUs are not reported consistently.** Jurisdictions may report them as 1-unit, as additions, or not at all, which undercounts the ADU scenario's outcome. State ADU reports (e.g. California HCD Annual Progress Reports) may be needed.
- Areas without permit requirements (parts of the rural South and West) are not covered.
- Monthly place-level figures for non-sampled places are imputed. Annual figures are more reliable.
- Reporting jurisdictions change over time, so check the universe before comparing years.
