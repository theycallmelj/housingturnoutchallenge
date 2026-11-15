//! Statewide baselines from `data/derived/state_baseline.csv`, built by
//! `scripts/fetch_state_baseline.py` from the Census Building Permits Survey
//! and Population Estimates (both public domain).
//!
//! The challenge asks for statewide impacts relative to a five-year baseline
//! under current rules. The baseline here is deliberately simple so a
//! policymaker can check it by hand:
//!
//! * outcome: housing units authorized by building permit (Census BPS);
//! * central: the 2021-2025 average annual permits × 5 years;
//! * range: the lowest and highest year in that window × 5.
//!
//! It assumes the recent pace continues with no policy change. It is a
//! starting point for scenario comparison, not a forecast of the economy.

use std::sync::OnceLock;

const CSV: &str = include_str!("../../data/derived/state_baseline.csv");
pub const PROVENANCE: &str = "research/data/census-building-permits-survey.md";
pub const HORIZON_YEARS: u64 = 5;

#[derive(Debug, Clone)]
pub struct StateBaseline {
    pub fips: String,
    pub abbr: String,
    pub name: String,
    pub population: u64,
    /// (year, units authorized), oldest first.
    pub permits: Vec<(u32, u64)>,
    /// Latest year by structure type: 1 unit, 2 units, 3-4 units, 5+ units.
    pub latest_by_type: [u64; 4],
}

/// Low, central and high five-year totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub low: u64,
    pub central: u64,
    pub high: u64,
}

impl StateBaseline {
    pub fn latest_year(&self) -> u32 {
        self.permits.last().map_or(0, |p| p.0)
    }

    pub fn average_annual(&self) -> f64 {
        self.permits.iter().map(|p| p.1 as f64).sum::<f64>() / self.permits.len() as f64
    }

    /// The five-year current-rules baseline (see module docs).
    pub fn baseline(&self) -> Range {
        let min = self.permits.iter().map(|p| p.1).min().unwrap_or(0);
        let max = self.permits.iter().map(|p| p.1).max().unwrap_or(0);
        Range {
            low: min * HORIZON_YEARS,
            central: (self.average_annual() * HORIZON_YEARS as f64).round() as u64,
            high: max * HORIZON_YEARS,
        }
    }

    /// Permits per year that one unit per 100,000 residents represents here,
    /// for converting per-capita rates reported by studies.
    pub fn per_100k(&self) -> f64 {
        self.population as f64 / 100_000.0
    }

    /// Plain-language profile given to the experts and shown to users.
    pub fn profile(&self) -> String {
        let b = self.baseline();
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
        format!(
            "STATE PROFILE {name} ({abbr})\n\
             Population (2025 estimate): {pop}\n\
             Housing units authorized by permit, by year: {years}\n\
             Average per year {first}-{last}: {avg}; per 100,000 residents per year: {per100k:.0}\n\
             {last} mix of NEW permits by building size (not the existing housing stock): 1 unit {s1:.0}%, 2 units {s2:.0}%, 3-4 units {s3:.0}%, 5+ units {s5:.0}%\n\
             FIVE-YEAR BASELINE (current rules, recent pace continues): central {central} units (range {low} to {high})\n\
             Rule: average annual permits x 5; range = slowest and fastest year x 5. Outcome = units authorized by permit, not completions.\n\
             Conversion: a rate of 1 unit per 100,000 residents per year = {conv:.0} units per year statewide (use the state.scale_rate tool rather than doing this by hand).\n\
             Source: Census Building Permits Survey and Population Estimates (public domain); provenance {PROVENANCE}",
            name = self.name,
            abbr = self.abbr,
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
            central = thousands(b.central),
            low = thousands(b.low),
            high = thousands(b.high),
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

fn parse() -> Vec<StateBaseline> {
    let mut lines = CSV.lines();
    let header: Vec<&str> = lines.next().expect("header").split(',').collect();
    let years: Vec<(usize, u32)> = header
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            h.strip_prefix("permits_")
                .and_then(|y| y.parse().ok())
                .map(|y| (i, y))
        })
        .collect();
    let col = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .unwrap_or_else(|| panic!("missing column {name}"))
    };
    let (c1, c2, c3, c5) = (
        col("permits_2025_1unit"),
        col("permits_2025_2unit"),
        col("permits_2025_3to4unit"),
        col("permits_2025_5plus"),
    );
    lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            let n = |i: usize| {
                f[i].trim()
                    .parse::<u64>()
                    .unwrap_or_else(|_| panic!("bad number in {l}"))
            };
            StateBaseline {
                fips: f[0].to_string(),
                abbr: f[1].to_string(),
                name: f[2].to_string(),
                population: n(3),
                permits: years.iter().map(|(i, y)| (*y, n(*i))).collect(),
                latest_by_type: [n(c1), n(c2), n(c3), n(c5)],
            }
        })
        .collect()
}

pub fn all() -> &'static [StateBaseline] {
    static STATES: OnceLock<Vec<StateBaseline>> = OnceLock::new();
    STATES.get_or_init(parse)
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
            .all(|s| s.permits.len() == 5 && s.population > 0));
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
    }

    #[test]
    fn profile_states_outcome_rule_and_provenance() {
        let p = find("PA").unwrap().profile();
        assert!(p.contains("FIVE-YEAR BASELINE"));
        assert!(p.contains("units authorized by permit"));
        assert!(p.contains(PROVENANCE));
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
