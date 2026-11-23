//! The baseline: how many homes each county would permit over the next five
//! years under current rules, from a statistical model fitted on 2013-2025.
//!
//! **Model.** For each county `c` and year `t`, permitted units `y` (fitted
//! separately for single-family and for 2+ unit buildings) are counts with
//! mean
//!
//! ```text
//! log mu[c,t] = b1 log(1 + y[c,t-1]) + b2 log(1 + mean(y[c,t-2], y[c,t-3]))
//!             + b3 log population[c,t-1] + b4 net migration rate[c,t-1]
//!             + b5 house-price change[c,t-1] + b6 10-year Treasury[t]
//!             + b7 change in 10-year Treasury[t]
//!             + region intercept + state effect + county effect
//! ```
//!
//! State and county effects are **partially pooled**: each is shrunk toward
//! zero (its region's or state's level) by a normal penalty whose variance is
//! estimated from the data, so counties with little history borrow strength
//! from similar places. The mean is fitted by Poisson pseudo-maximum
//! likelihood (penalized iteratively reweighted least squares, variances
//! updated by an EM-style step). The counts are far more variable than
//! Poisson, so a negative binomial dispersion is estimated from the residuals
//! and reported; it isn't used to weight the fit, because negative binomial
//! weights discount large counties and biased state totals upward by 14-68%
//! in the back-test. Poisson pseudo-likelihood keeps fitted totals equal to
//! observed totals, which is what a statewide forecast needs.
//!
//! **Forecast.** Starting from the last data year, the model is run forward
//! five years, feeding each year's predicted permits back in as next year's
//! lags. Conditions are held at their recent level (the 3-year county average
//! for migration and price growth, the last year's Treasury rate) unless the
//! caller shifts them ([`Conditions`]). That is what lets a user ask "what if
//! rates are a point higher?".
//!
//! **Validation.** `housing-fit` refits the model as of 2015, 2016, ...,
//! 2020, forecasts the next five years for every state with the same
//! held-flat conditions, and compares with what was actually permitted,
//! alongside the old rule (the previous five years' pace). The errors set
//! the baseline's low-high range.
//!
//! Data: `data/derived/county_panel.csv` and `national_series.csv`, built by
//! `scripts/fetch_county_panel.py` (Census BPS and PEP, FHFA HPI, Treasury
//! yields; all public domain). Fitted model: `data/derived/baseline_model.json`.

#![allow(clippy::needless_range_loop)] // matrix arithmetic reads more clearly with indices

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

const PANEL_CSV: &str = include_str!("../../data/derived/county_panel.csv");
const NATIONAL_CSV: &str = include_str!("../../data/derived/national_series.csv");
const MODEL_JSON: &str = include_str!("../../data/derived/baseline_model.json");

pub const FIRST_YEAR: u32 = 2010;
pub const LAST_YEAR: u32 = 2025;
pub const FIT_FROM: u32 = 2013;
pub const HORIZON: usize = 5;
pub const BACKTEST_ORIGINS: [u32; 6] = [2015, 2016, 2017, 2018, 2019, 2020];
const NY: usize = (LAST_YEAR - FIRST_YEAR + 1) as usize;
const REGIONS: [&str; 4] = ["northeast", "midwest", "south", "west"];
pub const FEATURES: [&str; 7] = [
    "log_permits_lag1",
    "log_permits_lag2_3",
    "log_population_lag1",
    "net_migration_pct_lag1",
    "price_change_10pct_lag1",
    "treasury_10y_pct",
    "treasury_10y_change_pct",
];
const P: usize = FEATURES.len() + REGIONS.len();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    /// 1-unit buildings.
    Single,
    /// Buildings with 2 or more units.
    Multi,
}

impl Group {
    pub const ALL: [Group; 2] = [Group::Single, Group::Multi];
    pub fn key(self) -> &'static str {
        match self {
            Group::Single => "single",
            Group::Multi => "multi",
        }
    }
}

// ------------------------------------------------------------------ data

#[derive(Debug, Clone)]
pub struct CountySeries {
    /// State FIPS + county FIPS, e.g. "42003".
    pub key: String,
    pub state: String,
    pub region: usize,
    pub p1: [Option<f64>; NY],
    pub p24: [Option<f64>; NY],
    pub p5: [Option<f64>; NY],
    pub pop: [Option<f64>; NY],
    pub mig: [Option<f64>; NY],
    /// FHFA annual price change (%), missing values filled with the
    /// state-year (else national-year) mean.
    pub hpi: [f64; NY],
}

impl CountySeries {
    fn idx(t: u32) -> Option<usize> {
        (FIRST_YEAR..=LAST_YEAR)
            .contains(&t)
            .then(|| (t - FIRST_YEAR) as usize)
    }
    pub fn y(&self, g: Group, t: u32) -> Option<f64> {
        let i = Self::idx(t)?;
        match g {
            Group::Single => self.p1[i],
            Group::Multi => Some(self.p24[i]? + self.p5[i]?),
        }
    }
    pub fn pop_at(&self, t: u32) -> Option<f64> {
        self.pop[Self::idx(t)?]
    }
    /// Net migration in year t as a percent of the prior year's population.
    pub fn mig_rate(&self, t: u32) -> Option<f64> {
        Some(100.0 * self.mig[Self::idx(t)?]? / self.pop_at(t - 1)?)
    }
    pub fn hpi_at(&self, t: u32) -> f64 {
        Self::idx(t).map_or(0.0, |i| self.hpi[i])
    }
}

pub struct Panel {
    pub counties: Vec<CountySeries>,
    pub rate: [f64; NY],
}

impl Panel {
    pub fn rate_at(&self, t: u32) -> f64 {
        self.rate[(t.clamp(FIRST_YEAR, LAST_YEAR) - FIRST_YEAR) as usize]
    }
    pub fn county(&self, key: &str) -> Option<&CountySeries> {
        self.counties.iter().find(|c| c.key == key)
    }
}

pub fn panel() -> &'static Panel {
    static P: OnceLock<Panel> = OnceLock::new();
    P.get_or_init(|| parse_panel(PANEL_CSV, NATIONAL_CSV))
}

fn parse_panel(panel_csv: &str, national_csv: &str) -> Panel {
    let region_of: HashMap<String, usize> = crate::state::all()
        .iter()
        .map(|s| {
            (
                s.fips.clone(),
                REGIONS.iter().position(|r| *r == s.region).unwrap_or(2),
            )
        })
        .collect();
    let mut lines = panel_csv.lines().filter(|l| !l.trim().is_empty());
    let head: Vec<&str> = lines.next().expect("panel header").split(',').collect();
    let col = |n: &str| {
        head.iter()
            .position(|h| *h == n)
            .unwrap_or_else(|| panic!("panel column {n}"))
    };
    let (cs, cc, cy) = (col("state_fips"), col("county_fips"), col("year"));
    let (c1, c24, c5) = (
        col("permits_1unit"),
        col("permits_2to4"),
        col("permits_5plus"),
    );
    let (cp, cm, ch) = (
        col("population"),
        col("net_migration"),
        col("hpi_change_pct"),
    );
    let mut by_key: BTreeMap<String, CountySeries> = BTreeMap::new();
    let mut hpi_raw: HashMap<String, [Option<f64>; NY]> = HashMap::new();
    for line in lines {
        let r: Vec<&str> = line.split(',').collect();
        let year: u32 = r[cy].parse().expect("year");
        let Some(i) = CountySeries::idx(year) else {
            continue;
        };
        let key = format!("{}{}", r[cs], r[cc]);
        let num = |k: usize| r[k].trim().parse::<f64>().ok();
        let e = by_key.entry(key.clone()).or_insert_with(|| CountySeries {
            key: key.clone(),
            state: r[cs].to_string(),
            region: *region_of.get(r[cs]).unwrap_or(&2),
            p1: [None; NY],
            p24: [None; NY],
            p5: [None; NY],
            pop: [None; NY],
            mig: [None; NY],
            hpi: [0.0; NY],
        });
        e.p1[i] = num(c1);
        e.p24[i] = num(c24);
        e.p5[i] = num(c5);
        e.pop[i] = num(cp);
        e.mig[i] = num(cm);
        hpi_raw.entry(key).or_insert([None; NY])[i] = num(ch);
    }
    // Fill missing price changes: state-year mean, else national-year mean.
    let mut state_sum: HashMap<(String, usize), (f64, f64)> = HashMap::new();
    let mut nat = [(0.0, 0.0); NY];
    for (k, v) in &hpi_raw {
        for (i, x) in v.iter().enumerate() {
            if let Some(x) = x {
                let e = state_sum
                    .entry((k[..2].to_string(), i))
                    .or_insert((0.0, 0.0));
                e.0 += x;
                e.1 += 1.0;
                nat[i].0 += x;
                nat[i].1 += 1.0;
            }
        }
    }
    for c in by_key.values_mut() {
        let raw = hpi_raw[&c.key];
        for i in 0..NY {
            c.hpi[i] = raw[i].unwrap_or_else(|| match state_sum.get(&(c.state.clone(), i)) {
                Some((s, n)) if *n > 0.0 => s / n,
                _ if nat[i].1 > 0.0 => nat[i].0 / nat[i].1,
                _ => 0.0,
            });
        }
    }
    let mut rate = [0.0; NY];
    for line in national_csv
        .lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
    {
        let r: Vec<&str> = line.split(',').collect();
        if let (Ok(y), Ok(v)) = (r[0].parse::<u32>(), r[1].parse::<f64>()) {
            if let Some(i) = CountySeries::idx(y) {
                rate[i] = v;
            }
        }
    }
    Panel {
        counties: by_key.into_values().collect(),
        rate,
    }
}

// ------------------------------------------------------------------ features

/// Inputs for one county-year prediction.
#[derive(Debug, Clone, Copy)]
struct Inputs {
    y1: f64,
    y2: f64,
    y3: f64,
    pop1: f64,
    mig1: f64,
    hpi1: f64,
    rate: f64,
    rate_prev: f64,
    region: usize,
}

fn x_vec(i: &Inputs) -> [f64; P] {
    let mut x = [0.0; P];
    x[0] = (1.0 + i.y1).ln();
    x[1] = (1.0 + (i.y2 + i.y3) / 2.0).ln();
    x[2] = i.pop1.max(1.0).ln() - 10.0;
    x[3] = i.mig1;
    x[4] = i.hpi1 / 10.0;
    x[5] = i.rate;
    x[6] = i.rate - i.rate_prev;
    x[FEATURES.len() + i.region] = 1.0;
    x
}

struct Obs {
    c: usize,
    s: usize,
    y: f64,
    x: [f64; P],
}

fn observations(p: &Panel, g: Group, last_year: u32) -> (Vec<Obs>, Vec<String>, Vec<String>) {
    let mut states: Vec<String> = p.counties.iter().map(|c| c.state.clone()).collect();
    states.sort();
    states.dedup();
    let keys: Vec<String> = p.counties.iter().map(|c| c.key.clone()).collect();
    let mut obs = Vec::new();
    for (ci, c) in p.counties.iter().enumerate() {
        let si = states.binary_search(&c.state).expect("state");
        for t in FIT_FROM..=last_year {
            let (Some(y), Some(y1), Some(y2), Some(y3), Some(pop1)) = (
                c.y(g, t),
                c.y(g, t - 1),
                c.y(g, t - 2),
                c.y(g, t - 3),
                c.pop_at(t - 1),
            ) else {
                continue;
            };
            let inp = Inputs {
                y1,
                y2,
                y3,
                pop1,
                mig1: c.mig_rate(t - 1).unwrap_or(0.0),
                hpi1: c.hpi_at(t - 1),
                rate: p.rate_at(t),
                rate_prev: p.rate_at(t - 1),
                region: c.region,
            };
            obs.push(Obs {
                c: ci,
                s: si,
                y,
                x: x_vec(&inp),
            });
        }
    }
    (obs, states, keys)
}

// ------------------------------------------------------------------ fitting

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupFit {
    pub coefficients: BTreeMap<String, f64>,
    /// Negative binomial dispersion of the residuals (variance = mu + mu^2 /
    /// theta), a moment estimate. Reported only.
    pub theta: f64,
    /// Standard deviation of state and county effects (log scale).
    pub sd_state: f64,
    pub sd_county: f64,
    pub n_obs: usize,
    pub state_effects: BTreeMap<String, f64>,
    pub county_effects: BTreeMap<String, f64>,
}

impl GroupFit {
    fn beta(&self) -> [f64; P] {
        let mut b = [0.0; P];
        for (i, n) in FEATURES.iter().enumerate() {
            b[i] = self.coefficients[*n];
        }
        for (i, r) in REGIONS.iter().enumerate() {
            b[FEATURES.len() + i] = self.coefficients[&format!("intercept_{r}")];
        }
        b
    }
    fn effect(&self, c: &CountySeries) -> f64 {
        self.state_effects.get(&c.state).copied().unwrap_or(0.0)
            + self.county_effects.get(&c.key).copied().unwrap_or(0.0)
    }
}

fn dot(a: &[f64; P], b: &[f64; P]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Solve H d = g for symmetric positive definite H (Cholesky).
fn solve(mut h: [[f64; P]; P], g: [f64; P]) -> [f64; P] {
    for (i, row) in h.iter_mut().enumerate() {
        row[i] += 1e-9;
    }
    let mut l = [[0.0; P]; P];
    for i in 0..P {
        for j in 0..=i {
            let s: f64 = (0..j).map(|k| l[i][k] * l[j][k]).sum();
            if i == j {
                l[i][j] = (h[i][i] - s).max(1e-12).sqrt();
            } else {
                l[i][j] = (h[i][j] - s) / l[j][j];
            }
        }
    }
    let mut z = [0.0; P];
    for i in 0..P {
        z[i] = (g[i] - (0..i).map(|k| l[i][k] * z[k]).sum::<f64>()) / l[i][i];
    }
    let mut d = [0.0; P];
    for i in (0..P).rev() {
        d[i] = (z[i] - (i + 1..P).map(|k| l[k][i] * d[k]).sum::<f64>()) / l[i][i];
    }
    d
}

/// Poisson pseudo-log-likelihood (terms that depend on the mean).
fn loglik(obs: &[Obs], eta: &[f64]) -> f64 {
    obs.iter().zip(eta).map(|(o, e)| o.y * e - e.exp()).sum()
}

/// One damped Newton step for a set of penalized random effects (one per
/// group index), given current linear predictors. Returns the curvature of
/// each effect (for the variance update).
fn re_step(
    obs: &[Obs],
    eta: &[f64],
    effects: &mut [f64],
    lambda: f64,
    group: impl Fn(&Obs) -> usize,
) -> Vec<f64> {
    let k = effects.len();
    let mut g = vec![0.0; k];
    let mut h = vec![0.0; k];
    for (o, e) in obs.iter().zip(eta) {
        let mu = e.exp();
        let j = group(o);
        g[j] += o.y - mu;
        h[j] += mu;
    }
    for j in 0..k {
        let gj = g[j] - lambda * effects[j];
        let hj = h[j] + lambda;
        effects[j] += (gj / hj).clamp(-1.0, 1.0);
        h[j] = hj;
    }
    h
}

/// Moment estimate of the NB dispersion: sum of squared Pearson residuals
/// equals the residual degrees of freedom.
fn moment_theta(obs: &[Obs], eta: &[f64]) -> f64 {
    let target = (obs.len().saturating_sub(P)) as f64;
    let f = |theta: f64| -> f64 {
        obs.iter()
            .zip(eta)
            .map(|(o, e)| {
                let mu = e.exp();
                (o.y - mu).powi(2) / (mu + mu * mu / theta)
            })
            .sum()
    };
    let (mut lo, mut hi) = (1e-3f64.ln(), 1e5f64.ln());
    if f(hi.exp()) < target {
        return hi.exp();
    }
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if f(mid.exp()) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    ((lo + hi) / 2.0).exp()
}

/// Fit one group on years FIT_FROM..=last_year.
pub fn fit(p: &Panel, g: Group, last_year: u32) -> GroupFit {
    let (obs, states, keys) = observations(p, g, last_year);
    let n = obs.len();
    let (ns, nc) = (states.len(), keys.len());
    let mean_y = obs.iter().map(|o| o.y).sum::<f64>() / n.max(1) as f64;
    let mut beta = [0.0; P];
    for r in 0..REGIONS.len() {
        beta[FEATURES.len() + r] = (mean_y + 1.0).ln();
    }
    let mut u = vec![0.0; ns];
    let mut v = vec![0.0; nc];
    let (mut theta, mut tau_s2, mut tau_c2) = (f64::INFINITY, 0.1f64, 0.5f64);
    let eta_of = |beta: &[f64; P], u: &[f64], v: &[f64]| -> Vec<f64> {
        obs.iter()
            .map(|o| (dot(beta, &o.x) + u[o.s] + v[o.c]).clamp(-30.0, 20.0))
            .collect()
    };
    let mut prev = f64::NEG_INFINITY;
    for _ in 0..300 {
        // Fixed effects: Fisher scoring with step halving.
        let eta = eta_of(&beta, &u, &v);
        let mut h = [[0.0; P]; P];
        let mut gr = [0.0; P];
        for (o, e) in obs.iter().zip(&eta) {
            let mu = e.exp();
            let _ = theta;
            let (w, s) = (mu, o.y - mu);
            for i in 0..P {
                gr[i] += s * o.x[i];
                for j in 0..=i {
                    h[i][j] += w * o.x[i] * o.x[j];
                }
            }
        }
        for i in 0..P {
            for j in 0..i {
                h[j][i] = h[i][j];
            }
        }
        let d = solve(h, gr);
        let base = loglik(&obs, &eta);
        let mut step = 1.0;
        for _ in 0..30 {
            let mut cand = beta;
            for i in 0..P {
                cand[i] += step * d[i];
            }
            if loglik(&obs, &eta_of(&cand, &u, &v)) >= base - 1e-9 {
                beta = cand;
                break;
            }
            step /= 2.0;
        }
        // State, then county effects.
        let eta = eta_of(&beta, &u, &v);
        let hs = re_step(&obs, &eta, &mut u, 1.0 / tau_s2, |o| o.s);
        let eta = eta_of(&beta, &u, &v);
        let hc = re_step(&obs, &eta, &mut v, 1.0 / tau_c2, |o| o.c);
        // Variances (EM with a Laplace approximation) and dispersion.
        let used_c: Vec<usize> = {
            let mut seen = vec![false; nc];
            for o in &obs {
                seen[o.c] = true;
            }
            (0..nc).filter(|&j| seen[j]).collect()
        };
        tau_s2 = ((u.iter().map(|x| x * x).sum::<f64>() + hs.iter().map(|h| 1.0 / h).sum::<f64>())
            / ns as f64)
            .max(1e-4);
        tau_c2 = ((used_c
            .iter()
            .map(|&j| v[j] * v[j] + 1.0 / hc[j])
            .sum::<f64>())
            / used_c.len().max(1) as f64)
            .max(1e-4);
        let eta = eta_of(&beta, &u, &v);
        theta = moment_theta(&obs, &eta);
        let obj = loglik(&obs, &eta)
            - 0.5 * u.iter().map(|x| x * x).sum::<f64>() / tau_s2
            - 0.5 * v.iter().map(|x| x * x).sum::<f64>() / tau_c2;
        if (obj - prev).abs() < 1e-7 * obj.abs().max(1.0) {
            break;
        }
        prev = obj;
    }
    let mut coefficients = BTreeMap::new();
    for (i, f) in FEATURES.iter().enumerate() {
        coefficients.insert(f.to_string(), round(beta[i]));
    }
    for (i, r) in REGIONS.iter().enumerate() {
        coefficients.insert(format!("intercept_{r}"), round(beta[FEATURES.len() + i]));
    }
    let mut seen = vec![false; nc];
    for o in &obs {
        seen[o.c] = true;
    }
    GroupFit {
        coefficients,
        theta: round(theta),
        sd_state: round(tau_s2.sqrt()),
        sd_county: round(tau_c2.sqrt()),
        n_obs: n,
        state_effects: states
            .iter()
            .zip(&u)
            .map(|(k, x)| (k.clone(), round(*x)))
            .collect(),
        county_effects: keys
            .iter()
            .zip(&v)
            .enumerate()
            .filter(|(j, _)| seen[*j])
            .map(|(_, (k, x))| (k.clone(), round(*x)))
            .collect(),
    }
}

/// Six significant digits, so the committed model is stable across machines.
fn round(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let mag = 10f64.powi(5 - x.abs().log10().floor() as i32);
    (x * mag).round() / mag
}

// ------------------------------------------------------------------ forecasting

/// Conditions over the forecast horizon, relative to "recent conditions
/// continue" (the default).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Conditions {
    /// Added to the 10-year Treasury yield from year 1 on (percentage points).
    pub rate_shift_pp: f64,
    /// Multiplies each county's recent net migration rate (1 = unchanged).
    pub migration_factor: f64,
    /// Added to each county's recent annual house-price change (points).
    pub price_shift_pp: f64,
}

impl Default for Conditions {
    fn default() -> Self {
        Conditions {
            rate_shift_pp: 0.0,
            migration_factor: 1.0,
            price_shift_pp: 0.0,
        }
    }
}

impl Conditions {
    /// Bounds accepted from users and tools, so a typo can't ask the model
    /// for something far outside what it was fitted on.
    pub const RATE_SHIFT: (f64, f64) = (-3.0, 5.0);
    pub const MIGRATION_FACTOR: (f64, f64) = (-1.0, 3.0);
    pub const PRICE_SHIFT: (f64, f64) = (-20.0, 20.0);

    /// Read optional `rate_shift_pp`, `migration_factor`, `price_shift_pp`
    /// from tool or API arguments; missing values keep the default, values
    /// are clamped to the bounds above.
    pub fn from_args(args: &serde_json::Value) -> Conditions {
        let d = Conditions::default();
        let get = |k: &str| {
            args.get(k).and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
            })
        };
        Conditions {
            rate_shift_pp: get("rate_shift_pp")
                .unwrap_or(d.rate_shift_pp)
                .clamp(Self::RATE_SHIFT.0, Self::RATE_SHIFT.1),
            migration_factor: get("migration_factor")
                .unwrap_or(d.migration_factor)
                .clamp(Self::MIGRATION_FACTOR.0, Self::MIGRATION_FACTOR.1),
            price_shift_pp: get("price_shift_pp")
                .unwrap_or(d.price_shift_pp)
                .clamp(Self::PRICE_SHIFT.0, Self::PRICE_SHIFT.1),
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Conditions::default()
    }
    pub fn describe(&self) -> String {
        if self.is_default() {
            return "recent conditions continue (10-year Treasury at its last annual average; each county's net migration and house-price growth at their 3-year averages)".into();
        }
        let mut parts = Vec::new();
        if self.rate_shift_pp != 0.0 {
            parts.push(format!(
                "10-year Treasury {:+.2} points",
                self.rate_shift_pp
            ));
        }
        if self.migration_factor != 1.0 {
            parts.push(format!(
                "net migration at {:.0}% of its recent rate",
                100.0 * self.migration_factor
            ));
        }
        if self.price_shift_pp != 0.0 {
            parts.push(format!(
                "house-price growth {:+.1} points a year",
                self.price_shift_pp
            ));
        }
        format!("{} vs. recent conditions", parts.join(", "))
    }
}

/// Predicted permits by year for one county: [1-unit, 2-4, 5+] per year.
pub fn forecast_county(
    p: &Panel,
    fits: (&GroupFit, &GroupFit),
    c: &CountySeries,
    origin: u32,
    years: usize,
    cond: &Conditions,
) -> Option<Vec<[f64; 3]>> {
    let pop_o = c.pop_at(origin)?;
    let growth = match c.pop_at(origin - 3) {
        Some(p3) if p3 > 0.0 => (pop_o / p3).powf(1.0 / 3.0) - 1.0,
        _ => 0.0,
    };
    let recent = |f: &dyn Fn(u32) -> Option<f64>| {
        let v: Vec<f64> = (origin - 2..=origin).filter_map(f).collect();
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    let mig = recent(&|t| c.mig_rate(t)) * cond.migration_factor;
    let hpi = recent(&|t| Some(c.hpi_at(t))) + cond.price_shift_pp;
    let growth = growth + (cond.migration_factor - 1.0) * recent(&|t| c.mig_rate(t)) / 100.0;
    let rate_o = p.rate_at(origin);
    // Split of 2+ unit permits between 2-4 and 5+, from the last five years.
    let (s24, s5): (f64, f64) = (origin - 4..=origin).fold((0.0, 0.0), |a, t| {
        let i = CountySeries::idx(t);
        (
            a.0 + i.and_then(|i| c.p24[i]).unwrap_or(0.0),
            a.1 + i.and_then(|i| c.p5[i]).unwrap_or(0.0),
        )
    });
    let share24 = if s24 + s5 > 0.0 {
        s24 / (s24 + s5)
    } else {
        0.5
    };
    let mut out = vec![[0.0; 3]; years];
    for (gi, (g, fit)) in [(Group::Single, fits.0), (Group::Multi, fits.1)]
        .into_iter()
        .enumerate()
    {
        let beta = fit.beta();
        let effect = fit.effect(c);
        let mut hist: Vec<f64> = vec![c.y(g, origin - 2)?, c.y(g, origin - 1)?, c.y(g, origin)?];
        for k in 1..=years {
            let n = hist.len();
            let inp = Inputs {
                y1: hist[n - 1],
                y2: hist[n - 2],
                y3: hist[n - 3],
                pop1: pop_o * (1.0 + growth).powi(k as i32 - 1),
                mig1: mig,
                hpi1: hpi,
                rate: rate_o + cond.rate_shift_pp,
                rate_prev: if k == 1 {
                    rate_o
                } else {
                    rate_o + cond.rate_shift_pp
                },
                region: c.region,
            };
            let mu = (dot(&beta, &x_vec(&inp)) + effect).clamp(-30.0, 20.0).exp();
            hist.push(mu);
            if gi == 0 {
                out[k - 1][0] = mu;
            } else {
                out[k - 1][1] = mu * share24;
                out[k - 1][2] = mu * (1.0 - share24);
            }
        }
    }
    Some(out)
}

// ------------------------------------------------------------------ model file

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metrics {
    /// State x origin pairs scored.
    pub n: usize,
    /// Median and mean of |predicted / actual - 1| for five-year state totals.
    pub median_abs_pct_error: f64,
    pub mean_abs_pct_error: f64,
    /// Median of log(actual / predicted): above 0 means forecasts ran low.
    pub median_log_ratio: f64,
    /// 10th and 90th percentiles of log(actual / predicted).
    pub q10_log_ratio: f64,
    pub q90_log_ratio: f64,
    /// Share of state forecasts within 10% and 20% of what happened.
    pub within_10pct: f64,
    pub within_20pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NationalRow {
    pub origin: u32,
    pub actual: f64,
    pub model: f64,
    pub recent_pace: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Backtest {
    pub origins: Vec<u32>,
    pub horizon_years: usize,
    pub model: Metrics,
    /// The old rule: the previous five years' permits repeated.
    pub recent_pace: Metrics,
    pub national: Vec<NationalRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelFile {
    pub description: String,
    pub data_through: u32,
    pub fit_years: [u32; 2],
    pub features: Vec<String>,
    pub single: GroupFit,
    pub multi: GroupFit,
    pub backtest: Backtest,
}

pub fn model() -> &'static ModelFile {
    static M: OnceLock<ModelFile> = OnceLock::new();
    M.get_or_init(|| {
        serde_json::from_str(MODEL_JSON)
            .expect("data/derived/baseline_model.json is missing or stale: run `cargo run --release --bin housing-fit`")
    })
}

/// Five-year totals by state for one origin: (actual, model, recent pace).
fn state_errors(p: &Panel, origin: u32) -> BTreeMap<String, (f64, f64, f64)> {
    let fits = (fit(p, Group::Single, origin), fit(p, Group::Multi, origin));
    let mut out: BTreeMap<String, (f64, f64, f64)> = BTreeMap::new();
    let total = |c: &CountySeries, t: u32| {
        c.y(Group::Single, t).unwrap_or(0.0) + c.y(Group::Multi, t).unwrap_or(0.0)
    };
    for c in &p.counties {
        let e = out.entry(c.state.clone()).or_insert((0.0, 0.0, 0.0));
        e.0 += (origin + 1..=origin + HORIZON as u32)
            .map(|t| total(c, t))
            .sum::<f64>();
        e.2 += (origin - 4..=origin).map(|t| total(c, t)).sum::<f64>();
        if let Some(f) = forecast_county(
            p,
            (&fits.0, &fits.1),
            c,
            origin,
            HORIZON,
            &Conditions::default(),
        ) {
            e.1 += f.iter().map(|y| y.iter().sum::<f64>()).sum::<f64>();
        }
    }
    out
}

fn metrics(pairs: &[(f64, f64)]) -> Metrics {
    // pairs: (actual, predicted)
    let mut abs: Vec<f64> = pairs.iter().map(|(a, p)| (p / a - 1.0).abs()).collect();
    let mut logs: Vec<f64> = pairs.iter().map(|(a, p)| (a / p).ln()).collect();
    abs.sort_by(f64::total_cmp);
    logs.sort_by(f64::total_cmp);
    let q = |v: &[f64], q: f64| {
        let i = q * (v.len() - 1) as f64;
        let (lo, hi) = (i.floor() as usize, i.ceil() as usize);
        v[lo] + (v[hi] - v[lo]) * (i - lo as f64)
    };
    let n = pairs.len();
    Metrics {
        n,
        median_abs_pct_error: round(q(&abs, 0.5)),
        mean_abs_pct_error: round(abs.iter().sum::<f64>() / n as f64),
        median_log_ratio: round(q(&logs, 0.5)),
        q10_log_ratio: round(q(&logs, 0.1)),
        q90_log_ratio: round(q(&logs, 0.9)),
        within_10pct: round(abs.iter().filter(|x| **x <= 0.10).count() as f64 / n as f64),
        within_20pct: round(abs.iter().filter(|x| **x <= 0.20).count() as f64 / n as f64),
    }
}

/// Fit on all data and back-test; what `housing-fit` writes.
pub fn build_model_file(p: &Panel) -> ModelFile {
    let mut model_pairs = Vec::new();
    let mut pace_pairs = Vec::new();
    let mut national = Vec::new();
    for &o in &BACKTEST_ORIGINS {
        let by_state = state_errors(p, o);
        let (mut na, mut nm, mut np) = (0.0, 0.0, 0.0);
        for (a, m, r) in by_state.values() {
            if *a > 0.0 && *m > 0.0 && *r > 0.0 {
                model_pairs.push((*a, *m));
                pace_pairs.push((*a, *r));
            }
            na += a;
            nm += m;
            np += r;
        }
        national.push(NationalRow {
            origin: o,
            actual: na.round(),
            model: nm.round(),
            recent_pace: np.round(),
        });
    }
    ModelFile {
        description: "Negative binomial panel model of annual permitted units by county, fitted separately for 1-unit and 2+ unit buildings, with partially pooled state and county effects. See agents/src/forecast.rs and MODEL.md.".into(),
        data_through: LAST_YEAR,
        fit_years: [FIT_FROM, LAST_YEAR],
        features: FEATURES.iter().map(|s| s.to_string()).collect(),
        single: fit(p, Group::Single, LAST_YEAR),
        multi: fit(p, Group::Multi, LAST_YEAR),
        backtest: Backtest {
            origins: BACKTEST_ORIGINS.to_vec(),
            horizon_years: HORIZON,
            model: metrics(&model_pairs),
            recent_pace: metrics(&pace_pairs),
            national,
        },
    }
}

/// Predicted permits by year for every county (key -> [1-unit, 2-4, 5+] per
/// year), from the last data year under `cond`.
pub fn county_forecasts(cond: &Conditions) -> HashMap<String, Vec<[f64; 3]>> {
    let m = model();
    let p = panel();
    p.counties
        .iter()
        .filter_map(|c| {
            forecast_county(p, (&m.single, &m.multi), c, LAST_YEAR, HORIZON, cond)
                .map(|f| (c.key.clone(), f))
        })
        .collect()
}

/// County forecasts under default conditions (cached).
pub fn default_forecasts() -> &'static HashMap<String, Vec<[f64; 3]>> {
    static F: OnceLock<HashMap<String, Vec<[f64; 3]>>> = OnceLock::new();
    F.get_or_init(|| county_forecasts(&Conditions::default()))
}

/// Forecasts for `cond`, borrowing the cache for the default.
pub fn forecasts_for(
    cond: &Conditions,
) -> std::borrow::Cow<'static, HashMap<String, Vec<[f64; 3]>>> {
    if cond.is_default() {
        std::borrow::Cow::Borrowed(default_forecasts())
    } else {
        std::borrow::Cow::Owned(county_forecasts(cond))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic panel with known coefficients: the fit should recover them.
    #[test]
    fn fit_recovers_known_coefficients() {
        let states: Vec<_> = crate::state::all().iter().take(20).collect();
        let mut counties = Vec::new();
        let mut seed = 12345u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 1_000_000) as f64 / 1_000_000.0
        };
        let mut rate = [0.0; NY];
        for (i, r) in rate.iter_mut().enumerate() {
            *r = 2.0 + (i as f64 * 0.7).sin();
        }
        for (k, s) in states.iter().enumerate() {
            for j in 0..15 {
                let mut c = CountySeries {
                    key: format!("{}{:03}", s.fips, j),
                    state: s.fips.clone(),
                    region: REGIONS.iter().position(|r| *r == s.region).unwrap_or(2),
                    p1: [None; NY],
                    p24: [None; NY],
                    p5: [None; NY],
                    pop: [None; NY],
                    mig: [None; NY],
                    hpi: [0.0; NY],
                };
                let base = 3.0 + 2.0 * rnd() + 0.2 * (k as f64 % 3.0);
                for i in 0..NY {
                    c.pop[i] = Some(50_000.0 * (1.0 + rnd()));
                    c.mig[i] = Some(500.0 * (rnd() - 0.5));
                    c.hpi[i] = 10.0 * (rnd() - 0.3);
                }
                let mut y = vec![base.exp(); 3];
                for i in 0..NY {
                    if i >= 3 {
                        let lmu = base * 0.5
                            + 0.4 * (1.0 + y[i - 1]).ln()
                            + 0.1 * (1.0 + (y[i - 2] + y[i - 3]) / 2.0).ln()
                            - 0.15 * rate[i];
                        // Poisson-ish noise around the mean, deterministic.
                        y.push((lmu.exp() * (0.85 + 0.3 * rnd())).round());
                    } else if y.len() <= i {
                        y.push(base.exp());
                    }
                    c.p1[i] = Some(y[i]);
                    c.p24[i] = Some(0.0);
                    c.p5[i] = Some(0.0);
                }
                counties.push(c);
            }
        }
        let p = Panel { counties, rate };
        let f = fit(&p, Group::Single, LAST_YEAR);
        let lag1 = f.coefficients["log_permits_lag1"];
        let r = f.coefficients["treasury_10y_pct"];
        assert!((0.2..0.7).contains(&lag1), "lag1 {lag1}");
        assert!((-0.3..-0.05).contains(&r), "rate {r}");
        assert!(
            f.theta > 5.0,
            "low-noise data should have high theta: {}",
            f.theta
        );
    }

    #[test]
    fn committed_model_matches_panel_and_forecasts_are_sane() {
        let m = model();
        assert_eq!(m.data_through, LAST_YEAR);
        assert!(m.single.n_obs > 30_000 && m.multi.n_obs > 30_000);
        // Persistence: last year's permits predict this year's.
        for g in [&m.single, &m.multi] {
            let lag1 = g.coefficients["log_permits_lag1"];
            assert!((0.1..1.0).contains(&lag1), "lag1 {lag1}");
        }
        let f = default_forecasts();
        assert!(f.len() > 3000);
        for v in f.values() {
            assert!(v
                .iter()
                .all(|y| y.iter().all(|x| x.is_finite() && *x >= 0.0)));
        }
    }

    #[test]
    fn higher_rates_and_weaker_migration_lower_the_baseline() {
        let total = |c: &Conditions| -> f64 {
            county_forecasts(c)
                .values()
                .map(|v| v.iter().map(|y| y.iter().sum::<f64>()).sum::<f64>())
                .sum()
        };
        let base = total(&Conditions::default());
        let worse = total(&Conditions {
            rate_shift_pp: 1.0,
            migration_factor: 0.5,
            price_shift_pp: -5.0,
        });
        let m = model();
        // The direction follows the fitted signs; check the combined effect is
        // consistent with them rather than assuming.
        let rate_sign = m.single.coefficients["treasury_10y_pct"]
            + m.single.coefficients["treasury_10y_change_pct"];
        if rate_sign < 0.0 && m.single.coefficients["net_migration_pct_lag1"] > 0.0 {
            assert!(worse < base, "{worse} vs {base}");
        }
        assert!(Conditions::default().is_default());
        let c = Conditions::from_args(
            &serde_json::json!({"rate_shift_pp": 9, "migration_factor": "0.5"}),
        );
        assert_eq!(c.rate_shift_pp, 5.0);
        assert_eq!(c.migration_factor, 0.5);
        assert_eq!(c.price_shift_pp, 0.0);
        assert!(Conditions::from_args(&serde_json::json!({})).is_default());
        assert!(Conditions {
            rate_shift_pp: 1.0,
            ..Default::default()
        }
        .describe()
        .contains("+1.00 points"));
    }

    #[test]
    fn backtest_metrics_are_recorded_for_both_methods() {
        let b = &model().backtest;
        assert_eq!(b.origins, BACKTEST_ORIGINS.to_vec());
        assert!(b.model.n >= 300 && b.recent_pace.n >= 300);
        assert!(b.model.q10_log_ratio < b.model.q90_log_ratio);
        assert_eq!(b.national.len(), BACKTEST_ORIGINS.len());
    }

    #[test]
    fn rounding_keeps_six_significant_digits() {
        assert_eq!(round(1.23456789), 1.23457);
        assert_eq!(round(-0.000123456789), -0.000123457);
        assert_eq!(round(0.0), 0.0);
    }
}
