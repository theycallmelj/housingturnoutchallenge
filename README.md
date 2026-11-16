# Housing Turnout Challenge

[![Rust (agents)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/rust.yml/badge.svg)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/rust.yml)
[![Web prototype](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/web.yml/badge.svg)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/web.yml)
[![CodeQL](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/codeql.yml/badge.svg)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/codeql.yml)
[![LLM evals](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/llm-evals.yml/badge.svg)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/llm-evals.yml)

Team repository for the [Housing Data and Technology Innovation Challenge](https://housingchallenge.turnout.rocks/handbook.html): open-source tools estimating how state housing policy reforms change housing production against a five-year baseline.

- [`AGENTS.md`](AGENTS.md): contribution rules (SBOM, data provenance, AI disclosure, licensing). Read it first.
- [`prototypes/`](prototypes/): UI and engine prototypes
- [`research/`](research/README.md): literature index and data-source provenance
- [`ontology/`](ontology/README.md): evidence ontology (77 studies, 41 datasets with license status)
- [`MODEL.md`](MODEL.md): **the scenario model**: statewide five-year effects of the five benchmark reforms for any state, with ranges, sensitivity and sources (`housing-model --state XX`)
- [`agents/`](agents/README.md): condesate expert panel (`housing-ask`) and MCP server (`housing-mcp`, registered in `.mcp.json`)
- [`.claude/skills/`](.claude/skills/): one evidence skill per policy domain
- [`AI_DISCLOSURE.md`](AI_DISCLOSURE.md): AI tool use log

## Continuous integration

| Workflow | Runs | What it checks |
| --- | --- | --- |
| `rust.yml` | every push and PR | `cargo fmt`, `clippy -D warnings` (with and without `llm`), tests (both feature sets), offline panel evals (100% required), and that `scripts/fetch_state_baseline.py` and `scripts/fetch_construction_timing.py` reproduce the committed county, state, and completion tables |
| `web.yml` | every push and PR | React prototype: frozen-lockfile install, unit tests, typecheck + production build |
| `llm-evals.yml` | on demand, weekly, PRs labelled `llm-evals` | Live-model panel evals on OpenRouter; needs the `OPENROUTER_API_KEY` repository secret |
| `codeql.yml` | every push, PR, weekly | Security analysis for Rust, TypeScript, and the workflows themselves (eligibility gate) |
| `dependabot.yml` | weekly | Cargo, npm, and GitHub Actions updates (eligibility gate) |

One-time repository settings: add the `OPENROUTER_API_KEY` secret, create an `llm-evals` label, and turn on secret scanning and push protection (Settings > Code security). Secret scanning is also an eligibility gate, and it can only be enabled in settings.

## License

Licensed under the [Apache License, Version 2.0](LICENSE). Third-party dependencies keep their own licenses, recorded in each project's SBOM. Datasets are not covered by the code license (Official Rules §7.b.iii); see each provenance file in [`research/data/`](research/data/).
