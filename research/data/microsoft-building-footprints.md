# Microsoft US Building Footprints

| Field | Value |
| --- | --- |
| **Name** | Microsoft US Building Footprints (also part of Microsoft Global ML Building Footprints) |
| **Source URL** | https://github.com/microsoft/USBuildingFootprints · https://github.com/microsoft/GlobalMLBuildingFootprints |
| **Publisher** | Microsoft (Bing Maps) |
| **Access method** | Per-state GeoJSON zip downloads linked from the GitHub README |
| **Version / vintage used** | TBD. Record the release tag and imagery dates from the README. |
| **Access date** | Not yet accessed |
| **License / terms** | **ODbL v1.0 (Open Data Commons Open Database License)**, a share-alike license. Derived *databases* we publish must also be ODbL with attribution. Flag this in the SBOM/provenance and keep derived footprint tables separate from Apache-2.0 code. |
| **Geography & time coverage** | All 50 states, about 130 million footprints. A single snapshot; imagery vintage varies by area. |
| **Size** | **Large**: tens of GB uncompressed nationally, hundreds of MB to several GB per state. Do not commit. |
| **Used for (scenarios)** | ADU and missing middle: count of existing structures per parcel, building coverage, and remaining rear-yard space for physical feasibility |

## Description

Building polygons extracted by machine learning from aerial and satellite imagery.

## Cleaning & transformation steps

None yet.

## Known limitations

- **No building use, height, or unit count**, so single-family and other uses can't be told apart without parcel or assessor data.
- ML errors: merged attached buildings, missed small structures (sheds or existing ADUs may or may not appear), and some false positives.
- Imagery dates vary, so recent construction may be missing.
- The ODbL share-alike obligation affects how derived data can be licensed.
