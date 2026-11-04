//! Ask the expert panel a question from the command line.
//!
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-ask -- "How many ADUs would a state law add?"
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-ask -- --audit "..."
//!
//! Runs offline by default. Build with `--features llm` and set
//! `ANTHROPIC_API_KEY` (and optionally `HOUSING_AGENT_MODEL`) to use Claude.

use housing_agents::{models::Backend, ontology::Ontology, panel::Panel};

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

    let backend = Backend::from_env();
    let answer = Panel::new(Ontology::bundled(), backend)?.ask(&question).await?;
    println!("{}", answer.text);
    println!("\n(backend: {backend:?}; experts consulted: {})", answer.consulted.join(", "));
    if show_audit {
        println!("\n--- permission audit trail ---");
        for line in &answer.audit {
            println!("{line}");
        }
    }
    Ok(())
}
