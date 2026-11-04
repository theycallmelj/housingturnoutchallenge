---
name: housing-data-provenance
description: Which datasets the housing challenge submission may use, their licenses and limitations, how to build the five-year baseline, and pilot-state data options. Use before adding any dataset, choosing a data source, or answering questions about permits vs. starts vs. completions.
---

# Data and provenance expert

Start with `mcp__housing-ontology__find_datasets` (`usable_only: true` for what the submission may depend on) or `mcp__housing-ontology__evidence_brief` `{"domain": "baseline_data"}`.

## The license rule (Official Rules §7.b, §10.4-10.5; AGENTS.md §2)

Only `open_verified` datasets are usable: public domain, or an explicit open license with no share-alike, noncommercial, or access restrictions, **and** a provenance file in `research/data/`. Everything else in the ontology is kept so we know what each study relied on:

| Status | Examples | Meaning |
| --- | --- | --- |
| `open_verified` | census_bps, census_pep_housing_units, census_soc, acs, census_tiger, bls_construction, fhfa_hpi, ntd, nlcd, nasa_modis_landcover | Usable |
| `unverified` | ca_hcd_apr, wisconsin_v12_parcels, massgis, nyc_pluto, hmda, cal_itp_gtfs | Check the license and write a provenance file first |
| `proprietary` | costar, corelogic, urbanfootprint, rsmeans | Never |
| `restricted` / `rejected_*` | hud_usps_vacancy, microsoft_footprints, wrluri_data, national_zoning_atlas, bts_national_transit_map | Removed; don't reintroduce |

To promote a source: verify its license text, copy `research/data/_TEMPLATE.md`, add a committed fetch script, set `license_status` to `open_verified` in the ontology, and run `cargo test --manifest-path agents/Cargo.toml`.

## Baseline

- Production: **census_bps** (permits). Stock: **census_pep_housing_units** (built from permits, so not independent).
- Permits → completions: **census_soc** lags and attrition, which exist at region level only, so applying them to states is an assumption.
- Forecast the baseline under the same economic assumptions as the scenarios (`paciorek_2013`, `cmhc_integrated_housing_model_2026`).

## Open gaps

Transit stop locations (TOD), building footprints (ADU/missing middle), and parcel-level zoning rules. Candidate pilot states from the team memo: CA (HCD APR), MA (Boston replication, MassGIS), WI (statewide parcels). See `pilot_states` in the ontology.
