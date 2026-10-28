# HUD Aggregated USPS Administrative Data on Address Vacancies

| Field | Value |
| --- | --- |
| **Name** | HUD Aggregated USPS Administrative Data on Address Vacancies |
| **Source URL** | https://www.huduser.gov/portal/datasets/usps.html · login: https://www.huduser.gov/apps/public/usps/home |
| **Publisher** | U.S. Department of Housing and Urban Development (from USPS) |
| **Access method** | **Registered login only.** Per HUD, under the agreement with USPS, data are available "only to governmental entities and non-profit organizations registered as users." |
| **Version / vintage used** | TBD (quarterly releases) |
| **Access date** | 2026-10-28 (access terms checked; data not accessed) |
| **License / terms** | **Restricted distribution.** Redistribution of the tract-level files is likely prohibited by the user agreement. Confirm before use. |
| **Geography & time coverage** | Census tract, quarterly, since about 2005 |
| **Size** | Moderate |
| **Used for (scenarios)** | Baseline vacancy and absorption context: are there already many vacant units in a market? |

## Description

Counts of residential and business addresses that USPS carriers flag as vacant (not collecting mail for 90+ days) or "no-stat", aggregated to census tracts.

## Cleaning & transformation steps

None yet.

## Known limitations

- **Access restriction is a likely eligibility problem.** The handbook requires that any data end users depend on be freely accessible, and it prohibits restricted-use data. A for-profit or individual user can't reproduce our results. **Recommendation:** don't make it a required input. Use ACS vacancy (B25004) instead, or confine USPS data to an optional, documented validation step and confirm with organizers.
- "Vacant" is defined by mail delivery, not occupancy or habitability. It doesn't record the reason for vacancy.
- Address counts aren't the same as housing-unit counts (multi-unit addressing varies by carrier route).
