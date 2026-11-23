# Housing Turnout Challenge

[![Rust (agents)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/rust.yml/badge.svg)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/rust.yml)
[![Web prototype](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/web.yml/badge.svg)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/web.yml)
[![CodeQL](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/codeql.yml/badge.svg)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/codeql.yml)
[![LLM evals](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/llm-evals.yml/badge.svg)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/llm-evals.yml)
[![Codespace setup](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/codespace.yml/badge.svg)](https://github.com/theycallmelj/housingturnoutchallenge/actions/workflows/codespace.yml)

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/theycallmelj/housingturnoutchallenge?quickstart=1)

Team repository for the [Housing Data and Technology Innovation Challenge](https://housingchallenge.turnout.rocks/handbook.html): open-source tools estimating how state housing policy reforms change housing production against a five-year baseline.

## Demo

[![Demo of the State Housing Policy Lab web UI](docs/demo/housing-web-demo.gif)](docs/demo/housing-web-demo.mp4)

[Watch the full recording (MP4, 2 min 25 s)](docs/demo/housing-web-demo.mp4). It shows the local web UI (`housing-web`) running with live Apache-2.0 models on OpenRouter (gpt-oss-120b leads, gpt-oss-20b experts):

1. Pennsylvania's five-year baseline, with permits, construction starts and completions by year, and a plain-language explanation of how it is built and back-tested.
2. Data-source links opening the publishers' pages: Census Building Permits Survey, Census Survey of Construction, FHFA House Price Index.
3. "What if the economy is worse?": higher interest rates and lower migration shrink the baseline.
4. A reform's detail view: the result in plain words, what moves it most, and each assumption with its basis and linked sources. A citation opens the study itself.
5. A question to the expert panel. The answer names the experts consulted, links every cited study, and passes the automatic citation check.

The wait for the models is sped up 8×; the on-screen timer shows real time. The recording predates the "How we tested the model" section (below).

## Try it in GitHub Codespaces (no install)

A codespace is a cloud computer with this repository already set up. Nothing is installed on your machine.

1. **Optional: add your OpenRouter API key.** Without one, everything works except that the expert panel uses offline stand-in models instead of live ones. To use live models, get a key at [openrouter.ai/keys](https://openrouter.ai/keys), then either:
   - add it once at [github.com/settings/codespaces](https://github.com/settings/codespaces) under **Codespaces secrets > New secret**, named `OPENROUTER_API_KEY`, with access to this repository; or
   - paste it when the codespace asks for `OPENROUTER_API_KEY` on the creation page (click **Open in GitHub Codespaces** above, then **Change options** if it isn't shown).
2. **Open the codespace:** click the **Open in GitHub Codespaces** badge above (or **Code > Codespaces > Create codespace on main**).
3. **Wait for the first build,** about 5-10 minutes while Rust compiles the tool. Later starts take seconds.
4. **The app opens by itself** in a new browser tab. If it doesn't, open the **Ports** tab at the bottom of the editor and click the globe icon next to port **8787**. To restart it, run `scripts/start.sh` in the terminal.

Billing for the live models goes to your OpenRouter account, per token, at the prices recorded in [`agents/model-registry.json`](agents/model-registry.json) (gpt-oss-120b $0.15/$0.60 and gpt-oss-20b $0.03/$0.13 per million input/output tokens on 2026-11-15). Codespaces usage counts against your GitHub account's Codespaces allowance. The key stays in your codespace and is never written to the repository.

## Run it on your own computer

You need [Rust](https://rustup.rs) (stable) and git; nothing else.

```bash
git clone https://github.com/theycallmelj/housingturnoutchallenge.git
cd housingturnoutchallenge
cp agents/.env.example agents/.env     # optional: put your OPENROUTER_API_KEY in it (gitignored)
scripts/start.sh                       # then open http://127.0.0.1:8787
```

`scripts/start.sh` uses live models when `OPENROUTER_API_KEY` is set (in `agents/.env` or the environment) and the offline models otherwise. The command-line tools work too: `cargo run --release --manifest-path agents/Cargo.toml --bin housing-model -- --state PA` prints a state's baseline and scenarios. See [`agents/README.md`](agents/README.md) for the rest.

### Using the same models later

The panel runs only open-weight models under Apache-2.0, pinned by id in [`agents/model-registry.json`](agents/model-registry.json): **gpt-oss-120b** leads and **gpt-oss-20b** answers as each domain expert (profile `openrouter`). The same weights can be run three ways, so the tool doesn't depend on any one host:

| Where | Setup | Model ids |
| --- | --- | --- |
| OpenRouter (hosted, default) | `OPENROUTER_API_KEY=...` | `openai/gpt-oss-120b`, `openai/gpt-oss-20b` |
| Ollama (your machine or server) | `ollama pull gpt-oss:120b && ollama pull gpt-oss:20b`, then `HOUSING_LLM_HOST=ollama HOUSING_LLM_BASE_URL=http://localhost:11434/v1 HOUSING_MODEL_PROFILE=server scripts/start.sh` | `gpt-oss:120b`, `gpt-oss:20b` |
| vLLM or any OpenAI-compatible server | `HOUSING_LLM_HOST=vllm HOUSING_LLM_BASE_URL=http://<server>/v1 HOUSING_MODEL_PROFILE=server scripts/start.sh` | `openai/gpt-oss-120b`, `openai/gpt-oss-20b` ([Hugging Face](https://huggingface.co/openai/gpt-oss-120b)) |

gpt-oss-120b needs about 65 GB of GPU or unified memory (gpt-oss-20b about 14 GB). For smaller machines, the `laptop` and `workstation` profiles use smaller Apache-2.0 models. Answers vary slightly between hosts and runs. The scenario model, the baseline and the back-tests are deterministic and don't use the language models at all.

## How we tested the model

The web UI's **How we tested the model** section shows a ten-year back-test of the baseline. For every start year from 2015 to 2024, the county permit model is refitted using only data available that year and forecasts the next one to five years. Those forecasts are compared with the permits actually issued in 2016-2025 and with two simple rules: repeat last year, and repeat the five-year average.

- **Five-year state totals:** typical miss 12.5%, against 12.9% for repeating last year and 20.0% for the five-year average.
- **One year ahead:** repeating last year is slightly better (8.7% vs. 9.8%).
- **Bias:** the model has tended to run low, so the baseline's range extends further above its central estimate than below.
- **COVID:** forecasts made from or for 2020-2021 are greyed out and left out of the scores by default, with a switch to include them. Without them, the model ties repeating last year three years ahead and pulls ahead after that (five years ahead: 16.5% vs. 18.4%).

Code: [`agents/src/backtest.rs`](agents/src/backtest.rs); results: [`data/derived/backtest.json`](data/derived/backtest.json). `housing-fit --check` re-runs it in CI and fails if any number changes.

## Contents

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
| `rust.yml` | every push and PR | `cargo fmt`, `clippy -D warnings` (with and without `llm`), tests (both feature sets), offline panel evals (100% required), that the committed baseline model and ten-year back-test match a fresh run (`housing-fit --check`), the pipeline's unit tests, and that the fetch scripts reproduce every committed table in `data/derived/` |
| `codespace.yml` | every push and PR | Builds the tool in the Codespaces image and checks that `scripts/start.sh` serves the UI |
| `web.yml` | every push and PR | React prototype: frozen-lockfile install, unit tests, typecheck + production build |
| `llm-evals.yml` | on demand, weekly, PRs labelled `llm-evals` | Live-model panel evals on OpenRouter; needs the `OPENROUTER_API_KEY` repository secret |
| `codeql.yml` | every push, PR, weekly | Security analysis for Rust, TypeScript, and the workflows themselves (eligibility gate) |
| `dependabot.yml` | weekly | Cargo, npm, and GitHub Actions updates (eligibility gate) |

One-time repository settings: add the `OPENROUTER_API_KEY` secret, create an `llm-evals` label, and turn on secret scanning and push protection (Settings > Code security). Secret scanning is also an eligibility gate, and it can only be enabled in settings.

## License

Licensed under the [Apache License, Version 2.0](LICENSE). Third-party dependencies keep their own licenses, recorded in each project's SBOM. Datasets are not covered by the code license (Official Rules §7.b.iii); see each provenance file in [`research/data/`](research/data/).
