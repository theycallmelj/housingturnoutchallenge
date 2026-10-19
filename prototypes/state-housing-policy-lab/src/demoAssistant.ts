/** Scripted local commands only; no model, document analysis or policy retrieval. */
import { policyIds, policyNames, settingSummary } from './scenario.ts';
import type { Scenario, Objective, Constraint } from './scenario.ts';
export interface AssistantContext { active: boolean; view: string; savedCount: number; documentName?: string; target: number; objective: Objective; constraint: Constraint }
export interface AssistantAction { text: string; scenario?: Scenario; navigate?: string; placeholders?: boolean; goal?: { target: number; objective: Objective; constraint: Constraint } }
export const prompts = [
  'Explain the five housing reforms in plain language and how they can work together.',
  'Explain my state’s current housing policy framework and show the sources that would support your answer.',
  'Combine fourplexes with parking reform in my current proposal and explain what changes.',
  'Change this proposal to six units per parcel while keeping my other settings.',
  'Show what changes if smaller municipalities are exempt from missing-middle reform.',
  'Explain why the combined effect differs from adding the standalone policy effects.',
  'Find proposals that add 6,000 permitted units with at most two reforms, and explain the tradeoffs.',
  'Compare my saved scenarios and explain the differences in their settings and results.',
  'Help me understand how a bill, draft proposal or proposal from another state could translate into my scenario.',
  'Explain the assumptions and sensitivities that drive the results in my current package.',
  'Summarize my current package, my goals and what I am working on now.'
];
export function scriptDemoAssistant(input: string, s: Scenario, context?: AssistantContext): AssistantAction {
  const q = input.toLowerCase(); const next = structuredClone(s); next.mode = 'custom';
  if (/currently|current law|current.*framework|state.*allow/.test(q)) return { text: 'Verified policy data will appear here. Verified state sources will be connected later; this demo cannot describe current law or the actual framework for your selected state. The intended cited-answer fields below are placeholders.', placeholders: true };
  if (/bill|document|draft|another state/.test(q)) return { text: `${context?.documentName ? `Your attached document is “${context.documentName}”. ` : ''}In Documents, you can attach a bill, draft proposal or proposal from another state and select an example interpretation to see a before/after comparison. This prototype does not read the file, interpret legislation or determine whether a proposal is legally applicable. You explicitly choose the settings before applying them.`, navigate: 'Documents' };
  if (/goal|target|find.*(package|proposal)|recommend/.test(q) && !/summar/.test(q)) {
    const number = q.match(/\d[\d,]*(?:\.\d+)?/); const target = number ? Number(number[0].replaceAll(',', '')) : context?.target ?? 6000;
    const objective: Objective = /capacity/.test(q) ? 'capacity' : 'permitted';
    const constraint: Constraint = /at most (two|2)|fewer|no more than (two|2)/.test(q) ? 'fewer' : /transit/.test(q) ? 'transit' : 'any';
    return { text: `I set your example goal to ${target.toLocaleString('en-US')} ${objective === 'capacity' ? 'additional capacity' : 'incremental permitted units'}${constraint === 'fewer' ? ' with at most two reforms' : constraint === 'transit' ? ' with transit included' : ''}. Start from the end now shows alternatives from the same finite demo search used by the form. More specific recommendations will require verified data and a real AI connection.`, navigate: 'Start from the end', goal: { target, objective, constraint } };
  }
  if (/summar|working on/.test(q)) return { text: `Your selected context is ${s.state}; all inputs remain fictional. ${policyIds.filter(id => s[id].enabled).map(id => `${policyNames[id]}: ${settingSummary(s, id)}`).join('; ') || 'No reforms are enabled'}. ${context ? `You are viewing ${context.view}, have ${context.savedCount} saved scenarios, and your goal is ${context.target.toLocaleString('en-US')} ${context.objective === 'capacity' ? 'additional capacity' : 'incremental permitted units'}. ${context.active ? 'The example demonstration is active.' : 'Results remain empty until a demonstration starts.'}` : ''}` };
  if (/explain.*(five|polic|reform)|plain language/.test(q) && !/standalone|combined|assumption/.test(q)) return { text: 'ADUs add a smaller home to an existing residential parcel. Missing middle allows several homes within a neighborhood-scale building. Transit-oriented density allows more height or floor area near major transit. Parking reform reduces required minimum spaces; developers may still supply parking. Fee relief reduces assumed upfront development costs. Combining these can create overlap or complementary effects. These are general descriptions, not claims about current state law.' };
  if (/fourplex|four.?.?unit/.test(q)) { next.middle.enabled = true; next.middle.units = 4; next.parking.enabled = true; next.parking.spaces = 0; return { scenario: next, text: 'Enabled missing middle at 4 units and parking reform at 0 required spaces per unit. Other settings are retained. Your policy controls and results now show the same updated package. These are example settings only.' }; }
  if (/six|6 units/.test(q)) { next.middle.enabled = true; next.middle.units = 6; return { scenario: next, text: 'Changed missing middle to 6 units. Other settings are retained. The same fixture engine recalculates the package and updates your controls and results.' }; }
  if (/smaller|exempt/.test(q)) { next.eligibility = 'larger'; return { scenario: next, text: 'Changed missing-middle eligibility to larger municipalities only. No actual municipality threshold or local data is represented.' }; }
  if (/combined|different|overlap|standalone/.test(q)) return { text: 'The arbitrary fixture subtracts ADU/missing-middle overlap and adds missing-middle/parking complementarity. Transit/missing-middle overlap is also subtracted. These adjustments illustrate interaction design; they are not empirical estimates.' };
  if (/compare|saved/.test(q)) return { text: `You have ${context?.savedCount ?? 0} saved scenarios. The comparison view shows their settings and fictional outcomes against the same baseline. Save alternatives from the toolbar, or load the three example benchmark configurations.`, navigate: 'Compare' };
  if (/assumption|sensitiv/.test(q)) return { text: 'All coefficients are arbitrary. The illustrative range uses 75% and 125% of incremental permitted units for presentation only. Eligibility and policy-interaction adjustments drive this fixture. Read the documented assumptions for sensitivity and future validation needs.', navigate: 'Sources and assumptions' };
  return { text: 'This is a scripted AI-assistant demonstration; real policy retrieval, document interpretation and AI are not connected. You can ask me to explain the reforms, describe the future current-framework answer, change this package, set a goal with constraints, compare saved scenarios or inspect assumptions.' };
}
