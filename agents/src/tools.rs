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
    vec![
        Arc::new(EvidenceBrief { ontology, domain: domain.to_string() }),
        Arc::new(FindStudies { ontology, domain: domain.to_string() }),
        Arc::new(FindDatasets { ontology, domain: domain.to_string() }),
        Arc::new(GetEntity { ontology }),
    ]
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
        let studies = self.ontology.find_studies(Some(&self.domain), scenario, query, 6);
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
        let usable_only = args.get("usable_only").and_then(Value::as_bool).unwrap_or(false);
        let datasets = self.ontology.find_datasets(Some(&self.domain), None, usable_only);
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
        self.ontology.describe(id).ok_or_else(|| anyhow!("no entity with id '{id}'"))
    }
}
