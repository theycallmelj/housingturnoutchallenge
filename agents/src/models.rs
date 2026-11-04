//! Model providers for the orchestrator and the experts.
//!
//! The offline models are deterministic and answer only from tool results,
//! so the whole panel runs, and is tested, without network access. With the
//! `llm` feature and `ANTHROPIC_API_KEY` set, [`provider`] returns a real
//! Anthropic model wrapped in condesate's prompted tool-calling adapter
//! instead; nothing above this module changes.

use crate::ontology::Ontology;
use anyhow::Result;
use async_trait::async_trait;
use condesate::{CompletionRequest, CompletionResponse, ModelProvider, Role, ToolCall};
use serde_json::json;
use std::sync::Arc;

/// Environment variable naming the Anthropic model for the `llm` feature.
pub const MODEL_ENV: &str = "HOUSING_AGENT_MODEL";
pub const DEFAULT_MODEL: &str = "claude-sonnet-5-5";

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

/// Which provider family to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Offline,
    #[cfg(feature = "llm")]
    Anthropic,
}

impl Backend {
    /// `Anthropic` when built with `llm` and a key is present, else `Offline`.
    pub fn from_env() -> Self {
        #[cfg(feature = "llm")]
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            return Backend::Anthropic;
        }
        Backend::Offline
    }
}

/// A real model for the `llm` feature, or the given offline fallback.
pub fn provider(backend: Backend, offline: Arc<dyn ModelProvider>) -> Result<Arc<dyn ModelProvider>> {
    match backend {
        Backend::Offline => Ok(offline),
        #[cfg(feature = "llm")]
        Backend::Anthropic => {
            let model = std::env::var(MODEL_ENV).unwrap_or_else(|_| DEFAULT_MODEL.to_string());
            let inner = Arc::new(condesate::AnthropicModel::from_env(model)?);
            Ok(Arc::new(condesate::PromptedToolModel::new(inner)))
        }
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
}
