# Research papers: comparative housing policy

Literature for estimating how state housing reforms change housing production against a five-year baseline. The papers are grouped by the challenge's five benchmark scenarios, plus methods and data.

## Get the PDFs

```bash
./research/papers/fetch_papers.sh
```

- [`papers.tsv`](papers.tsv) is the manifest (id, filename, URL, source, topic). Add new papers there.
- PDFs download to `pdfs/`, which is **gitignored**. Redistribution rights vary by paper (arXiv's default license does not grant redistribution), so we commit the script and the checksums, not the files.
- [`checksums.sha256`](checksums.sha256) records the SHA-256 of each PDF as fetched on 2026-10-28 ([`ACCESS_DATE`](ACCESS_DATE)). A mismatch usually means the authors posted a new version.

A note on arXiv: it has little on zoning and housing supply. Most of this literature is on NBER, SSRN, RePEc, and university working-paper series. The arXiv hits below are mostly methods, data, and measurement papers. Before quoting any number in the Model & Assumptions Overview, check the version and publication status.

---

## Evidence by benchmark scenario

### Scenarios 1 and 2: ADUs and missing middle (2–6 units on single-family lots)

| Paper | Setting & method | Key finding | Use for our model |
| --- | --- | --- | --- |
| **Greenaway-McGrevy (2025)**, *Can zoning reform increase housing construction? Evidence from Auckland*, EPC WP 017 | Auckland 2016 upzoning of about 75% of residential land; synthetic control | Upzoning roughly **doubled permits per capita within 5 years**. Over 7 years, about 52,200 extra permits, which is 46% of the 112,300 issued. | Upper-bound comparator for broad, by-right upzoning. The 5-year horizon matches the challenge's. |
| **Maltman & Greenaway-McGrevy (2024)**, *Going it Alone: Upzoning in Lower Hutt*, EPC WP 018 | A single municipality's upzoning (which also removed parking minimums); synthetic control | **3–4× consents per capita**, about 3× starts over 6 years. Net regional starts rose **+12–17%** after accounting for displacement. | Shows that **displacement/spillover** matters: local gains overstate the net statewide effect. |
| **Donovan & Maltman (2024)**, *Dispelling myths: Auckland zoning evidence*, Motu WP 24-07 | Review of the critiques of the Auckland studies | Finds the evidence robust that the reforms raised supply and reduced rents | Summarizes the debate, useful for the "limitations" section |
| **Terner Center (2021)**, *Will Allowing Duplexes and Lot Splits… Create New Homes? (SB 9)* | California parcel-level pro forma feasibility | SB 9 enables new development on **5.4% of single-family parcels**, which is about **700,000 market-feasible units**. That is capacity, not a forecast. Development is infeasible on about 80% of parcels either way. | A **pro forma / feasibility-filter method** we can copy: zoned capacity → feasible capacity → expected uptake |
| **Rose Institute (2024)**, *The Missing Middle: ADU and SB 9 Development in Ontario and Other SoCal Cities* | Comparative case study of CA cities, including San Diego and Long Beach | "SB 9 resulted in the construction of few new units, while ADU reforms led to a significant increase in construction." | Direct **ADU vs. missing-middle comparison** under the same state; supports a lower take-up rate for lot splits/duplexes than for ADUs |

### Scenario 3: Transit-oriented density

| Paper | Setting & method | Key finding | Use for our model |
| --- | --- | --- | --- |
| **Anagol, Ferreira & Rexer**, *Estimating the Economic Value of Zoning Reform*, NBER w29440 | São Paulo 2016 reform; FAR raised along transit corridors; spatial RD plus equilibrium model | Long run: **+1.9% housing stock, −0.5% prices**, with large heterogeneity across neighborhoods. Short run: more multifamily permit filings in upzoned blocks. | The cleanest causal estimate for **transit-corridor density**. It is also a template for turning a permit response into a change in stock. |

### Scenario 4: Parking reduction

| Paper | Setting & method | Key finding | Use for our model |
| --- | --- | --- | --- |
| **Pew (2024)**, *Minneapolis Land Use Reforms Offer a Blueprint for Housing Affordability* (testimony packet, MN Legislature) | Descriptive: Minneapolis 2040 plus parking-minimum repeal | Minneapolis stock rose 12% in 2017–2022 while rents rose 1%, against 4% and 14% elsewhere in MN | Supportive but **descriptive, not causal**. Pair it with the Minneapolis Fed counterpoint below. |
| Maltman & Greenaway-McGrevy (2024), listed above | Lower Hutt's package included removing parking minimums | See above | Hard to isolate parking from upzoning, so we should not double count |

### Scenario 5: Development fee reduction

There are no open PDFs yet. See the link-only list: Burge & Ihlanfeldt (2006) and HUD Cityscape 8(1). The consensus is that fees raise prices, and that the effect on *quantity* is mixed and depends on fee design (flat vs. per square foot) and on whether fees replace discretionary approvals. For the $50k → $25k benchmark, the standard approach is a pro forma: the fee cut lowers per-unit cost, which shifts the share of projects that are feasible, scaled by a supply elasticity.

### Cross-city, comparative, and regulation-measurement studies

| Paper | Key finding / contribution | Use for our model |
| --- | --- | --- |
| **Gyourko, Hartley & Krimmel**, *WRLURI 2018*, NBER w26573 | A regulatory index for about 2,450 US communities. The coasts are most regulated, regulation did not loosen after the Great Recession, and impact-fee use fell from about 75% to 50% of places. | A **state-level stringency covariate** for scaling responses across states |
| **Stacy, Freemark & Lo (2026)**, reply in *Econ Journal Watch* 23(1) | Defends their 2023 *Urban Studies* result: loosening reforms are associated with **+0.8% supply within 3–9 years** (180 reforms, 1,136 cities). It concedes classification limits. | The best **multi-city US average**, much smaller than Auckland's. Use both as a plausible range. |
| **Ben-Moshe & Genesove**, *Regulation and Frontier Housing Supply*, arXiv 2208.01969 | Estimates the "regulatory tax" from prices and quantities alone. Israel multifamily: a mean regulatory tax of **47% of price**, minimum efficient scale of about 5 floors. | A method for estimating how much regulation binds, and evidence on scale economies for mid-rise construction |
| **Adachi**, *Housing production function with unobserved land heterogeneity*, arXiv 2504.20429 | Capital-land substitution in Tokyo | Informs the density/height response in TOD and missing middle |
| **Salazar-Miranda & Talen (2025)**, *Zoning in American Cities: Are Reforms Making a Difference?*, arXiv 2502.00008 | NLP on zoning codes of more than 2,000 US places. Form-based codes are associated with higher FAR, smaller lots, and more multifamily housing. | A way to measure **which places already have reforms**, so we don't credit the baseline with them |
| **Oluku & Cheng (2023)**, arXiv 2305.02159 | WRLURI stringency and employment growth (878 jurisdictions) | A worked example of using the WRLURI panel |
| **Pérez García (2026)**, *Rent Controls: Evidence from Spain*, arXiv 2602.08631 | Difference-in-differences and event study on Catalonia's 2024 rent control | A methods template for a staggered state policy rollout. The topic is out of scope. |

### Methods, simulation, and data infrastructure

| Paper | Contribution | Use for our model |
| --- | --- | --- |
| **Furtado**, *PolicySpace2*, arXiv 2102.11929 | Open-source spatial agent-based model that compares housing policies | An alternative to a pro forma engine. It is probably too opaque for the judging criteria, but it is a reference for policy-comparison design. |
| **Katsumi, Fox et al. (2026)**, *Housing Potential Common Data Model and City Digital Twin*, arXiv 2605.05535 | A data standard for zoning, land use, and parcels for "housing potential" analysis | A schema idea for our parcel → capacity pipeline |
| **Peskoff et al. (2026)**, *LOCUS: Local Ordinance Corpus*, arXiv 2606.19334 | Machine-readable ordinance codes for 9,239 US cities and counties | A possible source for current local ADU, parking, and lot rules. Check its license. |

---

## Link-only (not auto-downloaded)

| Reference | Why it's here | Why not downloaded |
| --- | --- | --- |
| Burge & Ihlanfeldt (2006), *Impact fees and single-family home construction*, JUE. [Author copy](https://sites.create.ou.edu/gregburge1978/wp-content/uploads/sites/1914/2023/03/JUE2006_SFIF.pdf) | The core fee → supply paper (Scenario 5). Fees can *increase* supply when they replace exclusionary approval barriers. | Host timed out on 2026-10-28. Retry and add it to `papers.tsv`. |
| HUD *Cityscape* 8(1), ch. 4, impact fees. [Link](https://www.huduser.gov/periodicals/cityscpe/vol8num1/ch4.pdf) | Impact-fee review (Scenario 5) | The URL returned HTML, not a PDF. Find the current HUD User URL. |
| Stacy et al. (2023), *Land-use reforms and housing costs*, *Urban Studies* 60(14). [DOI 10.1177/00420980231159500](https://doi.org/10.1177/00420980231159500) | The original 180-reform study | Paywalled journal version |
| Murray (2021), *We zoned for density and got higher house prices* (Brisbane). [QUT ePrints](https://eprints.qut.edu.au/209104) | Counter-evidence: 78% of upzoned sites stayed undeveloped over 20 years | No direct PDF link found |
| Hess & Rehler (2021), Buffalo parking-minimum repeal, *JAPA* | Primary evidence for Scenario 4. Major developments built 21% fewer spaces than the old minimums required. | Paywalled. Look for an author copy. |
| Minneapolis Fed (2025), [*Unpacking supply and demand in rent trends since the Minneapolis 2040 plan*](https://www.minneapolisfed.org/article/2025/unpacking-supply-and-demand-in-rent-trends-since-the-minneapolis-2040-plan) | Counterpoint to Pew: slow rent growth may reflect a 2020 demand shock | Web article |
| Terner Center, [*SB 9 Turns One*](https://ternercenter.berkeley.edu/research-and-policy/sb-9-turns-one-applications/) | Early SB 9 uptake was minimal | Web article |
| Greenaway-McGrevy & Phillips, *The Impact of Upzoning on Housing Construction in Auckland*, EPC WP 006 / 015 | Earlier versions of WP 017 | Superseded |

---

## Takeaways for modeling

1. **Ranges are large.** Broad, by-right upzoning (Auckland) roughly doubled permits. Average US local reforms show about +0.8% stock. Our scenarios should present low/central/high ranges anchored to these.
2. **Zoned capacity is not production.** Terner's SB 9 feasibility work, Murray's Brisbane result, and Rose's ADU-vs-SB 9 comparison all say to filter capacity by financial feasibility and owner take-up.
3. **ADUs outperform lot splits and duplexes** under California's state preemption. Use different take-up rates for Scenarios 1 and 2.
4. **Net vs. gross:** Lower Hutt shows displacement within a region. Statewide totals should not simply sum local effects.
5. **Bundled reforms** (upzoning plus parking, as in Minneapolis and Lower Hutt) make single-lever effects hard to identify. Document how we split them to avoid double counting.
