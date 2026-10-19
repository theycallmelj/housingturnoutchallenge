# Prototype verification

**Design prototype - fictional example data - not a housing forecast**

## Build and engine

- TypeScript strict checking passed.
- Vite production build passed.
- Seven Node engine tests passed. All 32 policy switch combinations maintain baseline + incremental = selected package and internally ordered presentation ranges.

## Browser interaction evidence

- Landing state Virginia was retained on opening the simulator. All four result tiles were empty until starting the example.
- Manual fourplex + zero-parking settings and the same scripted command both produced `[24,000, 29,640, +5,640, +15,760]` for baseline, package, incremental permits and additional capacity. Undo restored previous settings.
- Saved and compared three named scenarios: fourplex + parking (29,640 package permits), six units + parking (31,340), and six units + parking with larger-municipality eligibility (29,560).
- Duplicating the latter and amending eligibility to statewide showed 29,560 before / 31,340 after / +1,780 difference. Apply produced the same 31,340 in the main tiles.
- Search at 6,000 incremental permits returned three alternatives; 999,999 returned the unmet-target explanation. Engine tests also cover capacity objectives, two-switch constraints and transit-required constraints.
- All five cards were enabled and their custom selects exercised. Benchmark mode restored defaults and disabled settings. Keyboard adjustment of the fee slider to 45% updated the assumed remainder to $27,500.
- Current-state policy prompt returned verified-data empty state and explicit citation placeholders. Unsupported typed question returned the unconnected-retrieval explanation.
- Desktop 1440 × 1000 and mobile 390 × 844 visually inspected. Mobile Results, Policies and Assistant tabs worked. Typed six-unit command updated the scenario from the mobile assistant. Left-arrow navigation moved the selected mobile tab from Assistant to Results.
- Document width did not exceed viewport width at 320, 390, 768 or 1440px; the persistent demo banner stayed visible. Empty tiles were rechecked at 320px.
- Loading the three example benchmarks, loading a saved ADU benchmark and exporting JSON were checked. The downloaded export carried the complete required demo label and all three saved scenarios. Browser console had no errors.

This verifies a frontend demonstration, not analytical validity, legal accuracy or final challenge compliance.

## Assistant and saved-work revision

- Twelve automated checks pass, including every complete suggested prompt, goal/constraint routing, live context summaries, persistent-data round trips and corrupt-data rejection.
- Landing-page goal conversation opened Start from the end, set 6,000 incremental permitted units / at most two reforms, and returned three alternatives using the same finite search as the form.
- Saved two named conversations and two run snapshots; reloaded the page and verified all four records remained available. Reopening the goal chat restored its zero-reform package. Restoring the fourplex/parking run restored 29,640 package permitted units and 15,760 additional capacity.
- Manually changing missing middle to two units was reflected in the assistant's current-package summary, along with the current view, saved count and target.
- Attached a fictional TXT file locally in Documents. The six-unit translation preview showed 27,940 before / 31,340 after; applying it produced the same 31,340 in Results. The assistant referenced the filename and explained that it had not read the content or assessed legislation.
- Desktop assistant was moved alongside the workspace header so its context, conversation, prompt list and composer are visible together. At 1440 × 1000, its top was 163px and composer bottom was 964px.
- Mobile assistant opened directly from the persistent header entry point at 390px, with no document overflow. Saved work remains per-browser, separate from the public prototype link.
