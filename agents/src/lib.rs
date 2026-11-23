//! Housing policy expert agents built on condesate.
//!
//! * [`ontology`] — typed view of `ontology/housing-ontology.json` and its queries.
//! * [`tools`] — condesate `Tool`s over the ontology, named `<domain>.<verb>`.
//! * [`models`] — Apache-2.0 model registry, leader/expert model plans, offline models.
//! * `openai_compat` (feature `llm`) — provider for Ollama/vLLM/llama.cpp servers.
//! * [`state`] — statewide permits, population and the five-year baseline.
//! * [`forecast`] — the baseline: a county negative binomial permit model, fitted by `housing-fit`.
//! * [`timing`] — permits -> starts -> completions curves from the Survey of Construction.
//! * [`panel`] — the orchestrator and one expert per domain, under condesate's kernel.
//! * [`engine`] — the scenario model: statewide five-year effects from `assumptions/scenarios.json`.
//! * [`evals`] — scored eval suite over the panel (offline in CI, live with a key).
//! * [`web`] — local web UI (`housing-web`) over the model and the panel.
//! * [`mcp`] — a stdio MCP server exposing the same queries and the panel.

pub mod engine;
pub mod evals;
pub mod forecast;
pub mod mcp;
pub mod models;
pub mod ontology;
#[cfg(feature = "llm")]
pub mod openai_compat;
pub mod panel;
pub mod state;
pub mod timing;
pub mod tools;
pub mod web;
