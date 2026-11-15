//! A `ModelProvider` for any OpenAI-compatible chat completions server
//! (Ollama, vLLM, llama.cpp `llama-server`), serving the open models in
//! `model-registry.json`.
//!
//! condesate's own `OpenAiModel` is fixed to api.openai.com and doesn't map
//! tool calls, so this provider builds the request itself:
//!
//! * the agent's tools are declared as **native function tools**, because
//!   models trained for function calling (gpt-oss, Qwen3) try to call them
//!   natively and some hosts fail the request if no tools were declared;
//! * native `tool_calls` in the reply become condesate `ToolCall`s;
//! * a plain-text `TOOL_CALL: <name> <json>` line is still accepted as a
//!   fallback for servers or models without native tool calling.

use anyhow::{Context, Result};
use async_trait::async_trait;
use condesate::{CompletionRequest, CompletionResponse, ModelProvider, Role, ToolCall, ToolSpec};
use serde_json::{json, Value};

const MAX_TOKENS: u32 = 2048;
const MAX_ATTEMPTS: u32 = 3;
/// Tool results are replayed to the model as user turns with this prefix.
const TOOL_RESULT_MARKER: &str = "[TOOL RESULT]";
const TOOL_CALL_PREFIX: &str = "TOOL_CALL:";

/// If the reply contains a tool call, reduce it to exactly one clean line,
/// `TOOL_CALL: <name> <json>`, and drop everything after the JSON. Models
/// sometimes repeat the call on the same line or keep writing an invented
/// tool result; condesate's parser would then miss the call entirely.
pub fn clean_tool_call(text: &str) -> String {
    let Some(start) = text.find(TOOL_CALL_PREFIX) else {
        return text.to_string();
    };
    let rest = text[start + TOOL_CALL_PREFIX.len()..].trim_start();
    let name_end = rest.find(|c: char| c.is_whitespace() || c == '{').unwrap_or(rest.len());
    let (name, after) = rest.split_at(name_end);
    if name.is_empty() {
        return text.to_string();
    }
    let after = after.trim_start();
    match serde_json::Deserializer::from_str(after).into_iter::<Value>().next() {
        Some(Ok(args)) => format!("{TOOL_CALL_PREFIX} {name} {args}"),
        _ if after.is_empty() || after.starts_with(TOOL_RESULT_MARKER) => format!("{TOOL_CALL_PREFIX} {name} {{}}"),
        _ => text.to_string(),
    }
}

/// Function names may only use `[a-zA-Z0-9_-]`, so `adu.evidence_brief`
/// travels as `adu__evidence_brief`.
pub fn wire_tool_name(name: &str) -> String {
    name.replace('.', "__")
}

/// condesate tool descriptions name their arguments as `{arg}`; turn those
/// into a JSON schema.
fn tool_parameters(description: &str) -> Value {
    let mut props = serde_json::Map::new();
    let mut required = Vec::new();
    let mut rest = description;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else { break };
        let arg = &rest[open + 1..open + close];
        rest = &rest[open + close + 1..];
        if arg.is_empty() || !arg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || props.contains_key(arg) {
            continue;
        }
        let kind = if arg.starts_with("usable") { "boolean" } else { "string" };
        props.insert(arg.to_string(), json!({ "type": kind }));
        if matches!(arg, "question" | "query" | "id") {
            required.push(arg.to_string());
        }
    }
    json!({ "type": "object", "properties": props, "required": required })
}

/// The chat completions body: transcript, declared tools, and limits.
pub fn build_body(model: &str, req: &CompletionRequest) -> Value {
    let messages: Vec<Value> = req
        .messages
        .iter()
        .map(|m| match m.role {
            Role::System => json!({ "role": "system", "content": m.content }),
            Role::User => json!({ "role": "user", "content": m.content }),
            Role::Assistant => json!({ "role": "assistant", "content": m.content }),
            // condesate doesn't keep tool-call ids, so results go back as
            // marked user turns rather than OpenAI `tool` messages.
            Role::Tool => json!({ "role": "user", "content": format!("{TOOL_RESULT_MARKER} {}", m.content) }),
        })
        .collect();
    let mut body = json!({
        "model": model,
        "messages": messages,
        "temperature": req.temperature,
        "max_tokens": MAX_TOKENS,
        "stream": false,
        // Stop before the model can write its own fake tool result.
        "stop": [TOOL_RESULT_MARKER],
    });
    if !req.tools.is_empty() {
        let tools: Vec<Value> = req
            .tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": wire_tool_name(&t.name),
                        "description": t.description,
                        "parameters": tool_parameters(&t.description),
                    }
                })
            })
            .collect();
        body["tools"] = tools.into();
        body["tool_choice"] = "auto".into();
    }
    body
}

/// Native tool calls first; otherwise a `TOOL_CALL:` text line; otherwise
/// the plain answer.
pub fn parse_response(value: &Value, tools: &[ToolSpec]) -> CompletionResponse {
    let msg = value.pointer("/choices/0/message").cloned().unwrap_or(Value::Null);
    let content = strip_reasoning(msg.get("content").and_then(Value::as_str).unwrap_or(""));
    let original = |wire: &str| {
        tools
            .iter()
            .find(|t| wire_tool_name(&t.name) == wire || t.name == wire)
            .map_or_else(|| wire.to_string(), |t| t.name.clone())
    };

    let native: Vec<ToolCall> = msg
        .get("tool_calls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let f = c.get("function")?;
            let name = original(f.get("name")?.as_str()?);
            let args = match f.get("arguments") {
                Some(Value::String(s)) => serde_json::from_str(s).unwrap_or_else(|_| json!({})),
                Some(v @ Value::Object(_)) => v.clone(),
                _ => json!({}),
            };
            Some(ToolCall { name, args })
        })
        .collect();
    if !native.is_empty() {
        let names: Vec<&str> = native.iter().map(|c| c.name.as_str()).collect();
        return CompletionResponse { content: format!("calling {}", names.join(", ")), tool_calls: native };
    }

    let cleaned = clean_tool_call(&content);
    if let Some(line) = cleaned.strip_prefix(TOOL_CALL_PREFIX) {
        let line = line.trim();
        let (name, args) = line.split_once(char::is_whitespace).unwrap_or((line, "{}"));
        if let Ok(args) = serde_json::from_str::<Value>(args.trim()) {
            return CompletionResponse {
                content: format!("calling {name}"),
                tool_calls: vec![ToolCall { name: original(name), args }],
            };
        }
    }
    CompletionResponse { content: cleaned, tool_calls: vec![] }
}

/// Set to print each raw model response summary to stderr.
pub const DEBUG_ENV: &str = "HOUSING_LLM_DEBUG";

pub struct OpenAiCompatModel {
    base_url: String,
    model: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl OpenAiCompatModel {
    /// `base_url` includes the API version prefix, e.g. `http://localhost:11434/v1`.
    pub fn new(base_url: &str, model: &str, api_key: Option<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

/// Drop `<think>...</think>` reasoning blocks (Qwen3, gpt-oss via some
/// servers) so the prompted tool-call parser only sees the final answer.
pub fn strip_reasoning(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        match rest[start..].find("</think>") {
            Some(end) => rest = &rest[start + end + "</think>".len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

#[async_trait]
impl ModelProvider for OpenAiCompatModel {
    fn name(&self) -> &str {
        &self.model
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let mut body = build_body(&self.model, &req);

        // Hosted routers (OpenRouter) occasionally return a choice that
        // failed mid-generation; a retry usually lands on another host.
        let mut last_err = None;
        let mut failed_hosts: Vec<String> = Vec::new();
        for attempt in 1..=MAX_ATTEMPTS {
            if self.is_openrouter() && !failed_hosts.is_empty() {
                // Steer the retry away from hosts that already failed.
                body["provider"] = serde_json::json!({ "ignore": failed_hosts });
            }
            match self.send(&body).await? {
                Ok(value) => return Ok(parse_response(&value, &req.tools)),
                Err((host, reason)) => {
                    failed_hosts.extend(host);
                    if std::env::var(DEBUG_ENV).is_ok() {
                        eprintln!("[{DEBUG_ENV}] attempt {attempt}/{MAX_ATTEMPTS} failed: {reason}");
                    }
                    last_err = Some(reason);
                }
            }
        }
        anyhow::bail!(
            "{} failed after {MAX_ATTEMPTS} attempts: {}",
            self.model,
            last_err.unwrap_or_default()
        )
    }
}

impl OpenAiCompatModel {
    /// One HTTP round trip. Outer `Err` is fatal (transport, HTTP status);
    /// inner `Err` is a failed generation worth retrying.
    fn is_openrouter(&self) -> bool {
        self.base_url.contains("openrouter.ai")
    }

    async fn send(&self, body: &Value) -> Result<std::result::Result<Value, (Option<String>, String)>> {
        let mut call = self.client.post(format!("{}/chat/completions", self.base_url)).json(body);
        if let Some(key) = &self.api_key {
            call = call.bearer_auth(key);
        }
        let resp = call
            .send()
            .await
            .with_context(|| format!("request to {} failed; is the model server running?", self.base_url))?;
        let status = resp.status();
        let value: Value = resp.json().await.context("model server returned non-JSON")?;
        if !status.is_success() {
            anyhow::bail!("model server returned {status}: {value}");
        }
        let choice = value.pointer("/choices/0");
        if std::env::var(DEBUG_ENV).is_ok() {
            eprintln!(
                "[{DEBUG_ENV}] model={} finish={} content={:?} tool_calls={} reasoning_chars={} provider={}",
                self.model,
                choice.and_then(|c| c.get("finish_reason")).unwrap_or(&Value::Null),
                choice.and_then(|c| c.pointer("/message/content")).unwrap_or(&Value::Null),
                choice.and_then(|c| c.pointer("/message/tool_calls")).unwrap_or(&Value::Null),
                choice.and_then(|c| c.pointer("/message/reasoning")).and_then(Value::as_str).map_or(0, str::len),
                value.get("provider").unwrap_or(&Value::Null),
            );
        }
        let host = value.get("provider").and_then(Value::as_str).map(str::to_string);
        Ok(generation_failure(&value).map_or(Ok(value), |reason| Err((host, reason))))
    }
}

/// Why a 200 response still carries no usable answer, if it doesn't.
pub fn generation_failure(value: &Value) -> Option<String> {
    let Some(choice) = value.pointer("/choices/0") else {
        return Some(format!("no choices in response: {value}"));
    };
    let finish = choice.get("finish_reason").and_then(Value::as_str).unwrap_or("");
    let content = choice.pointer("/message/content").and_then(Value::as_str).unwrap_or("");
    let has_calls = choice.pointer("/message/tool_calls").and_then(Value::as_array).is_some_and(|c| !c.is_empty());
    let provider = value.get("provider").and_then(Value::as_str).unwrap_or("unknown host");
    if finish == "error" {
        let detail = choice.get("error").map(|e| e.to_string()).unwrap_or_else(|| "no detail".into());
        return Some(format!("{provider} reported a generation error ({detail})"));
    }
    if content.trim().is_empty() && !has_calls {
        let hint = if finish == "length" { "; it ran out of tokens while reasoning" } else { "" };
        return Some(format!("{provider} returned an empty answer (finish_reason {finish:?}){hint}"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use condesate::Message;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn clean_tool_call_keeps_one_call_and_drops_invented_results() {
        let q = r#"{"question":"How many ADUs?"}"#;
        // Repeated on one line, then a fabricated result (observed from gpt-oss-120b).
        let raw = format!("TOOL_CALL: consult.adu {q}[TOOL RESULT] about 45,000 ADUs\nTOOL_CALL: consult.adu {q}");
        assert_eq!(clean_tool_call(&raw), format!("TOOL_CALL: consult.adu {q}"));
        assert_eq!(clean_tool_call(&format!("Sure.\nTOOL_CALL: consult.fees{q} trailing")), format!("TOOL_CALL: consult.fees {q}"));
        assert_eq!(clean_tool_call("TOOL_CALL: ontology.get"), "TOOL_CALL: ontology.get {}");
        assert_eq!(clean_tool_call("A plain answer."), "A plain answer.");
    }

    fn specs() -> Vec<ToolSpec> {
        vec![
            ToolSpec { name: "adu.evidence_brief".into(), description: "Evidence for {query}.".into() },
            ToolSpec { name: "adu.find_datasets".into(), description: "Datasets; set {usable_only}.".into() },
        ]
    }

    #[test]
    fn body_declares_tools_as_native_functions() {
        let req = CompletionRequest {
            messages: vec![Message::system("s"), Message::user("q"), Message::tool("42")],
            tools: specs(),
            temperature: 0.0,
        };
        let body = build_body("openai/gpt-oss-20b", &req);
        assert_eq!(body["tools"][0]["function"]["name"], "adu__evidence_brief");
        assert_eq!(body["tools"][0]["function"]["parameters"]["required"][0], "query");
        assert_eq!(body["tools"][1]["function"]["parameters"]["properties"]["usable_only"]["type"], "boolean");
        assert_eq!(body["tool_choice"], "auto");
        assert_eq!(body["messages"][2]["role"], "user");
        assert_eq!(body["messages"][2]["content"], "[TOOL RESULT] 42");
    }

    #[test]
    fn native_tool_calls_map_back_to_condesate_names() {
        let v = serde_json::json!({"choices":[{"finish_reason":"tool_calls","message":{"content":null,
            "tool_calls":[{"id":"1","type":"function","function":{"name":"adu__evidence_brief","arguments":"{\"query\":\"permits\"}"}}]}}]});
        assert!(generation_failure(&v).is_none(), "a tool call with no text is not a failure");
        let out = parse_response(&v, &specs());
        assert_eq!(out.tool_calls.len(), 1);
        assert_eq!(out.tool_calls[0].name, "adu.evidence_brief");
        assert_eq!(out.tool_calls[0].args["query"], "permits");
    }

    #[test]
    fn text_tool_call_is_a_fallback() {
        let v = serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":"TOOL_CALL: adu.evidence_brief {\"query\":\"x\"}[TOOL RESULT] made up"}}]});
        let out = parse_response(&v, &specs());
        assert_eq!(out.tool_calls[0].name, "adu.evidence_brief");
        let plain = serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":"Answer."}}]});
        assert!(parse_response(&plain, &specs()).tool_calls.is_empty());
    }

    #[test]
    fn generation_failure_spots_errors_and_empty_answers() {
        use serde_json::json;
        let ok = json!({"choices":[{"finish_reason":"stop","message":{"content":"hi"}}]});
        assert!(generation_failure(&ok).is_none());
        // Observed from OpenRouter: HTTP 200, finish_reason "error", null content.
        let err = json!({"provider":"Groq","choices":[{"finish_reason":"error","message":{"content":null}}]});
        assert!(generation_failure(&err).unwrap().contains("Groq reported a generation error"));
        let empty = json!({"choices":[{"finish_reason":"length","message":{"content":""}}]});
        assert!(generation_failure(&empty).unwrap().contains("ran out of tokens"));
    }

    #[test]
    fn strip_reasoning_removes_think_blocks() {
        assert_eq!(strip_reasoning("<think>TOOL_CALL: x {}</think>\nAnswer"), "Answer");
        assert_eq!(strip_reasoning("a<think>b</think>c<think>d</think>e"), "ace");
        assert_eq!(strip_reasoning("no reasoning"), "no reasoning");
        assert_eq!(strip_reasoning("<think>unterminated"), "");
    }

    /// One-shot HTTP server returning a canned chat completion, recording the request.
    async fn mock_server(reply: &'static str) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 64 * 1024];
            let mut req = String::new();
            loop {
                let n = sock.read(&mut buf).await.unwrap();
                req.push_str(&String::from_utf8_lossy(&buf[..n]));
                if let Some(i) = req.find("\r\n\r\n") {
                    let len: usize = req
                        .lines()
                        .find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap()))
                        .unwrap_or(0);
                    if req.len() >= i + 4 + len {
                        break;
                    }
                }
            }
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                reply.len()
            );
            sock.write_all(resp.as_bytes()).await.unwrap();
            req
        });
        (format!("http://{addr}/v1"), handle)
    }

    #[tokio::test]
    async fn posts_to_chat_completions_with_the_registry_tag() {
        let (url, server) =
            mock_server(r#"{"choices":[{"message":{"role":"assistant","content":"<think>hmm</think>Hello"}}]}"#).await;
        let model = OpenAiCompatModel::new(&url, "qwen3:4b", Some("k".into()));
        let req = CompletionRequest { messages: vec![Message::user("hi")], tools: vec![], temperature: 0.0 };
        let out = model.complete(req).await.unwrap();
        assert_eq!(out.content, "Hello");
        let raw = server.await.unwrap();
        assert!(raw.starts_with("POST /v1/chat/completions"), "{raw}");
        assert!(raw.contains("\"model\":\"qwen3:4b\""));
        assert!(raw.to_lowercase().contains("authorization: bearer k"));
        assert!(raw.contains(r#""stop":["[TOOL RESULT]"]"#), "stop sequence must be sent");
    }
}
