use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    error::{AuditError, Result},
    model::Severity,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Policy {
    pub schema_version: String,
    pub edition: String,
    pub catalog_url: String,
    pub repo_rules: Vec<RepoRule>,
    pub manual_rules: Vec<ManualRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepoRule {
    pub id: String,
    pub factor_number: u8,
    pub factor_slug: String,
    pub title: String,
    pub rationale: String,
    pub next_step: String,
    pub severity: Severity,
    #[serde(default)]
    pub any_path_suffixes: Vec<String>,
    #[serde(default)]
    pub any_path_fragments: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManualRule {
    pub id: String,
    pub factor_number: u8,
    pub factor_slug: String,
    pub title: String,
    pub rationale: String,
    pub next_step: String,
    pub severity: Severity,
}

impl Policy {
    pub fn embedded() -> Result<Self> {
        Self::parse(
            include_str!("../policies/default-v2.json"),
            "embedded policy",
        )
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let document = std::fs::read_to_string(path).map_err(|source| AuditError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&document, &path.display().to_string())
    }

    fn parse(document: &str, context: &str) -> Result<Self> {
        let policy: Self = serde_json::from_str(document).map_err(|source| AuditError::Json {
            context: context.into(),
            source,
        })?;
        policy.validate()?;
        Ok(policy)
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != "22-factor.policy/v1" {
            return Err(AuditError::Json {
                context: "policy schema_version".into(),
                source: serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "expected 22-factor.policy/v1",
                )),
            });
        }

        let mut ids = std::collections::BTreeSet::new();
        let mut factors = std::collections::BTreeSet::new();
        for (id, number) in self
            .repo_rules
            .iter()
            .map(|rule| (&rule.id, rule.factor_number))
            .chain(
                self.manual_rules
                    .iter()
                    .map(|rule| (&rule.id, rule.factor_number)),
            )
        {
            if !(1..=22).contains(&number) {
                return Err(AuditError::InvalidAssessment(format!(
                    "policy rule {id} uses factor {number}; expected 1 through 22"
                )));
            }
            if !ids.insert(id) {
                return Err(AuditError::InvalidAssessment(format!(
                    "policy rule id {id} is duplicated"
                )));
            }
            if !factors.insert(number) {
                return Err(AuditError::InvalidAssessment(format!(
                    "policy defines more than one primary rule for factor {number}"
                )));
            }
        }
        let expected = (2..=22).collect::<std::collections::BTreeSet<_>>();
        if factors != expected {
            return Err(AuditError::InvalidAssessment(
                "policy must define exactly one primary rule for every factor 2 through 22; factor 1 is supplied by source-history inspection".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedded_value() -> serde_json::Value {
        serde_json::from_str(include_str!("../policies/default-v2.json")).expect("policy JSON")
    }

    #[test]
    fn embedded_policy_covers_every_non_source_factor_once() {
        let policy = Policy::embedded().expect("embedded policy");
        let factors = policy
            .repo_rules
            .iter()
            .map(|rule| rule.factor_number)
            .chain(policy.manual_rules.iter().map(|rule| rule.factor_number))
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(factors, (2..=22).collect());
    }

    #[test]
    fn incomplete_policy_is_rejected() {
        let mut value = embedded_value();
        value["repo_rules"]
            .as_array_mut()
            .expect("repo rules")
            .retain(|rule| rule["factor_number"] != 20);
        let document = serde_json::to_string(&value).expect("serialize fixture");

        assert!(Policy::parse(&document, "incomplete fixture").is_err());
    }

    #[test]
    fn policy_cannot_redefine_the_source_history_factor() {
        let mut value = embedded_value();
        let rule = value["repo_rules"]
            .as_array_mut()
            .expect("repo rules")
            .iter_mut()
            .find(|rule| rule["factor_number"] == 20)
            .expect("factor 20");
        rule["factor_number"] = 1.into();
        let document = serde_json::to_string(&value).expect("serialize fixture");

        assert!(Policy::parse(&document, "source collision fixture").is_err());
    }
}
