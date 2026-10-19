/** FICTIONAL UI ARITHMETIC. No empirical coefficients or forecasts. */
import { createScenario, policyIds } from './scenario.ts';
import type { Scenario, DemoResult, Objective, Constraint, SearchPackage } from './scenario.ts';
import { DEMO_FIXTURES as FIX } from './demoFixtures.ts';
export * from './scenario.ts';
export * from './demoFixtures.ts';
export * from './demoAssistant.ts';
/** Arbitrary additive fixtures with explicit overlap/complementarity, rounded deterministically. */
export function runDemoSimulation(s: Scenario): DemoResult {
  const a = s.adu.enabled ? FIX.increment.adu * (s.adu.allowance === 'two' ? FIX.custom.twoAdus : 1) * (s.adu.geography === 'residential' ? FIX.custom.residential : 1) : 0;
  const m = s.middle.enabled ? s.middle.units * FIX.increment.middlePerUnit * (s.middle.stories === 2 ? FIX.custom.twoStories : 1) * (s.eligibility === 'larger' ? FIX.custom.largerMunicipalities : 1) : 0;
  const t = s.transit.enabled ? FIX.increment.transit * (s.transit.distance / 0.5) * (s.transit.stories / 6) : 0;
  const p = s.parking.enabled ? FIX.increment.parking * (1.5 - s.parking.spaces) / 1.5 : 0;
  const f = s.fees.enabled ? FIX.increment.fees * s.fees.reduction / 50 : 0;
  const overlap = a && m ? Math.min(a, m) * FIX.interaction.aduMiddleOverlap : 0;
  const complement = m && p ? Math.min(m, p) * FIX.interaction.middleParkingComplement : 0;
  const transitOverlap = t && m ? Math.min(t, m) * FIX.interaction.transitMiddleOverlap : 0;
  const incremental = Math.round((a + m + t + p + f - overlap + complement - transitOverlap) / 10) * 10;
  const capacity = Math.round((a * FIX.capacity.adu + m * FIX.capacity.middle + t * FIX.capacity.transit + p * FIX.capacity.parking + f * FIX.capacity.fees - overlap * FIX.capacity.interaction + complement * FIX.capacity.interaction - transitOverlap * FIX.capacity.interaction) / 10) * 10;
  const notes = [overlap ? 'ADUs and missing middle share an eligibility pool: the fixture subtracts an overlap adjustment.' : '', complement ? 'Parking relief complements missing middle: the fixture adds a combined-package adjustment.' : '', transitOverlap ? 'Transit and missing middle overlap: the fixture avoids adding their full standalone effects.' : ''].filter(Boolean);
  return { baseline: FIX.baseline, permitted: FIX.baseline + incremental, incremental, capacity, range: [Math.round(incremental * FIX.range.low / 10) * 10, Math.round(incremental * FIX.range.high / 10) * 10], notes: notes.length ? notes : ['This fixture adds enabled policy settings. Combined packages can include overlap and complementary adjustments.'] };
}
export function searchDemoPackages(s: Scenario, target: number, objective: Objective, constraint: Constraint): SearchPackage[] {
  const packages: SearchPackage[] = [];
  for (let mask = 1; mask < 32; mask++) {
    const next = createScenario(s.state);
    policyIds.forEach((key, i) => next[key].enabled = Boolean(mask & (1 << i)));
    const switches = policyIds.filter(key => next[key].enabled).length;
    if ((constraint === 'fewer' && switches > 2) || (constraint === 'transit' && !next.transit.enabled)) continue;
    const result = runDemoSimulation(next);
    if ((objective === 'capacity' ? result.capacity : result.incremental) >= target) packages.push({ scenario: next, result, switches });
  }
  return packages.sort((a, b) => a.switches - b.switches || a.result.incremental - b.result.incremental).slice(0, 3);
}
/** Replace this adapter with real retrieval and a validated numerical engine later. */
export interface HousingLabAdapter {
  simulate(s: Scenario): DemoResult;
  compare(scenarios: Scenario[]): DemoResult[];
  retrievePolicy(state: string): { status: 'unconnected'; message: string };
  search(s: Scenario, target: number, objective: Objective, constraint: Constraint): SearchPackage[];
}
export const demoAdapter: HousingLabAdapter = { simulate: runDemoSimulation, compare: scenarios => scenarios.map(runDemoSimulation), retrievePolicy: () => ({ status: 'unconnected', message: 'Verified policy data will appear here' }), search: searchDemoPackages };

