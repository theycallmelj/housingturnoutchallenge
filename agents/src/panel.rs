//! The expert panel: one orchestrator agent and one expert agent per domain,
//! run on condesate's agent loop and permission kernel.
//!
//! ```text
//!   question ─► orchestrator ──consult.<domain>──► expert.<domain> ──<domain>.*──► ontology
//!                 (Privileged)     spawn_child        (Sandboxed)     tools
//! ```
//!
//! The orchestrator holds a spawn grant for `expert.*`. Each consult tool
//! spawns its expert through `GuardedServices::spawn_child`, so the expert's
//! authority is attenuated from the orchestrator's: it may invoke only its
//! own `<domain>.*` tools plus `ontology.get`, and write only under
//! `findings/<domain>/`. Every check lands in the kernel's audit trail.

use crate::models::{self, Backend, OfflineExpertModel, OfflineOrchestratorModel};
use crate::ontology::{Domain, Ontology};
use crate::tools::domain_tools;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use condesate::{
    Action, Agent, AgentContext, AgentLoop, AgentManifest, BasicAgent, Bus,
    GrantSet, GuardedServices, HarnessId, InMemoryStorage, Kernel, MemoryAudit, Message,
    ModelClass, Pattern, ReActLoop, ResourcePattern, Rule, RuleSetPolicy, ServiceHandle, Storage,
    SubjectMatch, SystemClock, TenantId, Tool, ToolSpec, TrustTier,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

const ORCHESTRATOR: &str = "orchestrator";
const TENANT: &str = "housing-challenge";

fn model_class(backend: Backend) -> ModelClass {
    let (provider, family) = match backend {
        Backend::Offline => ("offline", "deterministic"),
        #[cfg(feature = "llm")]
        Backend::Anthropic => ("anthropic", "claude"),
    };
    ModelClass {
        provider: provider.into(),
        family: family.into(),
        revision: env!("CARGO_PKG_VERSION").into(),
        embedding_space: None,
        quantization: None,
    }
}

fn expert_name(domain: &str) -> String {
    format!("expert.{domain}")
}

/// Everything any principal in the panel may ever hold.
fn root_grants() -> GrantSet {
    GrantSet::new(vec![
        Rule::allow("tools", SubjectMatch::default(), &[Action::Invoke], ResourcePattern::Tool(Pattern::Any)),
        Rule::allow(
            "findings",
            SubjectMatch::default(),
            &[Action::Read, Action::Write],
            ResourcePattern::Memory(Pattern::parse("findings/*")),
        ),
        Rule::allow(
            "spawn-experts",
            SubjectMatch::agent(ORCHESTRATOR),
            &[Action::Spawn],
            ResourcePattern::Spawn(Pattern::parse("expert.*")),
        ),
    ])
}

/// The orchestrator asks for root's full shape: its tool and memory rules
/// keep an open subject so they can be delegated to experts, and only it may
/// spawn.
fn orchestrator_manifest(backend: Backend) -> AgentManifest {
    AgentManifest {
        harness: HarnessId::new(ORCHESTRATOR),
        agent: ORCHESTRATOR.into(),
        model_class: model_class(backend),
        tenant: TenantId::new(TENANT),
        requested_trust: TrustTier::Privileged,
        requested: root_grants().rules,
        cache_classes: vec![],
    }
}

/// An expert asks only for its own tools, the shared lookup, and its own
/// findings namespace.
pub fn expert_manifest(domain: &str, backend: Backend) -> AgentManifest {
    let name = expert_name(domain);
    AgentManifest {
        harness: HarnessId::new(name.clone()),
        agent: name.clone(),
        model_class: model_class(backend),
        tenant: TenantId::new(TENANT),
        requested_trust: TrustTier::Sandboxed,
        requested: vec![
            Rule::allow(
                "own-tools",
                SubjectMatch::agent(&name),
                &[Action::Invoke],
                ResourcePattern::Tool(Pattern::parse(&format!("{domain}.*"))),
            ),
            Rule::allow(
                "lookup",
                SubjectMatch::agent(&name),
                &[Action::Invoke],
                ResourcePattern::Tool(Pattern::parse("ontology.get")),
            ),
            Rule::allow(
                "own-findings",
                SubjectMatch::agent(&name),
                &[Action::Read, Action::Write],
                ResourcePattern::Memory(Pattern::parse(&format!("findings/{domain}/*"))),
            ),
        ],
        cache_classes: vec![],
    }
}

fn raw_services(me: &str, storage: Arc<dyn Storage>) -> ServiceHandle {
    ServiceHandle {
        me: HarnessId::new(me),
        roster: Arc::new(vec![HarnessId::new(me)]),
        storage,
        bus: Bus::new(HashMap::new()),
    }
}

fn expert_prompt(domain: &Domain, tools: &[ToolSpec], backend: Backend) -> String {
    let base = format!(
        "You are the {label} expert on a housing policy panel. Scope: {desc}\n\
         Always call {id}.evidence_brief with the question first, and answer only from tool results.\n\
         Rules: cite the study id for every figure, and state its outcome measure (permits, starts, stock, \
         feasible capacity, parking supply...) and evidence type. Never recommend a dataset marked NOT usable. \
         Say plainly when the evidence cannot support a number for the benchmark scenario.",
        label = domain.label,
        desc = domain.description,
        id = domain.id,
    );
    let _ = tools; // only the llm backend needs the tool list in the prompt
    match backend {
        Backend::Offline => base,
        #[cfg(feature = "llm")]
        Backend::Anthropic => format!("{base}\n\n{}", condesate::tool_instructions(tools)),
    }
}

fn orchestrator_prompt(tools: &[ToolSpec], backend: Backend) -> String {
    let base = "You coordinate a panel of housing policy experts for a state policy simulation challenge \
         (five-year horizon; ADU, missing middle, transit-oriented density, parking and 50% fee-cut scenarios). \
         Consult every expert whose domain the question touches, then synthesize. Keep each expert's figures \
         tied to their study ids and outcome measures. Never add effects across scenarios or outcome types; \
         flag interactions and disagreements instead.";
    let _ = tools; // only the llm backend needs the tool list in the prompt
    match backend {
        Backend::Offline => base.to_string(),
        #[cfg(feature = "llm")]
        Backend::Anthropic => format!("{base}\n\n{}", condesate::tool_instructions(tools)),
    }
}

/// `consult.<domain>`: spawn the domain expert under attenuated authority,
/// run it on the question, record its answer, and return it.
pub struct ConsultExpert {
    domain: String,
    agent: Arc<dyn Agent>,
    storage: Arc<dyn Storage>,
    backend: Backend,
}

#[async_trait]
impl Tool for ConsultExpert {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: format!("consult.{}", self.domain),
            description: format!("Ask the {} expert a {{question}}.", self.domain),
        }
    }

    async fn call(&self, args: Value, svc: &GuardedServices) -> Result<String> {
        let question = args
            .get("question")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing string arg 'question'"))?;
        let name = expert_name(&self.domain);
        let child = svc
            .spawn_child(&expert_manifest(&self.domain, self.backend), raw_services(&name, self.storage.clone()))
            .await
            .map_err(|r| anyhow!("spawn of {name} refused: {r}"))?;
        child.begin_activation(format!("{name}:{question}"), 1024 * 1024);

        let mut ctx = AgentContext::new(Arc::new(child));
        ctx.transcript.push(Message::user(question.to_string()));
        let outcome = ReActLoop { max_steps: 4 }.run(self.agent.as_ref(), &mut ctx).await?;
        ctx.services
            .storage_set(&format!("findings/{}/answer", self.domain), &outcome.final_text)
            .await?;
        Ok(outcome.final_text)
    }
}

/// The answer to one question, with the trail of what produced it.
pub struct PanelAnswer {
    pub text: String,
    /// Domains whose experts recorded findings.
    pub consulted: Vec<String>,
    /// One line per permission decision, in order.
    pub audit: Vec<String>,
}

pub struct Panel {
    ontology: &'static Ontology,
    backend: Backend,
    experts: HashMap<String, Arc<dyn Agent>>,
}

impl Panel {
    pub fn new(ontology: &'static Ontology, backend: Backend) -> Result<Self> {
        let mut experts: HashMap<String, Arc<dyn Agent>> = HashMap::new();
        for d in &ontology.domains {
            let tools = domain_tools(ontology, &d.id);
            let specs: Vec<ToolSpec> = tools.iter().map(|t| t.spec()).collect();
            let model = models::provider(backend, Arc::new(OfflineExpertModel { domain: d.id.clone() }))?;
            let agent = BasicAgent::new(expert_name(&d.id), expert_prompt(d, &specs, backend), model, tools);
            experts.insert(d.id.clone(), Arc::new(agent));
        }
        Ok(Self { ontology, backend, experts })
    }

    /// The agent for one domain expert, for callers that run it directly.
    pub fn expert(&self, domain: &str) -> Option<Arc<dyn Agent>> {
        self.experts.get(domain).cloned()
    }

    pub async fn ask(&self, question: &str) -> Result<PanelAnswer> {
        let storage = InMemoryStorage::new();
        let audit = MemoryAudit::new();
        let kernel = Kernel::new(root_grants(), Arc::new(RuleSetPolicy::new()), audit.clone(), Arc::new(SystemClock));
        let admission = kernel.admit(&orchestrator_manifest(self.backend), None);
        let services = Arc::new(kernel.attach(&admission, raw_services(ORCHESTRATOR, storage.clone())));
        services.begin_activation(format!("ask:{question}"), 8 * 1024 * 1024);

        let consult_tools: Vec<Arc<dyn Tool>> = self
            .ontology
            .domains
            .iter()
            .map(|d| {
                Arc::new(ConsultExpert {
                    domain: d.id.clone(),
                    agent: self.experts[&d.id].clone(),
                    storage: storage.clone(),
                    backend: self.backend,
                }) as Arc<dyn Tool>
            })
            .collect();
        let specs: Vec<ToolSpec> = consult_tools.iter().map(|t| t.spec()).collect();
        let model = models::provider(self.backend, Arc::new(OfflineOrchestratorModel { ontology: self.ontology }))?;
        let orchestrator = BasicAgent::new(ORCHESTRATOR, orchestrator_prompt(&specs, self.backend), model, consult_tools);

        let mut ctx = AgentContext::new(services.clone());
        ctx.transcript.push(Message::user(question.to_string()));
        let outcome = ReActLoop { max_steps: 10 }.run(&orchestrator, &mut ctx).await?;

        let mut consulted: Vec<String> = services
            .storage_keys("findings/")
            .await?
            .into_iter()
            .filter_map(|k| k.split('/').nth(1).map(str::to_string))
            .collect();
        consulted.sort();
        consulted.dedup();
        let audit = audit.events().await.iter().map(|e| e.summarize()).collect();
        Ok(PanelAnswer { text: outcome.final_text, consulted, audit })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Panel {
        Panel::new(Ontology::bundled(), Backend::Offline).unwrap()
    }

    #[tokio::test]
    async fn adu_question_is_answered_by_the_adu_expert_from_the_ontology() {
        let a = panel().ask("How many ADUs could a statewide ADU law add in five years?").await.unwrap();
        assert!(a.consulted.contains(&"adu".to_string()), "{:?}", a.consulted);
        assert!(a.text.contains("Accessory dwelling units"));
        assert!(a.text.contains("USABLE DATASETS"));
    }

    #[tokio::test]
    async fn cross_domain_question_consults_several_experts_and_warns_against_adding() {
        let a = panel()
            .ask("Would eliminating parking minimums near transit stations add housing?")
            .await
            .unwrap();
        assert!(a.consulted.contains(&"parking".to_string()));
        assert!(a.consulted.contains(&"tod".to_string()));
        assert!(a.text.contains("Don't add them together"));
    }

    #[tokio::test]
    async fn experts_are_spawned_children_and_every_decision_is_audited() {
        let a = panel().ask("What do impact fee cuts do?").await.unwrap();
        assert!(a.audit.iter().any(|l| l.contains("expert.fees") && l.contains("ALLOW")));
        assert!(a.audit.iter().all(|l| !l.contains("DENY")), "{:#?}", a.audit);
    }

    #[tokio::test]
    async fn an_expert_cannot_invoke_another_domains_tools() {
        let storage = InMemoryStorage::new();
        let kernel = Kernel::new(root_grants(), Arc::new(RuleSetPolicy::new()), MemoryAudit::new(), Arc::new(SystemClock));
        let adm = kernel.admit(&orchestrator_manifest(Backend::Offline), None);
        let orch = kernel.attach(&adm, raw_services(ORCHESTRATOR, storage.clone()));
        orch.begin_activation("test", u64::MAX);
        let adu = orch
            .spawn_child(&expert_manifest("adu", Backend::Offline), raw_services("expert.adu", storage))
            .await
            .unwrap();
        adu.begin_activation("test", u64::MAX);

        assert!(adu.authorize_tool("adu.evidence_brief").await.is_ok());
        assert!(adu.authorize_tool("ontology.get").await.is_ok());
        assert!(adu.authorize_tool("fees.evidence_brief").await.is_err());
        assert!(adu.storage_set("findings/adu/answer", "x").await.is_ok());
        assert!(adu.storage_set("findings/fees/answer", "x").await.is_err());
    }

    #[tokio::test]
    async fn an_expert_cannot_spawn_further_agents() {
        let storage = InMemoryStorage::new();
        let kernel = Kernel::new(root_grants(), Arc::new(RuleSetPolicy::new()), MemoryAudit::new(), Arc::new(SystemClock));
        let adm = kernel.admit(&orchestrator_manifest(Backend::Offline), None);
        let orch = kernel.attach(&adm, raw_services(ORCHESTRATOR, storage.clone()));
        orch.begin_activation("test", u64::MAX);
        let adu = orch
            .spawn_child(&expert_manifest("adu", Backend::Offline), raw_services("expert.adu", storage.clone()))
            .await
            .unwrap();
        adu.begin_activation("test", u64::MAX);
        assert!(adu
            .spawn_child(&expert_manifest("fees", Backend::Offline), raw_services("expert.fees", storage))
            .await
            .is_err());
    }
}
