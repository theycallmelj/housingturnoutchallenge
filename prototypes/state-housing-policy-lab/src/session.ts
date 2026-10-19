import type { Scenario, DemoResult, SavedScenario, Objective, Constraint } from './scenario.ts';
export const STORAGE_KEY = 'state-housing-policy-lab-v2';
export interface RunSnapshot { scenario: Scenario; active: boolean; result?: DemoResult; prompt: string; response: string; goal?: {target: number; objective: Objective; constraint: Constraint}; view?: string }
export interface ChatMessage { id: number; role: 'assistant' | 'user'; text: string; placeholders?: boolean; run?: RunSnapshot }
export interface SavedChat { id: string; name: string; createdAt: string; messages: ChatMessage[]; scenario: Scenario; active: boolean; target: string; objective: Objective; constraint: Constraint }
export interface SavedRun extends RunSnapshot { id: string; name: string; createdAt: string }
export interface LocalSession { version: 2; scenario: Scenario; active: boolean; saved: SavedScenario[]; messages: ChatMessage[]; chats: SavedChat[]; runs: SavedRun[]; target: string; objective: Objective; constraint: Constraint }
export function isScenario(value: unknown): value is Scenario {
  if (!value || typeof value !== 'object') return false;
  const s = value as Scenario;
  return typeof s.state === 'string' && ['benchmark','custom'].includes(s.mode) && ['statewide','larger'].includes(s.eligibility) &&
    ['adu','middle','transit','parking','fees'].every(k => typeof (s as unknown as Record<string,{enabled?:boolean}>)[k]?.enabled === 'boolean') &&
    ['one','two'].includes(s.adu.allowance) && ['eligible','residential'].includes(s.adu.geography) && [2,4,6].includes(s.middle.units) && [2,3].includes(s.middle.stories) && [0.25,0.5,1].includes(s.transit.distance) && [4,6,8].includes(s.transit.stories) && [0,0.5,1].includes(s.parking.spaces) && Number.isFinite(s.fees.reduction) && s.fees.reduction >= 0 && s.fees.reduction <= 50;
}
export function decodeSession(raw: string | null): LocalSession | null {
  try {
    if (!raw) return null;
    const s = JSON.parse(raw) as LocalSession;
    const validMessage = (m: ChatMessage) => m && typeof m.text === 'string' && ['assistant','user'].includes(m.role) && (!m.run || isScenario(m.run.scenario));
    if (s.version !== 2 || !isScenario(s.scenario) || typeof s.active !== 'boolean' || !Array.isArray(s.saved) || !s.saved.every(v=>isScenario(v.scenario)) || !Array.isArray(s.messages) || !s.messages.every(validMessage) || !Array.isArray(s.chats) || !s.chats.every(c=>isScenario(c.scenario) && Array.isArray(c.messages) && c.messages.every(validMessage)) || !Array.isArray(s.runs) || !s.runs.every(r=>isScenario(r.scenario)) || !['permitted','capacity'].includes(s.objective) || !['any','fewer','transit'].includes(s.constraint) || typeof s.target !== 'string') return null;
    return s;
  } catch { return null; }
}
export function readLocalSession(): LocalSession | null { try { return decodeSession(localStorage.getItem(STORAGE_KEY)); } catch { return null; } }
