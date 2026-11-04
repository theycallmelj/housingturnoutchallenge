//! A `ModelProvider` for any OpenAI-compatible chat completions server
//! (Ollama, vLLM, llama.cpp `llama-server`), serving the open models in
//! `model-registry.json`.
//!
//! condesate's own `OpenAiModel` is fixed to api.openai.com, so this reuses
//! its pure request/response mapping (`condesate::agent::wire`) with a
//! configurable base URL. Tool calls go through `PromptedToolModel` (see
//! `models.rs`), which needs no server-side tool-calling support.

use anyhow::{Context, Result};
use async_trait::async_trait;
use condesate::agent::wire;
use condesate::{CompletionRequest, CompletionResponse, ModelProvider};
use serde_json::Value;

const MAX_TOKENS: u32 = 2048;

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
        let mut body = wire::build_openai_body(&self.model, &req);
        body["max_tokens"] = MAX_TOKENS.into();
        body["stream"] = false.into();
        let mut call = self.client.post(format!("{}/chat/completions", self.base_url)).json(&body);
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
        let mut out = wire::parse_openai_response(&value)?;
        out.content = strip_reasoning(&out.content);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use condesate::Message;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
    }
}
