use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetKind {
    Repository,
    GithubOrganization,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceState {
    Observed,
    Missing,
    ManualReview,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Advisory,
    Important,
    Critical,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FactorRef {
    pub number: u8,
    pub slug: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Evidence {
    pub kind: String,
    pub location: String,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub factor: FactorRef,
    pub state: EvidenceState,
    pub severity: Severity,
    pub title: String,
    pub rationale: String,
    pub evidence: Vec<Evidence>,
    pub next_step: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Target {
    pub kind: TargetKind,
    pub locator: String,
    pub source_revision: Option<String>,
    pub dirty: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuditReport {
    pub schema_version: String,
    pub edition: String,
    pub catalog_url: String,
    pub generated_at_unix_seconds: u64,
    pub target: Target,
    pub findings: Vec<Finding>,
    pub interpretation: String,
}

impl AuditReport {
    pub fn new(target: Target, mut findings: Vec<Finding>) -> Self {
        findings.sort_by_key(|finding| (finding.factor.number, finding.rule_id.clone()));
        let generated_at_unix_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        Self {
            schema_version: "22-factor.audit-report/v1".into(),
            edition: crate::EDITION.into(),
            catalog_url: crate::DEFAULT_CATALOG_URL.into(),
            generated_at_unix_seconds,
            target,
            findings,
            interpretation: "Evidence discovery is not compliance certification. An observed file is a review lead, not proof that the factor is satisfied; missing evidence can also be supplied outside the repository.".into(),
        }
    }
}
