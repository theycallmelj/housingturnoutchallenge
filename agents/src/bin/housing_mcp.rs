//! MCP server over stdio exposing the housing policy ontology.
//!
//! Registered for Claude Code in the repo's `.mcp.json`; any MCP client can
//! launch it with `cargo run --release --manifest-path agents/Cargo.toml --bin housing-mcp`.

use housing_agents::{mcp, ontology::Ontology};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp::serve_stdio(Ontology::bundled()).await
}
