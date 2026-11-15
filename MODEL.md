# Scenario model: method and assumptions

This document is the technical core of the Model & Assumptions Overview the challenge requires (Official Rules §7a). It describes what the model estimates, how, from which data, under which assumptions, and what it cannot say.

## What it estimates

For any U.S. state (all 50 plus DC), the model gives the **additional housing units authorized by building permit over five years** under each of the five benchmark reforms, compared with a **current-rules baseline**. Every result is a range (low, central, high) with a sensitivity breakdown.

```bash
cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA                 # all five, side by side
cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA --scenario adu  # one, fully explained
cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA --json          # machine-readable
```

**Why permits?** The rules let teams choose the outcome if they justify it. Census building permits are published every year for every state on one consistent definition, in the public domain. Starts and completions have no comparable state-level series. Permits come before completions, and some are never built; the Census Survey of Construction reports that lag but only nationally and by region.

## Baseline

Source: Census Building Permits Survey, 2021-2025 (provenance: [`research/data/census-building-permits-survey.md`](research/data/census-building-permits-survey.md)).

- **Central:** average annual units permitted, 2021-2025, × 5 years.
- **Range:** the slowest year × 5 to the fastest year × 5.

This assumes the recent pace continues with no policy change. It doesn't forecast interest rates or the economy. The range shows how much the pace has varied recently.

## Mechanics

Every scenario is a product of factors applied to the state's own data. Code: [`agents/src/engine.rs`](agents/src/engine.rs). All numbers: [`agents/assumptions/scenarios.json`](agents/assumptions/scenarios.json).

| Scenario | Additional permits in year *y* |
| --- | --- |
| 1. ADUs | ADU rate per 100,000 residents × population/100,000 × (1 − legalization share) × implementation share × ramp[*y*] |
| 2. Missing middle | baseline annual permits × missing-middle uplift × implementation share × ramp[*y*] |
| 3. Transit-oriented density | baseline annual permits × share of permits near major transit × uplift near transit × implementation share × ramp[*y*] |
| 4. Parking reduction | baseline annual permits × parking uplift × implementation share × ramp[*y*] |
| 5. Fee reduction ($50,000 → $25,000 per unit) | baseline annual permits × fee-cut uplift × ramp[*y*] |

The five-year effect is the sum over years 1-5.

- **Implementation share:** state laws aren't implemented uniformly by local governments.
- **Ramp:** effects build up over the first years rather than arriving at once.
- **Fees:** no implementation share is applied, because the benchmark cuts fees across all categories statewide.

## Assumptions

Each assumption has a `low_case`, `central`, and `high_case` value, a unit, a **basis** (study, challenge rule, or team assumption), the **source** study ids from the [ontology](ontology/housing-ontology.json), and a rationale. `low_case` is always the value giving the smaller effect. Tests in `engine.rs` check that every assumption is ordered this way, that every source id exists, and that the code computes exactly the formulas above.

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
| Ramp, years 1-5 (shared) | slow / medium / fast build-up | Team assumption | `garcia_2017_adu`, `garcia_alameldin_2023`, `greenaway_mcgrevy_2025_wp017` |

Most values are **team assumptions anchored to studies**, not estimates from them. They are the first thing to revisit, and the JSON file is where to change them. No code change is needed.

## Ranges and sensitivity

- **Range:** low combines every low-case value and high every high-case value, so it is deliberately wide (it compounds the uncertainty).
- **Sensitivity:** each assumption is swung from its low to high case with all others at central. Results are sorted by how much they move the answer. This is the model's answer to "what is it most sensitive to?"

## What the model cannot tell you

- **It isn't a forecast.** It applies effects observed elsewhere, mostly in large cities, California, and New Zealand, to the state's own baseline.
- **It doesn't model eligibility from parcels or zoning.** Parcel data with clean licenses are still an open gap. It can't tell whether a state already allows a reform, so states that already do will see less than estimated.
- **Transit-oriented density rests on a placeholder** for the share of permits near transit.
- **Scenarios are estimated separately and must not be added.** Parking and transit reforms overlap, and density limits interact (`kulka_sood_chiumenti_2026`).
- **Permits are not completions.** It says nothing about prices, rents, affordability, or where in the state units are built.

## Agents and the model

The language-model panel (`agents/`) doesn't produce numbers. Each scenario expert calls `<scenario>.estimate` for the chosen state and leads with that result; the studies explain and question it. An automatic check flags any figure in an answer that no tool produced. See [`agents/README.md`](agents/README.md).
