# FHFA House Price Index (HPI)

| Field | Value |
| --- | --- |
| **Name** | FHFA House Price Index (purchase-only and all-transactions; expanded-data; annual county/ZIP/tract developmental indexes) |
| **Source URL** | https://www.fhfa.gov/data/hpi |
| **Publisher** | Federal Housing Finance Agency |
| **Access method** | CSV/XLSX download |
| **Version / vintage used** | TBD. The index is revised each release. |
| **Access date** | Not yet accessed |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | National, Census division, state, metro (quarterly/monthly), with annual county, ZIP5, and tract indexes. Since 1975 for some series. |
| **Size** | Small |
| **Used for (scenarios)** | Market strength by area, as a pro forma revenue input. Where prices are high relative to construction costs, reforms are more likely to produce units. Also used for scenario sensitivity. |

## Description

A weighted repeat-sales index of single-family home prices based on mortgages purchased or guaranteed by Fannie Mae and Freddie Mac.

## Cleaning & transformation steps

None yet.

## Known limitations

- It is an **index, not a price level**. Pair it with an ACS median value or other benchmark to get dollars.
- **Conforming mortgages only**: it excludes jumbo loans, many cash sales, and FHA/VA in the base index. That biases it toward the middle of the market.
- Repeat sales means **new construction is mostly excluded**, which matters when modeling new-unit prices.
- Single-family only. There is no rent or multifamily index, so rents need another source (e.g. ACS or HUD Fair Market Rents).
