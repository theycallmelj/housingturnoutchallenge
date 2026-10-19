# GitHub handoff: State Housing Policy Lab

**Design prototype - fictional example data - not a housing forecast**

This is the complete frontend prototype, stored in `prototypes/state-housing-policy-lab/` within `edprestera/state-housing-policy-lab` on branch `prototype-ed`. The folder can later be copied into the same location within `theycallmelj/housingturnoutchallenge`. Existing repository files and instructions can remain unchanged.

The original `README.md` is preserved. Its absolute Windows paths describe the original development computer; use the portable commands below on other computers.

## Start locally

Tested with Node.js 24.19 and pnpm 11.25.0. Install Node.js and pnpm if they are not already available. Open a terminal in this directory:

```sh
pnpm install --frozen-lockfile
pnpm dev
```

Open **http://127.0.0.1:5173**. No API key, account, backend service or environment file is required. Stop the preview with Ctrl+C.

## Check and build

```sh
pnpm test
pnpm build
```

The test command runs 12 fixture, assistant and saved-session checks. The build performs strict TypeScript checking and creates `dist/`. Generated output and `node_modules/` are excluded from version control.

## Source layout

- `src/main.tsx`, `src/styles.css`, `src/assistant-enhancements.css`: landing page and simulator UI.
- `src/scenario.ts`: shared typed scenario, policy presets and settings summaries.
- `src/demoFixtures.ts`, `src/demoEngine.ts`: explicitly fictional constants, mock simulation, finite package search and future API adapter.
- `src/demoAssistant.ts`: scripted prompts and commands using the same scenario state.
- `src/Documents.tsx`: local attachment metadata and explicit example proposal translation; no file reading or legislation analysis.
- `src/session.ts`, `src/SavedWork.tsx`: per-browser draft, chats, runs and saved-work library.
- `src/siteConfig.ts`: link to the existing hosted demonstration.
- `tests/engine.test.ts`: automated checks.
- `DEPENDENCIES.md`, `QA.md`, `README.md`: license inventory, verification record, fixtures and future integration notes.

The existing `.openai/hosting.json` identifies the original hosted prototype. It contains no credentials and is not needed to run locally. Copying the source or running a local build does not deploy anything.

## Provenance and limits

Original source commit: `8b9199cd5a51270de8910f58ad94332479492300`.

This handoff adds only this portable guide to the original source snapshot. All original source files and documentation are retained. The prototype is an independent team project, uses fictional values, and does not make current-law claims or satisfy the challenge's final analytical requirements.
