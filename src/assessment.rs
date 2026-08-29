use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{AuditError, Result};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Catalog {
    pub schema_version: String,
    pub edition: String,
    pub source: CatalogSource,
    pub factors: Vec<CatalogFactor>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogSource {
    pub repository: String,
    pub release: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogFactor {
    pub number: u8,
    pub numeral: String,
    pub slug: String,
    pub title: String,
    pub commandment: String,
    pub boundary: String,
    pub litmus_test: String,
    pub url: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssessmentStatus {
    NotAssessed,
    Satisfied,
    Partial,
    NotSatisfied,
    NotApplicable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AssessmentEvidence {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AssessmentFactor {
    pub number: u8,
    pub slug: String,
    pub title: String,
    pub commandment: String,
    pub boundary: String,
    pub litmus_test: String,
    pub status: AssessmentStatus,
    pub rationale: String,
    pub evidence: Vec<AssessmentEvidence>,
    pub owner: Option<String>,
    pub review_date: Option<String>,
    pub follow_up: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AssessmentSource {
    pub catalog_url: String,
    pub repository: String,
    pub release: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Assessment {
    pub schema_version: String,
    pub edition: String,
    pub target: String,
    pub source: AssessmentSource,
    pub factors: Vec<AssessmentFactor>,
    pub overlays: Vec<String>,
    pub interpretation: String,
}

pub fn load_catalog(source: &str) -> Result<Catalog> {
    let document = if source.starts_with("https://") || source.starts_with("http://") {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        let mut response = agent
            .get(source)
            .header("User-Agent", "twenty-two-factor-audit/0.1")
            .call()
            .map_err(|error| AuditError::Catalog {
                url: source.into(),
                message: error.to_string(),
            })?;
        response
            .body_mut()
            .read_to_string()
            .map_err(|error| AuditError::Catalog {
                url: source.into(),
                message: error.to_string(),
            })?
    } else {
        std::fs::read_to_string(source).map_err(|source_error| AuditError::Read {
            path: Path::new(source).to_path_buf(),
            source: source_error,
        })?
    };

    let mut catalog: Catalog =
        serde_json::from_str(&document).map_err(|source_error| AuditError::Json {
            context: source.into(),
            source: source_error,
        })?;
    catalog.factors.sort_by_key(|factor| factor.number);
    validate_catalog(&catalog)?;
    Ok(catalog)
}

pub fn initialize(catalog: &Catalog, catalog_url: &str, target: String) -> Assessment {
    Assessment {
        schema_version: "22-factor.assessment/v1".into(),
        edition: catalog.edition.clone(),
        target,
        source: AssessmentSource {
            catalog_url: catalog_url.into(),
            repository: catalog.source.repository.clone(),
            release: catalog.source.release.clone(),
        },
        factors: catalog
            .factors
            .iter()
            .map(|factor| AssessmentFactor {
                number: factor.number,
                slug: factor.slug.clone(),
                title: factor.title.clone(),
                commandment: factor.commandment.clone(),
                boundary: factor.boundary.clone(),
                litmus_test: factor.litmus_test.clone(),
                status: AssessmentStatus::NotAssessed,
                rationale: String::new(),
                evidence: Vec::new(),
                owner: None,
                review_date: None,
                follow_up: None,
            })
            .collect(),
        overlays: Vec::new(),
        interpretation: "This assessment records contextual evidence per factor. It intentionally has no aggregate maturity score; one critical failure must not disappear inside a total.".into(),
    }
}

pub fn read(path: &Path) -> Result<Assessment> {
    let document = std::fs::read_to_string(path).map_err(|source| AuditError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&document).map_err(|source| AuditError::Json {
        context: path.display().to_string(),
        source,
    })
}

pub fn write(path: &Path, assessment: &Assessment, force: bool) -> Result<()> {
    let mut document =
        serde_json::to_string_pretty(assessment).map_err(|source| AuditError::Json {
            context: path.display().to_string(),
            source,
        })?;
    document.push('\n');
    let mut options = std::fs::OpenOptions::new();
    options.write(true);
    if force {
        options.create(true).truncate(true);
    } else {
        options.create_new(true);
    }
    let mut file = options.open(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::AlreadyExists {
            AuditError::OutputExists(path.to_path_buf())
        } else {
            AuditError::Write {
                path: path.to_path_buf(),
                source,
            }
        }
    })?;
    std::io::Write::write_all(&mut file, document.as_bytes()).map_err(|source| AuditError::Write {
        path: path.to_path_buf(),
        source,
    })
}

pub fn validate(assessment: &Assessment) -> Result<()> {
    if assessment.schema_version != "22-factor.assessment/v1" {
        return invalid("schema_version must be 22-factor.assessment/v1");
    }
    if assessment.target.trim().is_empty() {
        return invalid("target must name the application or system being assessed");
    }
    if assessment.source.release.trim().is_empty() {
        return invalid("source.release must preserve the catalog release used");
    }
    validate_http_url("source.catalog_url", &assessment.source.catalog_url)?;
    validate_http_url("source.repository", &assessment.source.repository)?;

    if assessment.factors.len() != 22 {
        return invalid(format!(
            "expected 22 factor entries, found {}",
            assessment.factors.len()
        ));
    }

    let mut numbers = std::collections::BTreeSet::new();
    let mut slugs = std::collections::BTreeSet::new();
    for factor in &assessment.factors {
        if !numbers.insert(factor.number) {
            return invalid(format!("factor {} is duplicated", factor.number));
        }
        if !slugs.insert(factor.slug.as_str()) {
            return invalid(format!("factor slug {} is duplicated", factor.slug));
        }
        if matches!(factor.status, AssessmentStatus::NotApplicable)
            && factor.rationale.trim().is_empty()
        {
            return invalid(format!(
                "factor {} is not-applicable but has no rationale",
                factor.number
            ));
        }
        if matches!(factor.status, AssessmentStatus::Satisfied) && factor.evidence.is_empty() {
            return invalid(format!(
                "factor {} is satisfied but has no evidence link",
                factor.number
            ));
        }
        if factor.owner.as_deref().is_some_and(str::is_empty) {
            return invalid(format!("factor {} has an empty owner", factor.number));
        }
        if let Some(review_date) = &factor.review_date
            && !looks_like_iso_date(review_date)
        {
            return invalid(format!(
                "factor {} review_date must use YYYY-MM-DD",
                factor.number
            ));
        }
        for (index, evidence) in factor.evidence.iter().enumerate() {
            if evidence.label.trim().is_empty() {
                return invalid(format!(
                    "factor {} evidence {} has no label",
                    factor.number,
                    index + 1
                ));
            }
            validate_http_url(
                &format!("factor {} evidence {}", factor.number, index + 1),
                &evidence.url,
            )?;
        }
    }

    let expected = (1..=22).collect::<std::collections::BTreeSet<_>>();
    if numbers != expected {
        return invalid("factor numbers must be exactly 1 through 22");
    }
    Ok(())
}

fn validate_catalog(catalog: &Catalog) -> Result<()> {
    if catalog.schema_version != "22-factor.catalog/v1" {
        return invalid("catalog schema_version must be 22-factor.catalog/v1");
    }
    if catalog.factors.len() != 22 {
        return invalid(format!(
            "catalog must contain 22 factors, found {}",
            catalog.factors.len()
        ));
    }
    for (index, factor) in catalog.factors.iter().enumerate() {
        let expected = u8::try_from(index + 1).expect("22 factors fit in u8");
        if factor.number != expected {
            return invalid("catalog factor numbers must be exactly 1 through 22");
        }
        validate_http_url(&format!("factor {} url", factor.number), &factor.url)?;
    }
    validate_http_url("catalog source repository", &catalog.source.repository)
}

fn validate_http_url(field: &str, value: &str) -> Result<()> {
    let url = Url::parse(value)
        .map_err(|error| AuditError::InvalidAssessment(format!("{field} is invalid: {error}")))?;
    if !matches!(url.scheme(), "https" | "http") {
        return invalid(format!("{field} must use http or https"));
    }
    Ok(())
}

fn looks_like_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(AuditError::InvalidAssessment(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        Catalog {
            schema_version: "22-factor.catalog/v1".into(),
            edition: "test".into(),
            source: CatalogSource {
                repository: "https://example.com/source".into(),
                release: "v1".into(),
            },
            factors: (1..=22)
                .map(|number| CatalogFactor {
                    number,
                    numeral: number.to_string(),
                    slug: format!("factor-{number}"),
                    title: format!("Factor {number}"),
                    commandment: "Do the contextual thing.".into(),
                    boundary: "Do not cargo-cult the signal.".into(),
                    litmus_test: "Can you prove it?".into(),
                    url: format!("https://example.com/factors/{number}"),
                })
                .collect(),
        }
    }

    #[test]
    fn initialized_assessment_has_no_score_and_valid_source_provenance() {
        let assessment = initialize(&catalog(), "https://example.com/catalog.json", "app".into());
        validate(&assessment).expect("initial assessment is valid");
        let json = serde_json::to_value(&assessment).expect("serialize");
        assert!(json.get("score").is_none());
        assert_eq!(assessment.source.release, "v1");
    }

    #[test]
    fn not_applicable_requires_rationale() {
        let mut assessment =
            initialize(&catalog(), "https://example.com/catalog.json", "app".into());
        assessment.factors[0].status = AssessmentStatus::NotApplicable;
        assert!(validate(&assessment).is_err());
    }

    #[test]
    fn satisfied_requires_evidence() {
        let mut assessment =
            initialize(&catalog(), "https://example.com/catalog.json", "app".into());
        assessment.factors[0].status = AssessmentStatus::Satisfied;
        assert!(validate(&assessment).is_err());
    }

    #[test]
    fn assessment_output_is_fail_closed_by_default() {
        let temp = tempfile::tempdir().expect("temp directory");
        let path = temp.path().join("assessment.json");
        let assessment = initialize(&catalog(), "https://example.com/catalog.json", "app".into());

        write(&path, &assessment, false).expect("first write succeeds");
        assert!(matches!(
            write(&path, &assessment, false),
            Err(AuditError::OutputExists(existing)) if existing == path
        ));
        write(&path, &assessment, true).expect("explicit replacement succeeds");
    }
}
