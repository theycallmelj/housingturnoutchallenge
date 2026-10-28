# USGS National Land Cover Database (NLCD / Annual NLCD)

| Field | Value |
| --- | --- |
| **Name** | Annual NLCD land cover, impervious surface, and related products |
| **Source URL** | https://www.mrlc.gov/data · https://www.usgs.gov/centers/eros/science/annual-national-land-cover-database |
| **Publisher** | USGS / Multi-Resolution Land Characteristics (MRLC) Consortium |
| **Access method** | GeoTIFF bulk download from MRLC, or cloud-hosted copies |
| **Version / vintage used** | As of 2026-10-28 the latest is Annual NLCD Collection 1.2 (released 2026-06-10, covering 1985–2025) |
| **Access date** | 2026-10-28 (catalog page checked; data not yet downloaded) |
| **License / terms** | Public domain. USGS policy: USGS-produced data are U.S. government works with no copyright (https://www.usgs.gov/information-policies-and-instructions/copyrights-and-credits). |
| **Geography & time coverage** | Contiguous US (some products also cover AK/HI/PR), annual 1985–2025 |
| **Size** | **Large**: several GB per year nationally. Clip by state in the fetch script. Do not commit. |
| **Used for (scenarios)** | Identifying developed and residential land, and screening out undevelopable land (water, wetlands) when estimating eligible area |

## Description

30 m raster land-cover classes (e.g. developed open space/low/medium/high intensity, forest, wetlands) and percent impervious surface.

## Cleaning & transformation steps

None yet.

## Known limitations

- **30 m resolution is too coarse for parcels.** A cell can mix a house, yard, and street.
- "Developed intensity" is based on impervious surface, not zoning or the number of housing units.
- Classification error is higher at the urban/rural fringe and in change detection between years.
