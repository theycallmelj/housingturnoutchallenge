//! A small local web UI over the scenario model and the expert panel.
//!
//! Hand-rolled HTTP on tokio (already a dependency) so the SBOM doesn't
//! grow. Binds to 127.0.0.1 only. Routes:
//!
//! | route | returns |
//! | --- | --- |
//! | `GET /` | the single-page UI (`web/index.html`) |
//! | `GET /api/states` | every state's code and name |
//! | `GET /api/compare?state=PA` | baseline plus all five scenario estimates, with explanations |
//! | `POST /api/ask` `{question, state}` | the panel's answer, experts consulted, models, audit trail, citation check |

use crate::engine::Assumptions;
use crate::ontology::Ontology;
use crate::panel::Panel;
use crate::state;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

const INDEX: &str = include_str!("../web/index.html");
const MAX_BODY: usize = 64 * 1024;

/// A response: status, content type, body.
pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

fn json_reply(status: u16, v: Value) -> Reply {
    Reply {
        status,
        content_type: "application/json",
        body: v.to_string(),
    }
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix(key)?.strip_prefix('='))
}

/// Routes that don't need the panel. Pure, so it is unit-tested directly.
pub fn route_static(method: &str, path: &str) -> Option<Reply> {
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    match (method, path) {
        ("GET", "/") => Some(Reply {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: INDEX.to_string(),
        }),
        ("GET", "/api/states") => Some(json_reply(
            200,
            json!(state::all()
                .iter()
                .map(|s| json!({"abbr": s.abbr, "name": s.name}))
                .collect::<Vec<_>>()),
        )),
        ("GET", "/api/compare") => {
            let Some(s) = query_param(query, "state").and_then(state::find) else {
                return Some(json_reply(
                    400,
                    json!({"error": "unknown or missing state"}),
                ));
            };
            let a = Assumptions::bundled();
            let cond = crate::forecast::Conditions::from_args(&serde_json::json!({
                "rate_shift_pp": query_param(query, "rate"),
                "migration_factor": query_param(query, "migration"),
                "price_shift_pp": query_param(query, "price"),
            }));
            let b = s.baseline_with(&cond);
            let flow = s.baseline_flow(&cond);
            let pace = s.recent_pace();
            let bt = &crate::forecast::model().backtest;
            let scenarios: Vec<Value> = a
                .scenarios
                .iter()
                .filter_map(|spec| a.estimate_with(&spec.id, s, &cond).ok())
                .map(|e| {
                    json!({
                        "id": e.scenario,
                        "label": e.label,
                        "low": e.low.round(),
                        "central": e.central.round(),
                        "high": e.high.round(),
                        "central_by_year": e.central_by_year.iter().map(|v| v.round()).collect::<Vec<_>>(),
                        "completed": {"low": e.completed_low.round(), "central": e.completed_central.round(), "high": e.completed_high.round()},
                        "by_year": {
                            "permits": e.added.permits.iter().map(|v| v.round()).collect::<Vec<_>>(),
                            "starts": e.added.starts.iter().map(|v| v.round()).collect::<Vec<_>>(),
                            "completions": e.added.completions.iter().map(|v| v.round()).collect::<Vec<_>>(),
                        },
                        "exposure_factor": e.exposure_factor,
                        "top_counties": e.top_counties.iter().map(|(n, v)| json!({"county": n, "units": v.round()})).collect::<Vec<_>>(),
                        "top_sensitivity": e.sensitivity.first().map(|x| x.label.clone()),
                        "explanation": a.explain(&e, s),
                        "details": a.details(&e),
                    })
                })
                .collect();
            Some(json_reply(
                200,
                json!({
                    "state": {"abbr": s.abbr, "name": s.name, "population": s.population},
                    "permits": s.permits.iter().map(|(y, n)| json!({"year": y, "units": n})).collect::<Vec<_>>(),
                    "baseline": {"low": b.low, "central": b.central, "high": b.high},
                    "baseline_completed": flow.total_completions().round(),
                    "baseline_by_year": {
                        "years": (0..flow.permits.len()).map(|i| crate::forecast::LAST_YEAR + 1 + i as u32).collect::<Vec<_>>(),
                        "permits": flow.permits.iter().map(|v| v.round()).collect::<Vec<_>>(),
                        "starts": flow.starts.iter().map(|v| v.round()).collect::<Vec<_>>(),
                        "completions": flow.completions.iter().map(|v| v.round()).collect::<Vec<_>>(),
                    },
                    "recent_pace": pace.central,
                    "conditions": {"description": cond.describe(), "rate_shift_pp": cond.rate_shift_pp, "migration_factor": cond.migration_factor, "price_shift_pp": cond.price_shift_pp},
                    "backtest": {"median_abs_pct_error": bt.model.median_abs_pct_error, "recent_pace_median_abs_pct_error": bt.recent_pace.median_abs_pct_error, "within_20pct": bt.model.within_20pct},
                    "counties": s.counties().len(),
                    "outcome": a.meta.outcome,
                    "status": a.meta.status,
                    "studies": studies_in(&scenarios.iter().map(|v| v["details"]["assumptions"].to_string()).collect::<String>()),
                    "scenarios": scenarios,
                }),
            ))
        }
        ("POST", "/api/ask") => None,
        _ => Some(json_reply(404, json!({"error": "not found"}))),
    }
}

/// Title, authors, year and link for every ontology study id that appears in
/// `text`, so the page can show a readable citation instead of the bare id.
fn studies_in(text: &str) -> Value {
    let mut out = serde_json::Map::new();
    for st in &Ontology::bundled().studies {
        if text.contains(st.id.as_str()) {
            out.insert(
                st.id.clone(),
                json!({"title": st.title, "authors": st.authors, "year": st.year, "url": st.url, "evidence_type": st.evidence_type, "finding": st.finding}),
            );
        }
    }
    Value::Object(out)
}

async fn ask(panel: &Panel, body: &str) -> Reply {
    let Ok(req) = serde_json::from_str::<Value>(body) else {
        return json_reply(400, json!({"error": "body must be JSON {question, state}"}));
    };
    let question = req
        .get("question")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if question.is_empty() {
        return json_reply(400, json!({"error": "question is required"}));
    }
    let st = req.get("state").and_then(Value::as_str);
    match panel.ask_for_state(question, st).await {
        Ok(a) => json_reply(
            200,
            json!({
                "text": a.text,
                "consulted": a.consulted,
                "models": a.models,
                "audit": a.audit,
                "citation_issues": a.citation_issues,
                "experts": a.consulted.iter().map(|id| json!({
                    "id": id,
                    "label": Ontology::bundled().domain(id).map_or(id.as_str(), |d| d.label.as_str()),
                })).collect::<Vec<_>>(),
                "studies": studies_in(&a.text),
            }),
        ),
        Err(e) => json_reply(500, json!({"error": e.to_string()})),
    }
}

async fn handle(panel: &Panel, stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;
    let mut parts = request_line.split_whitespace();
    let (method, path) = (
        parts.next().unwrap_or("").to_string(),
        parts.next().unwrap_or("/").to_string(),
    );

    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
    }
    let reply = if content_length > MAX_BODY {
        json_reply(413, json!({"error": "request too large"}))
    } else if let Some(r) = route_static(&method, &path) {
        r
    } else {
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).await?;
        ask(panel, &String::from_utf8_lossy(&body)).await
    };
    let head = format!(
        "HTTP/1.1 {} {}\r\ncontent-type: {}\r\ncontent-length: {}\r\ncache-control: no-store\r\nx-content-type-options: nosniff\r\nconnection: close\r\n\r\n",
        reply.status,
        match reply.status {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            413 => "Payload Too Large",
            _ => "Internal Server Error",
        },
        reply.content_type,
        reply.body.len()
    );
    write.write_all(head.as_bytes()).await?;
    write.write_all(reply.body.as_bytes()).await?;
    write.flush().await?;
    Ok(())
}

/// Serve until the process is stopped.
pub async fn serve(panel: &'static Panel, port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    eprintln!(
        "housing-web: http://127.0.0.1:{port}  ({})",
        panel.plan().describe()
    );
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle(panel, stream).await {
                eprintln!("housing-web: {e}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_and_states_are_served() {
        let r = route_static("GET", "/").unwrap();
        assert_eq!(r.status, 200);
        assert!(r.body.contains("<title>"));
        let r = route_static("GET", "/api/states").unwrap();
        let v: Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 51);
    }

    #[test]
    fn compare_returns_baseline_and_five_scenarios() {
        let r = route_static("GET", "/api/compare?state=PA").unwrap();
        assert_eq!(r.status, 200);
        let v: Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["scenarios"].as_array().unwrap().len(), 5);
        assert_eq!(
            v["baseline"]["central"],
            state::find("PA").unwrap().baseline().central
        );
        assert!(v["scenarios"][0]["explanation"]
            .as_str()
            .unwrap()
            .starts_with("MODEL ESTIMATE"));
        assert_eq!(v["counties"], 67);
        for sc in v["scenarios"].as_array().unwrap() {
            let done = sc["completed"]["central"].as_f64().unwrap();
            assert!(done > 0.0 && done <= sc["central"].as_f64().unwrap());
        }
        assert!(v["scenarios"][0]["top_counties"].as_array().unwrap().len() == 5);
        // Structured details for the page, and every cited study resolves.
        for sc in v["scenarios"].as_array().unwrap() {
            let d = &sc["details"];
            assert!(!d["reform"].as_str().unwrap().is_empty());
            assert!(!d["sensitivity"].as_array().unwrap().is_empty());
            for a in d["assumptions"].as_array().unwrap() {
                for id in a["sources"].as_array().unwrap() {
                    assert!(
                        v["studies"][id.as_str().unwrap()]["title"].is_string(),
                        "{id}"
                    );
                }
            }
        }
    }

    #[test]
    fn bad_requests_are_rejected() {
        assert_eq!(
            route_static("GET", "/api/compare?state=Narnia")
                .unwrap()
                .status,
            400
        );
        assert_eq!(route_static("GET", "/nope").unwrap().status, 404);
        assert!(
            route_static("POST", "/api/ask").is_none(),
            "ask is handled by the panel"
        );
    }

    #[tokio::test]
    async fn ask_runs_the_panel() {
        let panel = Panel::offline(crate::ontology::Ontology::bundled());
        let r = ask(
            &panel,
            r#"{"question":"How many ADUs could a state law add?","state":"PA"}"#,
        )
        .await;
        assert_eq!(r.status, 200);
        let v: Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["consulted"][0], "adu");
        assert!(!v["audit"].as_array().unwrap().is_empty());
        assert_eq!(ask(&panel, "{}").await.status, 400);
    }
}
