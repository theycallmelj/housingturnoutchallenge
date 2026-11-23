# Dependency and model license review (housing-agents)

Reviewed 2026-11-03 from `cargo metadata` and the CycloneDX SBOM ([`sbom.cdx.json`](sbom.cdx.json), generated with `--all-features`).

## Crates

All 143 SBOM components, including the optional `llm` feature and platform-specific crates for every target (`--target all`, so the SBOM is the same on macOS, Linux and Windows), are permissive: MIT and/or Apache-2.0, plus ISC (`ring`, `rustls-webpki`), BSD, Zlib, and Unicode-3.0. Where a crate offers a choice (e.g. `r-efi`: `MIT OR Apache-2.0 OR LGPL-2.1-or-later`; `Unlicense OR MIT`), we take it under **MIT**. No component is MPL, GPL, CDLA, or OpenSSL-only.

| Direct dependency | Version | License | Purpose |
| --- | --- | --- | --- |
| condesate | 0.1.0 | Apache-2.0 | Agent loop, tools, permission kernel, audit (team member's crate: github.com/theycallmelj/ai-swarm). The `remote` feature is **not** used. |
| tokio | 1 | MIT | Async runtime; stdio for the MCP server |
| async-trait | 0.1 | MIT OR Apache-2.0 | Async trait objects for condesate's traits |
| anyhow | 1 | MIT OR Apache-2.0 | Errors |
| serde / serde_json | 1 | MIT OR Apache-2.0 | Ontology and registry JSON, MCP JSON-RPC |
| reqwest (optional, `llm`) | 0.12 | MIT OR Apache-2.0 | HTTP client for OpenAI-compatible model servers. Uses `rustls-tls-native-roots`, which reads the OS certificate store; no bundled CA list. |

**Resolved:** the earlier `llm` feature went through `condesate/remote` and `reqwest` 0.11, which bundled `webpki-roots` (MPL-2.0). It has been replaced, so that crate is no longer in the tree.

## Models (Official Rules §7.b.ii, §7.c)

The panel may only run models listed in [`model-registry.json`](model-registry.json) with license **Apache-2.0**. Each license was read from the model's Hugging Face card metadata on 2026-11-03. `ModelPlan::resolve` refuses any other model, and tests cover this.

| Registry id | Hugging Face repo | License |
| --- | --- | --- |
| qwen3-0.6b … qwen3-235b-a22b | Qwen/Qwen3-* | Apache-2.0 |
| gpt-oss-20b, gpt-oss-120b | openai/gpt-oss-* | Apache-2.0 |
| mistral-small-3.2-24b | mistralai/Mistral-Small-3.2-24B-Instruct-2506 | Apache-2.0 |
| granite-3.3-8b | ibm-granite/granite-3.3-8b-instruct | Apache-2.0 |
| **Rejected:** llama-3.1-8b, gemma-3-4b, claude | | llama3.1, gemma, proprietary |

Model weights aren't committed or bundled. Users download them through their own model server, e.g. `ollama pull qwen3:4b`. The model server (Ollama, MIT; vLLM, Apache-2.0; llama.cpp, MIT) is an external tool, not a dependency of this crate.
