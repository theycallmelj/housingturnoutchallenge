# Bureau of Labor Statistics: Construction Employment and Costs

| Field | Value |
| --- | --- |
| **Name** | QCEW (Quarterly Census of Employment & Wages), CES (Current Employment Statistics), and PPI (Producer Price Index) for construction inputs |
| **Source URL** | https://www.bls.gov/cew/ · https://www.bls.gov/ces/ · https://www.bls.gov/ppi/ · API: https://www.bls.gov/developers/ |
| **Publisher** | U.S. Bureau of Labor Statistics |
| **Access method** | BLS Public Data API v2 (free registration key for higher limits; set it as `BLS_API_KEY`) or flat-file downloads |
| **Version / vintage used** | TBD. Pin the series IDs, e.g. QCEW NAICS 2361 (residential building construction), 238 (specialty trade contractors); PPI inputs to residential construction. |
| **Access date** | Not yet accessed |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | QCEW: county/state/national, quarterly. CES: state/metro, monthly. PPI: national only. |
| **Size** | Small |
| **Used for (scenarios)** | Construction **capacity constraint** (can the labor force absorb extra units in 5 years?), construction cost escalation for pro forma inputs, and sensitivity bounds |

## Description

Employment and wages for the construction industry, and price indexes for construction materials and inputs.

## Cleaning & transformation steps

None yet.

## Known limitations

- PPI is **national only**, so regional cost differences need another source. RSMeans city cost indexes are paywalled, so we can't depend on them.
- QCEW suppresses county/industry cells to protect confidentiality, and it is released about 5–6 months after the reference quarter.
- Employment doesn't map one-to-one to housing output (productivity, nonresidential work).
