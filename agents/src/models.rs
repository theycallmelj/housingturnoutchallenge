//! Model providers for the orchestrator (leader) and the domain experts.
//!
//! Two backends:
//!
//! * **Offline** (default): deterministic models that answer only from tool
//!   results, so the panel runs and is tested without network access.
//! * **Open models** (`llm` feature + `HOUSING_LLM_BASE_URL`): any
//!   OpenAI-compatible server (Ollama, vLLM, llama.cpp) serving models from
//!   `model-registry.json`. Only Apache-2.0 models may be configured, and the
//!   leader must outrank every expert; [`ModelPlan::resolve`] refuses
//!   anything else.

use crate::ontology::Ontology;
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use condesate::{CompletionRequest, CompletionResponse, ModelClass, ModelProvider, Role, ToolCall};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

const REGISTRY: &str = include_str!("../model-registry.json");

/// OpenAI-compatible endpoint, e.g. `http://localhost:11434/v1` for Ollama.
pub const BASE_URL_ENV: &str = "HOUSING_LLM_BASE_URL";
/// Optional bearer token for hosted endpoints.
pub const API_KEY_ENV: &str = "HOUSING_LLM_API_KEY";
/// Profile from the registry: `laptop`, `workstation` or `server`.
pub const PROFILE_ENV: &str = "HOUSING_MODEL_PROFILE";
/// Registry id overriding the profile's leader.
pub const LEADER_ENV: &str = "HOUSING_LEADER_MODEL";
/// Registry id overriding the profile's expert model for every domain.
pub const EXPERT_ENV: &str = "HOUSING_EXPERT_MODEL";
/// Prefix for per-domain overrides, e.g. `HOUSING_EXPERT_MODEL_ADU`.
pub const EXPERT_DOMAIN_ENV_PREFIX: &str = "HOUSING_EXPERT_MODEL_";

fn assistant_turns(req: &CompletionRequest) -> usize {
    req.messages.iter().filter(|m| m.role == Role::Assistant).count()
}

fn last_user(req: &CompletionRequest) -> String {
    req.messages
        .iter()
        .rev()
        .find(|m| m.role == Role::User)
        .map(|m| m.content.clone())
        .unwrap_or_default()
}

fn tool_results(req: &CompletionRequest) -> Vec<String> {
    req.messages
        .iter()
        .filter(|m| m.role == Role::Tool)
        .map(|m| m.content.clone())
        .collect()
}

/// Keep the parts of an evidence brief an answer needs: the scenario lines,
/// the first `max_studies` study records, and the dataset verdicts.
fn condense_brief(brief: &str, max_studies: usize) -> String {
    let mut out = Vec::new();
    let mut studies = 0;
    let mut in_studies = false;
    for line in brief.lines() {
        if line.starts_with("SCENARIO ") {
            out.push(line.to_string());
        } else if line.starts_with("STUDIES") {
            in_studies = true;
        } else if line.starts_with("USABLE DATASETS") || line.starts_with("NOT USABLE") {
            in_studies = false;
            out.push(line.to_string());
        } else if in_studies {
            if line.starts_with('[') {
                studies += 1;
            }
            if studies <= max_studies && !line.trim_start().starts_with("source:") {
                out.push(line.to_string());
            }
        } else if line.starts_with("  - [") {
            out.push(line.to_string());
        }
    }
    out.join("\n")
}

/// Deterministic expert: fetch the domain's evidence brief for the question,
/// then answer by condensing it. Never states anything the tool didn't return.
pub struct OfflineExpertModel {
    pub domain: String,
}

#[async_trait]
impl ModelProvider for OfflineExpertModel {
    fn name(&self) -> &str {
        "offline-expert"
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        if assistant_turns(&req) == 0 {
            return Ok(CompletionResponse {
                content: format!("Pulling the {} evidence brief.", self.domain),
                tool_calls: vec![ToolCall {
                    name: format!("{}.evidence_brief", self.domain),
                    args: json!({ "query": last_user(&req) }),
                }],
            });
        }
        let brief = tool_results(&req).pop().unwrap_or_default();
        Ok(CompletionResponse {
            content: format!(
                "{}\nEach figure applies only to its own outcome measure and setting; treat them as comparison cases, not a forecast.",
                condense_brief(&brief, 4)
            ),
            tool_calls: vec![],
        })
    }
}

/// Deterministic orchestrator: route the question with the ontology's
/// domain keywords, consult each matching expert in one step, then stitch
/// their answers together under the panel's standing caveats.
pub struct OfflineOrchestratorModel {
    pub ontology: &'static Ontology,
}

#[async_trait]
impl ModelProvider for OfflineOrchestratorModel {
    fn name(&self) -> &str {
        "offline-orchestrator"
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let question = last_user(&req);
        let domains = self.ontology.route(&question);
        if assistant_turns(&req) == 0 {
            let labels: Vec<&str> = domains.iter().map(|d| d.label.as_str()).collect();
            return Ok(CompletionResponse {
                content: format!("Consulting: {}.", labels.join(", ")),
                tool_calls: domains
                    .iter()
                    .map(|d| ToolCall {
                        name: format!("consult.{}", d.id),
                        args: json!({ "question": question }),
                    })
                    .collect(),
            });
        }
        let answers = tool_results(&req);
        let mut out = format!("Question: {question}\n");
        for (d, a) in domains.iter().zip(answers.iter()) {
            out.push_str(&format!("\n## {} ({})\n{a}\n", d.label, d.expert));
        }
        if domains.len() > 1 {
            out.push_str("\n## Combining these\nThe experts' estimates use different outcome measures and settings. Don't add them together; policy effects interact (see the modeling domain, e.g. kulka_sood_chiumenti_2026).\n");
        }
        Ok(CompletionResponse { content: out, tool_calls: vec![] })
    }
}

#[derive(Debug, Deserialize)]
pub struct Registry {
    pub required_license: String,
    pub default_profile: String,
    pub profiles: BTreeMap<String, Profile>,
    pub models: Vec<ModelSpec>,
    pub rejected: Vec<RejectedModel>,
}

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub description: String,
    pub leader: String,
    pub expert: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ModelSpec {
    pub id: String,
    pub family: String,
    pub hf_repo: String,
    pub ollama_tag: String,
    pub license: String,
    pub params_b: f64,
    pub download_gb: f64,
    pub capability_rank: u32,
}

#[derive(Debug, Deserialize)]
pub struct RejectedModel {
    pub id: String,
    pub license: String,
    pub reason: String,
}

impl Registry {
    pub fn bundled() -> &'static Registry {
        use std::sync::OnceLock;
        static REG: OnceLock<Registry> = OnceLock::new();
        REG.get_or_init(|| serde_json::from_str(REGISTRY).expect("bundled model registry must parse"))
    }

    /// A model the panel may run, or an error explaining why not.
    pub fn allowed(&self, id: &str) -> Result<&ModelSpec> {
        if let Some(r) = self.rejected.iter().find(|r| r.id == id) {
            bail!("model '{id}' is not allowed: license {} ({}); only {} models may be used", r.license, r.reason, self.required_license);
        }
        let m = self
            .models
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| anyhow!("model '{id}' is not in agents/model-registry.json"))?;
        if m.license != self.required_license {
            bail!("model '{id}' has license {}, but {} is required", m.license, self.required_license);
        }
        Ok(m)
    }
}

/// Where model calls go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Backend {
    Offline,
    /// An OpenAI-compatible chat completions server. Needs the `llm` feature.
    OpenAiCompat { base_url: String, api_key: Option<String> },
}

impl Backend {
    /// `OpenAiCompat` when `HOUSING_LLM_BASE_URL` is set, else `Offline`.
    pub fn from_env() -> Self {
        match std::env::var(BASE_URL_ENV) {
            Ok(base_url) if !base_url.trim().is_empty() => Backend::OpenAiCompat {
                base_url: base_url.trim().trim_end_matches('/').to_string(),
                api_key: std::env::var(API_KEY_ENV).ok().filter(|k| !k.is_empty()),
            },
            _ => Backend::Offline,
        }
    }
}

/// Which model each agent runs, validated against the registry.
#[derive(Clone, Debug)]
pub struct ModelPlan {
    pub backend: Backend,
    pub profile: String,
    pub leader: ModelSpec,
    pub experts: BTreeMap<String, ModelSpec>,
}

impl ModelPlan {
    /// Resolve a plan from explicit choices. Every model must be allowed
    /// (Apache-2.0, in the registry) and the leader must outrank every expert.
    pub fn resolve(
        registry: &Registry,
        backend: Backend,
        profile: Option<&str>,
        leader: Option<&str>,
        expert: Option<&str>,
        per_domain: &BTreeMap<String, String>,
        domains: &[&str],
    ) -> Result<Self> {
        let profile_name = profile.unwrap_or(&registry.default_profile).to_string();
        let p = registry.profiles.get(&profile_name).ok_or_else(|| {
            anyhow!("unknown model profile '{profile_name}'; expected one of: {}", registry.profiles.keys().cloned().collect::<Vec<_>>().join(", "))
        })?;
        let leader = registry.allowed(leader.unwrap_or(&p.leader))?.clone();
        let default_expert = expert.unwrap_or(&p.expert);
        for d in per_domain.keys() {
            if !domains.contains(&d.as_str()) {
                bail!("expert model override for unknown domain '{d}'");
            }
        }
        let mut experts = BTreeMap::new();
        for d in domains {
            let id = per_domain.get(*d).map(String::as_str).unwrap_or(default_expert);
            let m = registry.allowed(id)?.clone();
            if m.capability_rank >= leader.capability_rank {
                bail!(
                    "leader '{}' (rank {}) must be stronger than the {d} expert '{}' (rank {}); pick a larger leader or a smaller expert",
                    leader.id, leader.capability_rank, m.id, m.capability_rank
                );
            }
            experts.insert(d.to_string(), m);
        }
        Ok(Self { backend, profile: profile_name, leader, experts })
    }

    /// Resolve from `HOUSING_*` environment variables.
    pub fn from_env(domains: &[&str]) -> Result<Self> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let per_domain: BTreeMap<String, String> = domains
            .iter()
            .filter_map(|d| var(&format!("{EXPERT_DOMAIN_ENV_PREFIX}{}", d.to_uppercase())).map(|m| (d.to_string(), m)))
            .collect();
        Self::resolve(
            Registry::bundled(),
            Backend::from_env(),
            var(PROFILE_ENV).as_deref(),
            var(LEADER_ENV).as_deref(),
            var(EXPERT_ENV).as_deref(),
            &per_domain,
            domains,
        )
    }

    /// The offline plan: registry defaults, used only for labels.
    pub fn offline(domains: &[&str]) -> Self {
        Self::resolve(Registry::bundled(), Backend::Offline, None, None, None, &BTreeMap::new(), domains)
            .expect("default profile must be valid")
    }

    fn provider(&self, spec: &ModelSpec, offline: Arc<dyn ModelProvider>) -> Result<Arc<dyn ModelProvider>> {
        match &self.backend {
            Backend::Offline => Ok(offline),
            #[cfg(feature = "llm")]
            Backend::OpenAiCompat { base_url, api_key } => {
                let inner = Arc::new(crate::openai_compat::OpenAiCompatModel::new(base_url, &spec.ollama_tag, api_key.clone()));
                Ok(Arc::new(condesate::PromptedToolModel::new(inner)))
            }
            #[cfg(not(feature = "llm"))]
            Backend::OpenAiCompat { .. } => {
                let _ = spec;
                bail!("{BASE_URL_ENV} is set but this binary was built without the `llm` feature; rebuild with --features llm")
            }
        }
    }

    pub fn leader_provider(&self, offline: Arc<dyn ModelProvider>) -> Result<Arc<dyn ModelProvider>> {
        self.provider(&self.leader, offline)
    }

    pub fn expert_provider(&self, domain: &str, offline: Arc<dyn ModelProvider>) -> Result<Arc<dyn ModelProvider>> {
        let spec = self.experts.get(domain).ok_or_else(|| anyhow!("no expert model for domain '{domain}'"))?;
        self.provider(spec, offline)
    }

    /// The model class recorded in an agent's manifest, and so in the audit trail.
    pub fn model_class(&self, domain: Option<&str>) -> ModelClass {
        let spec = domain.and_then(|d| self.experts.get(d)).unwrap_or(&self.leader);
        let (provider, family, revision) = match self.backend {
            Backend::Offline => ("offline".to_string(), "deterministic".to_string(), env!("CARGO_PKG_VERSION").to_string()),
            Backend::OpenAiCompat { .. } => ("openai-compat".to_string(), spec.family.clone(), spec.ollama_tag.clone()),
        };
        ModelClass { provider, family, revision, embedding_space: None, quantization: None }
    }

    pub fn is_offline(&self) -> bool {
        self.backend == Backend::Offline
    }

    pub fn describe(&self) -> String {
        if self.is_offline() {
            return format!("offline deterministic models (set {BASE_URL_ENV} to use the '{}' open-model profile)", self.profile);
        }
        let mut out = format!("profile {}: leader {} ({}, {})", self.profile, self.leader.id, self.leader.ollama_tag, self.leader.license);
        for (d, m) in &self.experts {
            out.push_str(&format!("; {d} {}", m.id));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condense_brief_keeps_scenarios_studies_and_dataset_verdicts() {
        let brief = Ontology::bundled().evidence_brief("adu", "permits", 8).unwrap();
        let short = condense_brief(&brief, 2);
        assert!(short.contains("SCENARIO"));
        assert_eq!(short.lines().filter(|l| l.starts_with('[')).count(), 2);
        assert!(short.contains("USABLE DATASETS"));
        assert!(!short.contains("source:"));
    }

    const DOMAINS: &[&str] = &["adu", "fees"];

    fn resolve(leader: Option<&str>, expert: Option<&str>, per: &[(&str, &str)]) -> Result<ModelPlan> {
        let per: BTreeMap<String, String> = per.iter().map(|(d, m)| (d.to_string(), m.to_string())).collect();
        ModelPlan::resolve(Registry::bundled(), Backend::Offline, None, leader, expert, &per, DOMAINS)
    }

    #[test]
    fn registry_only_lists_apache_models_and_every_profile_is_valid() {
        let reg = Registry::bundled();
        assert_eq!(reg.required_license, "Apache-2.0");
        assert!(reg.models.iter().all(|m| m.license == "Apache-2.0"));
        assert!(reg.rejected.iter().all(|r| reg.models.iter().all(|m| m.id != r.id)));
        for name in reg.profiles.keys() {
            ModelPlan::resolve(reg, Backend::Offline, Some(name), None, None, &BTreeMap::new(), DOMAINS)
                .unwrap_or_else(|e| panic!("profile {name}: {e}"));
        }
    }

    #[test]
    fn non_apache_models_are_refused_with_their_license() {
        let err = resolve(None, Some("llama-3.1-8b"), &[]).unwrap_err().to_string();
        assert!(err.contains("llama3.1") && err.contains("Apache-2.0"), "{err}");
        assert!(resolve(Some("claude"), None, &[]).is_err());
        assert!(resolve(None, Some("not-a-model"), &[]).is_err());
    }

    #[test]
    fn leader_must_outrank_every_expert() {
        let err = resolve(Some("qwen3-8b"), Some("qwen3-8b"), &[]).unwrap_err().to_string();
        assert!(err.contains("must be stronger"), "{err}");
        assert!(resolve(Some("qwen3-4b"), Some("qwen3-1.7b"), &[("fees", "qwen3-14b")]).is_err());
    }

    #[test]
    fn per_domain_overrides_give_experts_different_models() {
        let plan = resolve(Some("qwen3-32b"), Some("qwen3-8b"), &[("adu", "granite-3.3-8b")]).unwrap();
        assert_eq!(plan.experts["adu"].id, "granite-3.3-8b");
        assert_eq!(plan.experts["fees"].id, "qwen3-8b");
        assert_eq!(plan.leader.id, "qwen3-32b");
        assert!(resolve(None, None, &[("nope", "qwen3-0.6b")]).is_err());
    }
}
