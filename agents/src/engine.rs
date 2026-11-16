//! The scenario model: statewide five-year effects of the five benchmark
//! reforms, relative to the current-rules baseline.
//!
//! Mechanics live here; every number lives in `assumptions/scenarios.json`,
//! with its low/central/high values, sources and rationale. Each scenario is
//! computed **per county and summed to the state**:
//!
//! ```text
//! county base[y] = population/100k (ADU), or the county's baseline permit
//!                  forecast for year y (others; see crate::forecast)
//! county weight  = how much of the county's housing the reform touches vs.
//!                  the U.S. (single-family detached share for ADU and missing
//!                  middle; 5+-unit share for TOD and parking; 1 for fees),
//!                  capped to a bounded range
//! added[y]       = sum over counties (base[y] x weight) x the scenario's
//!                  factors x implementation share x ramp[y]
//! starts, completions = added permits through the Survey of Construction
//!                  curves for the reform's building type (crate::timing)
//! ```
//!
//! Because the base follows the baseline forecast, changing economic
//! conditions ([`Conditions`]) moves both the baseline and the reforms' effects.
//!
//! The low estimate uses every low_case value together, the high estimate
//! every high_case value. Sensitivity swings one assumption at a time
//! between its low and high case with the rest at central.

use crate::forecast::Conditions;
use crate::state::{thousands, StateBaseline, HORIZON_YEARS};
use crate::timing::{BType, Flow};
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
    pub exposure: ExposureRule,
}

/// Bounds on a county's weight, so one unusual county can't dominate.
#[derive(Debug, Deserialize)]
pub struct ExposureRule {
    pub min: f64,
    pub max: f64,
    pub rationale: String,
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
    /// Which county housing share weights the scenario: `sf_detached`,
    /// `multifamily` or `none`.
    #[serde(default = "none")]
    pub exposure: String,
    /// Building type for permit-to-completion timing: `1unit`, `2to4`,
    /// `5plus` or `state_mix` (the state's forecast permit mix).
    #[serde(default = "state_mix")]
    pub building_type: String,
    pub formula: String,
    pub params: BTreeMap<String, Param>,
    pub data_gaps: Vec<String>,
}

fn none() -> String {
    "none".into()
}

fn state_mix() -> String {
    "state_mix".into()
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
    /// Added homes completed within the horizon (low/central/high).
    pub completed_low: f64,
    pub completed_central: f64,
    pub completed_high: f64,
    /// Baseline homes completed within the horizon (central).
    pub baseline_completed_central: u64,
    /// The state's base-weighted average county weight (1 = like the U.S.).
    pub exposure_factor: f64,
    /// Counties contributing most to the central estimate.
    pub top_counties: Vec<(String, f64)>,
    /// Conditions the baseline and effects assume.
    pub conditions: Conditions,
    /// Central added permits, construction starts and completions by year.
    pub added: Flow,
    /// Central baseline permits, starts and completions by year (completions
    /// include homes permitted before the horizon).
    pub baseline: Flow,
    /// First horizon year (e.g. 2026).
    pub first_year: u32,
}

/// The state's county-weighted base for one scenario.
struct Base {
    /// Sum over counties of base[y] x weight, per horizon year.
    effective: Vec<f64>,
    /// effective / unweighted base: the state's net weight.
    factor: f64,
    /// (county name, share of `effective`), largest first.
    by_county: Vec<(String, f64)>,
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

    /// A county's weight for a scenario: its housing share relative to the
    /// U.S., bounded. Counties without ACS data use their state's share.
    pub fn county_weight(
        &self,
        spec: &ScenarioSpec,
        c: &crate::state::County,
        s: &StateBaseline,
    ) -> f64 {
        let nat = crate::state::national_stock();
        let (share, national) = match spec.exposure.as_str() {
            "sf_detached" => (
                c.stock
                    .and_then(|k| k.sf_detached_share())
                    .or(s.stock.sf_detached_share()),
                nat.sf_detached_share(),
            ),
            "multifamily" => (
                c.stock
                    .and_then(|k| k.multifamily_share())
                    .or(s.stock.multifamily_share()),
                nat.multifamily_share(),
            ),
            _ => return 1.0,
        };
        match (share, national) {
            (Some(v), Some(n)) if n > 0.0 => {
                (v / n).clamp(self.shared.exposure.min, self.shared.exposure.max)
            }
            _ => 1.0,
        }
    }

    /// Sum the scenario's base over the state's counties, weighted, per
    /// horizon year. `forecasts` are the county baseline forecasts.
    fn base(
        &self,
        spec: &ScenarioSpec,
        s: &StateBaseline,
        forecasts: &std::collections::HashMap<String, Vec<[f64; 3]>>,
    ) -> Base {
        let h = HORIZON_YEARS as usize;
        let mut raw = 0.0;
        let mut effective = vec![0.0; h];
        let mut by_county = Vec::new();
        for c in s.counties() {
            // Per-capita rates only count residents of permit-issuing counties:
            // the outcome is permits, and homes built elsewhere aren't counted.
            let path: Vec<f64> = match spec.base.as_str() {
                "population_per_100k" if c.permit_issuing => {
                    vec![c.population.unwrap_or(0) as f64 / 100_000.0; h]
                }
                "population_per_100k" => vec![0.0; h],
                _ => forecasts
                    .get(&c.key())
                    .map(|v| v.iter().map(|y| y.iter().sum()).collect())
                    .unwrap_or_else(|| vec![0.0; h]),
            };
            let total: f64 = path.iter().sum();
            if total <= 0.0 {
                continue;
            }
            let w = self.county_weight(spec, c, s);
            raw += total;
            for (e, b) in effective.iter_mut().zip(&path) {
                *e += b * w;
            }
            by_county.push((c.name.clone(), total * w));
        }
        let eff_total: f64 = effective.iter().sum();
        by_county.sort_by(|a, b| b.1.total_cmp(&a.1));
        for x in &mut by_county {
            x.1 /= eff_total.max(f64::MIN_POSITIVE);
        }
        Base {
            factor: if raw > 0.0 { eff_total / raw } else { 1.0 },
            effective,
            by_county,
        }
    }

    /// Five-year additional units with each assumption at the given case
    /// (`cases` names the case per assumption; missing names use `default`),
    /// and the per-year path.
    fn compute(
        &self,
        spec: &ScenarioSpec,
        base: &[f64],
        default: Case,
        cases: &BTreeMap<&str, Case>,
        ramp_case: Case,
    ) -> (f64, Vec<f64>) {
        let factor: f64 = self
            .params_for(spec)
            .iter()
            .map(|(name, p)| p.factor(*cases.get(name).unwrap_or(&default)))
            .product::<f64>();
        let by_year: Vec<f64> = self
            .shared
            .ramp
            .value(ramp_case)
            .iter()
            .zip(base)
            .map(|(r, b)| factor * b * r)
            .collect();
        (by_year.iter().sum(), by_year)
    }

    /// Starts and completions for a path of added permits, by the scenario's
    /// building type. Added permits have no pipeline before the horizon.
    fn added_flow(
        &self,
        spec: &ScenarioSpec,
        s: &StateBaseline,
        path: &[f64],
        mix: [f64; 3],
    ) -> Flow {
        let shares = match spec.building_type.as_str() {
            "1unit" => [1.0, 0.0, 0.0],
            "2to4" => [0.0, 1.0, 0.0],
            "5plus" => [0.0, 0.0, 1.0],
            _ => mix,
        };
        let mut out = Flow::default();
        for (k, b) in BType::ALL.into_iter().enumerate() {
            if shares[k] > 0.0 {
                let p: Vec<f64> = path.iter().map(|x| x * shares[k]).collect();
                out.add(&crate::timing::flow(&s.region, b, &p, &[]));
            }
        }
        out
    }

    pub fn estimate(&self, scenario: &str, s: &StateBaseline) -> Result<Estimate> {
        self.estimate_with(scenario, s, &Conditions::default())
    }

    pub fn estimate_with(
        &self,
        scenario: &str,
        s: &StateBaseline,
        cond: &Conditions,
    ) -> Result<Estimate> {
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
        let forecasts = crate::forecast::forecasts_for(cond);
        let fc = s.forecast_from(&forecasts);
        let mix = fc.mix();
        let base = self.base(spec, s, &forecasts);
        let eb = &base.effective;
        let none = BTreeMap::new();
        let (low, low_path) = self.compute(spec, eb, Case::Low, &none, Case::Low);
        let (central, central_by_year) =
            self.compute(spec, eb, Case::Central, &none, Case::Central);
        let (high, high_path) = self.compute(spec, eb, Case::High, &none, Case::High);
        let done = |path: &[f64]| self.added_flow(spec, s, path, mix).total_completions();
        let added = self.added_flow(spec, s, &central_by_year, mix);

        let mut sensitivity: Vec<Sensitivity> = self
            .params_for(spec)
            .iter()
            .map(|(name, p)| {
                let at = |c: Case| {
                    self.compute(
                        spec,
                        eb,
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
            low: self.compute(spec, eb, Case::Central, &none, Case::Low).0,
            high: self.compute(spec, eb, Case::Central, &none, Case::High).0,
        });
        sensitivity.sort_by(|a, b| (b.high - b.low).total_cmp(&(a.high - a.low)));

        let b = s.baseline_with(cond);
        let baseline = s.flow_for(&fc);
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
            completed_low: done(&low_path),
            completed_central: added.total_completions(),
            completed_high: done(&high_path),
            baseline_completed_central: baseline.total_completions().round() as u64,
            central_by_year,
            sensitivity,
            exposure_factor: base.factor,
            top_counties: base
                .by_county
                .into_iter()
                .take(5)
                .map(|(n, sh)| (n, sh * central))
                .collect(),
            conditions: *cond,
            added,
            baseline,
            first_year: crate::forecast::LAST_YEAR + 1,
        })
    }

    /// Plain-language explanation of one estimate: result, formula with the
    /// central numbers filled in, sensitivity, assumptions and limits.
    pub fn explain(&self, e: &Estimate, s: &StateBaseline) -> String {
        let spec = self
            .scenario(&e.scenario)
            .expect("estimate comes from a known scenario");
        let pct = |x: f64| 100.0 * x / e.baseline_central as f64;
        let yr = |v: &[f64]| {
            v.iter()
                .enumerate()
                .map(|(i, x)| format!("{} {}", e.first_year + i as u32, round10(*x)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut out = format!(
            "MODEL ESTIMATE {label}, {name} ({abbr})\n\
             Reform: {bench}\n\
             Baseline {y1}-{y5} (current rules; county permit model; {cond}): {base} units permitted (range {blo} to {bhi}), \
             {bstarts} construction starts, {bc} homes completed.\n\
             Additional units authorized by permit over {h} years: central {c} (range {lo} to {hi}); \
             {cp:.1}% of the five-year baseline of {base} (range {plo:.1}% to {phi:.1}%).\n\
             Completed within {h} years: central {cc} (range {clo} to {chi}), vs. about {bc} baseline homes completed; \
             homes permitted late in the window finish after it, and some permits never start ({timing}).\n\
             Added permits by year (central): {years}\n\
             Added construction starts by year: {starts}\n\
             Added completions by year: {comps}\n\
             {weight}\n\
             Largest county contributions (central): {top}\n\
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
            years = yr(&e.central_by_year),
            starts = yr(&e.added.starts),
            comps = yr(&e.added.completions),
            y1 = e.first_year,
            y5 = e.first_year + HORIZON_YEARS as u32 - 1,
            cond = e.conditions.describe(),
            blo = thousands(e.baseline_low),
            bhi = thousands(e.baseline_high),
            bstarts = round10(e.baseline.total_starts()),
            timing = match spec.building_type.as_str() {
                "1unit" => "1-unit construction timing",
                "2to4" => "2-4 unit construction timing",
                "5plus" => "5+ unit construction timing",
                _ => "timing for the state's permit mix",
            },
            cc = round10(e.completed_central),
            clo = round10(e.completed_low),
            chi = round10(e.completed_high),
            bc = thousands(e.baseline_completed_central),
            weight = match spec.exposure.as_str() {
                "sf_detached" => format!("County weighting: single-family detached share of homes vs. the U.S.; {}'s counties net out to {:.2}x the U.S.", s.name, e.exposure_factor),
                "multifamily" => format!("County weighting: share of homes in 5+ unit buildings vs. the U.S.; {}'s counties net out to {:.2}x the U.S.", s.name, e.exposure_factor),
                _ => "County weighting: none (the benchmark applies the same fee cut everywhere).".to_string(),
            },
            top = e.top_counties.iter().map(|(n, v)| format!("{n} {}", round10(*v))).collect::<Vec<_>>().join(", "),
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

    /// The pieces of [`Self::explain`] as structured data, so a UI can lay
    /// them out for a non-technical reader instead of showing the raw text.
    pub fn details(&self, e: &Estimate) -> serde_json::Value {
        use serde_json::json;
        let spec = self
            .scenario(&e.scenario)
            .expect("estimate comes from a known scenario");
        let row = |name: &str, p: &Param| {
            json!({
                "name": name,
                "label": p.label,
                "low": p.display(p.low_case),
                "central": p.display(p.central),
                "high": p.display(p.high_case),
                "unit": if p.unit.starts_with("share") { "" } else { p.unit.as_str() },
                "basis": p.basis,
                "sources": p.sources,
                "rationale": p.rationale,
            })
        };
        let mut assumptions: Vec<serde_json::Value> = self
            .params_for(spec)
            .into_iter()
            .map(|(n, p)| row(n, p))
            .collect();
        let r = &self.shared.ramp;
        assumptions.push(json!({
            "name": "ramp",
            "label": r.label,
            "low": pct_list(&r.low_case),
            "central": pct_list(&r.central),
            "high": pct_list(&r.high_case),
            "unit": "",
            "basis": r.basis,
            "sources": r.sources,
            "rationale": r.rationale,
        }));
        let mut sources: Vec<&str> = Vec::new();
        for (_, p) in self.params_for(spec) {
            for id in &p.sources {
                if !sources.contains(&id.as_str()) {
                    sources.push(id);
                }
            }
        }
        json!({
            "reform": spec.benchmark,
            "formula": spec.formula,
            "building_type": match spec.building_type.as_str() {
                "1unit" => "single-family and other 1-unit buildings",
                "2to4" => "2-4 unit buildings",
                "5plus" => "apartment buildings with 5+ units",
                _ => "the state's usual mix of building types",
            },
            "weighting": match spec.exposure.as_str() {
                "sf_detached" => "Counties with more detached single-family homes get more of the effect, because that is where the reform applies.",
                "multifamily" => "Counties with more apartment buildings (5+ units) get more of the effect, because that is where this kind of development happens.",
                _ => "Every county gets the same proportional effect, because the fee cut applies everywhere.",
            },
            "exposure_factor": e.exposure_factor,
            "sensitivity": e.sensitivity.iter().map(|x| json!({"label": x.label, "low": x.low.round(), "high": x.high.round()})).collect::<Vec<_>>(),
            "assumptions": assumptions,
            "evidence": sources,
            "gaps": spec.data_gaps,
        })
    }

    /// All five scenarios for one state, for side-by-side comparison.
    pub fn compare(&self, s: &StateBaseline) -> String {
        self.compare_with(s, &Conditions::default())
    }

    pub fn compare_with(&self, s: &StateBaseline, cond: &Conditions) -> String {
        let b = s.baseline_with(cond);
        let flow = s.baseline_flow(cond);
        let mut out = format!(
            "SCENARIO COMPARISON {name} ({abbr})\nFive-year baseline (current rules; county permit model; {cd}): {c} units authorized by permit (range {lo} to {hi}); {done} homes completed.\n\n\
             | Scenario | Additional units permitted, 5 yrs (central) | Range | Completed within 5 yrs (central) | % of baseline (central) | Most sensitive to |\n| --- | --- | --- | --- | --- | --- |\n",
            name = s.name,
            abbr = s.abbr,
            c = thousands(b.central),
            lo = thousands(b.low),
            hi = thousands(b.high),
            cd = cond.describe(),
            done = round10(flow.total_completions()),
        );
        for spec in &self.scenarios {
            if let Ok(e) = self.estimate_with(&spec.id, s, cond) {
                out.push_str(&format!(
                    "| {} | {} | {} to {} | {} | {:.1}% | {} |\n",
                    e.label,
                    round10(e.central),
                    round10(e.low),
                    round10(e.high),
                    round10(e.completed_central),
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
        let spec = a().scenarios.iter().find(|x| x.id == "adu").unwrap();
        let e = a().estimate("adu", s).unwrap();
        // Sum over permit-issuing counties of pop/100k x weight, then
        // 40 per 100k x (1 - 0.15) x 0.75 x (0.4+0.7+1+1+1)
        let base: f64 = s
            .counties()
            .into_iter()
            .filter(|c| c.permit_issuing)
            .map(|c| c.population.unwrap_or(0) as f64 / 100_000.0 * a().county_weight(spec, c, s))
            .sum();
        let expected = 40.0 * base * 0.85 * 0.75 * 4.1;
        assert!(
            (e.central - expected).abs() < 1e-6,
            "{} vs {expected}",
            e.central
        );
    }

    #[test]
    fn uplift_central_matches_the_published_formula() {
        let s = state::find("WI").unwrap();
        let ramp = [0.4, 0.7, 1.0, 1.0, 1.0];
        let f = crate::forecast::default_forecasts();
        let path = s.forecast(&Conditions::default()).path();
        let e = a().estimate("fees", s).unwrap();
        // Fees are unweighted: baseline forecast[y] x 0.02 x ramp[y] (no implementation share)
        let expected: f64 = path.iter().zip(ramp).map(|(b, r)| b * 0.02 * r).sum();
        assert!(
            (e.central - expected).abs() < 1e-6,
            "{} vs {expected}",
            e.central
        );
        assert!((e.exposure_factor - 1.0).abs() < 1e-12);
        let spec = a().scenarios.iter().find(|x| x.id == "tod").unwrap();
        let mut base = [0.0; 5];
        for c in s.counties() {
            if let Some(v) = f.get(&c.key()) {
                for y in 0..5 {
                    base[y] += v[y].iter().sum::<f64>() * a().county_weight(spec, c, s);
                }
            }
        }
        let tod = a().estimate("tod", s).unwrap();
        let expected: f64 = base
            .iter()
            .zip(ramp)
            .map(|(b, r)| b * 0.12 * 0.10 * 0.75 * r)
            .sum();
        assert!((tod.central - expected).abs() < 1e-6);
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
        // Uplift scenarios are a share of each state's baseline forecast
        // scaled by its county weighting and the ramp, so after removing the
        // exposure factor the shares differ only through the forecast's shape.
        let share = |e: &Estimate| e.central / e.baseline_central as f64 / e.exposure_factor;
        assert!(
            (share(&ca) / share(&wy) - 1.0).abs() < 0.05,
            "{} {}",
            share(&ca),
            share(&wy)
        );
    }

    #[test]
    fn flows_report_permits_starts_and_completions_by_year() {
        let s = state::find("PA").unwrap();
        for spec in &a().scenarios {
            let e = a().estimate(&spec.id, s).unwrap();
            assert_eq!(e.added.permits.len(), 5);
            for (x, y) in e.added.permits.iter().zip(&e.central_by_year) {
                assert!((x - y).abs() < 1e-9);
            }
            assert!(e.added.total_starts() <= e.central + 1e-6);
            assert!(e.added.total_completions() <= e.added.total_starts() + 1e-6);
            assert!((e.added.total_completions() - e.completed_central).abs() < 1e-9);
            assert_eq!(e.first_year, 2026);
            // The baseline flow carries completions from earlier permits.
            assert!(e.baseline.completions[0] > 0.0);
        }
        let text = a().explain(&a().estimate("missing_middle", s).unwrap(), s);
        assert!(text.contains("Added construction starts by year"));
        assert!(text.contains("2-4 unit construction timing"));
        assert!(text.contains("county permit model"));
    }

    #[test]
    fn conditions_move_the_baseline_and_the_effects_together() {
        let s = state::find("TX").unwrap();
        let base = a().estimate("missing_middle", s).unwrap();
        let worse = Conditions {
            rate_shift_pp: 2.0,
            migration_factor: 0.0,
            price_shift_pp: -5.0,
        };
        let e = a().estimate_with("missing_middle", s, &worse).unwrap();
        assert_ne!(e.baseline_central, base.baseline_central);
        // Uplift effects are proportional to the baseline path, so they move
        // in the same direction as the baseline.
        assert_eq!(
            e.baseline_central < base.baseline_central,
            e.central < base.central
        );
        // ADUs scale with population, not the permit forecast.
        let adu = a().estimate("adu", s).unwrap();
        let adu2 = a().estimate_with("adu", s, &worse).unwrap();
        assert!((adu.central - adu2.central).abs() < 1e-9);
        assert!(a().explain(&e, s).contains("net migration at 0%"));
        assert!(a()
            .compare_with(s, &worse)
            .contains("10-year Treasury +2.00 points"));
    }

    #[test]
    fn county_weights_are_bounded_and_vary_by_housing_mix() {
        let ex = &a().shared.exposure;
        let mm = a()
            .scenarios
            .iter()
            .find(|x| x.id == "missing_middle")
            .unwrap();
        let tod = a().scenarios.iter().find(|x| x.id == "tod").unwrap();
        for s in state::all() {
            for c in s.counties() {
                for spec in [mm, tod] {
                    let w = a().county_weight(spec, c, s);
                    assert!((ex.min..=ex.max).contains(&w), "{} {} {w}", s.abbr, c.name);
                }
            }
        }
        // New York (dense, multifamily) vs. Idaho (single-family): TOD
        // weights tilt toward NY, missing middle toward ID.
        let ny = a().estimate("tod", state::find("NY").unwrap()).unwrap();
        let id = a().estimate("tod", state::find("ID").unwrap()).unwrap();
        assert!(ny.exposure_factor > 1.0 && id.exposure_factor < 1.0);
        let ny = a()
            .estimate("missing_middle", state::find("NY").unwrap())
            .unwrap();
        let id = a()
            .estimate("missing_middle", state::find("ID").unwrap())
            .unwrap();
        assert!(ny.exposure_factor < id.exposure_factor);
    }

    #[test]
    fn completions_lag_and_never_exceed_permits() {
        for s in state::all() {
            for spec in &a().scenarios {
                let e = a().estimate(&spec.id, s).unwrap();
                assert!(
                    e.completed_central > 0.0 && e.completed_central < e.central,
                    "{} {}",
                    s.abbr,
                    spec.id
                );
                assert!(
                    e.completed_low <= e.completed_central
                        && e.completed_central <= e.completed_high
                );
                assert!(e.completed_high <= e.high + 1e-9);
                let sum: f64 = e.top_counties.iter().map(|x| x.1).sum();
                assert!(sum <= e.central + 1e-6);
            }
        }
        // ADUs (1-unit, fast) finish a larger share than missing middle (2+).
        let s = state::find("PA").unwrap();
        let adu = a().estimate("adu", s).unwrap();
        let mm = a().estimate("missing_middle", s).unwrap();
        assert!(adu.completed_central / adu.central > mm.completed_central / mm.central);
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
