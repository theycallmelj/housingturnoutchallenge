# Scenario model: method and assumptions

This document is the technical core of the Model & Assumptions Overview the challenge requires (Official Rules §7a). It describes what the model estimates, how, from which data, under which assumptions, and what it cannot say.

## What it estimates

For any U.S. state (all 50 plus DC), the model gives the **additional housing units authorized by building permit over five years** under each of the five benchmark reforms, compared with a **current-rules baseline**. It also reports **how many of those homes are completed within the five years**. Every result is a range (low, central, high) with a sensitivity breakdown.

The model works **county by county** (3,144 counties and equivalents, including 108 where no place issues permits, plus 14 retired or balance-of-state permit records) and sums to the state. The state baseline is the sum of its counties, and each reform's effect is computed per county and weighted by that county's housing mix.

```bash
cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA                 # all five, side by side
cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA --scenario adu  # one, fully explained
cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA --json          # machine-readable
```

**Why permits?** The rules let teams choose the outcome if they justify it. Census building permits are published every year for every state on one consistent definition, in the public domain. Starts and completions have no comparable state-level series. Permits come before completions, and some are never built. The model converts permits to completions with the Census Survey of Construction's regional completion ratios and permit-to-completion times (see [Completions](#completions)).

## Baseline

Source: Census Building Permits Survey annual **county** files, 2021-2025 (provenance: [`research/data/census-building-permits-survey.md`](research/data/census-building-permits-survey.md)). [`scripts/fetch_state_baseline.py`](scripts/fetch_state_baseline.py) builds [`data/derived/county_baseline.csv`](data/derived/county_baseline.csv) and sums it into [`data/derived/state_baseline.csv`](data/derived/state_baseline.csv). The script fails if any state's county sum differs from the Census state total by even one unit.

- **Central:** average annual units permitted, 2021-2025, × 5 years.
- **Range:** the slowest year × 5 to the fastest year × 5.

This assumes the recent pace continues with no policy change. It doesn't forecast interest rates or the economy. The range shows how much the pace has varied recently.

## Mechanics

Each scenario is computed per county and summed to the state. Code: [`agents/src/engine.rs`](agents/src/engine.rs). All numbers: [`agents/assumptions/scenarios.json`](agents/assumptions/scenarios.json).

**Step 1, county base × county weight.** For each county *c*:

- *base_c* = population/100,000 for ADUs (permit-issuing counties only), or the county's average annual permits for the other four.
- *weight_c* = the county's share of homes of the type the reform touches ÷ the U.S. share, bounded to 0.25-2.5 (ACS 2020-2024, table B25024):

| Scenario | County weight | Why |
| --- | --- | --- |
| 1. ADUs | single-family detached share ÷ U.S. (61.3%) | ADUs go on single-family lots |
| 2. Missing middle | single-family detached share ÷ U.S. | the reform opens single-family-zoned land |
| 3. Transit-oriented density | share of homes in 5+ unit buildings ÷ U.S. (19.1%) | proxy for dense, transit-served places until stop locations are added |
| 4. Parking reduction | share in 5+ unit buildings ÷ U.S. | minimums bind mostly on multifamily projects |
| 5. Fee reduction | 1 (none) | the benchmark cuts fees everywhere |

**Step 2, state factors.** The state's effective base, Σ *base_c* × *weight_c*, is multiplied by the scenario's factors:

| Scenario | Additional permits in year *y* |
| --- | --- |
| 1. ADUs | effective base × ADU rate per 100,000 × (1 − legalization share) × implementation share × ramp[*y*] |
| 2. Missing middle | effective base × missing-middle uplift × implementation share × ramp[*y*] |
| 3. Transit-oriented density | effective base × share of permits near major transit × uplift near transit × implementation share × ramp[*y*] |
| 4. Parking reduction | effective base × parking uplift × implementation share × ramp[*y*] |
| 5. Fee reduction ($50,000 → $25,000 per unit) | effective base × fee-cut uplift × ramp[*y*] |

The five-year effect is the sum over years 1-5. The output also reports the state's **net weight** (effective base ÷ unweighted base; 1.00 = like the U.S.) and the five counties contributing most.

Example: New York's TOD weight nets to 1.72 (New York City's multifamily stock) and Idaho's to 0.50. For missing middle the order flips.

- **Implementation share:** state laws aren't implemented uniformly by local governments.
- **Ramp:** effects build up over the first years rather than arriving at once.
- **Fees:** no implementation share is applied, because the benchmark cuts fees across all categories statewide.

## Completions

The partner review asked for units per year, not just permits. Source: Census Survey of Construction (provenance: [`research/data/census-survey-of-construction.md`](research/data/census-survey-of-construction.md)), built by [`scripts/fetch_construction_timing.py`](scripts/fetch_construction_timing.py) into [`data/derived/completion_factors.csv`](data/derived/completion_factors.csv).

For each permit added in year *y* (*y* = 1-5):

```text
completed by the end of year 5 = permits[y] × min(region completion ratio, 1)
    × clamp(6 − y − months/12, 0, 1)
```

- **Completion ratio:** units completed 2016-2025 ÷ units permitted 2015-2024 in the state's Census region (0.83 in the Northeast to 0.95 in the South). This allows for permits that are never built.
- **Months:** average permit-to-start plus start-to-completion, 2021-2025, by region: about 8-13 months for 1-unit buildings and 17-22 for 2+ units. Permits are assumed spread evenly through each year.
- **Building type:** ADUs use 1-unit timing, and missing middle and TOD use 2+ timing. Parking and fees use the state's 2025 permit mix.
- The baseline completions figure applies the completion ratio to baseline permits.

Homes permitted late in the five years finish after it, so completions are always below permits. A 2+ unit reform finishes a smaller share in time than an ADU reform.

## Assumptions

Each assumption has a `low_case`, `central`, and `high_case` value, a unit, a **basis** (study, challenge rule, or team assumption), the **source** study ids from the [ontology](ontology/housing-ontology.json), and a rationale. `low_case` is always the value giving the smaller effect. Tests in `engine.rs` and `state.rs` check that every assumption is ordered this way, that every source id exists, and that the code computes exactly the formulas above. They also check that county permits sum to the state and county populations to within 0.1% of the state estimate, that county weights stay in bounds, and that completions never exceed permits.

| Assumption | Low / central / high | Basis | Sources |
| --- | --- | --- | --- |
| ADU permits per 100,000 residents per year | 26 / 40 / 81 | Pew rates (low, high); central is a team choice nearer the non-California rate | `pew_2026_adu` |
| Share of ADU permits legalizing existing units | 30% / 15% / 0% | Team assumption | `marantz_elmendorf_kim_2023`, `crane_2020` |
| Missing-middle uplift on total permits | 2% / 6% / 15% | Assumption anchored to SB 9's weak uptake (low) and Lower Hutt's +12-17% net regional starts (high) | `garcia_alameldin_2023`, `rose_institute_2024`, `stacy_2023`, `maltman_greenaway_mcgrevy_2024` |
| Share of permits near major transit | 5% / 12% / 25% | **Placeholder (data gap)** until stop locations are added | none yet |
| Uplift near transit | 0% / 10% / 30% | Assumption anchored to Chicago's and LA's null permit effects (low) and large NYC upzonings (high) | `freemark_2019`, `kestelman_2025`, `su_curran_groome_freemark_2026`, `anagol_ferreira_rexer_2026` |
| Parking uplift on total permits | 0% / 2% / 5% | Assumption: minimums often don't bind (low); simulated stock effect (high) | `gabbe_pierce_clowers_2020`, `andersson_2016`, `tanrisever_2025`, `sohoni_lee_2026` |
| Fee-cut uplift on total permits | 0.5% / 2% / 5% | Assumption: land-price capitalization and infrastructure offsets (low); about 5% more projects viable when charges are halved (high) | `cmhc_2026_dev_charges`, `burge_ihlanfeldt_2006_sf`, `ihlanfeldt_shaughnessy_2004`, `skidmore_peddle_1998` |
| Implementation share (shared) | 50% / 75% / 100% | Team assumption | `lee_millard_ball_manville_2026`, `wielga_2023_cityscape` |
| County weight bounds (shared) | 0.25-2.5 | Team assumption about exposure, not a measured elasticity | none |
| Ramp, years 1-5 (shared) | slow / medium / fast build-up | Team assumption | `garcia_2017_adu`, `garcia_alameldin_2023`, `greenaway_mcgrevy_2025_wp017` |

Most values are **team assumptions anchored to studies**, not estimates from them. They are the first thing to revisit, and the JSON file is where to change them. No code change is needed.

## Ranges and sensitivity

- **Range:** low combines every low-case value and high every high-case value, so it is deliberately wide (it compounds the uncertainty).
- **Sensitivity:** each assumption is swung from its low to high case with all others at central. Results are sorted by how much they move the answer. This is the model's answer to "what is it most sensitive to?"

## What the model cannot tell you

- **It isn't a forecast.** It applies effects observed elsewhere, mostly in large cities, California, and New Zealand, to the state's own baseline.
- **County, not tract.** The Building Permits Survey is published for counties and permit-issuing places, not census tracts. Tract-level work would need state or local permit records (see [`research/data/state-local-open-data.md`](research/data/state-local-open-data.md)). The county weight is a proxy for exposure, not parcel-level eligibility.
- **Response rates aren't state-specific yet.** The county weights capture where each state's housing is exposed to a reform. How strongly each state responds still comes from studies of other places. Estimating state-specific responses (from zoning stringency, market strength, and past reforms) is the next step.
- **It doesn't model eligibility from parcels or zoning.** Parcel data with clean licenses are still an open gap. It can't tell whether a state already allows a reform, so states that already do will see less than estimated.
- **Transit-oriented density rests on a placeholder** for the share of permits near transit.
- **Scenarios are estimated separately and must not be added.** Parking and transit reforms overlap, and density limits interact (`kulka_sood_chiumenti_2026`).
- **Completion timing is regional.** The Survey of Construction has no state detail, so each state uses its Census region's ratio and lags. The model says nothing about prices, rents, or affordability.

## Agents and the model

The language-model panel (`agents/`) doesn't produce numbers. Each scenario expert calls `<scenario>.estimate` for the chosen state and leads with that result; the studies explain and question it. An automatic check flags any figure in an answer that no tool produced. See [`agents/README.md`](agents/README.md).
