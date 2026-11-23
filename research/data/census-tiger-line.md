# Census TIGER/Line Shapefiles

| Field | Value |
| --- | --- |
| **Name** | TIGER/Line Shapefiles and Cartographic Boundary Files |
| **Source URL** | https://www.census.gov/geographies/mapping-files/time-series/geo/tiger-line-file.html |
| **Publisher** | U.S. Census Bureau |
| **Access method** | Bulk download (https://www2.census.gov/geo/tiger/) |
| **Version / vintage used** | TBD. Match it to the ACS vintage. |
| **Access date** | Not yet accessed |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | All Census geographies (state, county, place, tract, block group, block), annual |
| **Size** | Large nationally (GBs). Download per state. |
| **Used for (scenarios)** | Joining ACS/BPS to maps, transit buffers (TOD), and rolling sub-state results up to state |

## Description

Boundary geometries for Census statistical and legal geographies, plus roads and water features.

## Cleaning & transformation steps

None yet.

## Known limitations

- Place boundaries are not zoning jurisdictions. Unincorporated county areas and special districts may follow different rules.
- Boundaries change annually, so use the vintage that matches the data being joined.
