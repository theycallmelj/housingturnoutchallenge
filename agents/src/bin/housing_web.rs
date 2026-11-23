//! Local web UI for the scenario model and the expert panel.
//!
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-web            # offline models
//!     cargo run --features llm --manifest-path agents/Cargo.toml --bin housing-web   # with HOUSING_LLM_* set
//!
//! Then open http://127.0.0.1:8787 (set `HOUSING_WEB_PORT` to change it).
//! Binds to localhost only.

use housing_agents::{ontology::Ontology, panel::Panel, web};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = std::env::var("HOUSING_WEB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8787);
    let panel: &'static Panel = Box::leak(Box::new(Panel::from_env(Ontology::bundled())?));
    web::serve(panel, port).await
}
