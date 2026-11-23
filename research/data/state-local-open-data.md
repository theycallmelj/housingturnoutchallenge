# State, local, tribal, and territorial open data portals (zoning, parcels, assessment)

The handbook lists these sources generically. This is **not a dataset**. It sets the rules for adding one. When we choose a target state, create a **separate provenance file per portal** using [`_TEMPLATE.md`](_TEMPLATE.md).

## License rule (must pass before use)

A portal dataset may be used only if its license or terms page **explicitly** says one of the following:

- public domain, U.S. government work, or CC0, **or**
- an open license with no share-alike, noncommercial, or redistribution restrictions.

**Reject** datasets that are ODbL / CC BY-SA, noncommercial (CC BY-NC), "for internal use only", behind a login or paywall (e.g. Regrid/ReportAll premium tiers), or that have **no stated license**. Record the license URL and a quote of the license text in the provenance file. (Official Rules §7.b.ii, §7.b.iii, §10.4, §10.5)

## What to look for

- Parcel and assessor data: lot size, land use code, building square footage, and owner occupancy (homestead flag), to count eligible ADU and missing-middle parcels
- Zoning district layers and code text: baseline rules for ADUs, lot size, and parking
- Local development fee schedules (Scenario 5)

Starting points: state GIS clearinghouses, county assessor portals, ArcGIS Hub (https://hub.arcgis.com).

## Known limitations

- Schemas differ by county, so harmonization is a major task.
- Parcel data often contains **owner names and addresses (PII)**. The handbook prohibits personally identifiable data, so **drop owner fields at ingest** and never commit them.
- Zoning layers may be out of date or missing for small jurisdictions.
- The handbook discourages parcel-level precision at the expense of transparency. Aggregate before presenting results.
