# AGENTS.md

Instructions for every AI coding agent (Claude Code, Codex, Copilot, Cursor, etc.) and every human contributing to this repository.

This repo is a submission to the [Housing Data and Technology Innovation Challenge](https://housingchallenge.turnout.rocks/handbook.html). The [Official Rules](https://housingchallenge.turnout.rocks/rules.html) control where they differ from the handbook. The handbook's submission checklist requires three items that **every code change must keep current**:

- [ ] Software Bill of Materials (SBOM)
- [ ] Data provenance documented
- [ ] AI tools, models, APIs, and dependencies disclosed

A change that adds a dependency, a dataset, or AI-generated code is **not done** until the matching artifact below is updated in the same commit or PR.

---

## 1. Software Bill of Materials (SBOM)

The handbook defines an SBOM as "an inventory of a project's software dependencies."

**Rules**

- Every deployable project (each folder with its own `package.json`, `pyproject.toml`, `Cargo.toml`, etc.) must have an SBOM at `<project>/sbom.cdx.json` in **CycloneDX JSON** format.
- Regenerate the SBOM whenever you add, remove, or upgrade a dependency, or change a lockfile. Never hand-edit it.
- Commit lockfiles (`pnpm-lock.yaml`, `uv.lock`, `poetry.lock`, `Cargo.lock`, …). An SBOM without a lockfile is not reproducible.
- Dependencies must use a **permissive, OSI-approved license** (MIT, BSD-2/3, Apache-2.0, ISC) (Official Rules §7.b.ii). **Do not add** copyleft (GPL/AGPL/LGPL/MPL), source-available, noncommercial, or unknown-license dependencies. If one is unavoidable, stop and flag it to a human before merging.
- No paid or proprietary dependencies unless end users can get free access. Disclose any that are used (see §3).
- Keep each project's human-readable license table (e.g. `DEPENDENCIES.md`) in sync with the SBOM.

**How to generate**

```bash
# Node / pnpm projects (run inside the project folder)
npx --yes @cyclonedx/cyclonedx-npm --output-format JSON --output-file sbom.cdx.json

# pnpm projects (cyclonedx-npm can't read pnpm lockfiles; cdxgen is Apache-2.0)
npx --yes @cyclonedx/cdxgen@12.9.0 -t pnpm --spec-version 1.5 --no-recurse -o sbom.cdx.json .

# Python projects
pipx run cyclonedx-bom -r -i requirements.txt -o sbom.cdx.json   # or: cyclonedx-py environment

# Rust projects (once: cargo install cargo-cyclonedx --locked)
cargo cyclonedx -f json --spec-version 1.5 --override-filename sbom.cdx

# Any language / container (alternative)
syft dir:. -o cyclonedx-json=sbom.cdx.json
```

If you cannot run the generator, say so in your summary. Do not claim the SBOM is current. CI checks it: `scripts/sbom_diff.py` fails the build when a committed SBOM's packages don't match a freshly generated one.

---

## 2. Data provenance

The handbook says: "Document every dataset's source, version, access date, license, cleaning steps, and known limitations." Sensitive, confidential, restricted-use, or personally identifiable data is **prohibited**. Large source datasets must **not** be committed; use an automated acquisition process instead.

**Rules**

- **Data license rule** (Official Rules §7.b.iii, §10.4, §10.5): only use datasets that are public domain / U.S. government works, CC0, or under an explicit open license with no share-alike, noncommercial, or access restrictions. **Do not use** restricted-access data (login-gated, government/non-profit only), ODbL or CC BY-SA data, noncommercial data, or data with no stated license. Sources already rejected for these reasons are listed under "Removed sources and open gaps" in [`research/data/README.md`](research/data/README.md). Don't reintroduce them.
- Every dataset the code reads must have a provenance file in [`research/data/`](research/data/) (one `.md` per source). Use [`research/data/_TEMPLATE.md`](research/data/_TEMPLATE.md) and fill in every field: name, URL, publisher, version/vintage, **access date**, license, description, geography and time coverage, **cleaning/transformation steps**, and **known limitations**.
- Raw data goes in `research/data/raw/`, which is gitignored. Every dataset must be fetchable by a committed script (e.g. `scripts/fetch_<source>.{sh,py}`) that pins the exact URL/API query and records a checksum. Small derived tables (< ~5 MB) may be committed if the script that produced them is also committed.
- Never hard-code a number taken from a dataset without a comment pointing to its provenance file and the exact table/field.
- Any modeling assumption that is *not* from data (elasticities, take-up rates, per-unit costs, the $50,000/unit fee benchmark, transit-proximity thresholds) must be recorded with its source paper or justification. Papers live in [`research/papers/`](research/papers/).
- If you change cleaning logic, update the "Cleaning steps" section of the affected provenance file in the same change.
- Never commit API keys. Read them from environment variables, and document the variable name in the provenance file.

---

## 3. AI tools, models, APIs, and dependencies disclosure

The handbook FAQ says AI use is allowed: "You must be able to explain, test, and secure the result, and disclose material use." It discourages unverified AI-generated code.

**Rules**

- Log material AI use in [`AI_DISCLOSURE.md`](AI_DISCLOSURE.md) at the repo root. Add a row per contribution with the date, tool, model name/ID, what it produced (files/areas), and how a human verified it (tests, review, manual check).
- **Agents: add your own row before finishing a task** that creates or substantially modifies code, data pipelines, or documentation. Use your exact model ID (e.g. `claude-opus-5-5`, `gpt-5-codex`), not just the vendor.
- Any AI model or external API called **at runtime** by the shipped tool (LLM endpoints, geocoders, map tiles, etc.) must be listed in `AI_DISCLOSURE.md` under "Runtime AI / external APIs", with its cost and access terms. It must also appear in the SBOM if it is a package dependency.
- AI-generated code must have tests covering its core logic before merge. Do not commit code you cannot explain.
- Do not paste secrets, credentials, or restricted data into AI tools.

---

## Other eligibility gates (pass/fail before judging)

- **License:** this repository is **Apache-2.0** (see [`LICENSE`](LICENSE)), per Official Rules §7.b.i. All first-party code is contributed under Apache-2.0. Don't add files under other licenses, and don't use public-domain dedications for our own code.
- **No third-party content in the repo:** don't commit papers, PDFs, images, fonts, code snippets, or datasets authored by others unless they are an OSI-licensed dependency recorded in the SBOM, or public-domain data that passes the data license rule. The submission must be the team's original work (§7.a) and must not infringe copyright (§10.3). Link to external material instead.
- **Reproducibility:** documented setup and run commands; automated tests for core logic (`pnpm test` or equivalent) must pass.
- **Security:** no committed secrets; zero unresolved secret-scanning alerts; no unaddressed critical/high Dependabot or CodeQL findings.
- **Language:** memory-safe languages only (TypeScript/JS, Python, Rust, Go, etc.).
- **Modeling expectations:** statewide aggregate impacts vs. a five-year baseline; at least three of the five benchmark scenarios (ADUs, missing middle, transit-oriented density, parking reduction, 50% fee reduction); show ranges and sensitivity, not single-point predictions; keep the model explainable to a non-technical policymaker.

## Repo map

- `prototypes/` holds UI/engine prototypes. `state-housing-policy-lab` uses fictional fixture data only.
- `ontology/housing-ontology.json` is the single source of truth for studies, datasets (with license status), scenarios, and expert domains. Edit it there, then run `cargo test --manifest-path agents/Cargo.toml`.
- `agents/` is the Rust crate on `condesate`: an orchestrator plus one sandboxed expert per domain (`housing-ask`), and the `housing-mcp` server registered in `.mcp.json`.
- `.claude/skills/` holds one evidence skill per domain plus the `housing-panel` router. Skills must cite only ids that exist in the ontology; a test enforces this.
- `research/papers/` holds the literature index, `papers.tsv` manifest, and `fetch_papers.sh` (PDFs are never committed; run the script to download them locally).
- `research/data/` holds one provenance `.md` per candidate data source.
- `AI_DISCLOSURE.md` is the AI use log.

## Definition of done (checklist for every PR)

- [ ] Tests pass and new core logic has tests (CI runs them: `.github/workflows/`)
- [ ] Changes to prompts, routing, models, or the ontology keep the offline evals at 100% (`housing-eval`); add an eval case for any new failure mode you fix
- [ ] SBOM regenerated if dependencies changed
- [ ] Provenance file added/updated for any new or changed dataset or cleaning step
- [ ] `AI_DISCLOSURE.md` row added for material AI assistance
- [ ] No secrets, PII, restricted data, or large raw files committed
- [ ] No third-party content committed; every new dependency and dataset passes the license rules above
