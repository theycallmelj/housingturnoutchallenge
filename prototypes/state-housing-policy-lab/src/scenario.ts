export type PolicyId = 'adu' | 'middle' | 'transit' | 'parking' | 'fees';
export type Eligibility = 'statewide' | 'larger';
export interface Scenario {
  state: string; mode: 'benchmark' | 'custom'; eligibility: Eligibility;
  adu: { enabled: boolean; allowance: 'one' | 'two'; geography: 'eligible' | 'residential' };
  middle: { enabled: boolean; units: 2 | 4 | 6; stories: 2 | 3 };
  transit: { enabled: boolean; distance: 0.25 | 0.5 | 1; stories: 4 | 6 | 8 };
  parking: { enabled: boolean; spaces: 0 | 0.5 | 1 };
  fees: { enabled: boolean; reduction: number };
}
export interface DemoResult { baseline: number; permitted: number; incremental: number; capacity: number; range: [number, number]; notes: string[] }
export interface SavedScenario { id: string; name: string; scenario: Scenario; result: DemoResult }
export type Objective = 'permitted' | 'capacity';
export type Constraint = 'any' | 'fewer' | 'transit';
export const policyNames: Record<PolicyId, string> = { adu: 'Accessory dwelling units', middle: 'Missing middle housing', transit: 'Transit-oriented density', parking: 'Parking requirement reduction', fees: 'Development fee reduction' };
export const policyIds = Object.keys(policyNames) as PolicyId[];
export function createScenario(state = 'Massachusetts'): Scenario {
  return { state, mode: 'benchmark', eligibility: 'statewide', adu: { enabled: false, allowance: 'one', geography: 'eligible' }, middle: { enabled: false, units: 6, stories: 3 }, transit: { enabled: false, distance: 0.5, stories: 6 }, parking: { enabled: false, spaces: 0 }, fees: { enabled: false, reduction: 50 } };
}
export function exampleScenario(state: string): Scenario {
  const s = createScenario(state); s.middle.enabled = true; s.middle.units = 4; s.parking.enabled = true; s.adu.enabled = true; s.mode = 'custom'; return s;
}
export function applyPreset(s: Scenario, id?: PolicyId): Scenario {
  const defaults = createScenario(s.state); const next = structuredClone(s);
  for (const key of id ? [id] : policyIds) Object.assign(next[key], defaults[key], { enabled: id ? true : s[key].enabled });
  if (id === 'middle' || !id) next.eligibility = 'statewide';
  return next;
}
export function settingSummary(s: Scenario, id: PolicyId): string {
  if (!s[id].enabled) return 'Off';
  switch (id) {
    case 'adu': return `${s.adu.allowance === 'one' ? '1 ADU' : '2 ADUs'} · ${s.adu.geography === 'eligible' ? 'existing single-family' : 'residential parcels'}`;
    case 'middle': return `${s.middle.units} units · ${s.middle.stories} stories · ${s.eligibility === 'statewide' ? 'statewide' : 'larger municipalities'}`;
    case 'transit': return `${s.transit.distance} mi · ${s.transit.stories} stories`;
    case 'parking': return `${s.parking.spaces} required spaces/unit`;
    case 'fees': return `${s.fees.reduction}% reduction · assumed $${(50000 * (1 - s.fees.reduction / 100)).toLocaleString()}/unit`;
  }
}
export interface SearchPackage { scenario: Scenario; result: DemoResult; switches: number }
