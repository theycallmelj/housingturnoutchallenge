//! Ten-year back-test of the baseline permit model.
//!
//! For every start year ("origin") from 2015 to 2024, the model is refitted
//! using only data through that year, then asked to forecast each county's
//! permits 1 to 5 years ahead (stopping at 2025, the last year with actual
//! counts). County forecasts are summed to states and compared with what was
//! actually permitted, so every year from 2016 to 2025 is predicted at least
//! once without the model seeing it.
//!
//! The same forecasts are scored against two simple rules a policymaker might
//! use instead: repeat last year's permits, or repeat the average of the last
//! five years. Conditions are held at their start-year values, as they would
//! be in a real forecast (so, for example, the 2022 rate rise is not known to
//! forecasts made in 2021).
//!
//! `housing-fit` writes the result to `data/derived/backtest.json`; the web UI
//! shows it under "How we tested the model".

use crate::forecast::{
    fit, forecast_county, metrics, round, Conditions, Group, Metrics, Panel, HORIZON, LAST_YEAR,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const BACKTEST_JSON: &str = include_str!("../../data/derived/backtest.json");

/// Start years tested: data through `origin`, forecasts for origin+1 onward.
pub const ORIGINS: [u32; 10] = [2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024];
/// First year of the actual series stored for charts.
pub const SERIES_FROM: u32 = 2010;

/// Errors for one forecast distance, for the model and the two simple rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HorizonRow {
    pub years_ahead: usize,
    pub model: Metrics,
    pub last_year: Metrics,
    pub five_year_avg: Metrics,
}

/// Errors by building type (the two halves of the model).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeRow {
    pub building_type: String,
    pub years_ahead: usize,
    pub model: Metrics,
    pub last_year: Metrics,
}

/// One forecast year, nationally, one year ahead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YearRow {
    pub year: u32,
    pub actual: f64,
    pub model: f64,
    pub last_year: f64,
    pub five_year_avg: f64,
}

/// A forecast made at `origin` for origin+1, origin+2, ...
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Path {
    pub origin: u32,
    pub predicted: Vec<f64>,
}

/// Actual permits by year (from [`SERIES_FROM`]) and every forecast path,
/// for one state or the nation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Series {
    pub actual: Vec<f64>,
    pub paths: Vec<Path>,
    /// Median miss one year ahead, over the ten start years.
    pub median_abs_pct_error_1y: f64,
    /// Median miss of the five-year total (start years 2015-2020).
    pub median_abs_pct_error_5y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FiveYear {
    pub origins: Vec<u32>,
    pub model: Metrics,
    pub last_year: Metrics,
    pub five_year_avg: Metrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecadeBacktest {
    pub description: String,
    pub origins: Vec<u32>,
    pub series_from: u32,
    pub tested_years: [u32; 2],
    /// State-level forecasts scored (state x start year x years ahead).
    pub forecasts_scored: usize,
    pub by_horizon: Vec<HorizonRow>,
    pub by_type: Vec<TypeRow>,
    pub five_year_totals: FiveYear,
    pub national_by_year: Vec<YearRow>,
    pub national: Series,
    /// Keyed by state FIPS code.
    pub states: BTreeMap<String, Series>,
}

/// The committed back-test (written by `housing-fit`).
pub fn decade() -> &'static DecadeBacktest {
    static B: OnceLock<DecadeBacktest> = OnceLock::new();
    B.get_or_init(|| {
        serde_json::from_str(BACKTEST_JSON)
            .expect("data/derived/backtest.json is missing or stale: run `cargo run --release --bin housing-fit`")
    })
}

fn idx(t: u32) -> usize {
    (t - SERIES_FROM) as usize
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Per-state, per-building-type numbers gathered while forecasting.
#[derive(Default)]
struct StateAcc {
    /// actual[g][year index]
    actual: [Vec<f64>; 2],
    /// pred[origin][g][h-1]
    pred: BTreeMap<u32, [Vec<f64>; 2]>,
}

/// Run the back-test on `p`. Refits the model once per start year.
pub fn run(p: &Panel) -> DecadeBacktest {
    let n_years = idx(LAST_YEAR) + 1;
    let mut acc: BTreeMap<String, StateAcc> = BTreeMap::new();
    for c in &p.counties {
        let a = acc.entry(c.state.clone()).or_insert_with(|| StateAcc {
            actual: [vec![0.0; n_years], vec![0.0; n_years]],
            ..Default::default()
        });
        for (gi, g) in Group::ALL.into_iter().enumerate() {
            for t in SERIES_FROM..=LAST_YEAR {
                a.actual[gi][idx(t)] += c.y(g, t).unwrap_or(0.0);
            }
        }
    }
    for &o in &ORIGINS {
        let fits = (fit(p, Group::Single, o), fit(p, Group::Multi, o));
        let years = HORIZON.min((LAST_YEAR - o) as usize);
        for a in acc.values_mut() {
            a.pred.insert(o, [vec![0.0; years], vec![0.0; years]]);
        }
        for c in &p.counties {
            if let Some(f) =
                forecast_county(p, (&fits.0, &fits.1), c, o, years, &Conditions::default())
            {
                let pr = acc.get_mut(&c.state).unwrap().pred.get_mut(&o).unwrap();
                for (k, y) in f.iter().enumerate() {
                    pr[0][k] += y[0];
                    pr[1][k] += y[1] + y[2];
                }
            }
        }
    }
    summarize(acc)
}

/// (actual, predicted) pairs.
type Pairs = Vec<(f64, f64)>;

fn summarize(acc: BTreeMap<String, StateAcc>) -> DecadeBacktest {
    let tot = |a: &StateAcc, t: u32| a.actual[0][idx(t)] + a.actual[1][idx(t)];
    let avg5 = |v: &dyn Fn(u32) -> f64, o: u32| (o - 4..=o).map(v).sum::<f64>() / 5.0;
    // (actual, predicted) pairs by horizon for model / last year / 5-yr avg.
    let mut pairs: Vec<[Pairs; 3]> = (0..HORIZON).map(|_| Default::default()).collect();
    // [years ahead][building type][model, last year]
    let mut by_type_pairs: Vec<[[Pairs; 2]; 2]> =
        (0..HORIZON).map(|_| Default::default()).collect();
    let mut five_model = Vec::new();
    let mut five_avg = Vec::new();
    let mut five_last = Vec::new();
    let mut states = BTreeMap::new();
    let ok = |a: f64, p: f64| a > 0.0 && p > 0.0;
    for (st, a) in &acc {
        let mut paths = Vec::new();
        let mut miss1 = Vec::new();
        let mut miss5 = Vec::new();
        for (&o, pr) in &a.pred {
            let predicted: Vec<f64> = (0..pr[0].len()).map(|k| pr[0][k] + pr[1][k]).collect();
            for (k, &m) in predicted.iter().enumerate() {
                let t = o + 1 + k as u32;
                let actual = tot(a, t);
                let last = tot(a, o);
                let avg = avg5(&|t| tot(a, t), o);
                if ok(actual, m) && ok(actual, last) && ok(actual, avg) {
                    pairs[k][0].push((actual, m));
                    pairs[k][1].push((actual, last));
                    pairs[k][2].push((actual, avg));
                    if k == 0 {
                        miss1.push((m / actual - 1.0).abs());
                    }
                }
                for gi in 0..2 {
                    let (ag, mg, lg) = (a.actual[gi][idx(t)], pr[gi][k], a.actual[gi][idx(o)]);
                    if ok(ag, mg) && ok(ag, lg) {
                        by_type_pairs[k][gi][0].push((ag, mg));
                        by_type_pairs[k][gi][1].push((ag, lg));
                    }
                }
            }
            if predicted.len() == HORIZON {
                let actual: f64 = (o + 1..=o + HORIZON as u32).map(|t| tot(a, t)).sum();
                let m: f64 = predicted.iter().sum();
                let avg = 5.0 * avg5(&|t| tot(a, t), o);
                let last = 5.0 * tot(a, o);
                if ok(actual, m) && ok(actual, avg) && ok(actual, last) {
                    five_model.push((actual, m));
                    five_avg.push((actual, avg));
                    five_last.push((actual, last));
                    miss5.push((m / actual - 1.0).abs());
                }
            }
            // Connecticut's start years 2020-2024 have no forecast: Census
            // replaced its counties with planning regions, so no county has
            // a consistent history at those dates.
            if predicted.iter().any(|v| *v > 0.0) {
                paths.push(Path {
                    origin: o,
                    predicted: predicted.iter().map(|v| v.round()).collect(),
                });
            }
        }
        states.insert(
            st.clone(),
            Series {
                actual: (SERIES_FROM..=LAST_YEAR)
                    .map(|t| tot(a, t).round())
                    .collect(),
                paths,
                median_abs_pct_error_1y: round(median(miss1)),
                median_abs_pct_error_5y: round(median(miss5)),
            },
        );
    }
    // National series: sums of the state numbers.
    let n_years = idx(LAST_YEAR) + 1;
    let nat_actual: Vec<f64> = (0..n_years)
        .map(|i| acc.values().map(|a| a.actual[0][i] + a.actual[1][i]).sum())
        .collect();
    // National comparisons for a start year use only the states that have
    // a forecast for it (all but Connecticut for 2020-2024).
    let forecast_at =
        |a: &StateAcc, o: u32| a.pred[&o][0].iter().chain(&a.pred[&o][1]).any(|v| *v > 0.0);
    let nat_pred = |o: u32| -> Vec<f64> {
        let len = acc.values().next().map_or(0, |a| a.pred[&o][0].len());
        (0..len)
            .map(|k| {
                acc.values()
                    .filter(|a| forecast_at(a, o))
                    .map(|a| a.pred[&o][0][k] + a.pred[&o][1][k])
                    .sum()
            })
            .collect()
    };
    let nat_cmp = |o: u32, t: u32| -> f64 {
        acc.values()
            .filter(|a| forecast_at(a, o))
            .map(|a| tot(a, t))
            .sum()
    };
    let national_by_year = ORIGINS
        .iter()
        .map(|&o| YearRow {
            year: o + 1,
            actual: nat_cmp(o, o + 1).round(),
            model: nat_pred(o)[0].round(),
            last_year: nat_cmp(o, o).round(),
            five_year_avg: avg5(&|t| nat_cmp(o, t), o).round(),
        })
        .collect();
    let nat_miss = |k: usize, origins: &[u32]| {
        median(
            origins
                .iter()
                .filter_map(|&o| {
                    let pr = nat_pred(o);
                    if k == 0 {
                        Some((pr[0] / nat_cmp(o, o + 1) - 1.0).abs())
                    } else if pr.len() == HORIZON {
                        let a: f64 = (o + 1..=o + HORIZON as u32).map(|t| nat_cmp(o, t)).sum();
                        Some((pr.iter().sum::<f64>() / a - 1.0).abs())
                    } else {
                        None
                    }
                })
                .collect(),
        )
    };
    let national = Series {
        actual: nat_actual.iter().map(|v| v.round()).collect(),
        paths: ORIGINS
            .iter()
            .map(|&o| Path {
                origin: o,
                predicted: nat_pred(o).iter().map(|v| v.round()).collect(),
            })
            .collect(),
        median_abs_pct_error_1y: round(nat_miss(0, &ORIGINS)),
        median_abs_pct_error_5y: round(nat_miss(1, &ORIGINS)),
    };
    let forecasts_scored = pairs.iter().map(|p| p[0].len()).sum();
    DecadeBacktest {
        description: "Every start year 2015-2024 (Connecticut only 2015-2019: Census replaced its counties with planning regions): the county permit model refitted on data through that year only, forecasting 1-5 years ahead (to 2025), summed to states and compared with actual permits and with two simple rules (repeat last year; repeat the last five years' average). Conditions held at start-year values. Built by agents/src/backtest.rs.".into(),
        origins: ORIGINS.to_vec(),
        series_from: SERIES_FROM,
        tested_years: [ORIGINS[0] + 1, LAST_YEAR],
        forecasts_scored,
        by_horizon: pairs
            .iter()
            .enumerate()
            .map(|(k, p)| HorizonRow {
                years_ahead: k + 1,
                model: metrics(&p[0]),
                last_year: metrics(&p[1]),
                five_year_avg: metrics(&p[2]),
            })
            .collect(),
        by_type: by_type_pairs
            .iter()
            .enumerate()
            .flat_map(|(k, g)| {
                g.iter().enumerate().map(move |(gi, p)| TypeRow {
                    building_type: ["1-unit buildings", "2+ unit buildings"][gi].into(),
                    years_ahead: k + 1,
                    model: metrics(&p[0]),
                    last_year: metrics(&p[1]),
                })
            })
            .collect(),
        five_year_totals: FiveYear {
            origins: ORIGINS
                .iter()
                .copied()
                .filter(|o| o + HORIZON as u32 <= LAST_YEAR)
                .collect(),
            model: metrics(&five_model),
            last_year: metrics(&five_last),
            five_year_avg: metrics(&five_avg),
        },
        national_by_year,
        national,
        states,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_backtest_covers_the_last_ten_years() {
        let b = decade();
        assert_eq!(b.origins, ORIGINS.to_vec());
        assert_eq!(b.tested_years, [2016, 2025]);
        assert_eq!(b.national_by_year.len(), 10);
        assert_eq!(b.by_horizon.len(), HORIZON);
        assert_eq!(b.states.len(), 51);
        // Every start year forecasts up to 2025 and no further.
        for p in &b.national.paths {
            assert_eq!(
                p.predicted.len(),
                HORIZON.min((LAST_YEAR - p.origin) as usize)
            );
        }
        // One year ahead, every state is scored for every start year except
        // Connecticut 2020-2024 (counties replaced by planning regions).
        assert_eq!(b.by_horizon[0].model.n, 51 * 10 - 5);
        assert_eq!(b.states["09"].paths.len(), 5);
        // Further ahead, fewer start years can be checked against actuals.
        assert!(b.by_horizon[4].model.n < b.by_horizon[0].model.n);
        // The national actual series matches the panel.
        let total_2025: f64 = b.states.values().map(|s| *s.actual.last().unwrap()).sum();
        assert!((total_2025 - b.national.actual.last().unwrap()).abs() <= 51.0);
    }

    #[test]
    fn summary_scores_a_known_forecast() {
        // One state with flat permits of 100 a year; a forecast of 110 every
        // year misses by 10%, and both simple rules are exact.
        let n = idx(LAST_YEAR) + 1;
        let mut pred = BTreeMap::new();
        for &o in &ORIGINS {
            let len = HORIZON.min((LAST_YEAR - o) as usize);
            pred.insert(o, [vec![60.0; len], vec![50.0; len]]);
        }
        let acc = BTreeMap::from([(
            "01".to_string(),
            StateAcc {
                actual: [vec![60.0; n], vec![40.0; n]],
                pred,
            },
        )]);
        let b = summarize(acc);
        let h1 = &b.by_horizon[0];
        assert!((h1.model.median_abs_pct_error - 0.10).abs() < 1e-9);
        assert_eq!(h1.last_year.median_abs_pct_error, 0.0);
        assert_eq!(h1.five_year_avg.median_abs_pct_error, 0.0);
        // By type: 1-unit exact, 2+ units 25% high.
        let multi = b
            .by_type
            .iter()
            .find(|r| r.years_ahead == 1 && r.building_type.starts_with('2'))
            .unwrap();
        assert!((multi.model.median_abs_pct_error - 0.25).abs() < 1e-9);
        assert_eq!(
            b.five_year_totals.origins,
            vec![2015, 2016, 2017, 2018, 2019, 2020]
        );
        assert!((b.states["01"].median_abs_pct_error_5y - 0.10).abs() < 1e-9);
        assert_eq!(b.national_by_year[0].year, 2016);
        assert_eq!(b.national_by_year[0].model, 110.0);
    }
}
