//! Run the scenario model directly: no agents, no language models.
//!
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA --scenario adu
//!     cargo run --manifest-path agents/Cargo.toml --bin housing-model -- --state PA --json
//!
//! Without `--scenario`, prints all five benchmark scenarios side by side,
//! then each one's full explanation. Every number comes from Census data and
//! `agents/assumptions/scenarios.json`.

use housing_agents::{engine::Assumptions, state};

fn main() -> anyhow::Result<()> {
    let mut state_arg = None;
    let mut scenario = None;
    let mut json = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--state" => state_arg = args.next(),
            "--scenario" => scenario = args.next(),
            "--json" => json = true,
            other => anyhow::bail!(
                "unknown argument {other}\nusage: housing-model --state <code or name> [--scenario adu|missing_middle|tod|parking|fees] [--json]"
            ),
        }
    }
    let q = state_arg.ok_or_else(|| anyhow::anyhow!("--state is required (e.g. --state PA)"))?;
    let s = state::find(&q).ok_or_else(|| anyhow::anyhow!("unknown state '{q}'"))?;
    let a = Assumptions::bundled();
    let ids: Vec<String> = match scenario {
        Some(id) => vec![id],
        None => a.scenarios.iter().map(|x| x.id.clone()).collect(),
    };
    let estimates = ids
        .iter()
        .map(|id| a.estimate(id, s))
        .collect::<anyhow::Result<Vec<_>>>()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&estimates)?);
        return Ok(());
    }
    if estimates.len() > 1 {
        println!("{}\n", a.compare(s));
    }
    for e in &estimates {
        println!("{}\n", a.explain(e, s));
    }
    println!(
        "Assumptions: agents/assumptions/scenarios.json ({})",
        a.meta.status
    );
    Ok(())
}
