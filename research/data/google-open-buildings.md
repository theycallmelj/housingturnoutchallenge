# Google Open Buildings

| Field | Value |
| --- | --- |
| **Name** | Google Open Buildings (v3 polygons; 2.5D temporal dataset) |
| **Source URL** | https://sites.research.google/gr/open-buildings/ |
| **Publisher** | Google Research |
| **Access method** | CSV/GeoJSON tiles via Google Cloud Storage, or Earth Engine |
| **Version / vintage used** | v3 (imagery processed May 2023) per the project site, checked 2026-10-28 |
| **Access date** | 2026-10-28 (site checked; data not downloaded) |
| **License / terms** | Dual-licensed **CC BY 4.0 or ODbL v1.0**, user's choice. Choose CC BY 4.0 to avoid share-alike. |
| **Geography & time coverage** | **Africa, South and Southeast Asia, Latin America, and the Caribbean. The US mainland is NOT covered.** The country list includes Puerto Rico and the US Virgin Islands only. |
| **Size** | Large |
| **Used for (scenarios)** | **Likely not usable** for a US state analysis. Only relevant if we model Puerto Rico or the USVI. |

## Description

Building footprint polygons with confidence scores, derived from high-resolution satellite imagery.

## Cleaning & transformation steps

Not applicable.

## Known limitations

- **No coverage of the 50 states.** Although the handbook lists it, use Microsoft Building Footprints (or OpenStreetMap) for US states instead.
- No building use or height in the polygon product.
