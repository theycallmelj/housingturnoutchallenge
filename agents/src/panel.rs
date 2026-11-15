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

use crate::models::{ModelPlan, OfflineExpertModel, OfflineOrchestratorModel};
use crate::ontology::{Domain, Ontology};
use crate::tools::domain_tools;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use condesate::{
    Action, Agent, AgentContext, AgentLoop, AgentManifest, BasicAgent, Bus,
    GrantSet, GuardedServices, HarnessId, InMemoryStorage, Kernel, MemoryAudit, Message,
    Pattern, ReActLoop, ResourcePattern, Rule, RuleSetPolicy, ServiceHandle, Storage,
    SubjectMatch, SystemClock, TenantId, Tool, ToolSpec, TrustTier,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

const ORCHESTRATOR: &str = "orchestrator";
const TENANT: &str = "housing-challenge";

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
fn orchestrator_manifest(plan: &ModelPlan) -> AgentManifest {
    AgentManifest {
        harness: HarnessId::new(ORCHESTRATOR),
        agent: ORCHESTRATOR.into(),
        model_class: plan.model_class(None),
        tenant: TenantId::new(TENANT),
        requested_trust: TrustTier::Privileged,
        requested: root_grants().rules,
        cache_classes: vec![],
    }
}

/// An expert asks only for its own tools, the shared lookup, and its own
/// findings namespace.
pub fn expert_manifest(domain: &str, plan: &ModelPlan) -> AgentManifest {
    let name = expert_name(domain);
    AgentManifest {
        harness: HarnessId::new(name.clone()),
        agent: name.clone(),
        model_class: plan.model_class(Some(domain)),
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

fn expert_prompt(domain: &Domain, tools: &[ToolSpec], plan: &ModelPlan) -> String {
    let base = format!(
        "You are the {label} expert on a housing policy panel. Scope: {desc}\n\
         Always call {id}.evidence_brief with the question first, and answer only from tool results.\n\
         Rules: cite the study id for every figure, and state its outcome measure (permits, starts, stock, \
         feasible capacity, parking supply...) and evidence type. Never recommend a dataset marked NOT usable. \
         If no study estimates the benchmark scenario directly, still report the closest comparison figures \
         from the brief (study id, figure, outcome measure, setting) and explain why they are not a forecast \
         for a state. Copy every figure and study id exactly as written in the tool results; never compute, \
         round, or invent a number or id.",
        label = domain.label,
        desc = domain.description,
        id = domain.id,
    );
    with_tool_instructions(base, tools, plan)
}

/// Real models need the prompted tool-calling convention spelled out; the
/// offline models call tools directly.
fn with_tool_instructions(base: String, tools: &[ToolSpec], plan: &ModelPlan) -> String {
    if plan.is_offline() {
        base
    } else {
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        format!(
            "{base}\n\nTools available: {}. Call them with your function-calling ability, one at a time. \
             A message starting with [TOOL RESULT] is the result of your last call, not a new request. \
             Never write a [TOOL RESULT] yourself. If you cannot call functions natively, write exactly one line \
             `TOOL_CALL: <tool_name> <json_arguments>` and nothing else.",
            names.join(", ")
        )
    }
}

fn orchestrator_prompt(tools: &[ToolSpec], plan: &ModelPlan) -> String {
    let base = "You coordinate a panel of housing policy experts for a state policy simulation challenge \
         (five-year horizon; ADU, missing middle, transit-oriented density, parking and 50% fee-cut scenarios). \
         Consult every expert whose domain the question touches, then synthesize. Keep each expert's figures \
         tied to their study ids and outcome measures. Use only figures and study ids that appear in the experts' \
         answers, copied exactly; never invent, compute, or round them. Never add effects across scenarios or \
         outcome types; flag interactions and disagreements instead.";
    with_tool_instructions(base.to_string(), tools, plan)
}

/// `consult.<domain>`: spawn the domain expert under attenuated authority,
/// run it on the question, record its answer, and return it.
pub struct ConsultExpert {
    domain: String,
    agent: Arc<dyn Agent>,
    storage: Arc<dyn Storage>,
    plan: Arc<ModelPlan>,
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
            .spawn_child(&expert_manifest(&self.domain, &self.plan), raw_services(&name, self.storage.clone()))
            .await
            .map_err(|r| anyhow!("spawn of {name} refused: {r}"))?;
        child.begin_activation(format!("{name}:{question}"), 1024 * 1024);

        let mut ctx = AgentContext::new(Arc::new(child));
        ctx.transcript.push(Message::user(question.to_string()));
        let outcome = ReActLoop { max_steps: 4 }.run(self.agent.as_ref(), &mut ctx).await?;
        // An expert that never called a tool is answering from memory.
        let grounded = ctx.transcript.iter().any(|m| m.role == condesate::Role::Tool);
        let answer = if grounded { outcome.final_text } else { EXPERT_UNGROUNDED.to_string() };
        ctx.services
            .storage_set(&format!("findings/{}/answer", self.domain), &answer)
            .await?;
        Ok(answer)
    }
}

/// Prefixed to any answer produced without consulting an expert.
pub const UNGROUNDED_LABEL: &str =
    "UNGROUNDED: no expert was consulted, so this answer is not based on the evidence base. Do not use its figures.";

const UNGROUNDED_NUDGE: &str = "You have not consulted any expert yet. Do not answer from memory. \
     Call the consult tool for the relevant domain now, using one TOOL_CALL line and nothing else.";

/// Returned instead of an expert's text when it never called a tool.
pub const EXPERT_UNGROUNDED: &str =
    "The expert did not consult the evidence base, so it has no grounded answer. Treat this domain as unanswered.";

/// Domains whose experts recorded findings in this activation.
async fn consulted_domains(services: &GuardedServices) -> Result<Vec<String>> {
    let mut consulted: Vec<String> = services
        .storage_keys("findings/")
        .await?
        .into_iter()
        .filter_map(|k| k.split('/').nth(1).map(str::to_string))
        .collect();
    consulted.sort();
    consulted.dedup();
    Ok(consulted)
}

/// The answer to one question, with the trail of what produced it.
pub struct PanelAnswer {
    pub text: String,
    /// Domains whose experts recorded findings.
    pub consulted: Vec<String>,
    /// One line per permission decision, in order.
    pub audit: Vec<String>,
    /// Which models answered.
    pub models: String,
    /// Citations or figures in a model answer that the ontology doesn't support.
    pub citation_issues: Vec<String>,
}

/// Snake_case tokens that look like study ids, e.g. `pew_2026_adu`.
fn id_like_tokens(line: &str) -> Vec<&str> {
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map(|t| t.trim_matches('_'))
        .filter(|t| t.contains('_') && t.chars().next().is_some_and(|c| c.is_ascii_lowercase()))
        .collect()
}

/// Numbers in a line, normalized (`100 000` and `100,000` become `100000`).
fn numbers(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = line.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let joins = matches!(c, ',' | ' ' | '\u{202f}' | '\u{a0}' | '.')
            && !cur.is_empty()
            && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit());
        if c.is_ascii_digit() {
            cur.push(c);
        } else if joins {
            if c == '.' {
                cur.push('.');
            }
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Study ids the answer cites that aren't in the ontology, and figures on a
/// line citing a study that don't appear anywhere in that study's record.
pub fn check_citations(ontology: &Ontology, text: &str) -> Vec<String> {
    let mut issues = Vec::new();
    for line in text.lines() {
        let mut cited = Vec::new();
        for tok in id_like_tokens(line) {
            if let Some(s) = ontology.study(tok) {
                cited.push(s);
            } else if tok.chars().any(|c| c.is_ascii_digit()) && ontology.dataset(tok).is_none() {
                let issue = format!("`{tok}` is not a study in the evidence base");
                if !issues.contains(&issue) {
                    issues.push(issue);
                }
            }
        }
        if cited.is_empty() {
            continue;
        }
        let record: String = cited
            .iter()
            .map(|s| format!("{} {} {} {} {} {}", s.id, s.title, s.venue, s.finding, s.limitation, s.relevance))
            .collect::<Vec<_>>()
            .join(" ");
        let known: Vec<String> = numbers(&record);
        let ids: Vec<&str> = cited.iter().map(|s| s.id.as_str()).collect();
        for n in numbers(line) {
            let is_year = n.len() == 4 && (1900..=2100).contains(&n.parse::<u32>().unwrap_or(0));
            if is_year || ids.iter().any(|id| id.contains(&n)) || known.contains(&n) {
                continue;
            }
            issues.push(format!("figure {n} next to {} does not appear in that study's record", ids.join(", ")));
        }
    }
    issues
}

pub struct Panel {
    ontology: &'static Ontology,
    plan: Arc<ModelPlan>,
    experts: HashMap<String, Arc<dyn Agent>>,
}

impl Panel {
    pub fn new(ontology: &'static Ontology, plan: ModelPlan) -> Result<Self> {
        let mut experts: HashMap<String, Arc<dyn Agent>> = HashMap::new();
        for d in &ontology.domains {
            let tools = domain_tools(ontology, &d.id);
            let specs: Vec<ToolSpec> = tools.iter().map(|t| t.spec()).collect();
            let model = plan.expert_provider(&d.id, Arc::new(OfflineExpertModel { domain: d.id.clone() }))?;
            let agent = BasicAgent::new(expert_name(&d.id), expert_prompt(d, &specs, &plan), model, tools);
            experts.insert(d.id.clone(), Arc::new(agent));
        }
        Ok(Self { ontology, plan: Arc::new(plan), experts })
    }

    /// The domain ids, in ontology order, that a model plan must cover.
    pub fn domain_ids(ontology: &Ontology) -> Vec<&str> {
        ontology.domains.iter().map(|d| d.id.as_str()).collect()
    }

    /// Models chosen from `HOUSING_*` environment variables (offline if
    /// `HOUSING_LLM_BASE_URL` is unset).
    pub fn from_env(ontology: &'static Ontology) -> Result<Self> {
        Self::new(ontology, ModelPlan::from_env(&Self::domain_ids(ontology))?)
    }

    /// Deterministic offline models only.
    pub fn offline(ontology: &'static Ontology) -> Self {
        Self::new(ontology, ModelPlan::offline(&Self::domain_ids(ontology))).expect("offline panel")
    }

    pub fn plan(&self) -> &ModelPlan {
        &self.plan
    }

    /// The agent for one domain expert, for callers that run it directly.
    pub fn expert(&self, domain: &str) -> Option<Arc<dyn Agent>> {
        self.experts.get(domain).cloned()
    }

    pub async fn ask(&self, question: &str) -> Result<PanelAnswer> {
        let storage = InMemoryStorage::new();
        let audit = MemoryAudit::new();
        let kernel = Kernel::new(root_grants(), Arc::new(RuleSetPolicy::new()), audit.clone(), Arc::new(SystemClock));
        let admission = kernel.admit(&orchestrator_manifest(&self.plan), None);
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
                    plan: self.plan.clone(),
                }) as Arc<dyn Tool>
            })
            .collect();
        let specs: Vec<ToolSpec> = consult_tools.iter().map(|t| t.spec()).collect();
        let model = self.plan.leader_provider(Arc::new(OfflineOrchestratorModel { ontology: self.ontology }))?;
        let orchestrator = BasicAgent::new(ORCHESTRATOR, orchestrator_prompt(&specs, &self.plan), model, consult_tools);

        let mut ctx = AgentContext::new(services.clone());
        ctx.transcript.push(Message::user(question.to_string()));
        let mut outcome = ReActLoop { max_steps: 10 }.run(&orchestrator, &mut ctx).await?;

        // A leader that answers without consulting anyone is answering from
        // memory, not from the evidence base. Nudge once, then label it.
        let mut consulted = consulted_domains(&services).await?;
        if consulted.is_empty() && !self.plan.is_offline() {
            ctx.transcript.push(Message::user(UNGROUNDED_NUDGE.to_string()));
            outcome = ReActLoop { max_steps: 10 }.run(&orchestrator, &mut ctx).await?;
            consulted = consulted_domains(&services).await?;
        }
        let text = if consulted.is_empty() && !self.plan.is_offline() {
            format!("{UNGROUNDED_LABEL}\n\n{}", outcome.final_text)
        } else {
            outcome.final_text
        };
        // Model answers are checked against the ontology; offline answers
        // are verbatim tool output and need no check.
        let citation_issues = if self.plan.is_offline() { Vec::new() } else { check_citations(self.ontology, &text) };
        let text = if citation_issues.is_empty() {
            text
        } else {
            format!(
                "{text}\n\n## Citation check (automatic)\nThe following could not be verified against the evidence base. \
                 Treat them as errors:\n{}",
                citation_issues.iter().map(|i| format!("- {i}")).collect::<Vec<_>>().join("\n")
            )
        };
        let audit = audit.events().await.iter().map(|e| e.summarize()).collect();
        Ok(PanelAnswer { text, consulted, audit, models: self.plan.describe(), citation_issues })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offline_plan() -> ModelPlan {
        ModelPlan::offline(&Panel::domain_ids(Ontology::bundled()))
    }

    #[test]
    fn manifests_record_each_agents_own_model() {
        use crate::models::{Backend, Host, Registry};
        use std::collections::BTreeMap;
        let per = BTreeMap::from([("adu".to_string(), "granite-3.3-8b".to_string())]);
        let plan = ModelPlan::resolve(
            Registry::bundled(),
            Backend::OpenAiCompat { host: Host::Ollama, base_url: "http://localhost:11434/v1".into(), api_key: None },
            Some("workstation"),
            None,
            None,
            &per,
            &Panel::domain_ids(Ontology::bundled()),
        )
        .unwrap();
        assert_eq!(orchestrator_manifest(&plan).model_class.revision, "qwen3:32b");
        assert_eq!(expert_manifest("adu", &plan).model_class.revision, "granite3.3:8b");
        assert_eq!(expert_manifest("fees", &plan).model_class.revision, "qwen3:8b");
    }

    fn panel() -> Panel {
        Panel::offline(Ontology::bundled())
    }

    #[test]
    fn citation_check_flags_invented_ids_and_figures() {
        let o = Ontology::bundled();
        // From a real gpt-oss-120b answer: one correct row, three invented ones.
        let answer = "| **pew_2026_adu** | permits | **81 ADU permits per 100 000 residents** (2025) |\n\
                      | **rose_2024_adu** | +12 % annual growth |\n\
                      | **marantz_elmendorf_kim_2023** | 4 % of total housing stock |\n\
                      The terner_sb9_2021 study found about 700,000 feasible units on 5.4% of parcels.";
        let issues = check_citations(o, answer);
        assert!(issues.iter().any(|i| i.contains("`rose_2024_adu` is not a study")), "{issues:#?}");
        assert!(issues.iter().any(|i| i.contains("figure 4 next to marantz_elmendorf_kim_2023")), "{issues:#?}");
        assert!(!issues.iter().any(|i| i.contains("pew_2026_adu")), "correct figures pass: {issues:#?}");
        assert!(!issues.iter().any(|i| i.contains("terner_sb9_2021")), "correct figures pass: {issues:#?}");
    }

    #[test]
    fn numbers_are_normalized() {
        assert_eq!(numbers("81 per 100 000, 5.4% and 700,000"), vec!["81", "100000", "5.4", "700000"]);
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
        let adm = kernel.admit(&orchestrator_manifest(&offline_plan()), None);
        let orch = kernel.attach(&adm, raw_services(ORCHESTRATOR, storage.clone()));
        orch.begin_activation("test", u64::MAX);
        let adu = orch
            .spawn_child(&expert_manifest("adu", &offline_plan()), raw_services("expert.adu", storage))
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
        let adm = kernel.admit(&orchestrator_manifest(&offline_plan()), None);
        let orch = kernel.attach(&adm, raw_services(ORCHESTRATOR, storage.clone()));
        orch.begin_activation("test", u64::MAX);
        let adu = orch
            .spawn_child(&expert_manifest("adu", &offline_plan()), raw_services("expert.adu", storage.clone()))
            .await
            .unwrap();
        adu.begin_activation("test", u64::MAX);
        assert!(adu
            .spawn_child(&expert_manifest("fees", &offline_plan()), raw_services("expert.fees", storage))
            .await
            .is_err());
    }
}
