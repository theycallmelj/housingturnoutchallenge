# Dependency license review (housing-agents)

Reviewed 2026-11-03 from `cargo metadata` and the CycloneDX SBOM ([`sbom.cdx.json`](sbom.cdx.json)).

## Default build (what the submission ships)

All 26 SBOM components are permissive (MIT and/or Apache-2.0, plus Unicode-3.0 for `unicode-ident`). Where a crate offers a choice (e.g. `MIT OR Apache-2.0 OR LGPL-2.1-or-later`, `Unlicense OR MIT`), we take it under **MIT**.

| Direct dependency | Version | License | Purpose |
| --- | --- | --- | --- |
| condesate | 0.1.0 | Apache-2.0 | Agent loop, tools, permission kernel, audit (team member's crate: github.com/theycallmelj/ai-swarm) |
| tokio | 1 | MIT | Async runtime, stdio for the MCP server |
| async-trait | 0.1 | MIT OR Apache-2.0 | Async trait objects for condesate's traits |
| anyhow | 1 | MIT OR Apache-2.0 | Errors |
| serde | 1 | MIT OR Apache-2.0 | Ontology deserialization |
| serde_json | 1 | MIT OR Apache-2.0 | Ontology JSON, MCP JSON-RPC |

## Optional `llm` feature: flagged exception (Official Rules §7.b.ii)

`--features llm` enables `condesate/remote`, which adds `reqwest` 0.11 with `rustls-tls`. That brings in **`webpki-roots` 0.25.4, licensed MPL-2.0** (Mozilla's root CA certificate bundle). MPL-2.0 is a weak, file-level copyleft and isn't on the challenge's permissive list.

**Status: flagged, unresolved. A team decision is needed.** Options:

1. Ship with `llm` off (the default). The offline panel needs no network. The `llm` feature stays a local development aid.
2. Ask the organizers whether an unmodified MPL-2.0 certificate bundle in an optional feature is acceptable.
3. Upgrade `condesate`'s `remote` feature to use the OS certificate store (`rustls-tls-native-roots` or `native-tls`), which drops `webpki-roots`.

Until this is resolved, the committed SBOM covers the default build only.
