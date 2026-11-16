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
    Action, Agent, AgentContext, AgentLoop, AgentManifest, BasicAgent, Bus, GrantSet,
    GuardedServices, HarnessId, InMemoryStorage, Kernel, MemoryAudit, Message, Pattern, ReActLoop,
    ResourcePattern, Rule, RuleSetPolicy, ServiceHandle, Storage, SubjectMatch, SystemClock,
    TenantId, Tool, ToolSpec, TrustTier,
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
        Rule::allow(
            "tools",
            SubjectMatch::default(),
            &[Action::Invoke],
            ResourcePattern::Tool(Pattern::Any),
        ),
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
                "state-data",
                SubjectMatch::agent(&name),
                &[Action::Invoke],
                ResourcePattern::Tool(Pattern::parse("state.*")),
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
         round, or invent a number or id.\n\
         The challenge needs statewide results: if the question starts with a line `STATE: XX`, also call \
         state.profile for that state, and relate the evidence to that state's five-year baseline (direction, \
         plausible magnitude, uncertainty), saying how the state differs from the places studied. To apply a \
         per-capita rate from a study to the state, call state.scale_rate; never do arithmetic yourself. \
         If you have a `<domain>.estimate` tool (or modeling.compare_scenarios), call it for the state and lead \
         your answer with its result, copied exactly: it is the team's scenario model, with its assumptions and \
         sensitivity. Use the studies to explain and question it, not to replace it. When a \
         study gives more than one rate, scale each one so the answer is a low-to-high range, and repeat the \
         study's limitation (for example, whether the rate is a total or the increase a law causes).",
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
         answers or the state profile, copied exactly; never invent, compute, convert, or round them. Never add effects \
         across scenarios or outcome types; flag interactions and disagreements instead. Pass the question to \
         experts unchanged, including its `STATE:` line.\n\
         Results must be statewide and compared with a five-year current-rules baseline. Write for a \
         non-technical policymaker, in these sections:\n\
         1. Statewide five-year effect vs. baseline: state the baseline's central figure and range from the state \
         profile, then the scenario model's estimate exactly as an expert reported it (central and range, \
         additional units authorized by permit). If no expert ran the model, say so and give only the direction \
         the evidence supports.\n\
         2. What drives it.\n\
         3. What it is most sensitive to: use the model's sensitivity list when there is one.\n\
         4. What this cannot tell you.\n\
         Cite the study id in backticks next to every figure or finding you use, e.g. `pew_2026_adu`; don't \
         replace ids with author names. When you report a model estimate, copy its \"Evidence behind this \
         estimate\" ids too, and name the source ids for each assumption you discuss. Never write source markers \
         such as 【consult.adu】 or [tool]; the page already lists which experts were consulted. Consult every expert listed as suggested in the context unless it is \
         clearly irrelevant.";
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
            .spawn_child(
                &expert_manifest(&self.domain, &self.plan),
                raw_services(&name, self.storage.clone()),
            )
            .await
            .map_err(|r| anyhow!("spawn of {name} refused: {r}"))?;
        child.begin_activation(format!("{name}:{question}"), 1024 * 1024);

        let mut ctx = AgentContext::new(Arc::new(child));
        // Run the scenario model deterministically for the expert's own
        // scenario (or all five for the modeling expert), so every answer is
        // anchored to it rather than depending on the model choosing to call it.
        let model_result = crate::state::split_state(question).0.and_then(|s| {
            let a = crate::engine::Assumptions::bundled();
            match a.estimate(&self.domain, s) {
                Ok(e) => Some(a.explain(&e, s)),
                Err(_) if self.domain == "modeling" => Some(a.compare(s)),
                Err(_) => None,
            }
        });
        if let (Some(m), false) = (&model_result, self.plan.is_offline()) {
            ctx.transcript.push(Message::user(format!(
                "The team's scenario model has already been run for this state. Lead your answer with this \
                 result, copying its figures exactly (central and range), then use the evidence to explain and \
                 question it. Don't attribute anything to a study beyond what the evidence brief says.\n{m}"
            )));
        }
        ctx.transcript.push(Message::user(question.to_string()));
        let outcome = ReActLoop { max_steps: 6 }
            .run(self.agent.as_ref(), &mut ctx)
            .await?;
        // An expert that never called a tool is answering from memory.
        let grounded = ctx
            .transcript
            .iter()
            .any(|m| m.role == condesate::Role::Tool);
        let answer = if grounded {
            outcome.final_text
        } else {
            EXPERT_UNGROUNDED.to_string()
        };
        ctx.services
            .storage_set(&format!("findings/{}/answer", self.domain), &answer)
            .await?;
        // Keep what the tools actually returned, so the citation check can
        // tell tool-sourced figures from figures the model introduced.
        let mut tool_output: Vec<&str> = ctx
            .transcript
            .iter()
            .filter(|m| m.role == condesate::Role::Tool)
            .map(|m| m.content.as_str())
            .collect();
        tool_output.extend(model_result.as_deref());
        ctx.services
            .storage_set(
                &format!("findings/{}/tool_output", self.domain),
                &tool_output.join("\n"),
            )
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

/// Feedback sent to the leader when its answer has unsourced figures.
fn revision_request(issues: &[String]) -> String {
    format!(
        "An automatic check found figures in your answer that no tool produced:\n{}\n\
         Rewrite the complete answer in the same four sections. Copy figures exactly as they appear in the \
         experts' answers or the state profile. Do not round, approximate (no \"about 17k\"), convert, or \
         compute percentages yourself; if you need a figure no tool produced, describe it in words instead.",
        issues.iter().map(|i| format!("- {i}")).collect::<Vec<_>>().join("\n")
    )
}

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
pub fn numbers(line: &str) -> Vec<String> {
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

/// Strip the source markers some models (gpt-oss) add, e.g. `【consult.adu】`,
/// `【consult__baseline_data】` or `【4†source】`. A marker that names an expert
/// becomes "(ADU expert)"-style attribution; anything else is dropped.
pub fn clean_markers(ontology: &Ontology, text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(['【', '[']) {
        let open = rest[i..].chars().next().unwrap();
        let after = &rest[i + open.len_utf8()..];
        // A `[` only opens a marker when it names a tool; `【` always does.
        let is_marker = open == '【' || after.trim_start().starts_with("consult");
        let close = after
            .char_indices()
            .take_while(|(_, c)| *c != '\n')
            .find(|(_, c)| *c == '】' || *c == ']')
            .filter(|(k, _)| *k <= 120);
        let (true, Some((k, c))) = (is_marker, close) else {
            out.push_str(&rest[..i + open.len_utf8()]);
            rest = after;
            continue;
        };
        let inner = after[..k].trim();
        let id = inner
            .trim_start_matches("consult")
            .trim_start_matches(['.', '_', ':', ' ']);
        let id = id.split(['†', ' ', ',', ';']).next().unwrap_or("");
        let before = rest[..i].trim_end_matches([' ', '\t']);
        out.push_str(before);
        match ontology.domain(id).filter(|_| inner.starts_with("consult")) {
            Some(d) => out.push_str(&format!(" ({} expert)", d.label)),
            None if rest[..i].len() != before.len()
                && !after[k + c.len_utf8()..].starts_with([' ', '\n', '.', ',', ';', ':', ')']) =>
            {
                out.push(' ')
            }
            None => {}
        }
        rest = &after[k + c.len_utf8()..];
    }
    out.push_str(rest);
    out
}

/// Study ids the answer cites that aren't in the ontology, and figures on a
/// line citing a study that don't appear anywhere in that study's record.
/// `allowed` holds extra figures that are legitimately in play, such as the
/// chosen state's baseline numbers.
pub fn check_citations(ontology: &Ontology, text: &str, allowed: &[String]) -> Vec<String> {
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
            // No study named: a figure is still suspect if no tool produced it.
            for n in numbers(line) {
                let is_year =
                    n.len() == 4 && (1900..=2100).contains(&n.parse::<u32>().unwrap_or(0));
                let small_ordinal = n
                    .parse::<f64>()
                    .is_ok_and(|v| v <= 10.0 && v.fract() == 0.0);
                // Rate denominators ("per 100,000", "100 k") aren't claims.
                let denominator = matches!(n.as_str(), "100" | "1000" | "100000");
                if !is_year
                    && !small_ordinal
                    && !denominator
                    && !allowed.contains(&n)
                    && !allowed.is_empty()
                {
                    issues.push(format!(
                        "figure {n} (\"{}\") did not come from any tool output in this run",
                        line.trim().chars().take(80).collect::<String>()
                    ));
                }
            }
            continue;
        }
        let record: String = cited
            .iter()
            .map(|s| {
                format!(
                    "{} {} {} {} {} {}",
                    s.id, s.title, s.venue, s.finding, s.limitation, s.relevance
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let known: Vec<String> = numbers(&record);
        let ids: Vec<&str> = cited.iter().map(|s| s.id.as_str()).collect();
        for n in numbers(line) {
            let is_year = n.len() == 4 && (1900..=2100).contains(&n.parse::<u32>().unwrap_or(0));
            // Rate denominators ("per 100,000") aren't claims.
            let denominator = matches!(n.as_str(), "100" | "1000" | "100000");
            if is_year
                || denominator
                || ids.iter().any(|id| id.contains(&n))
                || known.contains(&n)
                || allowed.contains(&n)
            {
                continue;
            }
            issues.push(format!(
                "figure {n} next to {} does not appear in that study's record",
                ids.join(", ")
            ));
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
            let model = plan.expert_provider(
                &d.id,
                Arc::new(OfflineExpertModel {
                    domain: d.id.clone(),
                }),
            )?;
            let agent = BasicAgent::new(
                expert_name(&d.id),
                expert_prompt(d, &specs, &plan),
                model,
                tools,
            );
            experts.insert(d.id.clone(), Arc::new(agent));
        }
        Ok(Self {
            ontology,
            plan: Arc::new(plan),
            experts,
        })
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

    /// Ask without choosing a state. The challenge wants statewide results,
    /// so prefer [`Self::ask_for_state`].
    pub async fn ask(&self, question: &str) -> Result<PanelAnswer> {
        self.ask_for_state(question, None).await
    }

    /// Ask about one state (USPS code, FIPS or name). Experts receive a
    /// `STATE:` line and can pull that state's profile; the leader gets the
    /// profile up front.
    pub async fn ask_for_state(&self, question: &str, state: Option<&str>) -> Result<PanelAnswer> {
        let state = match state {
            Some(s) => Some(crate::state::find(s).ok_or_else(|| {
                anyhow!("unknown state '{s}'; use a USPS code like CA or a full name")
            })?),
            None => None,
        };
        let message = crate::state::with_state(question, state);
        let storage = InMemoryStorage::new();
        let audit = MemoryAudit::new();
        let kernel = Kernel::new(
            root_grants(),
            Arc::new(RuleSetPolicy::new()),
            audit.clone(),
            Arc::new(SystemClock),
        );
        let admission = kernel.admit(&orchestrator_manifest(&self.plan), None);
        let services =
            Arc::new(kernel.attach(&admission, raw_services(ORCHESTRATOR, storage.clone())));
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
        let model = self
            .plan
            .leader_provider(Arc::new(OfflineOrchestratorModel {
                ontology: self.ontology,
            }))?;
        let orchestrator = BasicAgent::new(
            ORCHESTRATOR,
            orchestrator_prompt(&specs, &self.plan),
            model,
            consult_tools,
        );

        let mut ctx = AgentContext::new(services.clone());
        if let (Some(s), false) = (state, self.plan.is_offline()) {
            ctx.transcript.push(Message::user(format!(
                "Context for this question:\n{}",
                s.profile()
            )));
        }
        if !self.plan.is_offline() {
            // The keyword router is deterministic and checkable; give the
            // leader its suggestion rather than relying on recall alone.
            let suggested: Vec<String> = self
                .ontology
                .route(question)
                .iter()
                .map(|d| format!("consult.{} ({})", d.id, d.label))
                .collect();
            ctx.transcript.push(Message::user(format!(
                "Suggested experts for this question (keyword router): {}.",
                suggested.join(", ")
            )));
        }
        ctx.transcript.push(Message::user(message));
        let mut outcome = ReActLoop { max_steps: 10 }
            .run(&orchestrator, &mut ctx)
            .await?;

        // A leader that answers without consulting anyone is answering from
        // memory, not from the evidence base. Nudge once, then label it.
        let mut consulted = consulted_domains(&services).await?;
        if consulted.is_empty() && !self.plan.is_offline() {
            ctx.transcript
                .push(Message::user(UNGROUNDED_NUDGE.to_string()));
            outcome = ReActLoop { max_steps: 10 }
                .run(&orchestrator, &mut ctx)
                .await?;
            consulted = consulted_domains(&services).await?;
        }
        // Model answers are checked against the ontology; offline answers
        // are verbatim tool output and need no check. Figures are sourced if
        // a tool returned them in this run (evidence briefs, state profile,
        // scale_rate arithmetic).
        let mut allowed: Vec<String> = state.map(|s| numbers(&s.profile())).unwrap_or_default();
        for key in services.storage_keys("findings/").await? {
            if key.ends_with("/tool_output") {
                if let Some(out) = services.storage_get(&key).await? {
                    allowed.extend(numbers(&out));
                }
            }
        }
        let check = |t: &str| {
            if self.plan.is_offline() {
                Vec::new()
            } else {
                check_citations(self.ontology, t, &allowed)
            }
        };
        let mut answer = outcome.final_text;
        let mut citation_issues = check(&answer);
        // One revision pass: hand the leader its own unsourced figures and
        // keep the rewrite only if it is cleaner.
        if !citation_issues.is_empty() && !consulted.is_empty() {
            ctx.transcript
                .push(Message::user(revision_request(&citation_issues)));
            let revised = ReActLoop { max_steps: 4 }
                .run(&orchestrator, &mut ctx)
                .await?
                .final_text;
            let revised_issues = check(&revised);
            if revised_issues.len() < citation_issues.len() {
                answer = revised;
                citation_issues = revised_issues;
            }
        }
        let answer = clean_markers(self.ontology, &answer);
        let text = if consulted.is_empty() && !self.plan.is_offline() {
            format!("{UNGROUNDED_LABEL}\n\n{answer}")
        } else {
            answer
        };
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
        Ok(PanelAnswer {
            text,
            consulted,
            audit,
            models: self.plan.describe(),
            citation_issues,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offline_plan() -> ModelPlan {
        ModelPlan::offline(&Panel::domain_ids(Ontology::bundled()))
    }

    #[test]
    fn source_markers_become_attributions() {
        let o = Ontology::bundled();
        let t = "baseline **118,151**【consult__baseline_data】.\nADU adds 12,760 【consult.adu】 and [consult.tod】 x【4†source】.";
        assert_eq!(
            clean_markers(o, t),
            "baseline **118,151** (Baseline, data and provenance expert).\nADU adds 12,760 (Accessory dwelling units expert) and (Transit-oriented density expert) x."
        );
        // Ordinary brackets and markdown are left alone.
        let plain = "See [the model](MODEL.md) and `pew_2026_adu` [1].";
        assert_eq!(clean_markers(o, plain), plain);
        assert_eq!(clean_markers(o, "a 【note】 b"), "a b");
    }

    #[test]
    fn manifests_record_each_agents_own_model() {
        use crate::models::{Backend, Host, Registry};
        use std::collections::BTreeMap;
        let per = BTreeMap::from([("adu".to_string(), "granite-3.3-8b".to_string())]);
        let plan = ModelPlan::resolve(
            Registry::bundled(),
            Backend::OpenAiCompat {
                host: Host::Ollama,
                base_url: "http://localhost:11434/v1".into(),
                api_key: None,
            },
            Some("workstation"),
            None,
            None,
            &per,
            &Panel::domain_ids(Ontology::bundled()),
        )
        .unwrap();
        assert_eq!(
            orchestrator_manifest(&plan).model_class.revision,
            "qwen3:32b"
        );
        assert_eq!(
            expert_manifest("adu", &plan).model_class.revision,
            "granite3.3:8b"
        );
        assert_eq!(
            expert_manifest("fees", &plan).model_class.revision,
            "qwen3:8b"
        );
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
        let issues = check_citations(o, answer, &[]);
        assert!(
            issues
                .iter()
                .any(|i| i.contains("`rose_2024_adu` is not a study")),
            "{issues:#?}"
        );
        assert!(
            issues
                .iter()
                .any(|i| i.contains("figure 4 next to marantz_elmendorf_kim_2023")),
            "{issues:#?}"
        );
        assert!(
            !issues.iter().any(|i| i.contains("pew_2026_adu")),
            "correct figures pass: {issues:#?}"
        );
        assert!(
            !issues.iter().any(|i| i.contains("terner_sb9_2021")),
            "correct figures pass: {issues:#?}"
        );
    }

    #[test]
    fn citation_check_accepts_the_states_own_baseline_figures() {
        let o = Ontology::bundled();
        let s = crate::state::find("CA").unwrap();
        let central = s.baseline().central.to_string();
        let line = format!(
            "Against a baseline of {} units, pew_2026_adu reports 81 per 100 000 residents.",
            crate::state::thousands(s.baseline().central)
        );
        assert!(check_citations(o, &line, &[])
            .iter()
            .any(|i| i.contains(&central)));
        assert!(check_citations(o, &line, &numbers(&s.profile())).is_empty());
    }

    #[test]
    fn citation_check_flags_figures_no_tool_produced() {
        let o = Ontology::bundled();
        let s = crate::state::find("PA").unwrap();
        let allowed = numbers(&s.profile());
        // From a real gpt-oss-120b answer: 2.3 per 1,000 is 230 per 100,000, not 23.
        let issues = check_citations(
            o,
            "| PA state profile | per 1 000 residents | 2.3 (≈ 23 per 100 000) |",
            &allowed,
        );
        assert!(
            issues.iter().any(|i| i.contains("figure 23 ")),
            "{issues:#?}"
        );
        let from_tool = numbers(&crate::state::scale_rate(s, 81.0));
        let mut ok = allowed.clone();
        ok.extend(from_tool);
        let scaled =
            crate::state::thousands((81.0 * s.population as f64 / 100_000.0 * 5.0).round() as u64);
        assert!(
            check_citations(o, &format!("Scaled: {scaled} units over five years."), &ok).is_empty()
        );
    }

    #[tokio::test]
    async fn state_questions_lead_with_the_state_baseline_and_required_sections() {
        let a = panel()
            .ask_for_state("How many ADUs could a state law add?", Some("PA"))
            .await
            .unwrap();
        assert!(
            a.text.contains("## Statewide baseline: Pennsylvania"),
            "{}",
            a.text
        );
        assert!(a.text.contains("FIVE-YEAR BASELINE"));
        assert!(a.text.contains("Applying this to Pennsylvania"));
        assert!(a.text.contains("## What this cannot tell you"));
        assert!(
            a.audit.iter().any(|l| l.contains("expert.adu")
                && l.contains("tool:state.profile")
                && l.contains("ALLOW")),
            "{:#?}",
            a.audit
        );
        assert_eq!(
            a.consulted,
            vec!["adu".to_string()],
            "the STATE line must not change routing"
        );
        assert!(panel().ask_for_state("q", Some("Narnia")).await.is_err());
    }

    #[test]
    fn numbers_are_normalized() {
        assert_eq!(
            numbers("81 per 100 000, 5.4% and 700,000"),
            vec!["81", "100000", "5.4", "700000"]
        );
    }

    #[tokio::test]
    async fn adu_question_is_answered_by_the_adu_expert_from_the_ontology() {
        let a = panel()
            .ask("How many ADUs could a statewide ADU law add in five years?")
            .await
            .unwrap();
        assert!(
            a.consulted.contains(&"adu".to_string()),
            "{:?}",
            a.consulted
        );
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
        assert!(a
            .audit
            .iter()
            .any(|l| l.contains("expert.fees") && l.contains("ALLOW")));
        assert!(
            a.audit.iter().all(|l| !l.contains("DENY")),
            "{:#?}",
            a.audit
        );
    }

    #[tokio::test]
    async fn an_expert_cannot_invoke_another_domains_tools() {
        let storage = InMemoryStorage::new();
        let kernel = Kernel::new(
            root_grants(),
            Arc::new(RuleSetPolicy::new()),
            MemoryAudit::new(),
            Arc::new(SystemClock),
        );
        let adm = kernel.admit(&orchestrator_manifest(&offline_plan()), None);
        let orch = kernel.attach(&adm, raw_services(ORCHESTRATOR, storage.clone()));
        orch.begin_activation("test", u64::MAX);
        let adu = orch
            .spawn_child(
                &expert_manifest("adu", &offline_plan()),
                raw_services("expert.adu", storage),
            )
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
        let kernel = Kernel::new(
            root_grants(),
            Arc::new(RuleSetPolicy::new()),
            MemoryAudit::new(),
            Arc::new(SystemClock),
        );
        let adm = kernel.admit(&orchestrator_manifest(&offline_plan()), None);
        let orch = kernel.attach(&adm, raw_services(ORCHESTRATOR, storage.clone()));
        orch.begin_activation("test", u64::MAX);
        let adu = orch
            .spawn_child(
                &expert_manifest("adu", &offline_plan()),
                raw_services("expert.adu", storage.clone()),
            )
            .await
            .unwrap();
        adu.begin_activation("test", u64::MAX);
        assert!(adu
            .spawn_child(
                &expert_manifest("fees", &offline_plan()),
                raw_services("expert.fees", storage)
            )
            .await
            .is_err());
    }
}
