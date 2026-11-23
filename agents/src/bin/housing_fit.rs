//! Fit the baseline permit model and back-test it; writes
//! `data/derived/baseline_model.json`.
//!
//!     cargo run --release --manifest-path agents/Cargo.toml --bin housing-fit
//!     cargo run --release --manifest-path agents/Cargo.toml --bin housing-fit -- --check
//!
//! `--check` refits from the committed panel and fails if the result differs
//! from the committed model file (relative tolerance 1e-4), so CI catches a
//! model file that is out of date with its data or code.

use housing_agents::forecast;
use serde_json::Value;

const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../data/derived/baseline_model.json"
);

fn diffs(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap_or(0.0), y.as_f64().unwrap_or(0.0));
            if (x - y).abs() > 1e-6 + 1e-4 * x.abs().max(y.abs()) {
                out.push(format!("{path}: {x} vs {y}"));
            }
        }
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                match y.get(k) {
                    Some(w) => diffs(v, w, &format!("{path}.{k}"), out),
                    None => out.push(format!("{path}.{k}: missing")),
                }
            }
            for k in y.keys().filter(|k| !x.contains_key(*k)) {
                out.push(format!("{path}.{k}: extra"));
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (v, w)) in x.iter().zip(y).enumerate() {
                diffs(v, w, &format!("{path}[{i}]"), out);
            }
        }
        _ if a != b => out.push(format!("{path}: differs")),
        _ => {}
    }
}

fn main() -> anyhow::Result<()> {
    let check = std::env::args().any(|a| a == "--check");
    let started = std::time::Instant::now();
    let m = forecast::build_model_file(forecast::panel());
    let fresh = serde_json::to_value(&m)?;
    eprintln!(
        "fitted and back-tested in {:.1}s",
        started.elapsed().as_secs_f64()
    );
    let b = &m.backtest;
    eprintln!(
        "back-test, five-year state totals ({} state x origin pairs, origins {:?}):\n  model:       median error {:.1}%, mean {:.1}%, within 10% {:.0}%, within 20% {:.0}%, median actual/predicted {:.3}\n  recent pace: median error {:.1}%, mean {:.1}%, within 10% {:.0}%, within 20% {:.0}%, median actual/predicted {:.3}",
        b.model.n,
        b.origins,
        100.0 * b.model.median_abs_pct_error,
        100.0 * b.model.mean_abs_pct_error,
        100.0 * b.model.within_10pct,
        100.0 * b.model.within_20pct,
        b.model.median_log_ratio.exp(),
        100.0 * b.recent_pace.median_abs_pct_error,
        100.0 * b.recent_pace.mean_abs_pct_error,
        100.0 * b.recent_pace.within_10pct,
        100.0 * b.recent_pace.within_20pct,
        b.recent_pace.median_log_ratio.exp(),
    );
    for r in &b.national {
        eprintln!(
            "  U.S., {}-{}: actual {:.0}, model {:.0} ({:+.1}%), recent pace {:.0} ({:+.1}%)",
            r.origin + 1,
            r.origin + 5,
            r.actual,
            r.model,
            100.0 * (r.model / r.actual - 1.0),
            r.recent_pace,
            100.0 * (r.recent_pace / r.actual - 1.0)
        );
    }
    for (name, g) in [("1-unit", &m.single), ("2+ units", &m.multi)] {
        eprintln!(
            "{name}: theta {:.3}, sd state {:.3}, sd county {:.3}, n {}",
            g.theta, g.sd_state, g.sd_county, g.n_obs
        );
        for (k, v) in &g.coefficients {
            eprintln!("    {k:<28} {v:>10.5}");
        }
    }
    if check {
        let committed: Value = serde_json::from_str(&std::fs::read_to_string(PATH)?)?;
        let mut out = Vec::new();
        diffs(&fresh, &committed, "model", &mut out);
        if !out.is_empty() {
            for d in out.iter().take(20) {
                eprintln!("  {d}");
            }
            anyhow::bail!(
                "{} values differ from {PATH}; rerun housing-fit and commit",
                out.len()
            );
        }
        eprintln!("committed model matches a fresh fit");
    } else {
        std::fs::write(PATH, serde_json::to_string_pretty(&fresh)? + "\n")?;
        eprintln!("wrote {PATH}");
    }
    Ok(())
}
