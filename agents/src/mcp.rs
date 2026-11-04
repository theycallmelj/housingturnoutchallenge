//! A minimal MCP server (JSON-RPC 2.0 over stdio, newline-delimited) exposing
//! the ontology queries as tools to any MCP client, e.g. Claude Code via the
//! repo's `.mcp.json`.
//!
//! Hand-rolled rather than built on an SDK: the server only needs
//! `initialize`, `tools/list` and `tools/call`, and keeping it dependency-free
//! keeps the SBOM small. [`handle`] is a pure request → response function so
//! it can be tested without a transport.

use crate::models::Backend;
use crate::ontology::Ontology;
use crate::panel::Panel;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const PROTOCOL_VERSION: &str = "2025-06-18";

fn tool_list() -> Value {
    json!([
        {
            "name": "route_query",
            "description": "Which expert domains (adu, missing_middle, tod, parking, fees, baseline_data, modeling) a housing policy question belongs to.",
            "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}, "required": ["query"]}
        },
        {
            "name": "evidence_brief",
            "description": "Evidence brief for one domain: scenario definition, mechanisms, most relevant studies (with outcome measure, evidence type and limitations), and which datasets are usable under the challenge license rules.",
            "inputSchema": {"type": "object", "properties": {"domain": {"type": "string"}, "query": {"type": "string"}}, "required": ["domain"]}
        },
        {
            "name": "find_studies",
            "description": "Search the 77 reviewed studies. Filter by domain and/or scenario; rank by query terms and evidence strength.",
            "inputSchema": {"type": "object", "properties": {
                "query": {"type": "string"},
                "domain": {"type": "string"},
                "scenario": {"type": "string", "enum": ["baseline", "adu", "missing_middle", "tod", "parking", "fees"]},
                "limit": {"type": "integer", "minimum": 1, "maximum": 25}
            }}
        },
        {
            "name": "find_datasets",
            "description": "List datasets with license status. usable_only=true keeps only open_verified sources the submission may depend on.",
            "inputSchema": {"type": "object", "properties": {
                "domain": {"type": "string"},
                "scenario": {"type": "string"},
                "usable_only": {"type": "boolean"}
            }}
        },
        {
            "name": "ask_experts",
            "description": "Ask the full expert panel: the orchestrator routes the question to the relevant domain experts (each limited to its own tools), then returns their combined answer and which experts were consulted.",
            "inputSchema": {"type": "object", "properties": {"question": {"type": "string"}}, "required": ["question"]}
        },
        {
            "name": "get_entity",
            "description": "Look up any ontology entity by id: study, dataset, scenario, domain, mechanism or outcome.",
            "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]}
        }
    ])
}

fn text_result(text: String, is_error: bool) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

fn call_tool(o: &Ontology, name: &str, args: &Value) -> Value {
    let s = |k: &str| args.get(k).and_then(Value::as_str);
    match name {
        "route_query" => {
            let q = s("query").unwrap_or("");
            let lines: Vec<String> = o
                .route(q)
                .into_iter()
                .map(|d| format!("{} ({}): {}", d.id, d.expert, d.description))
                .collect();
            text_result(lines.join("\n"), false)
        }
        "evidence_brief" => match s("domain").and_then(|d| o.evidence_brief(d, s("query").unwrap_or(""), 8)) {
            Some(t) => text_result(t, false),
            None => text_result(format!("unknown domain; expected one of: {}", o.domains.iter().map(|d| d.id.as_str()).collect::<Vec<_>>().join(", ")), true),
        },
        "find_studies" => {
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(8).clamp(1, 25) as usize;
            let studies = o.find_studies(s("domain"), s("scenario"), s("query").unwrap_or(""), limit);
            let text = studies.into_iter().map(|x| o.format_study(x)).collect::<Vec<_>>().join("\n");
            text_result(if text.is_empty() { "no matching studies".into() } else { text }, false)
        }
        "find_datasets" => {
            let usable_only = args.get("usable_only").and_then(Value::as_bool).unwrap_or(false);
            let ds = o.find_datasets(s("domain"), s("scenario"), usable_only);
            text_result(ds.into_iter().map(|d| o.format_dataset(d)).collect::<Vec<_>>().join("\n"), false)
        }
        "get_entity" => match s("id").and_then(|id| o.describe(id)) {
            Some(t) => text_result(t, false),
            None => text_result("no entity with that id".into(), true),
        },
        other => text_result(format!("unknown tool '{other}'"), true),
    }
}

/// Handle one JSON-RPC message. Returns `None` for notifications.
pub fn handle(o: &Ontology, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let id = id?; // notifications (no id) never get a response
    let result = match method {
        "initialize" => {
            let version = msg
                .pointer("/params/protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL_VERSION);
            json!({
                "protocolVersion": version,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "housing-ontology", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "Housing policy evidence ontology. Start with route_query, then evidence_brief for each domain. Report every number with its outcome measure (permits, starts, stock, feasible capacity...) and evidence type, and only rely on datasets marked USABLE."
            })
        }
        "ping" => json!({}),
        "tools/list" => json!({"tools": tool_list()}),
        "tools/call" => {
            let name = msg.pointer("/params/name").and_then(Value::as_str).unwrap_or("");
            let args = msg.pointer("/params/arguments").cloned().unwrap_or(json!({}));
            call_tool(o, name, &args)
        }
        _ => {
            return Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": format!("method not found: {method}")}
            }))
        }
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

/// [`handle`], plus `ask_experts`, which needs the async expert panel.
pub async fn handle_with_panel(o: &Ontology, panel: &Panel, msg: &Value) -> Option<Value> {
    let is_ask = msg.get("method").and_then(Value::as_str) == Some("tools/call")
        && msg.pointer("/params/name").and_then(Value::as_str) == Some("ask_experts");
    if !is_ask {
        return handle(o, msg);
    }
    let id = msg.get("id").cloned()?;
    let question = msg.pointer("/params/arguments/question").and_then(Value::as_str).unwrap_or("");
    let result = match panel.ask(question).await {
        Ok(a) => text_result(format!("{}\n\nExperts consulted: {}", a.text, a.consulted.join(", ")), false),
        Err(e) => text_result(format!("panel error: {e}"), true),
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

/// Serve on stdin/stdout until EOF.
pub async fn serve_stdio(o: &'static Ontology) -> anyhow::Result<()> {
    let panel = Panel::new(o, Backend::from_env())?;
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle_with_panel(o, &panel, &msg).await,
            Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}})),
        };
        if let Some(r) = response {
            stdout.write_all(r.to_string().as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o() -> &'static Ontology {
        Ontology::bundled()
    }

    #[test]
    fn initialize_echoes_protocol_version_and_advertises_tools() {
        let r = handle(o(), &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26"}})).unwrap();
        assert_eq!(r["result"]["protocolVersion"], "2025-03-26");
        assert!(r["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn notifications_get_no_response() {
        assert!(handle(o(), &json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none());
    }

    #[test]
    fn tools_list_has_input_schemas() {
        let r = handle(o(), &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})).unwrap();
        let tools = r["result"]["tools"].as_array().unwrap();
        assert!(tools.len() >= 5);
        assert!(tools.iter().all(|t| t["inputSchema"]["type"] == "object"));
    }

    #[test]
    fn tools_call_find_datasets_usable_only() {
        let r = handle(o(), &json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "find_datasets", "arguments": {"usable_only": true}}})).unwrap();
        let text = r["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("census_bps"));
        assert!(!text.contains("NOT usable"));
    }

    #[test]
    fn unknown_entity_is_a_tool_error_not_a_protocol_error() {
        let r = handle(o(), &json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": {"name": "get_entity", "arguments": {"id": "nope"}}})).unwrap();
        assert_eq!(r["result"]["isError"], true);
    }

    #[tokio::test]
    async fn ask_experts_runs_the_panel() {
        let panel = Panel::new(o(), Backend::Offline).unwrap();
        let r = handle_with_panel(o(), &panel, &json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call",
            "params": {"name": "ask_experts", "arguments": {"question": "Do duplex legalizations get built?"}}}))
            .await
            .unwrap();
        let text = r["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Experts consulted: missing_middle"), "{text}");
    }

    #[test]
    fn unknown_method_is_method_not_found() {
        let r = handle(o(), &json!({"jsonrpc": "2.0", "id": 5, "method": "resources/list"})).unwrap();
        assert_eq!(r["error"]["code"], -32601);
    }
}
