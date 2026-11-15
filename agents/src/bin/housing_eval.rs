//! Run the panel evals and write a report.
//!
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-eval
//!     cargo run --features llm --manifest-path agents/Cargo.toml --bin housing-eval -- \
//!         --min-pass-rate 0.7 --out eval-report.json --markdown eval-summary.md
//!
//! Offline unless `HOUSING_LLM_HOST`/`HOUSING_LLM_BASE_URL` select a model
//! backend (see agents/README.md). Exits 1 if the pass rate is below
//! `--min-pass-rate` (default 1.0, which offline runs must meet).

use housing_agents::{evals, ontology::Ontology, panel::Panel};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut cases = None;
    let mut out = None;
    let mut markdown = None;
    let mut filter = None;
    let mut min_pass_rate = 1.0_f64;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| anyhow::anyhow!("{a} needs a value"))
        };
        match a.as_str() {
            "--cases" => cases = Some(value()?),
            "--out" => out = Some(value()?),
            "--markdown" => markdown = Some(value()?),
            "--filter" => filter = Some(value()?),
            "--min-pass-rate" => min_pass_rate = value()?.parse()?,
            other => anyhow::bail!(
                "unknown argument {other}\nusage: housing-eval [--cases path] [--filter id] [--min-pass-rate 0.0-1.0] [--out report.json] [--markdown summary.md]"
            ),
        }
    }

    let suite = match &cases {
        Some(path) => evals::Suite::load(path)?,
        None => evals::Suite::bundled()?,
    };
    let panel = Panel::from_env(Ontology::bundled())?;
    eprintln!(
        "running {} eval case(s) with {}",
        suite.cases.len(),
        panel.plan().describe()
    );
    let report = evals::run(&panel, &suite, filter.as_deref()).await?;

    let summary = report.markdown(min_pass_rate);
    println!("{summary}");
    if let Some(path) = out {
        std::fs::write(&path, serde_json::to_string_pretty(&report)?)?;
        eprintln!("wrote {path}");
    }
    if let Some(path) = markdown {
        std::fs::write(&path, &summary)?;
        eprintln!("wrote {path}");
    }
    if report.pass_rate < min_pass_rate {
        std::process::exit(1);
    }
    Ok(())
}
