# NASA Land Use / Land Cover (MODIS Land Cover Type)

| Field | Value |
| --- | --- |
| **Name** | MODIS/Terra+Aqua Land Cover Type Yearly L3 Global 500 m (MCD12Q1 v061) |
| **Source URL** | https://lpdaac.usgs.gov/products/mcd12q1v061/ · NASA Earthdata: https://www.earthdata.nasa.gov/topics/land-surface/land-use-land-cover |
| **Publisher** | NASA (distributed by the LP DAAC at USGS EROS) |
| **Access method** | NASA Earthdata Search / AppEEARS (free Earthdata Login required) |
| **Version / vintage used** | TBD |
| **Access date** | Not yet accessed |
| **License / terms** | NASA Earth science data policy: no restrictions on use. Cite the DOI. |
| **Geography & time coverage** | Global, annual since 2001 |
| **Size** | Large globally. Subset by state bounding box. |
| **Used for (scenarios)** | Cross-check on NLCD and on long-run urban expansion. Probably secondary for this challenge. |

## Description

The handbook lists "NASA Land Use/Land Cover database" without naming a product. MCD12Q1 is NASA's standard annual land-cover product, so it is the stand-in here. Update this file if the organizers' curated list names a different product.

## Cleaning & transformation steps

None yet.

## Known limitations

- **500 m resolution** is much coarser than NLCD, so it is not useful at parcel or neighborhood scale.
- A single "urban and built-up" class that doesn't distinguish residential density.
- Requires an Earthdata login. It is free, but document it as an access step for reproducibility.
