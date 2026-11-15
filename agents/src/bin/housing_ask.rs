//! Ask the expert panel a question from the command line.
//!
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-ask -- "How many ADUs would a state law add?"
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-ask -- --state PA "How many ADUs?"
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-ask -- --audit --state CA "..."
//!
//! The challenge asks for statewide results, so pass `--state` (USPS code or
//! name). Without it the panel answers from the evidence base alone.
//!
//! Runs offline by default. To use open Apache-2.0 models, build with
//! `--features llm` and point `HOUSING_LLM_BASE_URL` at an OpenAI-compatible
//! server (e.g. `http://localhost:11434/v1` for Ollama). Pick models with
//! `HOUSING_MODEL_PROFILE`, `HOUSING_LEADER_MODEL`, `HOUSING_EXPERT_MODEL` and
//! `HOUSING_EXPERT_MODEL_<DOMAIN>`; see agents/model-registry.json.

use housing_agents::{ontology::Ontology, panel::Panel};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut show_audit = false;
    let mut state = None;
    let mut words = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--audit" => show_audit = true,
            "--state" => state = args.next(),
            _ => words.push(a),
        }
    }
    let question = words.join(" ");
    if question.trim().is_empty() {
        eprintln!("usage: housing-ask [--audit] [--state <code or name>] <question>");
        std::process::exit(2);
    }
    if state.is_none() {
        eprintln!("note: no --state given; the challenge asks for statewide results");
    }

    let answer = Panel::from_env(Ontology::bundled())?.ask_for_state(&question, state.as_deref()).await?;
    println!("{}", answer.text);
    println!("\n(models: {}; experts consulted: {})", answer.models, answer.consulted.join(", "));
    if !answer.citation_issues.is_empty() {
        eprintln!("warning: {} citation issue(s); see the Citation check section", answer.citation_issues.len());
    }
    if show_audit {
        println!("\n--- permission audit trail ---");
        for line in &answer.audit {
            println!("{line}");
        }
    }
    Ok(())
}
