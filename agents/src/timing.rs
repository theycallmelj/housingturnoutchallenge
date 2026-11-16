//! Permits -> construction starts -> completed homes, year by year.
//!
//! Curves come from `data/derived/construction_curves.csv`, built by
//! `scripts/fetch_construction_timing.py` from the Census Survey of
//! Construction (provenance: `research/data/census-survey-of-construction.md`).
//! For each Census region and building type, a curve gives the share of one
//! year's permits that start, and that are completed, in the issue year
//! (offset 1), the next year (offset 2), and so on to offset 6. The rest
//! starts or finishes later, or never starts.
//!
//! [`flow`] applies the curves to a five-year path of permits, plus the
//! permits issued in the years before the horizon (the pipeline), giving
//! starts and completions in each horizon year.

use std::collections::HashMap;
use std::sync::OnceLock;

const CURVES_CSV: &str = include_str!("../../data/derived/construction_curves.csv");
pub const PROVENANCE: &str = "research/data/census-survey-of-construction.md";
/// Year offsets the curves cover (1 = the year the permit is issued).
pub const OFFSETS: usize = 6;

/// Building types with their own timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BType {
    OneUnit,
    TwoToFour,
    FivePlus,
}

impl BType {
    pub const ALL: [BType; 3] = [BType::OneUnit, BType::TwoToFour, BType::FivePlus];
    fn key(self) -> &'static str {
        match self {
            BType::OneUnit => "1unit",
            BType::TwoToFour => "2to4",
            BType::FivePlus => "5plus",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            BType::OneUnit => "1-unit buildings",
            BType::TwoToFour => "2-4 unit buildings",
            BType::FivePlus => "5+ unit buildings",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Curve {
    /// Share of permitted units that ever start construction.
    pub start_rate: f64,
    /// Share started in year offset 1..=6 after issue.
    pub started: [f64; OFFSETS],
    /// Share completed in year offset 1..=6 after issue.
    pub completed: [f64; OFFSETS],
    /// Share completed after offset 6.
    pub completed_after: f64,
    pub months_to_start: f64,
    pub months_to_complete: f64,
}

impl Curve {
    /// Share of a cohort completed within `years` year offsets.
    pub fn completed_within(&self, years: usize) -> f64 {
        self.completed.iter().take(years).sum()
    }
}

fn table() -> &'static HashMap<(String, &'static str, String), Curve> {
    static T: OnceLock<HashMap<(String, &'static str, String), Curve>> = OnceLock::new();
    T.get_or_init(|| {
        let mut lines = CURVES_CSV.lines().filter(|l| !l.trim().is_empty());
        let head: Vec<&str> = lines.next().expect("header").split(',').collect();
        let col = |n: &str| {
            head.iter()
                .position(|h| *h == n)
                .unwrap_or_else(|| panic!("missing {n}"))
        };
        let mut out = HashMap::new();
        for line in lines {
            let r: Vec<&str> = line.split(',').collect();
            let f = |n: &str| {
                r[col(n)]
                    .parse::<f64>()
                    .unwrap_or_else(|_| panic!("bad {n}"))
            };
            let btype = BType::ALL
                .into_iter()
                .find(|b| b.key() == r[col("building_type")])
                .expect("building type")
                .key();
            let mut started = [0.0; OFFSETS];
            let mut completed = [0.0; OFFSETS];
            for j in 0..OFFSETS {
                started[j] = f(&format!("started_y{}", j + 1));
                completed[j] = f(&format!("completed_y{}", j + 1));
            }
            out.insert(
                (
                    r[col("region")].to_string(),
                    btype,
                    r[col("variant")].to_string(),
                ),
                Curve {
                    start_rate: f("start_rate"),
                    started,
                    completed,
                    completed_after: f("completed_after"),
                    months_to_start: f("mean_months_permit_to_start"),
                    months_to_complete: f("mean_months_start_to_completion"),
                },
            );
        }
        out
    })
}

/// The curve for a region and building type, averaged over the 2015-2024
/// permit cohorts.
pub fn curve(region: &str, b: BType) -> &'static Curve {
    curve_variant(region, b, "all")
}

/// `variant` is `all` or `excl_covid` (drops the 2020-2021 cohorts).
pub fn curve_variant(region: &str, b: BType, variant: &str) -> &'static Curve {
    table()
        .get(&(region.to_string(), b.key(), variant.to_string()))
        .or_else(|| table().get(&("south".to_string(), b.key(), variant.to_string())))
        .expect("construction curve")
}

/// Permits, starts and completions in each horizon year.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct Flow {
    pub permits: Vec<f64>,
    pub starts: Vec<f64>,
    pub completions: Vec<f64>,
}

impl Flow {
    pub fn total_permits(&self) -> f64 {
        self.permits.iter().sum()
    }
    pub fn total_starts(&self) -> f64 {
        self.starts.iter().sum()
    }
    pub fn total_completions(&self) -> f64 {
        self.completions.iter().sum()
    }
    pub fn add(&mut self, o: &Flow) {
        let n = self.permits.len().max(o.permits.len());
        for v in [&mut self.permits, &mut self.starts, &mut self.completions] {
            v.resize(n, 0.0);
        }
        for i in 0..o.permits.len() {
            self.permits[i] += o.permits[i];
            self.starts[i] += o.starts[i];
            self.completions[i] += o.completions[i];
        }
    }
}

/// Starts and completions in horizon years 1..=n from `path[y]` permits
/// issued in horizon year y+1, plus `prior[k]` permits issued k+1 years
/// before the horizon (prior[0] = the year just before).
pub fn flow(region: &str, b: BType, path: &[f64], prior: &[f64]) -> Flow {
    let c = curve(region, b);
    let n = path.len();
    let mut starts = vec![0.0; n];
    let mut completions = vec![0.0; n];
    for (y, out_s) in starts.iter_mut().enumerate() {
        let out_c = &mut completions[y];
        // Permits issued in horizon year i (0-based) affect year y at offset y - i + 1.
        for (i, p) in path.iter().enumerate().take(y + 1) {
            let off = y - i;
            if off < OFFSETS {
                *out_s += p * c.started[off];
                *out_c += p * c.completed[off];
            }
        }
        for (k, p) in prior.iter().enumerate() {
            let off = y + k + 1;
            if off < OFFSETS {
                *out_s += p * c.started[off];
                *out_c += p * c.completed[off];
            }
        }
    }
    Flow {
        permits: path.to_vec(),
        starts,
        completions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_are_complete_and_ordered() {
        for r in ["northeast", "midwest", "south", "west"] {
            for b in BType::ALL {
                for v in ["all", "excl_covid"] {
                    let c = curve_variant(r, b, v);
                    let total = c.completed.iter().sum::<f64>() + c.completed_after;
                    assert!((total - c.start_rate).abs() < 1e-3, "{r} {b:?} {v}");
                    assert!(c.start_rate > 0.6 && c.start_rate <= 1.0);
                }
            }
            // Bigger buildings take longer.
            let one = curve(r, BType::OneUnit);
            let five = curve(r, BType::FivePlus);
            assert!(one.completed_within(2) > five.completed_within(2));
            assert!(one.months_to_complete < five.months_to_complete);
        }
    }

    #[test]
    fn flow_applies_curves_by_offset() {
        let c = curve("south", BType::OneUnit);
        // 100 permits in year 1 only.
        let f = flow("south", BType::OneUnit, &[100.0, 0.0, 0.0, 0.0, 0.0], &[]);
        for y in 0..5 {
            assert!((f.completions[y] - 100.0 * c.completed[y]).abs() < 1e-9);
            assert!((f.starts[y] - 100.0 * c.started[y]).abs() < 1e-9);
        }
        // A pipeline permit issued the year before lands at offset 2 in year 1.
        let p = flow("south", BType::OneUnit, &[0.0; 5], &[100.0]);
        assert!((p.completions[0] - 100.0 * c.completed[1]).abs() < 1e-9);
        assert_eq!(p.total_permits(), 0.0);
        // A steady pace with a full pipeline completes about start_rate x permits.
        let steady = flow("south", BType::OneUnit, &[100.0; 5], &[100.0; 5]);
        assert!((steady.completions[4] - 100.0 * c.completed_within(6)).abs() < 1e-6);
    }
}
