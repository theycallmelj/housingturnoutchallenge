//! Typed view of `ontology/housing-ontology.json` plus the queries every
//! agent, tool and MCP handler shares.
//!
//! The JSON is compiled into the binary with `include_str!`, so answers are
//! reproducible from the commit alone and no file path has to be configured.

use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};

const BUNDLED: &str = include_str!("../../ontology/housing-ontology.json");

/// The one license status a dataset must have to be used in the submission.
pub const USABLE_STATUS: &str = "open_verified";

#[derive(Debug, Deserialize)]
pub struct Ontology {
    pub meta: Meta,
    pub enums: Enums,
    pub scenarios: Vec<Scenario>,
    pub domains: Vec<Domain>,
    pub outcomes: Vec<Outcome>,
    pub mechanisms: Vec<Mechanism>,
    pub datasets: Vec<Dataset>,
    pub studies: Vec<Study>,
    pub pilot_states: Vec<PilotState>,
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct Enums {
    pub license_status: BTreeMap<String, String>,
    pub evidence_type: BTreeMap<String, String>,
    pub verification: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub label: String,
    pub benchmark_definition: String,
    pub domains: Vec<String>,
    #[serde(default)]
    pub key_mechanisms: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Domain {
    pub id: String,
    pub label: String,
    pub expert: String,
    pub scenarios: Vec<String>,
    pub description: String,
    pub keywords: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Outcome {
    pub id: String,
    pub label: String,
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub struct Mechanism {
    pub id: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct Dataset {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub url: String,
    pub license_status: String,
    pub license: String,
    #[serde(default)]
    pub provenance_file: Option<String>,
    pub domains: Vec<String>,
    pub scenarios: Vec<String>,
    pub role: String,
    pub limitations: Vec<String>,
}

impl Dataset {
    pub fn usable(&self) -> bool {
        self.license_status == USABLE_STATUS
    }
}

#[derive(Debug, Deserialize)]
pub struct Study {
    pub id: String,
    pub title: String,
    pub authors: String,
    pub year: u32,
    pub venue: String,
    pub evidence_type: String,
    pub domains: Vec<String>,
    pub scenarios: Vec<String>,
    pub outcome: String,
    pub finding: String,
    pub datasets: Vec<String>,
    pub limitation: String,
    pub relevance: String,
    pub verification: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct PilotState {
    pub state: String,
    pub rationale: String,
    pub datasets: Vec<String>,
}

/// How much a study's design supports a quantity in our model. Used only to
/// order results; every result still carries its own evidence type.
fn evidence_weight(evidence_type: &str) -> u32 {
    match evidence_type {
        "causal_quasi_experimental" => 5,
        "structural_model" => 4,
        "pro_forma_simulation" | "associational" => 3,
        "descriptive" | "survey" | "review" | "agent_based_simulation" => 2,
        "measurement" | "framework" => 1,
        _ => 0,
    }
}

fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(str::to_string)
        .collect()
}

impl Ontology {
    pub fn bundled() -> &'static Ontology {
        use std::sync::OnceLock;
        static ONTOLOGY: OnceLock<Ontology> = OnceLock::new();
        ONTOLOGY.get_or_init(|| Self::parse(BUNDLED).expect("bundled ontology must parse"))
    }

    pub fn parse(json: &str) -> anyhow::Result<Ontology> {
        Ok(serde_json::from_str(json)?)
    }

    pub fn domain(&self, id: &str) -> Option<&Domain> {
        self.domains.iter().find(|d| d.id == id)
    }

    pub fn dataset(&self, id: &str) -> Option<&Dataset> {
        self.datasets.iter().find(|d| d.id == id)
    }

    pub fn study(&self, id: &str) -> Option<&Study> {
        self.studies.iter().find(|s| s.id == id)
    }

    pub fn outcome(&self, id: &str) -> Option<&Outcome> {
        self.outcomes.iter().find(|o| o.id == id)
    }

    /// Domains whose keywords appear in `query`, most matches first. Falls
    /// back to the modeling domain so every question gets an answer.
    pub fn route(&self, query: &str) -> Vec<&Domain> {
        let q = query.to_lowercase();
        let mut scored: Vec<(usize, &Domain)> = self
            .domains
            .iter()
            .map(|d| (d.keywords.iter().filter(|k| q.contains(k.as_str())).count(), d))
            .filter(|(n, _)| *n > 0)
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
        if scored.is_empty() {
            return self.domain("modeling").into_iter().collect();
        }
        scored.into_iter().map(|(_, d)| d).collect()
    }

    /// Studies filtered by domain and/or scenario, ranked by keyword overlap
    /// with `query` and then by evidence weight. Leads and citation-only
    /// entries sort last because they carry no finding.
    pub fn find_studies(
        &self,
        domain: Option<&str>,
        scenario: Option<&str>,
        query: &str,
        limit: usize,
    ) -> Vec<&Study> {
        let q: HashSet<String> = tokens(query).into_iter().collect();
        let mut hits: Vec<(u32, &Study)> = self
            .studies
            .iter()
            .filter(|s| domain.map_or(true, |d| s.domains.iter().any(|x| x == d)))
            .filter(|s| scenario.map_or(true, |sc| s.scenarios.iter().any(|x| x == sc)))
            .map(|s| {
                let text = format!("{} {} {} {}", s.title, s.finding, s.relevance, s.authors);
                let overlap = tokens(&text).iter().filter(|t| q.contains(*t)).count() as u32;
                (overlap * 2 + evidence_weight(&s.evidence_type), s)
            })
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
        hits.into_iter().take(limit).map(|(_, s)| s).collect()
    }

    pub fn find_datasets(
        &self,
        domain: Option<&str>,
        scenario: Option<&str>,
        usable_only: bool,
    ) -> Vec<&Dataset> {
        self.datasets
            .iter()
            .filter(|d| domain.map_or(true, |x| d.domains.iter().any(|y| y == x)))
            .filter(|d| scenario.map_or(true, |x| d.scenarios.iter().any(|y| y == x)))
            .filter(|d| !usable_only || d.usable())
            .collect()
    }

    pub fn format_study(&self, s: &Study) -> String {
        let outcome = self.outcome(&s.outcome).map_or(s.outcome.as_str(), |o| o.label.as_str());
        format!(
            "[{id}] {authors} ({year}), \"{title}\". {venue}.\n  evidence: {etype}; outcome: {outcome}; verification: {verif}\n  finding: {finding}\n  limitation: {limitation}\n  use: {relevance}\n  source: {url}",
            id = s.id,
            authors = s.authors,
            year = if s.year == 0 { "n.d.".to_string() } else { s.year.to_string() },
            title = s.title,
            venue = s.venue,
            etype = s.evidence_type,
            verif = s.verification,
            finding = s.finding,
            limitation = s.limitation,
            relevance = s.relevance,
            url = s.url,
        )
    }

    pub fn format_dataset(&self, d: &Dataset) -> String {
        format!(
            "[{id}] {name} ({publisher}). license: {status} ({license}){usable}\n  role: {role}\n  limitations: {lims}\n  url: {url}{prov}",
            id = d.id,
            name = d.name,
            publisher = d.publisher,
            status = d.license_status,
            license = d.license,
            usable = if d.usable() { "; USABLE" } else { "; NOT usable in the submission" },
            role = d.role,
            lims = d.limitations.join("; "),
            url = d.url,
            prov = d.provenance_file.as_ref().map(|p| format!("\n  provenance: {p}")).unwrap_or_default(),
        )
    }

    /// Any entity by id, rendered as text. Studies and datasets get their
    /// full record; other classes get their defining fields.
    pub fn describe(&self, id: &str) -> Option<String> {
        if let Some(s) = self.study(id) {
            return Some(self.format_study(s));
        }
        if let Some(d) = self.dataset(id) {
            return Some(self.format_dataset(d));
        }
        if let Some(sc) = self.scenarios.iter().find(|x| x.id == id) {
            return Some(format!("[{}] {}: {} Mechanisms: {}", sc.id, sc.label, sc.benchmark_definition, sc.key_mechanisms.join(", ")));
        }
        if let Some(d) = self.domain(id) {
            return Some(format!("[{}] {} (expert: {}): {}", d.id, d.label, d.expert, d.description));
        }
        if let Some(m) = self.mechanisms.iter().find(|x| x.id == id) {
            return Some(format!("[{}] {}: {}", m.id, m.label, m.description));
        }
        if let Some(o) = self.outcome(id) {
            return Some(format!("[{}] {}: {}", o.id, o.label, o.note));
        }
        None
    }

    /// The evidence an expert works from: the domain's scope, its best-ranked
    /// studies for `query`, the usable datasets, and the scenario caveats.
    pub fn evidence_brief(&self, domain_id: &str, query: &str, limit: usize) -> Option<String> {
        let domain = self.domain(domain_id)?;
        let mut out = format!("DOMAIN {}: {}\n{}\n", domain.id, domain.label, domain.description);
        for sc in self.scenarios.iter().filter(|s| domain.scenarios.contains(&s.id)) {
            out.push_str(&format!("\nSCENARIO {}: {}\n", sc.label, sc.benchmark_definition));
            for m in &sc.key_mechanisms {
                if let Some(mech) = self.mechanisms.iter().find(|x| &x.id == m) {
                    out.push_str(&format!("  - mechanism {}: {}\n", mech.label, mech.description));
                }
            }
        }
        out.push_str("\nSTUDIES (most relevant first; quote outcome and evidence type with every number)\n");
        for s in self.find_studies(Some(domain_id), None, query, limit) {
            out.push_str(&self.format_study(s));
            out.push('\n');
        }
        let usable = self.find_datasets(Some(domain_id), None, true);
        out.push_str("\nUSABLE DATASETS (license open_verified)\n");
        if usable.is_empty() {
            out.push_str("  (none yet; see research/data/README.md for open gaps)\n");
        }
        for d in &usable {
            out.push_str(&format!("  - [{}] {}: {}\n", d.id, d.name, d.role));
        }
        let blocked: Vec<&str> = self
            .find_datasets(Some(domain_id), None, false)
            .into_iter()
            .filter(|d| !d.usable())
            .map(|d| d.id.as_str())
            .collect();
        if !blocked.is_empty() {
            out.push_str(&format!("NOT USABLE (license status not open_verified): {}\n", blocked.join(", ")));
        }
        Some(out)
    }

    /// Integrity problems: duplicate ids, dangling references, unknown enum
    /// values, usable datasets without a provenance file. Empty means valid.
    pub fn validate(&self) -> Vec<String> {
        let mut errs = Vec::new();
        let domains: HashSet<&str> = self.domains.iter().map(|d| d.id.as_str()).collect();
        let scenarios: HashSet<&str> = self.scenarios.iter().map(|s| s.id.as_str()).collect();
        let datasets: HashSet<&str> = self.datasets.iter().map(|d| d.id.as_str()).collect();
        let mechanisms: HashSet<&str> = self.mechanisms.iter().map(|m| m.id.as_str()).collect();

        let mut seen = HashSet::new();
        let all_ids = self
            .studies
            .iter()
            .map(|s| &s.id)
            .chain(self.datasets.iter().map(|d| &d.id));
        for id in all_ids {
            if !seen.insert(id.as_str()) {
                errs.push(format!("duplicate id {id}"));
            }
        }
        for s in &self.studies {
            for d in &s.domains {
                if !domains.contains(d.as_str()) {
                    errs.push(format!("study {} has unknown domain {d}", s.id));
                }
            }
            for sc in &s.scenarios {
                if !scenarios.contains(sc.as_str()) {
                    errs.push(format!("study {} has unknown scenario {sc}", s.id));
                }
            }
            for d in &s.datasets {
                if !datasets.contains(d.as_str()) {
                    errs.push(format!("study {} references unknown dataset {d}", s.id));
                }
            }
            if self.outcome(&s.outcome).is_none() {
                errs.push(format!("study {} has unknown outcome {}", s.id, s.outcome));
            }
            if !self.enums.evidence_type.contains_key(&s.evidence_type) {
                errs.push(format!("study {} has unknown evidence_type {}", s.id, s.evidence_type));
            }
            if !self.enums.verification.contains_key(&s.verification) {
                errs.push(format!("study {} has unknown verification {}", s.id, s.verification));
            }
        }
        for d in &self.datasets {
            if !self.enums.license_status.contains_key(&d.license_status) {
                errs.push(format!("dataset {} has unknown license_status {}", d.id, d.license_status));
            }
            if d.usable() && d.provenance_file.is_none() {
                errs.push(format!("usable dataset {} has no provenance_file", d.id));
            }
            for x in &d.domains {
                if !domains.contains(x.as_str()) {
                    errs.push(format!("dataset {} has unknown domain {x}", d.id));
                }
            }
        }
        for sc in &self.scenarios {
            for m in &sc.key_mechanisms {
                if !mechanisms.contains(m.as_str()) {
                    errs.push(format!("scenario {} has unknown mechanism {m}", sc.id));
                }
            }
        }
        for p in &self.pilot_states {
            for d in &p.datasets {
                if !datasets.contains(d.as_str()) {
                    errs.push(format!("pilot state {} references unknown dataset {d}", p.state));
                }
            }
        }
        errs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn ontology_is_internally_consistent() {
        let errs = Ontology::bundled().validate();
        assert!(errs.is_empty(), "{errs:#?}");
    }

    #[test]
    fn ontology_matches_workbook_study_count() {
        assert_eq!(Ontology::bundled().studies.len(), 77);
    }

    #[test]
    fn ontology_provenance_files_exist() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        for d in &Ontology::bundled().datasets {
            if let Some(p) = &d.provenance_file {
                assert!(root.join(p).exists(), "{} -> {p}", d.id);
            }
        }
    }

    /// Skills cite study and dataset ids in backticks; every snake_case id
    /// they cite must exist so the skills can't drift from the ontology.
    #[test]
    fn ontology_ids_cited_in_skills_exist() {
        let o = Ontology::bundled();
        let mut known: HashSet<&str> = HashSet::new();
        known.extend(o.studies.iter().map(|x| x.id.as_str()));
        known.extend(o.datasets.iter().map(|x| x.id.as_str()));
        known.extend(o.domains.iter().map(|x| x.id.as_str()));
        known.extend(o.scenarios.iter().map(|x| x.id.as_str()));
        known.extend(o.enums.license_status.keys().map(String::as_str));
        known.extend(o.enums.evidence_type.keys().map(String::as_str));
        known.extend(["usable_only", "license_status", "pilot_states"]);

        let skills = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.claude/skills");
        for entry in std::fs::read_dir(&skills).expect("skills dir") {
            let path = entry.unwrap().path().join("SKILL.md");
            let text = std::fs::read_to_string(&path).unwrap();
            for (i, span) in text.split('`').enumerate() {
                let is_id = i % 2 == 1
                    && span.contains('_')
                    && !span.starts_with("mcp__")
                    && span.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
                if is_id {
                    assert!(known.contains(span), "{} cites unknown id `{span}`", path.display());
                }
            }
        }
    }

    #[test]
    fn ontology_every_benchmark_scenario_has_an_expert_domain() {
        let o = Ontology::bundled();
        for sc in &o.scenarios {
            assert!(
                o.domains.iter().any(|d| d.scenarios.contains(&sc.id)),
                "scenario {} has no domain",
                sc.id
            );
        }
    }

    #[test]
    fn route_picks_domains_by_keyword_and_falls_back_to_modeling() {
        let o = Ontology::bundled();
        let ids = |q: &str| o.route(q).iter().map(|d| d.id.clone()).collect::<Vec<_>>();
        assert!(ids("How many ADUs would a state ADU law add?").contains(&"adu".into()));
        assert!(ids("Does cutting impact fees by 50% raise construction?").contains(&"fees".into()));
        assert_eq!(ids("hello"), vec!["modeling".to_string()]);
    }

    #[test]
    fn find_datasets_usable_only_excludes_restricted_and_proprietary() {
        let o = Ontology::bundled();
        let usable = o.find_datasets(None, None, true);
        assert!(usable.iter().all(|d| d.usable()));
        assert!(usable.iter().all(|d| d.id != "hud_usps_vacancy" && d.id != "costar"));
        assert!(usable.iter().any(|d| d.id == "census_bps"));
    }

    #[test]
    fn find_studies_ranks_query_matches_first() {
        let o = Ontology::bundled();
        let top = o.find_studies(Some("missing_middle"), None, "SB 9 lot splits feasible", 3);
        assert_eq!(top[0].id, "terner_sb9_2021");
    }

    #[test]
    fn evidence_brief_flags_unusable_datasets() {
        let brief = Ontology::bundled().evidence_brief("adu", "permits", 5).unwrap();
        assert!(brief.contains("USABLE DATASETS"));
        assert!(brief.contains("microsoft_footprints"), "rejected sources are named as not usable");
    }
}
