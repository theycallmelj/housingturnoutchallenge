//! The scenario model: statewide five-year effects of the five benchmark
//! reforms, relative to the current-rules baseline.
//!
//! Mechanics live here; every number lives in `assumptions/scenarios.json`,
//! with its low/central/high values, sources and rationale. Each scenario is
//! a product of factors applied to state data (Census permits and
//! population):
//!
//! ```text
//! ADU:          rate per 100k x population/100k x (1 - legalization) x implementation x ramp[y]
//! uplift ones:  baseline annual permits x factors... x implementation x ramp[y]
//! ```
//!
//! The low estimate uses every low_case value together, the high estimate
//! every high_case value. Sensitivity swings one assumption at a time
//! between its low and high case with the rest at central.

use crate::state::{thousands, StateBaseline, HORIZON_YEARS};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const ASSUMPTIONS: &str = include_str!("../assumptions/scenarios.json");

#[derive(Debug, Deserialize)]
pub struct Assumptions {
    pub meta: Meta,
    pub shared: Shared,
    pub scenarios: Vec<ScenarioSpec>,
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    pub outcome: String,
    pub horizon_years: u64,
    pub how_to_read: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct Shared {
    pub implementation_share: Param,
    pub ramp: RampParam,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Param {
    pub label: String,
    pub unit: String,
    pub low_case: f64,
    pub central: f64,
    pub high_case: f64,
    pub basis: String,
    pub sources: Vec<String>,
    pub rationale: String,
    /// Enters the formula as `(1 - value)` rather than `value`.
    #[serde(default)]
    pub subtract: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RampParam {
    pub label: String,
    pub unit: String,
    pub low_case: Vec<f64>,
    pub central: Vec<f64>,
    pub high_case: Vec<f64>,
    pub basis: String,
    pub sources: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Deserialize)]
pub struct ScenarioSpec {
    pub id: String,
    pub label: String,
    pub benchmark: String,
    /// `population_per_100k` or `baseline_annual_permits`.
    pub base: String,
    pub uses_implementation: bool,
    pub formula: String,
    pub params: BTreeMap<String, Param>,
    pub data_gaps: Vec<String>,
}

/// Which value of each assumption to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Case {
    Low,
    Central,
    High,
}

impl Param {
    pub fn value(&self, case: Case) -> f64 {
        match case {
            Case::Low => self.low_case,
            Case::Central => self.central,
            Case::High => self.high_case,
        }
    }

    /// Shares print as percentages, which is how a policymaker reads them
    /// (and how models restate them, so the citation check sees the same figure).
    pub fn display(&self, v: f64) -> String {
        if self.unit.starts_with("share") {
            pct(v)
        } else {
            format!("{v}")
        }
    }

    fn display_unit(&self) -> &str {
        if self.unit.starts_with("share") {
            "(percent)"
        } else {
            &self.unit
        }
    }

    /// The factor this assumption contributes to the product.
    fn factor(&self, case: Case) -> f64 {
        let v = self.value(case);
        if self.subtract {
            1.0 - v
        } else {
            v
        }
    }
}

impl RampParam {
    pub fn value(&self, case: Case) -> &[f64] {
        match case {
            Case::Low => &self.low_case,
            Case::Central => &self.central,
            Case::High => &self.high_case,
        }
    }
}

/// One assumption's effect on the result, others held at central.
#[derive(Debug, Clone, Serialize)]
pub struct Sensitivity {
    pub assumption: String,
    pub label: String,
    pub low: f64,
    pub high: f64,
}

/// A scenario's statewide result.
#[derive(Debug, Clone, Serialize)]
pub struct Estimate {
    pub scenario: String,
    pub label: String,
    pub state: String,
    pub baseline_low: u64,
    pub baseline_central: u64,
    pub baseline_high: u64,
    /// Additional units authorized over five years.
    pub low: f64,
    pub central: f64,
    pub high: f64,
    /// Central additional units by year.
    pub central_by_year: Vec<f64>,
    pub sensitivity: Vec<Sensitivity>,
}

impl Assumptions {
    pub fn bundled() -> &'static Assumptions {
        static A: OnceLock<Assumptions> = OnceLock::new();
        A.get_or_init(|| serde_json::from_str(ASSUMPTIONS).expect("bundled assumptions must parse"))
    }

    pub fn scenario(&self, id: &str) -> Option<&ScenarioSpec> {
        self.scenarios.iter().find(|s| s.id == id)
    }

    /// Every adjustable assumption for a scenario, shared ones included.
    fn params_for<'a>(&'a self, spec: &'a ScenarioSpec) -> Vec<(&'a str, &'a Param)> {
        let mut out: Vec<(&str, &Param)> =
            spec.params.iter().map(|(k, p)| (k.as_str(), p)).collect();
        if spec.uses_implementation {
            out.push(("implementation_share", &self.shared.implementation_share));
        }
        out
    }

    /// Five-year additional units with each assumption at the given case
    /// (`cases` names the case per assumption; missing names use `default`),
    /// and the per-year path.
    fn compute(
        &self,
        spec: &ScenarioSpec,
        s: &StateBaseline,
        default: Case,
        cases: &BTreeMap<&str, Case>,
        ramp_case: Case,
    ) -> (f64, Vec<f64>) {
        let base = match spec.base.as_str() {
            "population_per_100k" => s.per_100k(),
            _ => s.average_annual(),
        };
        let annual: f64 = self
            .params_for(spec)
            .iter()
            .map(|(name, p)| p.factor(*cases.get(name).unwrap_or(&default)))
            .product::<f64>()
            * base;
        let by_year: Vec<f64> = self
            .shared
            .ramp
            .value(ramp_case)
            .iter()
            .map(|r| annual * r)
            .collect();
        (by_year.iter().sum(), by_year)
    }

    pub fn estimate(&self, scenario: &str, s: &StateBaseline) -> Result<Estimate> {
        let spec = self.scenario(scenario).ok_or_else(|| {
            anyhow!(
                "unknown scenario '{scenario}'; expected one of: {}",
                self.scenarios
                    .iter()
                    .map(|x| x.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
        let none = BTreeMap::new();
        let (low, _) = self.compute(spec, s, Case::Low, &none, Case::Low);
        let (central, central_by_year) = self.compute(spec, s, Case::Central, &none, Case::Central);
        let (high, _) = self.compute(spec, s, Case::High, &none, Case::High);

        let mut sensitivity: Vec<Sensitivity> = self
            .params_for(spec)
            .iter()
            .map(|(name, p)| {
                let at = |c: Case| {
                    self.compute(
                        spec,
                        s,
                        Case::Central,
                        &BTreeMap::from([(*name, c)]),
                        Case::Central,
                    )
                    .0
                };
                Sensitivity {
                    assumption: name.to_string(),
                    label: p.label.clone(),
                    low: at(Case::Low),
                    high: at(Case::High),
                }
            })
            .collect();
        let none = BTreeMap::new();
        sensitivity.push(Sensitivity {
            assumption: "ramp".into(),
            label: self.shared.ramp.label.clone(),
            low: self.compute(spec, s, Case::Central, &none, Case::Low).0,
            high: self.compute(spec, s, Case::Central, &none, Case::High).0,
        });
        sensitivity.sort_by(|a, b| (b.high - b.low).total_cmp(&(a.high - a.low)));

        let b = s.baseline();
        Ok(Estimate {
            scenario: spec.id.clone(),
            label: spec.label.clone(),
            state: s.abbr.clone(),
            baseline_low: b.low,
            baseline_central: b.central,
            baseline_high: b.high,
            low,
            central,
            high,
            central_by_year,
            sensitivity,
        })
    }

    /// Plain-language explanation of one estimate: result, formula with the
    /// central numbers filled in, sensitivity, assumptions and limits.
    pub fn explain(&self, e: &Estimate, s: &StateBaseline) -> String {
        let spec = self
            .scenario(&e.scenario)
            .expect("estimate comes from a known scenario");
        let pct = |x: f64| 100.0 * x / e.baseline_central as f64;
        let mut out = format!(
            "MODEL ESTIMATE {label}, {name} ({abbr})\n\
             Reform: {bench}\n\
             Additional units authorized by permit over {h} years: central {c} (range {lo} to {hi}); \
             {cp:.1}% of the five-year baseline of {base} (range {plo:.1}% to {phi:.1}%).\n\
             Central by year: {years}\n\
             Formula: {formula}\n",
            label = e.label,
            name = s.name,
            abbr = s.abbr,
            bench = spec.benchmark,
            h = HORIZON_YEARS,
            c = round10(e.central),
            lo = round10(e.low),
            hi = round10(e.high),
            cp = pct(e.central),
            base = thousands(e.baseline_central),
            plo = pct(e.low),
            phi = pct(e.high),
            years = e.central_by_year.iter().enumerate().map(|(i, v)| format!("y{} {}", i + 1, round10(*v))).collect::<Vec<_>>().join(", "),
            formula = spec.formula,
        );
        let mut sources: Vec<&str> = Vec::new();
        for (_, p) in self.params_for(spec) {
            for id in &p.sources {
                if !sources.contains(&id.as_str()) {
                    sources.push(id);
                }
            }
        }
        out.push_str(&format!(
            "Evidence behind this estimate (cite these ids): {}\n",
            sources
                .iter()
                .map(|id| format!("`{id}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        out.push_str("Most sensitive to (one assumption at a time, others central):\n");
        for x in &e.sensitivity {
            out.push_str(&format!(
                "  - {}: {} to {}\n",
                x.label,
                round10(x.low),
                round10(x.high)
            ));
        }
        out.push_str("Assumptions (low / central / high; basis; sources):\n");
        for (name, p) in self.params_for(spec) {
            out.push_str(&format!(
                "  - {name} = {} / {} / {} {} ({}; {}): {}\n",
                p.display(p.low_case),
                p.display(p.central),
                p.display(p.high_case),
                p.display_unit(),
                p.basis,
                if p.sources.is_empty() {
                    "no source yet".into()
                } else {
                    p.sources.join(", ")
                },
                p.rationale
            ));
        }
        let r = &self.shared.ramp;
        out.push_str(&format!(
            "  - ramp = {} / {} / {} ({}; {}): {}\n",
            pct_list(&r.low_case),
            pct_list(&r.central),
            pct_list(&r.high_case),
            r.basis,
            r.sources.join(", "),
            r.rationale
        ));
        out.push_str("Known gaps:\n");
        for g in &spec.data_gaps {
            out.push_str(&format!("  - {g}\n"));
        }
        out.push_str("The range combines all low-case or all high-case assumptions, so it is deliberately wide. It is an evidence-anchored estimate, not a forecast.");
        out
    }

    /// All five scenarios for one state, for side-by-side comparison.
    pub fn compare(&self, s: &StateBaseline) -> String {
        let b = s.baseline();
        let mut out = format!(
            "SCENARIO COMPARISON {name} ({abbr})\nFive-year baseline (current rules): {c} units authorized by permit (range {lo} to {hi}).\n\n\
             | Scenario | Additional units, 5 yrs (central) | Range | % of baseline (central) | Most sensitive to |\n| --- | --- | --- | --- | --- |\n",
            name = s.name,
            abbr = s.abbr,
            c = thousands(b.central),
            lo = thousands(b.low),
            hi = thousands(b.high),
        );
        for spec in &self.scenarios {
            if let Ok(e) = self.estimate(&spec.id, s) {
                out.push_str(&format!(
                    "| {} | {} | {} to {} | {:.1}% | {} |\n",
                    e.label,
                    round10(e.central),
                    round10(e.low),
                    round10(e.high),
                    100.0 * e.central / b.central as f64,
                    e.sensitivity.first().map_or("", |x| x.label.as_str())
                ));
            }
        }
        out.push_str("\nScenarios are estimated separately. Don't add them: they overlap (e.g. parking and transit reforms) and interact.");
        out
    }
}

fn pct(v: f64) -> String {
    let p = (v * 1000.0).round() / 10.0;
    if p.fract() == 0.0 {
        format!("{p:.0}%")
    } else {
        format!("{p}%")
    }
}

fn pct_list(v: &[f64]) -> String {
    format!(
        "[{}]",
        v.iter().map(|x| pct(*x)).collect::<Vec<_>>().join(", ")
    )
}

/// Round to the nearest 10 for display, to avoid false precision.
pub fn round10(x: f64) -> String {
    thousands(((x / 10.0).round() * 10.0).max(0.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::Ontology;
    use crate::state;

    fn a() -> &'static Assumptions {
        Assumptions::bundled()
    }

    #[test]
    fn covers_all_five_benchmark_scenarios() {
        let ids: Vec<&str> = a().scenarios.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["adu", "missing_middle", "tod", "parking", "fees"]);
        let o = Ontology::bundled();
        for id in ids {
            assert!(
                o.scenarios.iter().any(|s| s.id == id),
                "{id} not an ontology scenario"
            );
        }
    }

    #[test]
    fn every_source_is_in_the_ontology() {
        let o = Ontology::bundled();
        let mut all: Vec<&String> = a()
            .shared
            .implementation_share
            .sources
            .iter()
            .chain(&a().shared.ramp.sources)
            .collect();
        for s in &a().scenarios {
            for p in s.params.values() {
                all.extend(&p.sources);
            }
        }
        for id in all {
            assert!(o.study(id).is_some(), "assumption cites unknown study {id}");
        }
    }

    #[test]
    fn every_assumption_is_ordered_so_low_case_gives_the_smaller_effect() {
        let check = |name: &str, p: &Param| {
            let (l, c, h) = (
                p.factor(Case::Low),
                p.factor(Case::Central),
                p.factor(Case::High),
            );
            assert!(l <= c && c <= h, "{name}: factors {l} {c} {h} not ordered");
            assert!(p.low_case >= 0.0 && p.high_case >= 0.0, "{name} negative");
        };
        check("implementation_share", &a().shared.implementation_share);
        for s in &a().scenarios {
            for (n, p) in &s.params {
                check(n, p);
            }
        }
        let r = &a().shared.ramp;
        assert!(r.low_case.len() == 5 && r.central.len() == 5 && r.high_case.len() == 5);
        let sum = |v: &[f64]| v.iter().sum::<f64>();
        assert!(sum(&r.low_case) <= sum(&r.central) && sum(&r.central) <= sum(&r.high_case));
        assert!(r.high_case.iter().all(|x| (0.0..=1.0).contains(x)));
    }

    #[test]
    fn adu_central_matches_the_published_formula() {
        let s = state::find("PA").unwrap();
        let e = a().estimate("adu", s).unwrap();
        // 40 per 100k x pop/100k x (1 - 0.15) x 0.75 x (0.4+0.7+1+1+1)
        let expected = 40.0 * s.population as f64 / 100_000.0 * 0.85 * 0.75 * 4.1;
        assert!(
            (e.central - expected).abs() < 1e-6,
            "{} vs {expected}",
            e.central
        );
    }

    #[test]
    fn uplift_central_matches_the_published_formula() {
        let s = state::find("WI").unwrap();
        let e = a().estimate("fees", s).unwrap();
        // baseline annual x 0.02 x 4.1 (no implementation share for fees)
        let expected = s.average_annual() * 0.02 * 4.1;
        assert!((e.central - expected).abs() < 1e-6);
        let tod = a().estimate("tod", s).unwrap();
        assert!((tod.central - s.average_annual() * 0.12 * 0.10 * 0.75 * 4.1).abs() < 1e-6);
    }

    #[test]
    fn ranges_are_ordered_and_sensitivities_bracket_central() {
        for s in state::all() {
            for spec in &a().scenarios {
                let e = a().estimate(&spec.id, s).unwrap();
                assert!(
                    e.low <= e.central && e.central <= e.high,
                    "{} {}",
                    s.abbr,
                    spec.id
                );
                assert!((e.central_by_year.iter().sum::<f64>() - e.central).abs() < 1e-6);
                for x in &e.sensitivity {
                    assert!(
                        x.low <= e.central + 1e-6 && e.central <= x.high + 1e-6,
                        "{} {} {}",
                        s.abbr,
                        spec.id,
                        x.assumption
                    );
                    assert!(e.low <= x.low + 1e-6 && x.high <= e.high + 1e-6);
                }
                let swings: Vec<f64> = e.sensitivity.iter().map(|x| x.high - x.low).collect();
                assert!(
                    swings.windows(2).all(|w| w[0] >= w[1]),
                    "sensitivity not sorted"
                );
            }
        }
    }

    #[test]
    fn effects_scale_with_the_state() {
        let ca = a()
            .estimate("missing_middle", state::find("CA").unwrap())
            .unwrap();
        let wy = a()
            .estimate("missing_middle", state::find("WY").unwrap())
            .unwrap();
        assert!(ca.central > wy.central);
        // Uplift scenarios are a fixed share of each state's own baseline.
        let share = |e: &Estimate| e.central / e.baseline_central as f64;
        assert!((share(&ca) - share(&wy)).abs() < 1e-9);
    }

    #[test]
    fn explanation_shows_formula_sources_gaps_and_caveat() {
        let s = state::find("MA").unwrap();
        let e = a().estimate("tod", s).unwrap();
        let text = a().explain(&e, s);
        for needle in [
            "MODEL ESTIMATE",
            "Formula:",
            "Most sensitive to",
            "freemark_2019",
            "Known gaps:",
            "not a forecast",
            &round10(e.central),
        ] {
            assert!(text.contains(needle), "missing {needle}");
        }
        assert!(a().compare(s).contains("Don't add them"));
        assert!(a().estimate("nope", s).is_err());
    }

    #[test]
    fn shares_print_as_percentages() {
        assert_eq!(pct(0.75), "75%");
        assert_eq!(pct(0.005), "0.5%");
        assert_eq!(pct_list(&[0.4, 1.0]), "[40%, 100%]");
        let s = state::find("PA").unwrap();
        let text = a().explain(&a().estimate("adu", s).unwrap(), s);
        assert!(
            text.contains("implementation_share = 50% / 75% / 100%"),
            "{text}"
        );
        assert!(text.contains("adu_rate_per_100k = 26 / 40 / 81"));
    }

    #[test]
    fn round10_avoids_false_precision() {
        assert_eq!(round10(18_265.4), "18,270");
        assert_eq!(round10(4.0), "0");
        assert_eq!(round10(-3.0), "0");
    }
}
