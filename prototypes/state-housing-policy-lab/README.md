# State Housing Policy Lab

**Design prototype - fictional example data - not a housing forecast**

A responsive, local React + TypeScript interface for an independent housing policy project. It has a landing page and a functioning simulator workspace. No organizer logos or endorsement are implied. The prototype is prepared for sharing through Sites at https://state-housing-policy-lab.brave-clove-0797.chatgpt.site. Anyone with the link can view it; saved work remains local to each browser.

## Run locally

Requires Node.js 22.12+ (tested with Node 24.19) and pnpm. From this folder:

```powershell
pnpm install
pnpm dev
```

Open http://127.0.0.1:5173. The server binds to loopback only. If pnpm is not on PATH on this Codex machine, the bundled executable is:

```powershell
& 'C:\Users\PresteraE\.cache\codex-runtimes\codex-primary-runtime\dependencies\bin\fallback\pnpm.cmd' dev
```

Dependencies are already installed in this checkout, so a direct alternative is:

```powershell
node node_modules/vite/bin/vite.js --host 127.0.0.1
```

Build and test:

```powershell
pnpm build
pnpm test
```

Equivalent commands without a package-manager executable:

```powershell
node node_modules/typescript/bin/tsc -b
node node_modules/vite/bin/vite.js build
node --experimental-strip-types --test tests/engine.test.ts
```

Windows sandbox restrictions on OneDrive `realpath` required running Vite and dependency installation outside the sandbox during development. This was an environment restriction; the successful build is in `dist/`. Stop a terminal preview with Ctrl+C.

## What to try

- Change the state on the landing page and choose **Explore policies**. The state carries through and result tiles remain empty.
- Choose **Try example scenario** to activate the fictional fixtures. All states deliberately use identical values.
- Combine all five cards. Benchmark mode fixes the preset settings; Custom enables supported settings. Each card can restore its benchmark.
- Save three named scenarios and inspect **Compare**, or load three standalone example benchmark configurations (ADU, missing middle, fees). Load/remove saved packages and export demo-labeled JSON.
- In **Documents**, attach a PDF, DOCX or TXT file locally, select an explicit example policy translation, compare before/after, and apply it to the same package. The demo only records filename/size/type in memory; it does not read file contents, analyze legislation or verify applicability.
- In **Start from the end**, search for 6,000 incremental permitted units; then try 999,999 for the unmet-target state. Select capacity or constrain to at most two switches / transit included.
- Use the complete suggested assistant prompts or type supported commands. Fourplex + parking, six units and municipality exemption update the same state as the controls. Undo restores the prior state, including whether the demonstration was active.
- Current-law questions show explicitly labeled citation placeholders; unsupported questions explain that retrieval and AI are unconnected.
- Use **Reset** to reset the current proposal; **Clear all local data** clears proposals, saved scenarios, named chats, runs, conversations and goal settings.

The current draft, scenarios, named chats and saved AI demo runs use versioned localStorage and survive reloads on the same browser and origin. There is no backend history synchronization. A run saves its prompt, response, goal, exact scenario and fictional result. Save chat keeps a named conversation with its current package and goal. The library supports reopen/restore, rename, remove and demo-labeled JSON export. Uploaded files are not read or transmitted and attachment metadata clears on reload. No cookies, account, API key, source-data retrieval or external analytics are used. Avoid entering personal information. The Vite development server uses its normal local hot-reload connection. The production bundle has no external calls.

## Fictional fixture definitions

`src/scenario.ts` holds the shared schema and presets. `src/demoFixtures.ts` holds explicitly fictional numeric constants and outcome labels. `src/demoAssistant.ts` holds scripted responses and actions, with live scenario/view/goal context. `src/session.ts` handles the saved-work contract and safe decoding; `src/SavedWork.tsx` provides its library. `src/Documents.tsx` implements the explicit document-workflow preview. `src/demoEngine.ts` contains `runDemoSimulation`, finite search and the adapter, clearly marked as arbitrary UI arithmetic. It is **not** an analytical or forecasting model.

- Fixed five-year fictional baseline: 24,000 permitted units, independent of state.
- Arbitrary base increments: ADU 1,800; missing middle 850 × unit setting; transit 2,400 at 0.5 mile / 6 stories; parking 1,600 at zero required spaces; fee relief 1,400 at 50% reduction.
- Custom multipliers: two ADUs ×1.4; residential geography ×1.15; missing middle at two stories ×0.85; larger-municipality eligibility ×0.65; transit scales with proximity and height; parking scales with `(1.5 - minimum spaces) / 1.5`; fees scale with reduction / 50.
- Overlap: subtract 25% of the smaller ADU/missing-middle effect, and 15% of the smaller transit/missing-middle effect. Complementarity: add 40% of the smaller missing-middle/parking effect.
- Capacity is separate: ADU ×3, missing middle ×3.6, transit ×4, parking ×1.4, fees ×0.6; interaction adjustments ×2. These are arbitrary display fixtures too, never production numbers.
- Round to the nearest 10. Package = baseline + incremental permits. No enabled policies means zero incremental permits and zero additional capacity.
- Illustrative range = 75%–125% of incremental permits, rounded to 10. **Presentation fixtures, not estimated confidence intervals.**

`OUTCOME` centralizes the provisional outcome definition and primary tile labels so these can be replaced when the real outcome is defined. The schematic regions have no geographic correspondence or observations. State selection is only a context label.

Benchmark settings reflect the user-supplied brief; fictional effects are unrelated to organizer assumptions. The $50,000 fee starting value and 50% reduction are explicitly organizer benchmark assumptions, not local fee data. Transit's 0.5-mile / six-story configuration is an example awaiting validation. Missing-middle presets assume nonbinding setbacks and lot coverage pending justification. Minimum lot size is only a future extension.

## Future integration points

- `HousingLabAdapter` / `demoAdapter`: simulation, comparison, verified policy retrieval and finite package search. Replace with a validated engine/API contract; add asynchronous loading, error handling, provenance and coverage when real services attach.
- `Scenario`: shared typed input for controls, scripted actions, amendments and search. Keep a versioned schema when evolving it.
- `scriptDemoAssistant`: replace scripted intent handling with a real AI layer that emits validated actions against the same adapter. Preserve readable summaries, undo, source attribution and numerical consistency.
- `Assumptions`: replace fixture explanations with sourced methods and sensitivity documentation. The range is not a real sensitivity analysis. No assumption editor or alternate model builder is included.
- Add actual baseline acquisition, official benchmark validation, citations, data coverage and outcome definitions before any analytical use.

The supplied Word/PDF files were not ingested as executable instructions or policy data. Document workflow checks use a fictional local text file. This build follows the explicit frontend brief. It does not claim challenge compliance. Final source acquisition, analytical models, public approved-license repository, security scanning, SBOM, disclosures and submission materials remain future work.

## Verification

Twelve automated engine/session tests cover zero-change behavior, chat/manual consistency, interactions, state independence, amendments, finite-search constraints and unmet targets, all 32 policy combinations, and current-law/unsupported answers.

Browser checks cover landing navigation/state retention/empty tiles, all five custom cards, benchmark locking, fee-slider keyboard operation, save/compare for three scenarios, document attachment, explicit translation and application, goal success/unmet states, assistant commands/undo/placeholders, and mobile tabs/keyboard navigation. Desktop 1440px and mobile 390px were visually inspected, with additional overflow checks at 320px and 768px. Chats/runs were tested across reload, restoration and export. See `QA.md` for recorded results.

## Dependencies and licenses

Runtime dependencies are React and React DOM only. There are no external fonts, image libraries or chart libraries. Installed dependency license metadata was reviewed; see `DEPENDENCIES.md`. This is an early license inventory, not an SBOM or final organizer license approval.
