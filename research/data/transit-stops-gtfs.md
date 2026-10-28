# Transit stop locations: GTFS feeds and the BTS National Transit Map

| Field | Value |
| --- | --- |
| **Name** | NTAD National Transit Map (stops/routes), and agency GTFS feeds via the Mobility Database |
| **Source URL** | https://geodata.bts.gov (search "National Transit Map") · https://mobilitydatabase.org |
| **Publisher** | Bureau of Transportation Statistics (NTAD); MobilityData (catalog); individual transit agencies (feeds) |
| **Access method** | ArcGIS REST / file download (NTAD); GTFS zip per agency (Mobility Database catalog and API) |
| **Version / vintage used** | TBD. Record the feed date for each agency. |
| **Access date** | Not yet accessed |
| **License / terms** | NTAD: public domain. **GTFS feeds: license set by each agency** (many are open, some have terms of use). Record each one. |
| **Geography & time coverage** | Stop points nationwide (agency coverage varies). Current schedules only, unless archived feeds are used. |
| **Size** | Small to moderate per state |
| **Used for (scenarios)** | **TOD scenario**: stop locations and computed headways to define "major transit stops" and draw 0.25/0.5-mile buffers |

## Description

Standardized transit schedule data (stops, routes, trips, stop_times), from which service frequency at each stop can be computed.

## Cleaning & transformation steps

None yet.

## Known limitations

- GTFS reflects **current schedules**. Planned lines and service changes within the 5-year horizon are not included.
- "Major transit stop" definitions differ by state law (e.g. rail or BRT, or two bus lines at ≤15-minute peak headway). Our threshold must be documented and justified, as the handbook requires.
- Feed quality and coverage vary, and some small agencies publish no GTFS.
