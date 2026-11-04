//! Ask the expert panel a question from the command line.
//!
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-ask -- "How many ADUs would a state law add?"
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-ask -- --audit "..."
//!
//! Runs offline by default. To use open Apache-2.0 models, build with
//! `--features llm` and point `HOUSING_LLM_BASE_URL` at an OpenAI-compatible
//! server (e.g. `http://localhost:11434/v1` for Ollama). Pick models with
//! `HOUSING_MODEL_PROFILE`, `HOUSING_LEADER_MODEL`, `HOUSING_EXPERT_MODEL` and
//! `HOUSING_EXPERT_MODEL_<DOMAIN>`; see agents/model-registry.json.

use housing_agents::{ontology::Ontology, panel::Panel};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let show_audit = args.first().is_some_and(|a| a == "--audit");
    if show_audit {
        args.remove(0);
    }
    let question = args.join(" ");
    if question.trim().is_empty() {
        eprintln!("usage: housing-ask [--audit] <question>");
        std::process::exit(2);
    }

    let answer = Panel::from_env(Ontology::bundled())?.ask(&question).await?;
    println!("{}", answer.text);
    println!("\n(models: {}; experts consulted: {})", answer.models, answer.consulted.join(", "));
    if show_audit {
        println!("\n--- permission audit trail ---");
        for line in &answer.audit {
            println!("{line}");
        }
    }
    Ok(())
}
