//! State and county data, built from public-domain Census data:
//!
//! * `data/derived/county_baseline.csv` (`scripts/fetch_state_baseline.py`):
//!   per county, permits 2021-2025, 2025 population, housing units by
//!   structure type (ACS 5-year).
//! * `data/derived/state_baseline.csv`: per state, the SUM of its counties
//!   (the pipeline checks the sums match Census's own state totals).
//!
//! The five-year **baseline** (current rules) comes from the county permit
//! model in [`crate::forecast`]: each county's permits are forecast for
//! 2026-2030 and summed to the state. The range is the model's back-test
//! error (10th to 90th percentile of five-year state totals, 2016-2025).
//! Permits are converted to construction starts and completed homes with the
//! Survey of Construction curves in [`crate::timing`], including homes
//! already in the pipeline from 2021-2025 permits.
//!
//! The previous rule (the 2021-2025 pace repeated) is kept as
//! [`StateBaseline::recent_pace`] for comparison.

use std::collections::HashMap;
use std::sync::OnceLock;

pub use crate::forecast::Conditions;
use crate::timing::{BType, Flow};

const STATE_CSV: &str = include_str!("../../data/derived/state_baseline.csv");
const COUNTY_CSV: &str = include_str!("../../data/derived/county_baseline.csv");
pub const PROVENANCE: &str = "research/data/census-building-permits-survey.md";
pub const HORIZON_YEARS: u64 = 5;

/// Housing units by structure type (ACS 5-year 2020-2024, table B25024).
#[derive(Debug, Clone, Copy, Default)]
pub struct Stock {
    pub total: u64,
    pub sf_detached: u64,
    pub sf_attached: u64,
    pub u2to4: u64,
    pub u5plus: u64,
}

impl Stock {
    pub fn sf_detached_share(&self) -> Option<f64> {
        (self.total > 0).then(|| self.sf_detached as f64 / self.total as f64)
    }
    pub fn multifamily_share(&self) -> Option<f64> {
        (self.total > 0).then(|| self.u5plus as f64 / self.total as f64)
    }
}

#[derive(Debug, Clone)]
pub struct StateBaseline {
    pub fips: String,
    pub abbr: String,
    pub name: String,
    /// Census region: northeast, midwest, south or west.
    pub region: String,
    pub population: u64,
    /// (year, units authorized), oldest first. Sum of the state's counties.
    pub permits: Vec<(u32, u64)>,
    /// Latest year by structure type: 1 unit, 2 units, 3-4 units, 5+ units.
    pub latest_by_type: [u64; 4],
    pub stock: Stock,
}

/// One county (or Census "balance of state" permit record).
#[derive(Debug, Clone)]
pub struct County {
    pub state_fips: String,
    pub county_fips: String,
    pub abbr: String,
    pub name: String,
    /// False for rural counties where no place issues building permits;
    /// homes built there never appear in permit counts.
    pub permit_issuing: bool,
    /// Missing for retired geographies (e.g. Connecticut's pre-2022
    /// counties) and balance-of-state records; their people are counted
    /// in other rows.
    pub population: Option<u64>,
    pub permits: Vec<(u32, u64)>,
    pub latest_by_type: [u64; 4],
    pub stock: Option<Stock>,
}

impl County {
    pub fn average_annual(&self) -> f64 {
        self.permits.iter().map(|p| p.1 as f64).sum::<f64>() / self.permits.len().max(1) as f64
    }
}

/// Low, central and high five-year totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub low: u64,
    pub central: u64,
    pub high: u64,
}

impl County {
    /// Panel key: state FIPS + county FIPS.
    pub fn key(&self) -> String {
        format!("{}{}", self.state_fips, self.county_fips)
    }
}

/// A state's baseline forecast: permits by year and building type.
#[derive(Debug, Clone)]
pub struct StateForecast {
    /// Per horizon year: [1-unit, 2-4, 5+] permits.
    pub by_year: Vec<[f64; 3]>,
}

impl StateForecast {
    pub fn total(&self) -> f64 {
        self.by_year.iter().map(|y| y.iter().sum::<f64>()).sum()
    }
    pub fn path(&self) -> Vec<f64> {
        self.by_year.iter().map(|y| y.iter().sum()).collect()
    }
    /// Shares of forecast permits by building type.
    pub fn mix(&self) -> [f64; 3] {
        let t = self.total().max(f64::MIN_POSITIVE);
        let mut m = [0.0; 3];
        for y in &self.by_year {
            for k in 0..3 {
                m[k] += y[k] / t;
            }
        }
        m
    }
}

/// Model back-test multipliers for the baseline range: exp of the 10th and
/// 90th percentile of log(actual / predicted) five-year state totals.
pub fn range_multipliers() -> (f64, f64) {
    let m = &crate::forecast::model().backtest.model;
    (m.q10_log_ratio.exp(), m.q90_log_ratio.exp())
}

impl StateBaseline {
    pub fn latest_year(&self) -> u32 {
        self.permits.last().map_or(0, |p| p.0)
    }

    pub fn average_annual(&self) -> f64 {
        self.permits.iter().map(|p| p.1 as f64).sum::<f64>() / self.permits.len() as f64
    }

    /// The model's forecast of permits by year and building type.
    pub fn forecast(&self, cond: &Conditions) -> StateForecast {
        let f = crate::forecast::forecasts_for(cond);
        self.forecast_from(&f)
    }

    /// Same, from a precomputed set of county forecasts.
    pub fn forecast_from(&self, f: &HashMap<String, Vec<[f64; 3]>>) -> StateForecast {
        let mut by_year = vec![[0.0; 3]; HORIZON_YEARS as usize];
        for c in self.counties() {
            if let Some(v) = f.get(&c.key()) {
                for (y, row) in v.iter().enumerate() {
                    for k in 0..3 {
                        by_year[y][k] += row[k];
                    }
                }
            }
        }
        StateForecast { by_year }
    }

    /// The five-year current-rules baseline in permits (see module docs).
    pub fn baseline(&self) -> Range {
        self.baseline_with(&Conditions::default())
    }

    pub fn baseline_with(&self, cond: &Conditions) -> Range {
        let (lo, hi) = range_multipliers();
        let c = self.forecast(cond).total();
        Range {
            low: (c * lo).round() as u64,
            central: c.round() as u64,
            high: (c * hi).round() as u64,
        }
    }

    /// The previous rule: 2021-2025 average x 5 (range: slowest and fastest
    /// year x 5). Kept for comparison with the model.
    pub fn recent_pace(&self) -> Range {
        let min = self.permits.iter().map(|p| p.1).min().unwrap_or(0);
        let max = self.permits.iter().map(|p| p.1).max().unwrap_or(0);
        Range {
            low: min * HORIZON_YEARS,
            central: (self.average_annual() * HORIZON_YEARS as f64).round() as u64,
            high: max * HORIZON_YEARS,
        }
    }

    /// Actual permits by building type in the years before the horizon,
    /// most recent first: [1-unit, 2-4, 5+] for 2025, 2024, ...
    pub fn prior_by_type(&self) -> Vec<[f64; 3]> {
        let p = crate::forecast::panel();
        let mut out = vec![[0.0; 3]; 5];
        for c in p.counties.iter().filter(|c| c.state == self.fips) {
            for (k, row) in out.iter_mut().enumerate() {
                let t = crate::forecast::LAST_YEAR - k as u32;
                let i = (t - crate::forecast::FIRST_YEAR) as usize;
                row[0] += c.p1[i].unwrap_or(0.0);
                row[1] += c.p24[i].unwrap_or(0.0);
                row[2] += c.p5[i].unwrap_or(0.0);
            }
        }
        out
    }

    /// Baseline permits, starts and completions by year, including homes
    /// already in the pipeline from 2021-2025 permits.
    pub fn baseline_flow(&self, cond: &Conditions) -> Flow {
        self.flow_for(&self.forecast(cond))
    }

    pub fn flow_for(&self, f: &StateForecast) -> Flow {
        let prior = self.prior_by_type();
        let mut total = Flow::default();
        for (k, b) in BType::ALL.into_iter().enumerate() {
            let path: Vec<f64> = f.by_year.iter().map(|y| y[k]).collect();
            let pr: Vec<f64> = prior.iter().map(|y| y[k]).collect();
            total.add(&crate::timing::flow(&self.region, b, &path, &pr));
        }
        total
    }

    /// Baseline homes completed over the five years (range scaled like the
    /// permit range).
    pub fn baseline_completions(&self) -> Range {
        let (lo, hi) = range_multipliers();
        let c = self
            .baseline_flow(&Conditions::default())
            .total_completions();
        Range {
            low: (c * lo).round() as u64,
            central: c.round() as u64,
            high: (c * hi).round() as u64,
        }
    }

    /// Permits per year that one unit per 100,000 residents represents here,
    /// for converting per-capita rates reported by studies.
    pub fn per_100k(&self) -> f64 {
        self.population as f64 / 100_000.0
    }

    pub fn counties(&self) -> Vec<&'static County> {
        counties()
            .iter()
            .filter(|c| c.state_fips == self.fips)
            .collect()
    }

    /// Share of residents living in counties where no place issues permits.
    pub fn non_permit_population_share(&self) -> f64 {
        let outside: u64 = self
            .counties()
            .iter()
            .filter(|c| !c.permit_issuing)
            .filter_map(|c| c.population)
            .sum();
        outside as f64 / self.population as f64
    }

    /// Plain-language profile given to the experts and shown to users.
    pub fn profile(&self) -> String {
        let b = self.baseline();
        let pace = self.recent_pace();
        let flow = self.baseline_flow(&Conditions::default());
        let f = self.forecast(&Conditions::default());
        let mix = f.mix();
        let bt = &crate::forecast::model().backtest;
        let years: Vec<String> = self
            .permits
            .iter()
            .map(|(y, n)| format!("{y}: {}", thousands(*n)))
            .collect();
        let total_latest: u64 = self.latest_by_type.iter().sum();
        let share = |n: u64| {
            if total_latest == 0 {
                0.0
            } else {
                100.0 * n as f64 / total_latest as f64
            }
        };
        let pct = |x: Option<f64>| x.map_or("n/a".to_string(), |v| format!("{:.0}%", 100.0 * v));
        let by_year = |v: &[f64]| {
            v.iter()
                .enumerate()
                .map(|(i, x)| {
                    format!(
                        "{} {}",
                        crate::forecast::LAST_YEAR + 1 + i as u32,
                        thousands(x.round() as u64)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        let c1 = crate::timing::curve(&self.region, BType::OneUnit);
        let c5 = crate::timing::curve(&self.region, BType::FivePlus);
        format!(
            "STATE PROFILE {name} ({abbr}), {region} region, {counties} counties\n\
             Population (2025 estimate): {pop}; {nonpermit:.1}% live in counties where no place issues building permits (homes built there aren't counted)\n\
             Housing units authorized by permit, by year (sum of counties): {years}\n\
             Average per year {first}-{last}: {avg}; per 100,000 residents per year: {per100k:.0}\n\
             {last} mix of NEW permits by building size (not the existing housing stock): 1 unit {s1:.0}%, 2 units {s2:.0}%, 3-4 units {s3:.0}%, 5+ units {s5:.0}%\n\
             Existing homes (ACS 2020-2024): {hu} units; single-family detached {sfd}, buildings with 5+ units {mf}\n\
             FIVE-YEAR BASELINE {y1}-{y5} (current rules; county permit model, recent conditions continue): central {central} units permitted (range {low} to {high}); \
             {starts} construction starts; {done} homes completed (including homes already in the pipeline from earlier permits)\n\
             Baseline permits by year: {pby}. Completions by year: {cby}. Forecast mix: 1 unit {m1:.0}%, 2-4 units {m2:.0}%, 5+ units {m5:.0}%\n\
             Method: each county's permits forecast from its own recent permits, population, net migration, house prices and the 10-year Treasury rate, summed to the state. \
             Range = the model's back-test errors (10th-90th percentile; median state error {med:.0}% vs {pmed:.0}% for repeating the last five years). \
             For comparison, repeating the {first}-{last} pace gives {pace}.\n\
             Timing ({region} region, Survey of Construction 2015-2024 permit cohorts): {sr1:.0}% of single-family and {sr5:.0}% of 2+ unit permits start construction; \
             permit to completion takes about {t1:.0} months (single-family) or {t5:.0} months (5+ units).\n\
             Conversion: a rate of 1 unit per 100,000 residents per year = {conv:.0} units per year statewide (use the state.scale_rate tool rather than doing this by hand).\n\
             Source: Census Building Permits Survey, Population Estimates, ACS and Survey of Construction; FHFA HPI; U.S. Treasury (all public domain); provenance {PROVENANCE}",
            name = self.name,
            abbr = self.abbr,
            region = self.region,
            counties = self.counties().len(),
            nonpermit = 100.0 * self.non_permit_population_share(),
            pop = thousands(self.population),
            years = years.join(", "),
            first = self.permits.first().map_or(0, |p| p.0),
            last = self.latest_year(),
            avg = thousands(self.average_annual().round() as u64),
            per100k = self.average_annual() / self.per_100k(),
            s1 = share(self.latest_by_type[0]),
            s2 = share(self.latest_by_type[1]),
            s3 = share(self.latest_by_type[2]),
            s5 = share(self.latest_by_type[3]),
            hu = thousands(self.stock.total),
            sfd = pct(self.stock.sf_detached_share()),
            mf = pct(self.stock.multifamily_share()),
            y1 = crate::forecast::LAST_YEAR + 1,
            y5 = crate::forecast::LAST_YEAR + HORIZON_YEARS as u32,
            central = thousands(b.central),
            low = thousands(b.low),
            high = thousands(b.high),
            starts = thousands(flow.total_starts().round() as u64),
            done = thousands(flow.total_completions().round() as u64),
            pby = by_year(&flow.permits),
            cby = by_year(&flow.completions),
            m1 = 100.0 * mix[0],
            m2 = 100.0 * mix[1],
            m5 = 100.0 * mix[2],
            med = 100.0 * bt.model.median_abs_pct_error,
            pmed = 100.0 * bt.recent_pace.median_abs_pct_error,
            pace = thousands(pace.central),
            sr1 = 100.0 * c1.start_rate,
            sr5 = 100.0 * c5.start_rate,
            t1 = c1.months_to_start + c1.months_to_complete,
            t5 = c5.months_to_start + c5.months_to_complete,
            conv = self.per_100k(),
        )
    }
}

/// Scale a per-capita rate reported by a study to this state, showing the
/// arithmetic so a reader can check it.
pub fn scale_rate(s: &StateBaseline, per_100k_per_year: f64) -> String {
    let per_year = per_100k_per_year * s.per_100k();
    let five = per_year * HORIZON_YEARS as f64;
    let share = 100.0 * five / s.baseline().central as f64;
    format!(
        "{rate} per 100,000 residents per year in {name} (population {pop}) = {per_year} units per year = {five} units over {h} years.\n\
         Arithmetic: {rate} x {pop} / 100,000 = {per_year} per year; x {h} years = {five}.\n\
         For scale: {five} is {share:.1}% of the five-year baseline of {base} units (all housing types).\n\
         This applies another place's rate to this state as-is; it is a comparison, not a forecast.",
        rate = trim_float(per_100k_per_year),
        name = s.name,
        pop = thousands(s.population),
        per_year = thousands(per_year.round() as u64),
        five = thousands(five.round() as u64),
        h = HORIZON_YEARS,
        base = thousands(s.baseline().central),
    )
}

fn trim_float(x: f64) -> String {
    if x.fract() == 0.0 {
        format!("{x:.0}")
    } else {
        format!("{x}")
    }
}

/// `1234567` -> `1,234,567`.
pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Split one CSV line, honoring double-quoted fields (county names can
/// contain commas).
fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            ('"', _) => quoted = !quoted,
            (',', false) => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// Header lookup plus typed field access for one of the derived CSVs.
struct Table {
    header: HashMap<String, usize>,
    years: Vec<(usize, u32)>,
    rows: Vec<Vec<String>>,
}

impl Table {
    fn parse(csv: &str) -> Table {
        let mut lines = csv.lines().filter(|l| !l.trim().is_empty());
        let head = split_csv(lines.next().expect("header"));
        let years = head
            .iter()
            .enumerate()
            .filter_map(|(i, h)| {
                h.strip_prefix("permits_")
                    .and_then(|y| y.parse().ok())
                    .map(|y| (i, y))
            })
            .collect();
        Table {
            header: head.into_iter().enumerate().map(|(i, h)| (h, i)).collect(),
            years,
            rows: lines.map(split_csv).collect(),
        }
    }
    fn col(&self, name: &str) -> usize {
        *self
            .header
            .get(name)
            .unwrap_or_else(|| panic!("missing column {name}"))
    }
    fn num(&self, row: &[String], name: &str) -> Option<u64> {
        row[self.col(name)].trim().parse().ok()
    }
    fn permits(&self, row: &[String]) -> Vec<(u32, u64)> {
        self.years
            .iter()
            .map(|(i, y)| (*y, row[*i].trim().parse().expect("permits")))
            .collect()
    }
    fn mix(&self, row: &[String]) -> [u64; 4] {
        [
            "permits_2025_1unit",
            "permits_2025_2unit",
            "permits_2025_3to4unit",
            "permits_2025_5plus",
        ]
        .map(|c| self.num(row, c).unwrap_or(0))
    }
    fn stock(&self, row: &[String]) -> Option<Stock> {
        Some(Stock {
            total: self.num(row, "hu_total")?,
            sf_detached: self.num(row, "hu_sf_detached")?,
            sf_attached: self.num(row, "hu_sf_attached")?,
            u2to4: self.num(row, "hu_2to4")?,
            u5plus: self.num(row, "hu_5plus")?,
        })
    }
}

pub fn all() -> &'static [StateBaseline] {
    static STATES: OnceLock<Vec<StateBaseline>> = OnceLock::new();
    STATES.get_or_init(|| {
        let t = Table::parse(STATE_CSV);
        t.rows
            .iter()
            .map(|r| StateBaseline {
                fips: r[t.col("fips")].clone(),
                abbr: r[t.col("abbr")].clone(),
                name: r[t.col("name")].clone(),
                region: r[t.col("region")].clone(),
                population: t.num(r, "population_2025").expect("state population"),
                permits: t.permits(r),
                latest_by_type: t.mix(r),
                stock: t.stock(r).unwrap_or_default(),
            })
            .collect()
    })
}

pub fn counties() -> &'static [County] {
    static COUNTIES: OnceLock<Vec<County>> = OnceLock::new();
    COUNTIES.get_or_init(|| {
        let t = Table::parse(COUNTY_CSV);
        t.rows
            .iter()
            .map(|r| County {
                state_fips: r[t.col("state_fips")].clone(),
                county_fips: r[t.col("county_fips")].clone(),
                abbr: r[t.col("abbr")].clone(),
                name: r[t.col("name")].clone(),
                permit_issuing: t.num(r, "permit_issuing") == Some(1),
                population: t.num(r, "population_2025"),
                permits: t.permits(r),
                latest_by_type: t.mix(r),
                stock: t.stock(r),
            })
            .collect()
    })
}

/// National housing-stock shares, for comparing a county or state with the
/// U.S. as a whole.
pub fn national_stock() -> Stock {
    all().iter().fold(Stock::default(), |a, s| Stock {
        total: a.total + s.stock.total,
        sf_detached: a.sf_detached + s.stock.sf_detached,
        sf_attached: a.sf_attached + s.stock.sf_attached,
        u2to4: a.u2to4 + s.stock.u2to4,
        u5plus: a.u5plus + s.stock.u5plus,
    })
}

/// Prefix of the context line that carries the chosen state in a question.
pub const STATE_LINE: &str = "STATE: ";

/// `STATE: CA` line plus the question, the form agents pass along.
pub fn with_state(question: &str, state: Option<&StateBaseline>) -> String {
    match state {
        Some(s) => format!("{STATE_LINE}{}\n{question}", s.abbr),
        None => question.to_string(),
    }
}

/// Split a message into its state (if a `STATE:` line names one) and the
/// question text without that line.
pub fn split_state(text: &str) -> (Option<&'static StateBaseline>, String) {
    let mut state = None;
    let mut rest = Vec::new();
    for line in text.lines() {
        match line.trim().strip_prefix(STATE_LINE.trim_end()) {
            Some(code) if state.is_none() && find(code.trim()).is_some() => {
                state = find(code.trim())
            }
            _ => rest.push(line),
        }
    }
    (state, rest.join("\n").trim().to_string())
}

/// Look up by USPS abbreviation, FIPS code or full name (case-insensitive).
pub fn find(query: &str) -> Option<&'static StateBaseline> {
    let q = query.trim();
    all()
        .iter()
        .find(|s| s.abbr.eq_ignore_ascii_case(q) || s.fips == q || s.name.eq_ignore_ascii_case(q))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fifty_states_and_dc_load() {
        assert_eq!(all().len(), 51);
        assert!(all()
            .iter()
            .all(|s| s.permits.len() == 5 && s.population > 0 && s.stock.total > 0));
        assert!(all()
            .iter()
            .all(|s| ["northeast", "midwest", "south", "west"].contains(&s.region.as_str())));
    }

    #[test]
    fn state_permits_are_the_sum_of_their_counties() {
        for s in all() {
            let cs = s.counties();
            assert!(!cs.is_empty(), "{} has no counties", s.abbr);
            for (i, (year, total)) in s.permits.iter().enumerate() {
                let sum: u64 = cs.iter().map(|c| c.permits[i].1).sum();
                assert_eq!(sum, *total, "{} {year}", s.abbr);
            }
        }
    }

    #[test]
    fn county_population_adds_up_to_the_state() {
        // Retired geographies and balance-of-state rows have no population;
        // everyone is counted once in a current county.
        for s in all() {
            let sum: u64 = s.counties().iter().filter_map(|c| c.population).sum();
            let gap = (sum as f64 - s.population as f64).abs() / s.population as f64;
            assert!(
                gap < 0.001,
                "{}: counties {sum} vs state {}",
                s.abbr,
                s.population
            );
        }
    }

    #[test]
    fn find_accepts_abbreviation_name_and_fips() {
        assert_eq!(find("ca").unwrap().name, "California");
        assert_eq!(find("Pennsylvania").unwrap().abbr, "PA");
        assert_eq!(find("10").unwrap().abbr, "DE");
        assert!(find("Narnia").is_none());
    }

    #[test]
    fn baseline_is_the_sum_of_county_forecasts_with_backtest_range() {
        let s = find("DE").unwrap();
        let f = crate::forecast::default_forecasts();
        let sum: f64 = s
            .counties()
            .iter()
            .filter_map(|c| f.get(&c.key()))
            .map(|v| v.iter().map(|y| y.iter().sum::<f64>()).sum::<f64>())
            .sum();
        let b = s.baseline();
        assert_eq!(b.central, sum.round() as u64);
        let (lo, hi) = range_multipliers();
        assert!(lo < 1.0 && hi > 1.0);
        assert_eq!(b.low, (sum * lo).round() as u64);
        assert!(b.low <= b.central && b.central <= b.high);
        // The old rule is still available for comparison.
        let total: u64 = s.permits.iter().map(|p| p.1).sum();
        assert_eq!(s.recent_pace().central, total);
    }

    #[test]
    fn baseline_flow_counts_the_pipeline_and_lags_permits() {
        for s in all() {
            let flow = s.baseline_flow(&Conditions::default());
            assert_eq!(flow.permits.len(), 5);
            // Year 1 completions come mostly from homes permitted before 2026.
            assert!(flow.completions[0] > 0.0, "{}", s.abbr);
            assert!(flow.total_starts() <= flow.total_permits() * 1.2 + 1.0);
            let bc = s.baseline_completions();
            assert!(bc.low <= bc.central && bc.central <= bc.high);
        }
        // Prior permits by type add up to the state's own annual totals.
        let s = find("PA").unwrap();
        let prior = s.prior_by_type();
        let last = s.permits.last().unwrap().1 as f64;
        assert!((prior[0].iter().sum::<f64>() - last).abs() < 0.5);
    }

    #[test]
    fn profile_states_outcome_rule_and_provenance() {
        let p = find("PA").unwrap().profile();
        assert!(p.contains("FIVE-YEAR BASELINE"));
        assert!(p.contains("units authorized by permit"));
        assert!(p.contains("homes completed"));
        assert!(p.contains("county permit model"));
        assert!(p.contains(PROVENANCE));
    }

    #[test]
    fn csv_split_handles_quotes() {
        assert_eq!(split_csv(r#"a,"b, c",d"#), vec!["a", "b, c", "d"]);
        assert_eq!(split_csv(r#""say ""hi""",x"#), vec![r#"say "hi""#, "x"]);
    }

    #[test]
    fn state_line_round_trips() {
        let msg = with_state("How many ADUs?", find("WI"));
        let (st, q) = split_state(&msg);
        assert_eq!(st.unwrap().abbr, "WI");
        assert_eq!(q, "How many ADUs?");
        assert!(split_state("no state here").0.is_none());
    }

    #[test]
    fn scale_rate_shows_its_arithmetic() {
        let s = find("PA").unwrap();
        let out = scale_rate(s, 81.0);
        let per_year = (81.0 * s.population as f64 / 100_000.0).round() as u64;
        assert!(
            out.contains(&format!("= {} units per year", thousands(per_year))),
            "{out}"
        );
        assert!(
            out.contains(&format!(
                "= {} units over 5 years",
                thousands((81.0 * s.population as f64 / 100_000.0 * 5.0).round() as u64)
            )),
            "{out}"
        );
        assert!(out.contains("not a forecast"));
    }

    #[test]
    fn profile_gives_the_rate_per_100k_directly() {
        let s = find("PA").unwrap();
        let expected = (s.average_annual() / s.per_100k()).round() as u64;
        assert!(
            s.profile()
                .contains(&format!("per 100,000 residents per year: {expected}")),
            "{}",
            s.profile()
        );
    }

    #[test]
    fn thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }
}
