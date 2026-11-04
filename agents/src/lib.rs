//! Housing policy expert agents built on condesate.
//!
//! * [`ontology`] — typed view of `ontology/housing-ontology.json` and its queries.
//! * [`tools`] — condesate `Tool`s over the ontology, named `<domain>.<verb>`.
//! * [`mcp`] — a stdio MCP server exposing the same queries to any MCP client.

pub mod mcp;
pub mod ontology;
pub mod tools;
