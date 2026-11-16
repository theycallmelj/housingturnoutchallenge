//! State and county baselines, built by `scripts/fetch_state_baseline.py`
//! and `scripts/fetch_construction_timing.py` from public-domain Census data:
//!
//! * `data/derived/county_baseline.csv`: per county, permits 2021-2025,
//!   2025 population, housing units by structure type (ACS 5-year).
//! * `data/derived/state_baseline.csv`: per state, the SUM of its counties
//!   (the pipeline checks the sums match Census's own state totals).
//! * `data/derived/completion_factors.csv`: per Census region, how many
//!   permitted units get completed and how long that takes.
//!
//! The baseline is deliberately simple so a policymaker can check it by hand:
//!
//! * outcome: housing units authorized by building permit (Census BPS);
//! * central: the 2021-2025 average annual permits × 5 years;
//! * range: the lowest and highest year in that window × 5;
//! * completions: the same pace × the region's completion ratio.
//!
//! It assumes the recent pace continues with no policy change. It is a
//! starting point for scenario comparison, not a forecast of the economy.

use std::collections::HashMap;
use std::sync::OnceLock;

const STATE_CSV: &str = include_str!("../../data/derived/state_baseline.csv");
const COUNTY_CSV: &str = include_str!("../../data/derived/county_baseline.csv");
const COMPLETION_CSV: &str = include_str!("../../data/derived/completion_factors.csv");
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

/// How permitted units turn into completed homes in one Census region.
#[derive(Debug, Clone, Copy)]
pub struct Completion {
    /// Completions / permits a year earlier, 2016-2025. Capped at 1 when used.
    pub ratio: f64,
    pub months_1unit: f64,
    pub months_2plus: f64,
}

/// Low, central and high five-year totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub low: u64,
    pub central: u64,
    pub high: u64,
}

/// Share of a year's permits whose homes are finished by the end of the
/// horizon. Permits are spread evenly through year `y` (1-based); one issued
/// at time `y - 1 + u` finishes at `y - 1 + u + months / 12`.
pub fn finished_by_horizon(year: usize, months: f64) -> f64 {
    (HORIZON_YEARS as f64 + 1.0 - year as f64 - months / 12.0).clamp(0.0, 1.0)
}

impl StateBaseline {
    pub fn latest_year(&self) -> u32 {
        self.permits.last().map_or(0, |p| p.0)
    }

    pub fn average_annual(&self) -> f64 {
        self.permits.iter().map(|p| p.1 as f64).sum::<f64>() / self.permits.len() as f64
    }

    /// The five-year current-rules baseline in permits (see module docs).
    pub fn baseline(&self) -> Range {
        let min = self.permits.iter().map(|p| p.1).min().unwrap_or(0);
        let max = self.permits.iter().map(|p| p.1).max().unwrap_or(0);
        Range {
            low: min * HORIZON_YEARS,
            central: (self.average_annual() * HORIZON_YEARS as f64).round() as u64,
            high: max * HORIZON_YEARS,
        }
    }

    /// The baseline in completed homes. At a steady pace, homes finished in
    /// the window match permits × the completion ratio (earlier permits
    /// finish inside it as later ones finish after it).
    pub fn baseline_completions(&self) -> Range {
        let r = self.completion().ratio.min(1.0);
        let b = self.baseline();
        let f = |x: u64| (x as f64 * r).round() as u64;
        Range {
            low: f(b.low),
            central: f(b.central),
            high: f(b.high),
        }
    }

    pub fn completion(&self) -> Completion {
        completion(&self.region)
    }

    /// Share of the latest year's permits in single-unit buildings.
    pub fn one_unit_share(&self) -> f64 {
        let t: u64 = self.latest_by_type.iter().sum();
        if t == 0 {
            0.5
        } else {
            self.latest_by_type[0] as f64 / t as f64
        }
    }

    /// Homes finished within the horizon from a per-year path of NEW
    /// permits that starts in year 1 (a reform's added permits).
    pub fn completions_from_path(&self, path: &[f64], one_unit_share: f64) -> f64 {
        let c = self.completion();
        path.iter()
            .enumerate()
            .map(|(i, p)| {
                let y = i + 1;
                p * c.ratio.min(1.0)
                    * (one_unit_share * finished_by_horizon(y, c.months_1unit)
                        + (1.0 - one_unit_share) * finished_by_horizon(y, c.months_2plus))
            })
            .sum()
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
        let bc = self.baseline_completions();
        let c = self.completion();
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
        format!(
            "STATE PROFILE {name} ({abbr}), {region} region, {counties} counties\n\
             Population (2025 estimate): {pop}; {nonpermit:.1}% live in counties where no place issues building permits (homes built there aren't counted)\n\
             Housing units authorized by permit, by year (sum of counties): {years}\n\
             Average per year {first}-{last}: {avg}; per 100,000 residents per year: {per100k:.0}\n\
             {last} mix of NEW permits by building size (not the existing housing stock): 1 unit {s1:.0}%, 2 units {s2:.0}%, 3-4 units {s3:.0}%, 5+ units {s5:.0}%\n\
             Existing homes (ACS 2020-2024): {hu} units; single-family detached {sfd}, buildings with 5+ units {mf}\n\
             FIVE-YEAR BASELINE (current rules, recent pace continues): central {central} units permitted (range {low} to {high}); about {ccentral} completed (range {clow} to {chigh})\n\
             Rule: average annual permits x 5; range = slowest and fastest year x 5. Completed = permits x {ratio:.0}% ({region} region: completions vs permits a year earlier, 2016-2025). Homes take about {m1:.0} months (single-family) or {m2:.0} months (2+ units) from permit to completion.\n\
             Conversion: a rate of 1 unit per 100,000 residents per year = {conv:.0} units per year statewide (use the state.scale_rate tool rather than doing this by hand).\n\
             Source: Census Building Permits Survey, Population Estimates, ACS and Survey of Construction (public domain); provenance {PROVENANCE}",
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
            central = thousands(b.central),
            low = thousands(b.low),
            high = thousands(b.high),
            ccentral = thousands(bc.central),
            clow = thousands(bc.low),
            chigh = thousands(bc.high),
            ratio = 100.0 * c.ratio.min(1.0),
            m1 = c.months_1unit,
            m2 = c.months_2plus,
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

/// Completion factors for a Census region (falls back to the U.S.).
pub fn completion(region: &str) -> Completion {
    static TABLE: OnceLock<HashMap<String, Completion>> = OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let t = Table::parse(COMPLETION_CSV);
        t.rows
            .iter()
            .map(|r| {
                let f = |c: &str| {
                    r[t.col(c)]
                        .trim()
                        .parse::<f64>()
                        .expect("completion factor")
                };
                (
                    r[t.col("region")].clone(),
                    Completion {
                        ratio: f("completion_ratio"),
                        months_1unit: f("months_permit_to_completion_1unit"),
                        months_2plus: f("months_permit_to_completion_2plus"),
                    },
                )
            })
            .collect()
    });
    *t.get(region)
        .or_else(|| t.get("united_states"))
        .expect("U.S. completion factors")
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
    fn baseline_follows_the_stated_rule() {
        let s = find("DE").unwrap();
        let total: u64 = s.permits.iter().map(|p| p.1).sum();
        let b = s.baseline();
        // central = average x 5 = the five-year sum when the window is five years
        assert_eq!(b.central, total);
        assert_eq!(b.low, s.permits.iter().map(|p| p.1).min().unwrap() * 5);
        assert_eq!(b.high, s.permits.iter().map(|p| p.1).max().unwrap() * 5);
        assert!(b.low <= b.central && b.central <= b.high);
        let bc = s.baseline_completions();
        assert!(bc.central < b.central && bc.low <= bc.central && bc.central <= bc.high);
    }

    #[test]
    fn completion_factors_are_plausible_and_used_by_region() {
        for r in ["united_states", "northeast", "midwest", "south", "west"] {
            let c = completion(r);
            assert!((0.6..=1.2).contains(&c.ratio), "{r} ratio {}", c.ratio);
            assert!(
                c.months_1unit > 3.0 && c.months_2plus > c.months_1unit,
                "{r}"
            );
        }
        assert_eq!(
            find("PA").unwrap().completion().ratio,
            completion("northeast").ratio
        );
    }

    #[test]
    fn finished_by_horizon_follows_the_timing_rule() {
        // Year-1 permits with a 12-month build all finish; year-5 permits
        // with a 12-month build finish only if issued at the very start.
        assert_eq!(finished_by_horizon(1, 12.0), 1.0);
        assert_eq!(finished_by_horizon(5, 12.0), 0.0);
        assert!((finished_by_horizon(5, 6.0) - 0.5).abs() < 1e-12);
        let s = find("PA").unwrap();
        let path = [100.0; 5];
        let done = s.completions_from_path(&path, 1.0);
        let c = s.completion();
        let expected: f64 = (1..=5)
            .map(|y| 100.0 * c.ratio.min(1.0) * finished_by_horizon(y, c.months_1unit))
            .sum();
        assert!((done - expected).abs() < 1e-9 && done < 500.0);
    }

    #[test]
    fn profile_states_outcome_rule_and_provenance() {
        let p = find("PA").unwrap().profile();
        assert!(p.contains("FIVE-YEAR BASELINE"));
        assert!(p.contains("units authorized by permit"));
        assert!(p.contains("completed"));
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
