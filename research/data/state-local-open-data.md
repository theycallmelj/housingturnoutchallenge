# State, local, tribal, and territorial open data portals (zoning, parcels, assessment)

| Field | Value |
| --- | --- |
| **Name** | State and local GIS / open-data portals: parcels, assessor records, zoning districts |
| **Source URL** | Varies by state. Starting points: state GIS clearinghouses (e.g. search "<state> statewide parcels"), county assessor portals, ArcGIS Hub (https://hub.arcgis.com) |
| **Publisher** | State GIS offices, counties, municipalities, tribal and territorial governments |
| **Access method** | Varies: ArcGIS REST, Socrata, shapefile/GeoPackage downloads |
| **Version / vintage used** | Record per source |
| **Access date** | Record per source |
| **License / terms** | **Varies widely.** Some are public domain; some have terms of use that restrict redistribution or commercial use. Record each one. **Avoid paywalled aggregators** (e.g. Regrid/ReportAll premium tiers) unless end users can access them free. |
| **Geography & time coverage** | Parcel-level where available. Some states have statewide parcel layers; most require county-by-county assembly. |
| **Size** | **Large** (statewide parcels can be several GB) |
| **Used for (scenarios)** | ADU and missing middle: lot size, land use code, building square footage, and owner-occupancy (from homestead exemptions) to count eligible parcels. Fees: local fee schedules. Parking: local parking minimums. |

## Description

The handbook lists these sources generically. When we choose a target state, create a **separate provenance file per portal** using [`_TEMPLATE.md`](_TEMPLATE.md).

## Cleaning & transformation steps

Not applicable at this level. Document per portal.

## Known limitations

- **Heterogeneous schemas**: land-use codes, field names, and update cycles differ by county. Harmonization is a major task.
- Parcel data may include **owner names and addresses (PII)**. The handbook prohibits personally identifiable data, so **drop owner fields at ingest** and never commit them.
- Zoning layers may be out of date or missing for small jurisdictions.
- The handbook discourages parcel-level precision at the expense of transparency. Aggregate before presenting results.
