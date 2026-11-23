//! Model providers for the orchestrator (leader) and the domain experts.
//!
//! Two backends:
//!
//! * **Offline** (default): deterministic models that answer only from tool
//!   results, so the panel runs and is tested without network access.
//! * **Open models** (`llm` feature): OpenRouter (`HOUSING_LLM_HOST=openrouter`)
//!   or any OpenAI-compatible server you run (Ollama, vLLM, llama.cpp; set
//!   `HOUSING_LLM_BASE_URL`), serving models from `model-registry.json`. Only Apache-2.0 models may be configured, and the
//!   leader must outrank every expert; [`ModelPlan::resolve`] refuses
//!   anything else.

use crate::ontology::Ontology;
use crate::state::split_state;
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use condesate::{CompletionRequest, CompletionResponse, ModelClass, ModelProvider, Role, ToolCall};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

const REGISTRY: &str = include_str!("../model-registry.json");

/// Where the models are served: `ollama` (default), `vllm` or `openrouter`.
pub const HOST_ENV: &str = "HOUSING_LLM_HOST";
/// OpenAI-compatible endpoint, e.g. `http://localhost:11434/v1` for Ollama.
/// Optional for OpenRouter, which has a fixed default.
pub const BASE_URL_ENV: &str = "HOUSING_LLM_BASE_URL";
/// Bearer token. Optional for local servers; required for OpenRouter.
pub const API_KEY_ENV: &str = "HOUSING_LLM_API_KEY";
/// Fallback key variable for OpenRouter, matching its own docs.
pub const OPENROUTER_KEY_ENV: &str = "OPENROUTER_API_KEY";
/// Profile from the registry: `laptop`, `workstation` or `server`.
pub const PROFILE_ENV: &str = "HOUSING_MODEL_PROFILE";
/// Registry id overriding the profile's leader.
pub const LEADER_ENV: &str = "HOUSING_LEADER_MODEL";
/// Registry id overriding the profile's expert model for every domain.
pub const EXPERT_ENV: &str = "HOUSING_EXPERT_MODEL";
/// Prefix for per-domain overrides, e.g. `HOUSING_EXPERT_MODEL_ADU`.
pub const EXPERT_DOMAIN_ENV_PREFIX: &str = "HOUSING_EXPERT_MODEL_";

fn assistant_turns(req: &CompletionRequest) -> usize {
    req.messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .count()
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
        let (state, question) = split_state(&last_user(&req));
        if assistant_turns(&req) == 0 {
            let mut calls = vec![ToolCall {
                name: format!("{}.evidence_brief", self.domain),
                args: json!({ "query": question }),
            }];
            if let Some(s) = state {
                calls.push(ToolCall {
                    name: "state.profile".into(),
                    args: json!({ "state": s.abbr }),
                });
                if crate::engine::Assumptions::bundled()
                    .scenario(&self.domain)
                    .is_some()
                {
                    calls.push(ToolCall {
                        name: format!("{}.estimate", self.domain),
                        args: json!({ "state": s.abbr }),
                    });
                } else if self.domain == "modeling" {
                    calls.push(ToolCall {
                        name: "modeling.compare_scenarios".into(),
                        args: json!({ "state": s.abbr }),
                    });
                }
            }
            return Ok(CompletionResponse {
                content: format!("Pulling the {} evidence brief.", self.domain),
                tool_calls: calls,
            });
        }
        let results = tool_results(&req);
        let brief = results
            .iter()
            .find(|r| r.starts_with("DOMAIN "))
            .cloned()
            .unwrap_or_default();
        // The model's own estimate (or comparison) leads; the evidence follows.
        let model = results
            .iter()
            .find(|r| r.starts_with("MODEL ESTIMATE") || r.starts_with("SCENARIO COMPARISON"));
        let mut out = model.map(|m| format!("{m}\n\n")).unwrap_or_default();
        out.push_str(&condense_brief(&brief, 4));
        if let Some(s) = state {
            out.push_str(&format!(
                "\nApplying this to {}: these figures come from other places and periods. They show direction and plausible magnitude, not a statewide forecast; scale per-capita rates with the state profile's conversion line.",
                s.name
            ));
        }
        out.push_str("\nEach figure applies only to its own outcome measure and setting; treat them as comparison cases, not a forecast.");
        Ok(CompletionResponse {
            content: out,
            tool_calls: vec![],
        })
    }
}

/// What every panel answer must say the model cannot tell policymakers.
pub const STANDING_LIMITS: &str = "\
- The evidence comes from specific cities, states and countries; none is a validated forecast for this state.
- Figures use different outcomes (permits, applications, stock, feasible capacity, parking spaces) and must not be added together.
- The baseline assumes the 2021-2025 permit pace continues; interest rates, recessions or construction costs could move it either way.
- Permits are not completions; some permitted units are never built, and the rest take a year or more.
- Local implementation of state law varies, so statewide totals can differ from what a uniform rollout would imply.";

/// What every panel answer lists as the inputs results are most sensitive to.
pub const STANDING_SENSITIVITIES: &str = "\
- Which comparison rate or take-up assumption is borrowed from the studies above; compare their low and high anchors.
- Whether the reform actually binds: places whose current rules already allow the change gain little.
- Market strength: rents and prices relative to construction, land and financing costs.
- Local implementation and approval speed after a state law passes.
- The baseline pace itself (see its low to high range).";

/// Deterministic orchestrator: route the question with the ontology's
/// domain keywords, consult each matching expert in one step, then stitch
/// their answers together in the structure the challenge asks for.
pub struct OfflineOrchestratorModel {
    pub ontology: &'static Ontology,
}

impl OfflineOrchestratorModel {
    /// The causal mechanisms the ontology lists for the routed domains' scenarios.
    fn drivers(&self, domains: &[&crate::ontology::Domain]) -> String {
        let mut seen = Vec::new();
        let mut out = String::new();
        for sc in self
            .ontology
            .scenarios
            .iter()
            .filter(|sc| domains.iter().any(|d| d.scenarios.contains(&sc.id)))
        {
            for m in &sc.key_mechanisms {
                if seen.contains(m) {
                    continue;
                }
                seen.push(m.clone());
                if let Some(mech) = self.ontology.mechanisms.iter().find(|x| &x.id == m) {
                    out.push_str(&format!("- {}: {}\n", mech.label, mech.description));
                }
            }
        }
        if out.is_empty() {
            out.push_str("- See the mechanisms in each evidence section above.\n");
        }
        out
    }
}

#[async_trait]
impl ModelProvider for OfflineOrchestratorModel {
    fn name(&self) -> &str {
        "offline-orchestrator"
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let message = last_user(&req);
        let (state, question) = split_state(&message);
        // Route on the question only, so the context line doesn't steer it.
        let domains = self.ontology.route(&question);
        if assistant_turns(&req) == 0 {
            let labels: Vec<&str> = domains.iter().map(|d| d.label.as_str()).collect();
            return Ok(CompletionResponse {
                content: format!("Consulting: {}.", labels.join(", ")),
                tool_calls: domains
                    .iter()
                    .map(|d| ToolCall {
                        name: format!("consult.{}", d.id),
                        args: json!({ "question": message }),
                    })
                    .collect(),
            });
        }
        let answers = tool_results(&req);
        let mut out = format!("Question: {question}\n");
        match state {
            Some(s) => out.push_str(&format!(
                "\n## Statewide baseline: {}\n{}\n",
                s.name,
                s.profile()
            )),
            None => out.push_str("\n(No state selected. The challenge asks for statewide results; pass a state to get its baseline.)\n"),
        }
        if state.is_some() {
            // Headline: each expert's model result, before the evidence.
            let headlines: Vec<&str> = answers
                .iter()
                .flat_map(|a| a.lines())
                .filter(|l| {
                    l.starts_with("MODEL ESTIMATE")
                        || l.starts_with("Additional units authorized")
                        || l.starts_with("Completed within")
                })
                .collect();
            out.push_str("\n## Statewide five-year effect vs. baseline\n");
            if headlines.is_empty() {
                out.push_str("This question doesn't map to one of the five benchmark scenarios, so the model gives no estimate; see the evidence below.\n");
            } else {
                out.push_str(&headlines.join("\n"));
                out.push('\n');
            }
        }
        for (d, a) in domains.iter().zip(answers.iter()) {
            out.push_str(&format!("\n## Evidence: {} ({})\n{a}\n", d.label, d.expert));
        }
        if domains.len() > 1 {
            out.push_str("\n## Combining these\nThe experts' estimates use different outcome measures and settings. Don't add them together; policy effects interact (see the modeling domain, e.g. kulka_sood_chiumenti_2026).\n");
        }
        out.push_str(&format!("\n## What drives it\n{}", self.drivers(&domains)));
        out.push_str(&format!(
            "\n## What it is most sensitive to\n{STANDING_SENSITIVITIES}\n"
        ));
        out.push_str(&format!(
            "\n## What this cannot tell you\n{STANDING_LIMITS}\n"
        ));
        Ok(CompletionResponse {
            content: out,
            tool_calls: vec![],
        })
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
    /// The model's id on OpenRouter, if OpenRouter lists it.
    #[serde(default)]
    pub openrouter_id: Option<String>,
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
        REG.get_or_init(|| {
            serde_json::from_str(REGISTRY).expect("bundled model registry must parse")
        })
    }

    /// A model the panel may run, or an error explaining why not.
    pub fn allowed(&self, id: &str) -> Result<&ModelSpec> {
        if let Some(r) = self.rejected.iter().find(|r| r.id == id) {
            bail!(
                "model '{id}' is not allowed: license {} ({}); only {} models may be used",
                r.license,
                r.reason,
                self.required_license
            );
        }
        let m = self
            .models
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| anyhow!("model '{id}' is not in agents/model-registry.json"))?;
        if m.license != self.required_license {
            bail!(
                "model '{id}' has license {}, but {} is required",
                m.license,
                self.required_license
            );
        }
        Ok(m)
    }
}

/// Who serves the models. Each host names the same model differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    /// Ollama, or any server using Ollama tags (`qwen3:4b`).
    Ollama,
    /// vLLM, or any server using Hugging Face repo names (`Qwen/Qwen3-4B`).
    Vllm,
    /// OpenRouter (`openai/gpt-oss-120b`).
    OpenRouter,
}

impl Host {
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "" | "ollama" => Ok(Host::Ollama),
            "vllm" | "hf" | "huggingface" => Ok(Host::Vllm),
            "openrouter" => Ok(Host::OpenRouter),
            other => bail!("unknown {HOST_ENV} '{other}'; expected ollama, vllm or openrouter"),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Host::Ollama => "ollama",
            Host::Vllm => "vllm",
            Host::OpenRouter => "openrouter",
        }
    }

    /// Hosted services have a fixed URL; self-hosted servers must be named.
    pub fn default_base_url(self) -> Option<&'static str> {
        match self {
            Host::OpenRouter => Some("https://openrouter.ai/api/v1"),
            Host::Ollama | Host::Vllm => None,
        }
    }

    /// The profile used when none is chosen.
    pub fn default_profile(self) -> Option<&'static str> {
        match self {
            Host::OpenRouter => Some("openrouter"),
            Host::Ollama | Host::Vllm => None,
        }
    }

    /// This host's name for a registry model, if it serves it.
    pub fn model_id(self, spec: &ModelSpec) -> Option<&str> {
        match self {
            Host::Ollama => Some(spec.ollama_tag.as_str()),
            Host::Vllm => Some(spec.hf_repo.as_str()),
            Host::OpenRouter => spec.openrouter_id.as_deref(),
        }
    }
}

/// Where model calls go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Backend {
    Offline,
    /// An OpenAI-compatible chat completions server. Needs the `llm` feature.
    OpenAiCompat {
        host: Host,
        base_url: String,
        api_key: Option<String>,
    },
}

impl Backend {
    /// `OpenAiCompat` when a base URL is set or the host has a default one
    /// (OpenRouter); otherwise `Offline`.
    pub fn from_env() -> Result<Self> {
        let var = |k: &str| {
            std::env::var(k)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let host = Host::parse(&var(HOST_ENV).unwrap_or_default())?;
        let Some(base_url) =
            var(BASE_URL_ENV).or_else(|| host.default_base_url().map(str::to_string))
        else {
            return Ok(Backend::Offline);
        };
        let mut api_key = var(API_KEY_ENV);
        if host == Host::OpenRouter {
            api_key = api_key.or_else(|| var(OPENROUTER_KEY_ENV));
        }
        Ok(Backend::OpenAiCompat {
            host,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    pub fn host(&self) -> Option<Host> {
        match self {
            Backend::Offline => None,
            Backend::OpenAiCompat { host, .. } => Some(*host),
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
        let host = backend.host();
        if let Backend::OpenAiCompat {
            host: Host::OpenRouter,
            api_key: None,
            ..
        } = &backend
        {
            bail!("OpenRouter needs an API key: set {API_KEY_ENV} or {OPENROUTER_KEY_ENV}");
        }
        let profile_name = profile
            .or_else(|| host.and_then(Host::default_profile))
            .unwrap_or(&registry.default_profile)
            .to_string();
        let p = registry.profiles.get(&profile_name).ok_or_else(|| {
            anyhow!(
                "unknown model profile '{profile_name}'; expected one of: {}",
                registry
                    .profiles
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
        let served = |m: &ModelSpec| -> Result<()> {
            match host {
                Some(h) if h.model_id(m).is_none() => bail!(
                    "model '{}' is not available on {} (no {}_id in agents/model-registry.json); choose another model or host",
                    m.id,
                    h.name(),
                    h.name()
                ),
                _ => Ok(()),
            }
        };
        let leader = registry.allowed(leader.unwrap_or(&p.leader))?.clone();
        served(&leader)?;
        let default_expert = expert.unwrap_or(&p.expert);
        for d in per_domain.keys() {
            if !domains.contains(&d.as_str()) {
                bail!("expert model override for unknown domain '{d}'");
            }
        }
        let mut experts = BTreeMap::new();
        for d in domains {
            let id = per_domain
                .get(*d)
                .map(String::as_str)
                .unwrap_or(default_expert);
            let m = registry.allowed(id)?.clone();
            served(&m)?;
            if m.capability_rank >= leader.capability_rank {
                bail!(
                    "leader '{}' (rank {}) must be stronger than the {d} expert '{}' (rank {}); pick a larger leader or a smaller expert",
                    leader.id, leader.capability_rank, m.id, m.capability_rank
                );
            }
            experts.insert(d.to_string(), m);
        }
        Ok(Self {
            backend,
            profile: profile_name,
            leader,
            experts,
        })
    }

    /// Resolve from `HOUSING_*` environment variables.
    pub fn from_env(domains: &[&str]) -> Result<Self> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let per_domain: BTreeMap<String, String> = domains
            .iter()
            .filter_map(|d| {
                var(&format!("{EXPERT_DOMAIN_ENV_PREFIX}{}", d.to_uppercase()))
                    .map(|m| (d.to_string(), m))
            })
            .collect();
        Self::resolve(
            Registry::bundled(),
            Backend::from_env()?,
            var(PROFILE_ENV).as_deref(),
            var(LEADER_ENV).as_deref(),
            var(EXPERT_ENV).as_deref(),
            &per_domain,
            domains,
        )
    }

    /// The offline plan: registry defaults, used only for labels.
    pub fn offline(domains: &[&str]) -> Self {
        Self::resolve(
            Registry::bundled(),
            Backend::Offline,
            None,
            None,
            None,
            &BTreeMap::new(),
            domains,
        )
        .expect("default profile must be valid")
    }

    /// The name the configured host uses for `spec` (the registry id offline).
    pub fn served_id<'a>(&self, spec: &'a ModelSpec) -> &'a str {
        self.backend
            .host()
            .and_then(|h| h.model_id(spec))
            .unwrap_or(&spec.id)
    }

    fn provider(
        &self,
        spec: &ModelSpec,
        offline: Arc<dyn ModelProvider>,
    ) -> Result<Arc<dyn ModelProvider>> {
        match &self.backend {
            Backend::Offline => Ok(offline),
            #[cfg(feature = "llm")]
            Backend::OpenAiCompat {
                base_url, api_key, ..
            } => Ok(Arc::new(crate::openai_compat::OpenAiCompatModel::new(
                base_url,
                self.served_id(spec),
                api_key.clone(),
            ))),
            #[cfg(not(feature = "llm"))]
            Backend::OpenAiCompat { .. } => {
                let _ = spec;
                bail!("{BASE_URL_ENV} is set but this binary was built without the `llm` feature; rebuild with --features llm")
            }
        }
    }

    pub fn leader_provider(
        &self,
        offline: Arc<dyn ModelProvider>,
    ) -> Result<Arc<dyn ModelProvider>> {
        self.provider(&self.leader, offline)
    }

    pub fn expert_provider(
        &self,
        domain: &str,
        offline: Arc<dyn ModelProvider>,
    ) -> Result<Arc<dyn ModelProvider>> {
        let spec = self
            .experts
            .get(domain)
            .ok_or_else(|| anyhow!("no expert model for domain '{domain}'"))?;
        self.provider(spec, offline)
    }

    /// The model class recorded in an agent's manifest, and so in the audit trail.
    pub fn model_class(&self, domain: Option<&str>) -> ModelClass {
        let spec = domain
            .and_then(|d| self.experts.get(d))
            .unwrap_or(&self.leader);
        let (provider, family, revision) = match self.backend {
            Backend::Offline => (
                "offline".to_string(),
                "deterministic".to_string(),
                env!("CARGO_PKG_VERSION").to_string(),
            ),
            Backend::OpenAiCompat { host, .. } => (
                host.name().to_string(),
                spec.family.clone(),
                self.served_id(spec).to_string(),
            ),
        };
        ModelClass {
            provider,
            family,
            revision,
            embedding_space: None,
            quantization: None,
        }
    }

    pub fn is_offline(&self) -> bool {
        self.backend == Backend::Offline
    }

    pub fn describe(&self) -> String {
        if self.is_offline() {
            return format!("offline deterministic models (set {BASE_URL_ENV} to use the '{}' open-model profile)", self.profile);
        }
        let host = self.backend.host().map_or("offline", Host::name);
        let mut out = format!(
            "{host} profile {}: leader {} ({}, {})",
            self.profile,
            self.leader.id,
            self.served_id(&self.leader),
            self.leader.license
        );
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
        let brief = Ontology::bundled()
            .evidence_brief("adu", "permits", 8)
            .unwrap();
        let short = condense_brief(&brief, 2);
        assert!(short.contains("SCENARIO"));
        assert_eq!(short.lines().filter(|l| l.starts_with('[')).count(), 2);
        assert!(short.contains("USABLE DATASETS"));
        assert!(!short.contains("source:"));
    }

    const DOMAINS: &[&str] = &["adu", "fees"];

    fn resolve(
        leader: Option<&str>,
        expert: Option<&str>,
        per: &[(&str, &str)],
    ) -> Result<ModelPlan> {
        let per: BTreeMap<String, String> = per
            .iter()
            .map(|(d, m)| (d.to_string(), m.to_string()))
            .collect();
        ModelPlan::resolve(
            Registry::bundled(),
            Backend::Offline,
            None,
            leader,
            expert,
            &per,
            DOMAINS,
        )
    }

    #[test]
    fn registry_only_lists_apache_models_and_every_profile_is_valid() {
        let reg = Registry::bundled();
        assert_eq!(reg.required_license, "Apache-2.0");
        assert!(reg.models.iter().all(|m| m.license == "Apache-2.0"));
        assert!(reg
            .rejected
            .iter()
            .all(|r| reg.models.iter().all(|m| m.id != r.id)));
        for name in reg.profiles.keys() {
            ModelPlan::resolve(
                reg,
                Backend::Offline,
                Some(name),
                None,
                None,
                &BTreeMap::new(),
                DOMAINS,
            )
            .unwrap_or_else(|e| panic!("profile {name}: {e}"));
        }
    }

    #[test]
    fn non_apache_models_are_refused_with_their_license() {
        let err = resolve(None, Some("llama-3.1-8b"), &[])
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("llama3.1") && err.contains("Apache-2.0"),
            "{err}"
        );
        assert!(resolve(Some("claude"), None, &[]).is_err());
        assert!(resolve(None, Some("not-a-model"), &[]).is_err());
    }

    #[test]
    fn leader_must_outrank_every_expert() {
        let err = resolve(Some("qwen3-8b"), Some("qwen3-8b"), &[])
            .unwrap_err()
            .to_string();
        assert!(err.contains("must be stronger"), "{err}");
        assert!(resolve(
            Some("qwen3-4b"),
            Some("qwen3-1.7b"),
            &[("fees", "qwen3-14b")]
        )
        .is_err());
    }

    fn openrouter(key: Option<&str>) -> Backend {
        Backend::OpenAiCompat {
            host: Host::OpenRouter,
            base_url: "https://openrouter.ai/api/v1".into(),
            api_key: key.map(str::to_string),
        }
    }

    #[test]
    fn openrouter_defaults_to_its_profile_and_uses_openrouter_ids() {
        let plan = ModelPlan::resolve(
            Registry::bundled(),
            openrouter(Some("k")),
            None,
            None,
            None,
            &BTreeMap::new(),
            DOMAINS,
        )
        .unwrap();
        assert_eq!(plan.profile, "openrouter");
        assert_eq!(plan.served_id(&plan.leader), "openai/gpt-oss-120b");
        assert_eq!(plan.served_id(&plan.experts["adu"]), "openai/gpt-oss-20b");
        assert_eq!(plan.model_class(Some("adu")).provider, "openrouter");
    }

    #[test]
    fn openrouter_refuses_models_it_does_not_serve_and_missing_keys() {
        let err = ModelPlan::resolve(
            Registry::bundled(),
            openrouter(Some("k")),
            Some("laptop"),
            None,
            None,
            &BTreeMap::new(),
            DOMAINS,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("not available on openrouter"), "{err}");
        let err = ModelPlan::resolve(
            Registry::bundled(),
            openrouter(None),
            None,
            None,
            None,
            &BTreeMap::new(),
            DOMAINS,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("API key"), "{err}");
    }

    #[test]
    fn each_host_names_models_its_own_way() {
        let spec = Registry::bundled().allowed("qwen3-32b").unwrap();
        assert_eq!(Host::Ollama.model_id(spec), Some("qwen3:32b"));
        assert_eq!(Host::Vllm.model_id(spec), Some("Qwen/Qwen3-32B"));
        assert_eq!(Host::OpenRouter.model_id(spec), Some("qwen/qwen3-32b"));
        assert!(Host::parse("bedrock").is_err());
    }

    #[test]
    fn per_domain_overrides_give_experts_different_models() {
        let plan = resolve(
            Some("qwen3-32b"),
            Some("qwen3-8b"),
            &[("adu", "granite-3.3-8b")],
        )
        .unwrap();
        assert_eq!(plan.experts["adu"].id, "granite-3.3-8b");
        assert_eq!(plan.experts["fees"].id, "qwen3-8b");
        assert_eq!(plan.leader.id, "qwen3-32b");
        assert!(resolve(None, None, &[("nope", "qwen3-0.6b")]).is_err());
    }
}
