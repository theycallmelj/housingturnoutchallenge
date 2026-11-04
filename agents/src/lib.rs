//! Housing policy expert agents built on condesate.
//!
//! * [`ontology`] — typed view of `ontology/housing-ontology.json` and its queries.
//! * [`tools`] — condesate `Tool`s over the ontology, named `<domain>.<verb>`.
//! * [`models`] — offline deterministic models, or Anthropic with the `llm` feature.
//! * [`panel`] — the orchestrator and one expert per domain, under condesate's kernel.
//! * [`mcp`] — a stdio MCP server exposing the same queries and the panel.

pub mod mcp;
pub mod models;
pub mod ontology;
pub mod panel;
pub mod tools;
