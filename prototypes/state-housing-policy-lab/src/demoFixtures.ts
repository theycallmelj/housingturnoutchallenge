/** FICTIONAL UI FIXTURES. No empirical coefficients, actual laws or forecasts. */
export const DEMO_LABEL = 'Design prototype - fictional example data - not a housing forecast';
export const OUTCOME = { label: 'permitted units', horizon: '5-year horizon', tiles: ['Baseline five-year permitted units', 'Selected-package permitted units', 'Incremental permitted units', 'Additional modeled capacity'], definition: 'Provisional outcome: units represented as permitted over a five-year period in this fictional interface fixture. Permitted capacity is distinct from permits and completed homes.' };

/** Arbitrary coefficients solely for interface demonstrations. */
export const DEMO_FIXTURES = {
  baseline: 24000,
  increment: { adu: 1800, middlePerUnit: 850, transit: 2400, parking: 1600, fees: 1400 },
  custom: { twoAdus: 1.4, residential: 1.15, twoStories: 0.85, largerMunicipalities: 0.65 },
  interaction: { aduMiddleOverlap: 0.25, middleParkingComplement: 0.4, transitMiddleOverlap: 0.15 },
  capacity: { adu: 3, middle: 3.6, transit: 4, parking: 1.4, fees: 0.6, interaction: 2 },
  range: { low: 0.75, high: 1.25 }
} as const;
