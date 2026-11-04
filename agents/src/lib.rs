//! Housing policy expert agents built on condesate.
//!
//! * [`ontology`] — typed view of `ontology/housing-ontology.json` and its queries.
//! * [`tools`] — condesate `Tool`s over the ontology, named `<domain>.<verb>`.
//! * [`models`] — Apache-2.0 model registry, leader/expert model plans, offline models.
//! * `openai_compat` (feature `llm`) — provider for Ollama/vLLM/llama.cpp servers.
//! * [`panel`] — the orchestrator and one expert per domain, under condesate's kernel.
//! * [`mcp`] — a stdio MCP server exposing the same queries and the panel.

pub mod mcp;
pub mod models;
pub mod ontology;
#[cfg(feature = "llm")]
pub mod openai_compat;
pub mod panel;
pub mod tools;
