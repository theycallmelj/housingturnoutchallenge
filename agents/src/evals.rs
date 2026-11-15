//! Panel evals: run the orchestrator-plus-experts panel on fixed questions
//! and score each answer against what the challenge rules and our own
//! safeguards require.
//!
//! The same suite runs in two modes:
//!
//! * offline (every CI run): deterministic, so every case must pass. It
//!   guards routing, structure, the state baseline and expected citations;
//! * live models (scheduled or on demand in CI, with an API key): output
//!   varies, so the suite is held to a pass-rate threshold instead.
//!
//! Checks, per case:
//!
//! | check | passes when |
//! | --- | --- |
//! | `routed` | every expected domain's expert was consulted |
//! | `grounded` | the answer isn't labeled UNGROUNDED |
//! | `sections` | it covers baseline, drivers, sensitivity and limits (Official Rules / handbook) |
//! | `state_baseline` | the state's five-year central baseline figure appears |
//! | `cites_evidence` | at least one expected study or dataset id appears |
//! | `figures_sourced` | the citation check found no unsourced figures or unknown ids |
//! | `no_known_errors` | none of the case's forbidden strings appear |

use crate::panel::{numbers, Panel, PanelAnswer, UNGROUNDED_LABEL};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Instant;

const BUNDLED_CASES: &str = include_str!("../evals/cases.json");

#[derive(Debug, Deserialize)]
pub struct Suite {
    pub cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
pub struct Case {
    pub id: String,
    pub question: String,
    pub state: String,
    pub expect_domains: Vec<String>,
    pub cite_any: Vec<String>,
    #[serde(default)]
    pub forbid: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct CaseResult {
    pub id: String,
    pub state: String,
    pub passed: bool,
    pub seconds: f64,
    pub checks: Vec<Check>,
    pub consulted: Vec<String>,
    pub answer: String,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub models: String,
    pub passed: usize,
    pub total: usize,
    pub pass_rate: f64,
    pub check_pass_rates: Vec<(String, f64)>,
    pub results: Vec<CaseResult>,
}

impl Suite {
    pub fn bundled() -> Result<Suite> {
        Ok(serde_json::from_str(BUNDLED_CASES)?)
    }

    pub fn load(path: &str) -> Result<Suite> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}

/// Score one answer. Pure, so it can be tested on recorded answers.
pub fn score(case: &Case, answer: &PanelAnswer) -> Vec<Check> {
    let text = &answer.text;
    let lower = text.to_lowercase();
    let mut checks = Vec::new();

    let missing: Vec<&String> = case
        .expect_domains
        .iter()
        .filter(|d| !answer.consulted.contains(d))
        .collect();
    checks.push(Check {
        name: "routed",
        passed: missing.is_empty(),
        detail: if missing.is_empty() {
            format!("consulted {}", answer.consulted.join(", "))
        } else {
            format!(
                "did not consult {:?} (consulted {:?})",
                missing, answer.consulted
            )
        },
    });

    let grounded = !text.starts_with(UNGROUNDED_LABEL) && !answer.consulted.is_empty();
    checks.push(Check {
        name: "grounded",
        passed: grounded,
        detail: if grounded {
            "answer built from expert findings".into()
        } else {
            "no expert consulted".into()
        },
    });

    let sections = [
        ("baseline", "baseline"),
        ("drivers", "drive"),
        ("sensitivity", "sensitiv"),
        ("limits", "cannot"),
    ];
    let absent: Vec<&str> = sections
        .iter()
        .filter(|(_, k)| !lower.contains(k))
        .map(|(n, _)| *n)
        .collect();
    checks.push(Check {
        name: "sections",
        passed: absent.is_empty(),
        detail: if absent.is_empty() {
            "all four required sections".into()
        } else {
            format!("missing {absent:?}")
        },
    });

    let state = crate::state::find(&case.state);
    let (has_baseline, baseline_detail) = match state {
        Some(s) => {
            let central = s.baseline().central.to_string();
            (
                numbers(text).contains(&central),
                format!(
                    "{} central baseline {}",
                    s.abbr,
                    crate::state::thousands(s.baseline().central)
                ),
            )
        }
        None => (false, format!("unknown state {}", case.state)),
    };
    checks.push(Check {
        name: "state_baseline",
        passed: has_baseline,
        detail: baseline_detail,
    });

    let cited: Vec<&String> = case
        .cite_any
        .iter()
        .filter(|id| text.contains(id.as_str()))
        .collect();
    checks.push(Check {
        name: "cites_evidence",
        passed: !cited.is_empty(),
        detail: if cited.is_empty() {
            format!("none of {:?}", case.cite_any)
        } else {
            format!("cites {cited:?}")
        },
    });

    checks.push(Check {
        name: "figures_sourced",
        passed: answer.citation_issues.is_empty(),
        detail: if answer.citation_issues.is_empty() {
            "no unsourced figures or unknown ids".into()
        } else {
            answer.citation_issues.join(" | ")
        },
    });

    let hits: Vec<&String> = case
        .forbid
        .iter()
        .filter(|f| lower.contains(&f.to_lowercase()))
        .collect();
    checks.push(Check {
        name: "no_known_errors",
        passed: hits.is_empty(),
        detail: if hits.is_empty() {
            "none".into()
        } else {
            format!("contains {hits:?}")
        },
    });

    checks
}

/// Run every case through the panel, one at a time.
pub async fn run(panel: &Panel, suite: &Suite, filter: Option<&str>) -> Result<Report> {
    let cases: Vec<&Case> = suite
        .cases
        .iter()
        .filter(|c| filter.is_none_or(|f| c.id.contains(f)))
        .collect();
    if cases.is_empty() {
        return Err(anyhow!("no eval cases match the filter"));
    }
    let mut results = Vec::new();
    for case in cases {
        let start = Instant::now();
        let outcome = panel.ask_for_state(&case.question, Some(&case.state)).await;
        let seconds = start.elapsed().as_secs_f64();
        let result = match outcome {
            Ok(answer) => {
                let checks = score(case, &answer);
                CaseResult {
                    id: case.id.clone(),
                    state: case.state.clone(),
                    passed: checks.iter().all(|c| c.passed),
                    seconds,
                    checks,
                    consulted: answer.consulted,
                    answer: answer.text,
                }
            }
            Err(e) => CaseResult {
                id: case.id.clone(),
                state: case.state.clone(),
                passed: false,
                seconds,
                checks: vec![Check {
                    name: "ran",
                    passed: false,
                    detail: e.to_string(),
                }],
                consulted: vec![],
                answer: String::new(),
            },
        };
        results.push(result);
    }

    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let mut names: Vec<&'static str> = Vec::new();
    for r in &results {
        for c in &r.checks {
            if !names.contains(&c.name) {
                names.push(c.name);
            }
        }
    }
    let check_pass_rates = names
        .into_iter()
        .map(|n| {
            let all: Vec<&Check> = results
                .iter()
                .flat_map(|r| r.checks.iter())
                .filter(|c| c.name == n)
                .collect();
            (
                n.to_string(),
                all.iter().filter(|c| c.passed).count() as f64 / all.len() as f64,
            )
        })
        .collect();
    Ok(Report {
        models: panel.plan().describe(),
        passed,
        total,
        pass_rate: passed as f64 / total as f64,
        check_pass_rates,
        results,
    })
}

impl Report {
    /// Markdown summary for CI job summaries and PR review.
    pub fn markdown(&self, min_pass_rate: f64) -> String {
        let verdict = if self.pass_rate >= min_pass_rate {
            "PASS"
        } else {
            "FAIL"
        };
        let mut out = format!(
            "## Panel evals: {verdict}\n\n{} of {} cases passed ({:.0}%; threshold {:.0}%).\n\nModels: {}\n\n",
            self.passed,
            self.total,
            self.pass_rate * 100.0,
            min_pass_rate * 100.0,
            self.models
        );
        out.push_str("| Check | Pass rate |\n| --- | --- |\n");
        for (name, rate) in &self.check_pass_rates {
            out.push_str(&format!("| `{name}` | {:.0}% |\n", rate * 100.0));
        }
        out.push_str("\n| Case | State | Result | Seconds | Failed checks |\n| --- | --- | --- | --- | --- |\n");
        for r in &self.results {
            let failed: Vec<String> = r
                .checks
                .iter()
                .filter(|c| !c.passed)
                .map(|c| format!("`{}`: {}", c.name, c.detail.replace('|', "/")))
                .collect();
            out.push_str(&format!(
                "| {} | {} | {} | {:.1} | {} |\n",
                r.id,
                r.state,
                if r.passed { "pass" } else { "**fail**" },
                r.seconds,
                if failed.is_empty() {
                    "".into()
                } else {
                    failed.join("<br>")
                }
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::Ontology;

    fn answer(text: &str, consulted: &[&str], issues: &[&str]) -> PanelAnswer {
        PanelAnswer {
            text: text.into(),
            consulted: consulted.iter().map(|s| s.to_string()).collect(),
            audit: vec![],
            models: "test".into(),
            citation_issues: issues.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn case() -> Case {
        Suite::bundled()
            .unwrap()
            .cases
            .into_iter()
            .find(|c| c.id == "adu-pennsylvania")
            .unwrap()
    }

    #[test]
    fn cases_reference_real_states_domains_and_ids() {
        let o = Ontology::bundled();
        for c in Suite::bundled().unwrap().cases {
            assert!(
                crate::state::find(&c.state).is_some(),
                "{}: state {}",
                c.id,
                c.state
            );
            for d in &c.expect_domains {
                assert!(o.domain(d).is_some(), "{}: domain {d}", c.id);
            }
            for id in &c.cite_any {
                assert!(
                    o.study(id).is_some() || o.dataset(id).is_some(),
                    "{}: id {id}",
                    c.id
                );
            }
        }
    }

    #[test]
    fn score_fails_each_check_on_the_matching_defect() {
        let s = crate::state::find("PA").unwrap();
        let base = crate::state::thousands(s.baseline().central);
        let good = format!(
            "Five-year baseline {base}. pew_2026_adu: 81. What drives it: uptake. Most sensitive to the rate. This cannot tell you completions."
        );
        assert!(score(&case(), &answer(&good, &["adu"], &[]))
            .iter()
            .all(|c| c.passed));

        let failed = |text: &str, consulted: &[&str], issues: &[&str]| -> Vec<&'static str> {
            score(&case(), &answer(text, consulted, issues))
                .into_iter()
                .filter(|c| !c.passed)
                .map(|c| c.name)
                .collect()
        };
        assert_eq!(failed(&good, &["fees"], &[]), vec!["routed"]);
        assert_eq!(
            failed(&good, &["adu"], &["figure 52735 ..."]),
            vec!["figures_sourced"]
        );
        assert_eq!(
            failed(&good.replace("cannot", "can't"), &["adu"], &[]),
            vec!["sections"]
        );
        assert_eq!(
            failed(&good.replace(&base, "lots"), &["adu"], &[]),
            vec!["state_baseline"]
        );
        assert_eq!(
            failed(&good.replace("pew_2026_adu", "Pew"), &["adu"], &[]),
            vec!["cites_evidence"]
        );
        assert_eq!(
            failed(&format!("{good} about 23 per 100 000"), &["adu"], &[]),
            vec!["no_known_errors"]
        );
        assert!(failed(&format!("{UNGROUNDED_LABEL}\n\n{good}"), &[], &[]).contains(&"grounded"));
    }

    #[tokio::test]
    async fn offline_panel_passes_every_case() {
        let panel = Panel::offline(Ontology::bundled());
        let report = run(&panel, &Suite::bundled().unwrap(), None).await.unwrap();
        let failures: Vec<String> = report
            .results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| {
                format!(
                    "{}: {:?}",
                    r.id,
                    r.checks
                        .iter()
                        .filter(|c| !c.passed)
                        .map(|c| &c.detail)
                        .collect::<Vec<_>>()
                )
            })
            .collect();
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(report.markdown(1.0).contains("PASS"));
    }
}
