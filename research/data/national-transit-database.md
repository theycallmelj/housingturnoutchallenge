# National Transit Database (NTD)

| Field | Value |
| --- | --- |
| **Name** | National Transit Database: annual and monthly agency data |
| **Source URL** | https://www.transit.dot.gov/ntd · data: https://data.transportation.gov (search "NTD") |
| **Publisher** | Federal Transit Administration (US DOT) |
| **Access method** | CSV/XLSX downloads, or the Socrata API on data.transportation.gov |
| **Version / vintage used** | TBD |
| **Access date** | Not yet accessed |
| **License / terms** | U.S. government work, public domain |
| **Geography & time coverage** | Transit agency and mode, with urbanized-area attribution. Annual since 1990s, plus monthly ridership. |
| **Size** | Small |
| **Used for (scenarios)** | TOD: defining a "major transit stop" by mode and service level (e.g. rail, BRT, frequency), and justifying the proximity threshold |

## Description

Agency-reported ridership, service supplied (vehicle revenue miles/hours), fleet, and finances for federally funded transit providers.

## Cleaning & transformation steps

None yet.

## Known limitations

- **No stop or station locations.** A stop-location source with a clear open license is still needed for TOD buffers. See "Removed sources and open gaps" in [README.md](README.md).
- Data are at agency/mode level, not route or stop frequency.
- Small rural or non-federally-funded operators may be missing or reported in reduced form.
