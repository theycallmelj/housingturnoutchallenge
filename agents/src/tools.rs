//! condesate `Tool`s over the ontology.
//!
//! Domain tools are named `<domain>.<verb>` (e.g. `adu.evidence_brief`) so an
//! expert's grants can be scoped with a prefix pattern: the ADU expert is
//! admitted with `Tool(Prefix("adu."))` and the kernel refuses anything else
//! before `call` runs. `ontology.get` is shared by every expert.

use crate::ontology::Ontology;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use condesate::{GuardedServices, Tool, ToolSpec};
use serde_json::Value;
use std::sync::Arc;

fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

/// The tools one domain expert may call.
pub fn domain_tools(ontology: &'static Ontology, domain: &str) -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(EvidenceBrief {
            ontology,
            domain: domain.to_string(),
        }),
        Arc::new(FindStudies {
            ontology,
            domain: domain.to_string(),
        }),
        Arc::new(FindDatasets {
            ontology,
            domain: domain.to_string(),
        }),
        Arc::new(GetEntity { ontology }),
        Arc::new(StateProfile),
        Arc::new(ScaleRate),
    ];
    // Scenario experts can run the model for their own scenario only (the
    // tool is named `<domain>.estimate`, inside their grant); the modeling
    // expert can compare all five.
    if crate::engine::Assumptions::bundled()
        .scenario(domain)
        .is_some()
    {
        tools.push(Arc::new(ScenarioEstimate {
            scenario: domain.to_string(),
        }));
    }
    if domain == "modeling" {
        tools.push(Arc::new(CompareScenarios));
    }
    tools
}

/// `state.scale_rate`: apply a per-100,000-residents rate to a state, with
/// the arithmetic shown, so models never do the conversion themselves.
pub struct ScaleRate;

#[async_trait]
impl Tool for ScaleRate {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "state.scale_rate".into(),
            description: "Convert a rate of {rate_per_100k} units per 100,000 residents per year into statewide units per year and over five years for {state}, with the arithmetic shown.".into(),
        }
    }
    async fn call(&self, args: Value, _svc: &GuardedServices) -> Result<String> {
        let q = str_arg(&args, "state").ok_or_else(|| anyhow!("missing string arg 'state'"))?;
        let s = crate::state::find(q).ok_or_else(|| anyhow!("unknown state '{q}'"))?;
        let rate = match args.get("rate_per_100k") {
            Some(Value::Number(n)) => n.as_f64(),
            Some(Value::String(v)) => v.trim().replace(',', "").parse().ok(),
            _ => None,
        }
        .filter(|r: &f64| r.is_finite() && *r >= 0.0)
        .ok_or_else(|| anyhow!("rate_per_100k must be a non-negative number"))?;
        Ok(crate::state::scale_rate(s, rate))
    }
}

/// `state.profile`: population, recent permits and the five-year
/// current-rules baseline for one state. Shared by every expert.
pub struct StateProfile;

#[async_trait]
impl Tool for StateProfile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "state.profile".into(),
            description: "Statewide population, housing units permitted per year 2021-2025, the five-year current-rules baseline forecast for {state} (USPS code or name) in permits, construction starts and completions, and permit-to-completion timing.".into(),
        }
    }
    async fn call(&self, args: Value, _svc: &GuardedServices) -> Result<String> {
        let q = str_arg(&args, "state").ok_or_else(|| anyhow!("missing string arg 'state'"))?;
        crate::state::find(q)
            .map(|s| s.profile())
            .ok_or_else(|| anyhow!("unknown state '{q}'; use a USPS code like CA or a full name"))
    }
}

/// The domain's scope, best-ranked studies, usable datasets and caveats.
pub struct EvidenceBrief {
    pub ontology: &'static Ontology,
    pub domain: String,
}

#[async_trait]
impl Tool for EvidenceBrief {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: format!("{}.evidence_brief", self.domain),
            description: format!(
                "Evidence brief for the {} domain relevant to {{query}}: scenario definitions, mechanisms, top studies with outcome and evidence type, and usable datasets.",
                self.domain
            ),
        }
    }
    async fn call(&self, args: Value, _svc: &GuardedServices) -> Result<String> {
        let query = str_arg(&args, "query").unwrap_or("");
        self.ontology
            .evidence_brief(&self.domain, query, 8)
            .ok_or_else(|| anyhow!("unknown domain {}", self.domain))
    }
}

/// Studies in this domain matching {query}.
pub struct FindStudies {
    pub ontology: &'static Ontology,
    pub domain: String,
}

#[async_trait]
impl Tool for FindStudies {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: format!("{}.find_studies", self.domain),
            description: format!(
                "Find {} studies matching {{query}}, optionally limited to {{scenario}}.",
                self.domain
            ),
        }
    }
    async fn call(&self, args: Value, _svc: &GuardedServices) -> Result<String> {
        let query = str_arg(&args, "query").unwrap_or("");
        let scenario = str_arg(&args, "scenario");
        let studies = self
            .ontology
            .find_studies(Some(&self.domain), scenario, query, 6);
        if studies.is_empty() {
            return Ok("no matching studies".into());
        }
        Ok(studies
            .into_iter()
            .map(|s| self.ontology.format_study(s))
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

/// Datasets in this domain, with license status and whether usable.
pub struct FindDatasets {
    pub ontology: &'static Ontology,
    pub domain: String,
}

#[async_trait]
impl Tool for FindDatasets {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: format!("{}.find_datasets", self.domain),
            description: format!(
                "List datasets for the {} domain with license status; set {{usable_only}} to true to keep only open_verified ones.",
                self.domain
            ),
        }
    }
    async fn call(&self, args: Value, _svc: &GuardedServices) -> Result<String> {
        let usable_only = args
            .get("usable_only")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let datasets = self
            .ontology
            .find_datasets(Some(&self.domain), None, usable_only);
        Ok(datasets
            .into_iter()
            .map(|d| self.ontology.format_dataset(d))
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

/// Any ontology entity (study, dataset, scenario, domain, mechanism, outcome) by {id}.
pub struct GetEntity {
    pub ontology: &'static Ontology,
}

#[async_trait]
impl Tool for GetEntity {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "ontology.get".into(),
            description: "Look up any ontology entity by {id}: study, dataset, scenario, domain, mechanism or outcome.".into(),
        }
    }
    async fn call(&self, args: Value, _svc: &GuardedServices) -> Result<String> {
        let id = str_arg(&args, "id").ok_or_else(|| anyhow!("missing string arg 'id'"))?;
        self.ontology
            .describe(id)
            .ok_or_else(|| anyhow!("no entity with id '{id}'"))
    }
}

/// `<scenario>.estimate`: the scenario model's statewide five-year result
/// for one state, with formula, sensitivity, assumptions and gaps.
pub struct ScenarioEstimate {
    pub scenario: String,
}

#[async_trait]
impl Tool for ScenarioEstimate {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: format!("{}.estimate", self.scenario),
            description: format!(
                "Run the scenario model for the {} benchmark in {{state}}: additional units permitted, started and completed by year over five years vs. the baseline (low/central/high), what it is most sensitive to, the assumptions with sources, and known gaps. Optional economic conditions vs. recent levels: {{rate_shift_pp}} (10-year Treasury, points), {{migration_factor}} (1 = recent net migration), {{price_shift_pp}} (annual house-price growth, points).",
                self.scenario
            ),
        }
    }
    async fn call(&self, args: Value, _svc: &GuardedServices) -> Result<String> {
        let q = str_arg(&args, "state").ok_or_else(|| anyhow!("missing string arg 'state'"))?;
        let s = crate::state::find(q).ok_or_else(|| anyhow!("unknown state '{q}'"))?;
        let a = crate::engine::Assumptions::bundled();
        let e = a.estimate_with(
            &self.scenario,
            s,
            &crate::forecast::Conditions::from_args(&args),
        )?;
        Ok(a.explain(&e, s))
    }
}

/// `modeling.compare_scenarios`: all five benchmark scenarios side by side.
pub struct CompareScenarios;

#[async_trait]
impl Tool for CompareScenarios {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "modeling.compare_scenarios".into(),
            description: "Compare all five benchmark scenarios for {state}: central and range of additional units permitted and completed over five years, share of the baseline, and the most influential assumption for each. Optional {rate_shift_pp}, {migration_factor}, {price_shift_pp} change economic conditions.".into(),
        }
    }
    async fn call(&self, args: Value, _svc: &GuardedServices) -> Result<String> {
        let q = str_arg(&args, "state").ok_or_else(|| anyhow!("missing string arg 'state'"))?;
        let s = crate::state::find(q).ok_or_else(|| anyhow!("unknown state '{q}'"))?;
        Ok(crate::engine::Assumptions::bundled()
            .compare_with(s, &crate::forecast::Conditions::from_args(&args)))
    }
}
