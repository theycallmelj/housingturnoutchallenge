# Scenario model: method and assumptions

This document is the technical core of the Model & Assumptions Overview the challenge requires (Official Rules §7a). It describes what the model estimates, how, from which data, under which assumptions, and what it cannot say.

## What it estimates

For any U.S. state (all 50 plus DC), the model gives, for 2026-2030:

- a **current-rules baseline**: housing units permitted each year, forecast by a statistical model of every county's permits;
- the **additional units** each of the five benchmark reforms would add, with a low/central/high range and sensitivity;
- for both, **construction starts and completed homes by year**, from Census Survey of Construction timing curves.

The model works **county by county** (3,144 counties and equivalents, including 108 where no place issues permits, plus 14 retired or balance-of-state permit records) and sums to the state.

```bash
cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA                 # all five, side by side
cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA --scenario adu  # one, fully explained
cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA --json          # machine-readable
cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA --rate-shift 1 --migration 0.5   # "what if the economy is worse?"
cargo run --release --manifest-path agents/Cargo.toml --bin housing-fit                       # refit and back-test the baseline model
```

**Why permits?** The rules let teams choose the outcome if they justify it. Census building permits are published every year for every county on one consistent definition, in the public domain, so the baseline model can be fitted and tested on actual permits. Starts and completions have no state-level series, so the model derives them from permits with regional timing curves (see [Permits, starts and completions](#permits-starts-and-completions)).

## Baseline

**What it is.** A forecast of each county's permitted units for 2026-2030 under current rules, summed to the state. It is fitted separately for 1-unit buildings and for buildings with 2+ units, so the mix (and with it the construction timing) can shift. Code: [`agents/src/forecast.rs`](agents/src/forecast.rs). Fitted model: [`data/derived/baseline_model.json`](data/derived/baseline_model.json), written by `housing-fit`.

**Data** (all public domain; provenance in [`research/data/`](research/data/)): county permits by building type 2010-2025 (Census BPS); county population and net migration (Census PEP); county house-price change (FHFA HPI); the 10-year Treasury yield (U.S. Treasury). [`scripts/fetch_county_panel.py`](scripts/fetch_county_panel.py) builds [`data/derived/county_panel.csv`](data/derived/county_panel.csv).

**Model.** Permits are counts, so the model is a log-linear count regression (Poisson pseudo-maximum likelihood). For county *c* in year *t*:

```text
log E[permits c,t] = b1 log(1 + permits c,t-1) + b2 log(1 + avg permits c,t-2..t-3)
                   + b3 log population c,t-1 + b4 net migration % c,t-1
                   + b5 house-price change c,t-1 + b6 10-yr Treasury t + b7 change in Treasury t
                   + region intercept + state effect + county effect
```

- **Partial pooling.** State and county effects are shrunk toward their region's and state's level. The amount of shrinkage is estimated from the data, so a county with a short or noisy history borrows from similar places.
- **Poisson, not negative binomial, for the mean.** Counts are far more spread out than Poisson (the negative binomial dispersion is reported in the model file). But negative binomial weights discount large counties: in the back-test they overstated national five-year totals by 14-68%. Poisson pseudo-likelihood keeps fitted totals equal to observed totals, which a statewide forecast needs.
- **Forecast.** Starting from 2025, each year's predicted permits feed into the next year's lags. By default, conditions stay at their recent level: the 10-year Treasury at its 2025 average (4.29%), and each county's net migration and price growth at their 2023-2025 averages. A user can shift them (`--rate-shift`, `--migration`, `--price-shift`; sliders in the web UI; optional tool arguments for the agents). Every scenario except ADUs scales with the baseline path, so the shifts move the reforms' effects too.

Fitted coefficients (2013-2025, about 39,000 county-years each):

| Term | 1-unit | 2+ units | Reading |
| --- | --- | --- | --- |
| log permits, last year | 0.54 | 0.17 | single-family permits are much more persistent |
| log permits, 2-3 years ago | 0.14 | 0.06 | |
| log population | -0.02 | 1.12 | multifamily scales with county size; single-family doesn't once history is known |
| net migration (% of population) | 0.041 | 0.002 | +1 point of in-migration ≈ +4% single-family permits |
| house-price change (per 10 points) | -0.03 | 0.14 | price growth predicts multifamily, not single-family |
| 10-year Treasury (per point) | -0.047 | -0.079 | +1 point ≈ -5% (1-unit) and -8% (2+) per year |
| change in Treasury (per point) | 0.009 | 0.088 | offsets the level effect in the first year for 2+ units |

The national rate terms are imprecise: they rest on 13 years of one series and partly pick up other national swings (see [`research/data/treasury-yield-curve.md`](research/data/treasury-yield-curve.md)).

**Back-test** (`housing-fit` refits as of 2015, 2016, ..., 2020 and forecasts the next five years with held-flat conditions, as in production; 305 state × origin pairs):

| Five-year state totals | County model | Repeat the previous five years |
| --- | --- | --- |
| Median error | 12.5% | 20.0% |
| Mean error | 15.5% | 21.6% |
| Within 10% of actual | 39% | 22% |
| Within 20% of actual | 76% | 50% |
| Median actual ÷ forecast | 1.11 | 1.24 |

Nationally, both methods ran low during the 2016-2024 recovery (the model by 13-19%, the old rule by 19-30%). From 2020 the model was within 2.2% for 2021-2025.

**Range.** The 10th to 90th percentile of actual ÷ forecast in the back-test: ×0.91 to ×1.32 the central forecast. It leans upward because the model has tended to run low. The central value isn't bias-corrected, because the past six windows were mostly a recovery and permits have fallen since 2022.

**Ten-year back-test** ([`agents/src/backtest.rs`](agents/src/backtest.rs), results in [`data/derived/backtest.json`](data/derived/backtest.json), shown in the web UI under "How we tested the model"). For every start year 2015-2024, `housing-fit` refits the model on data through that year only and forecasts 1-5 years ahead, stopping at 2025. Conditions are held at start-year values, as in production. Every year 2016-2025 is forecast before the model sees it: 2,025 state forecasts in all. Median error of state totals (lower is better):

| Forecast | State forecasts | County model | Repeat last year | Repeat 5-year average |
| --- | --- | --- | --- | --- |
| 1 year ahead | 505 | 9.8% | 8.7% | 13.7% |
| 2 years ahead | 455 | 14.0% | 12.1% | 16.2% |
| 3 years ahead | 405 | 16.5% | 14.0% | 17.9% |
| 4 years ahead | 355 | 17.4% | 15.9% | 20.5% |
| 5 years ahead | 305 | 18.6% | 19.3% | 24.3% |
| **Five-year total** | 305 | **12.5%** | 12.9% | 20.0% |

- Over five years the model is about as accurate as repeating last year and much better than the five-year average.
- One to four years ahead, repeating last year is slightly better, because permits change little from one year to the next.
- Nationally, the one-year-ahead forecast came in low in 9 of 10 years.
- By building type, one year ahead: 1-unit 8.6% vs. 8.0% for repeating last year; 2+ units 19.3% vs. 19.1%.
- Connecticut is tested for start years 2015-2019 only: Census replaced its counties with planning regions in 2022, so no county has a consistent history after that.
- The test uses today's revised data, not the vintages a forecaster would have seen at the time.
- The model's value over a simple rule is that it responds to rates, migration and prices, works county by county, and gives the scenarios a base that moves with conditions. It is not more accurate at short range.

For comparison, every output also shows the previous rule (the 2021-2025 pace repeated). Example: Pennsylvania's model baseline is 118,150 permits, against 151,880 for the 2021-2025 pace. That pace includes Philadelphia's 2021 spike of 21,900 permits in 5+ unit buildings ahead of a tax-abatement change, which the model doesn't project forward.

## Mechanics

Each scenario is computed per county and summed to the state. Code: [`agents/src/engine.rs`](agents/src/engine.rs). All numbers: [`agents/assumptions/scenarios.json`](agents/assumptions/scenarios.json).

**Step 1, county base × county weight.** For each county *c*:

- *base_c[y]* = population/100,000 for ADUs (permit-issuing counties only), or the county's **baseline forecast for year y** for the other four.
- *weight_c* = the county's share of homes of the type the reform touches ÷ the U.S. share, bounded to 0.25-2.5 (ACS 2020-2024, table B25024):

| Scenario | County weight | Why |
| --- | --- | --- |
| 1. ADUs | single-family detached share ÷ U.S. (61.3%) | ADUs go on single-family lots |
| 2. Missing middle | single-family detached share ÷ U.S. | the reform opens single-family-zoned land |
| 3. Transit-oriented density | share of homes in 5+ unit buildings ÷ U.S. (19.1%) | proxy for dense, transit-served places until stop locations are added |
| 4. Parking reduction | share in 5+ unit buildings ÷ U.S. | minimums bind mostly on multifamily projects |
| 5. Fee reduction | 1 (none) | the benchmark cuts fees everywhere |

**Step 2, state factors.** The state's effective base for each year, Σ *base_c[y]* × *weight_c*, is multiplied by the scenario's factors:

| Scenario | Additional permits in year *y* |
| --- | --- |
| 1. ADUs | effective base[*y*] × ADU rate per 100,000 × (1 − legalization share) × implementation share × ramp[*y*] |
| 2. Missing middle | effective base[*y*] × missing-middle uplift × implementation share × ramp[*y*] |
| 3. Transit-oriented density | effective base[*y*] × share of permits near major transit × uplift near transit × implementation share × ramp[*y*] |
| 4. Parking reduction | effective base[*y*] × parking uplift × implementation share × ramp[*y*] |
| 5. Fee reduction ($50,000 → $25,000 per unit) | effective base[*y*] × fee-cut uplift × ramp[*y*] |

The five-year effect is the sum over years 1-5. The output also reports the state's **net weight** (effective base ÷ unweighted base; 1.00 = like the U.S.) and the five counties contributing most.

Example: New York's TOD weight nets to 1.72 (New York City's multifamily stock) and Idaho's to 0.50. For missing middle the order flips.

- **Implementation share:** state laws aren't implemented uniformly by local governments.
- **Ramp:** effects build up over the first years rather than arriving at once.
- **Fees:** no implementation share is applied, because the benchmark cuts fees across all categories statewide.

## Permits, starts and completions

A permit is an approval, not a home. Every result is reported as **permits, construction starts and completed homes by year**, using curves built from the Census Survey of Construction by [`scripts/fetch_construction_timing.py`](scripts/fetch_construction_timing.py) into [`data/derived/construction_curves.csv`](data/derived/construction_curves.csv) (provenance and step-by-step method: [`research/data/census-survey-of-construction.md`](research/data/census-survey-of-construction.md)).

1. **Permit → start (does it get built, and when it starts).** Census's monthly Starts Rates files give the share of each month's permits that start in each later month, by region, for 1-unit and 2+ unit buildings. Following each permit month of 2015-2024 across the files gives its start-lag distribution. The sum is the share that ever starts.
2. **Start → completion.** Census's yearly tables give the distribution of months from start to completion by region and building size. The open "13 months or more" bucket gets a tail whose mean matches Census's published average.
3. **Combine** the two (a convolution) and place each start and completion in year 1, 2, ... after the permit year. One curve per permit cohort 2015-2024, averaged. A variant without the 2020-2021 COVID cohorts differs by at most 1.5 points and is kept for testing.
4. **Apply** to each year's permits by building type. The baseline also counts homes already in the pipeline from 2021-2025 permits. Reform permits start from zero in 2026.

Share of a year's permits completed within five years, and when:

| Region | Type | Ever start | Year 1 | Year 2 | Year 3 | Years 4-5 | Within 5 years |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Northeast | 1 unit | 99% | 24% | 57% | 14% | 4% | 99% |
| Northeast | 5+ units | 75% | 1% | 26% | 29% | 16% | 72% |
| Midwest | 1 unit | 100% | 35% | 58% | 6% | 1% | 100% |
| Midwest | 5+ units | 71% | 3% | 40% | 23% | 4% | 71% |
| South | 1 unit | 99% | 41% | 53% | 4% | 1% | 99% |
| South | 5+ units | 79% | 2% | 37% | 31% | 8% | 78% |
| West | 1 unit | 100% | 35% | 56% | 7% | 2% | 100% |
| West | 5+ units | 77% | 1% | 32% | 31% | 11% | 75% |

(2-4 unit rows are in the CSV; they start like 5+ and finish faster.) About a quarter of multifamily permits never start, close to Census's own 2025 figure of multifamily starts running 20% below permits.

**Check against actual completions.** Applying the curves to actual regional permits reproduces Census's published completions for 2019-2025 at 0.94-1.01 of the actual total, on average by region.

ADUs use 1-unit curves, missing middle the 2-4 unit curves, TOD the 5+ curves, and parking and fees the state's forecast mix. A reform's added permits in 2029-2030 mostly finish after 2030, so five-year completions are below permits, by much more for multifamily reforms.

## Assumptions

Each assumption has a `low_case`, `central`, and `high_case` value, a unit, a **basis** (study, challenge rule, or team assumption), the **source** study ids from the [ontology](ontology/housing-ontology.json), and a rationale. `low_case` is always the value giving the smaller effect. Tests in `engine.rs`, `state.rs`, `forecast.rs` and `timing.rs` check that every assumption is ordered this way, that every source id exists, and that the code computes exactly the formulas above. They also check that:
- county permits sum to the state, and county populations to within 0.1% of the state estimate;
- county weights stay in bounds;
- the fitter recovers known coefficients from synthetic data;
- the committed model matches a fresh fit (`housing-fit --check` in CI);
- the timing curves account for every permit;
- completions never exceed starts, and starts never exceed permits.

`scripts/tests/` covers the pipeline's parsing and curve arithmetic.

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

- **The reform effects aren't forecasts.** The baseline is a tested statistical forecast. The reforms' effects apply results observed elsewhere, mostly in large cities, California, and New Zealand, to that baseline.
- **The baseline doesn't know about policy.** It projects each county's own recent behavior plus economic conditions. Zoning, fees and local approval capacity aren't inputs yet. They are the main thing that would let the model say *why* one state builds more than another, and the next step for estimating reform responses state by state.
- **Economic inputs are coarse.** There are no construction-cost or employment series yet (BLS files couldn't be fetched without supplying contact details), and the interest-rate effect rests on one national series.
- **County, not tract.** The Building Permits Survey is published for counties and permit-issuing places, not census tracts. Tract-level work would need state or local permit records (see [`research/data/state-local-open-data.md`](research/data/state-local-open-data.md)). The county weight is a proxy for exposure, not parcel-level eligibility.
- **Response rates aren't state-specific yet.** The county weights capture where each state's housing is exposed to a reform. How strongly each state responds still comes from studies of other places. Estimating state-specific responses (from zoning stringency, market strength, and past reforms) is the next step.
- **It doesn't model eligibility from parcels or zoning.** Parcel data with clean licenses are still an open gap. It can't tell whether a state already allows a reform, so states that already do will see less than estimated.
- **Transit-oriented density rests on a placeholder** for the share of permits near transit.
- **Scenarios are estimated separately and must not be added.** Parking and transit reforms overlap, and density limits interact (`kulka_sood_chiumenti_2026`).
- **Timing is regional.** The Survey of Construction has no state detail, so each state uses its Census region's curves. The model says nothing about prices, rents, or affordability.

## Agents and the model

The language-model panel (`agents/`) doesn't produce numbers. Each scenario expert calls `<scenario>.estimate` for the chosen state and leads with that result; the studies explain and question it. An automatic check flags any figure in an answer that no tool produced. See [`agents/README.md`](agents/README.md).
