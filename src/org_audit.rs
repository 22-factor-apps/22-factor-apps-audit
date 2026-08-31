use serde::Deserialize;

use crate::{
    error::Result,
    github::GithubClient,
    model::{
        AuditReport, Evidence, EvidenceState, FactorRef, Finding, Severity, Target, TargetKind,
    },
};

#[derive(Debug, Deserialize)]
struct Organization {
    html_url: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Repository {
    name: String,
    html_url: String,
    description: Option<String>,
    default_branch: String,
    archived: bool,
    disabled: bool,
    fork: bool,
}

pub fn audit(owner: &str, client: &GithubClient) -> Result<AuditReport> {
    let organization: Organization = client.get(&format!("orgs/{owner}"))?;
    let repositories = list_repositories(owner, client)?;
    let active = repositories
        .iter()
        .filter(|repo| !repo.archived && !repo.disabled && !repo.fork)
        .collect::<Vec<_>>();

    let common_security = any_path_exists(
        client,
        owner,
        ".github",
        &["SECURITY.md", ".github/SECURITY.md"],
    )?;
    let mut workflows = Vec::new();
    let mut formal_methods = Vec::new();
    let mut safe_languages = Vec::new();
    for repository in &active {
        if client.path_exists(&format!(
            "repos/{owner}/{}/contents/.github/workflows",
            repository.name
        ))? {
            workflows.push(repository.html_url.clone());
        }
        if any_path_exists(
            client,
            owner,
            &repository.name,
            &[
                "docs/formal-methods",
                "formal",
                "specs",
                "models",
                "model.tla",
                "spec.tla",
            ],
        )? {
            formal_methods.push(repository.html_url.clone());
        }
        if any_path_exists(
            client,
            owner,
            &repository.name,
            &[
                "Cargo.toml",
                "rust-toolchain.toml",
                "analysis_options.yaml",
                "tsconfig.json",
                "docs/language-safety.md",
            ],
        )? {
            safe_languages.push(repository.html_url.clone());
        }
    }

    let findings = vec![
        source_portfolio_finding(&active),
        repository_context_finding(&active),
        security_baseline_finding(common_security, owner),
        release_automation_finding(&active, &workflows),
        formal_methods_finding(&formal_methods),
        safe_languages_finding(&safe_languages),
    ];

    Ok(AuditReport::new(
        Target {
            kind: TargetKind::GithubOrganization,
            locator: organization.html_url,
            source_revision: None,
            dirty: None,
        },
        findings,
    ))
}

fn list_repositories(owner: &str, client: &GithubClient) -> Result<Vec<Repository>> {
    let mut repositories = Vec::new();
    for page in 1..=10 {
        let mut batch: Vec<Repository> = client.get(&format!(
            "orgs/{owner}/repos?type=all&sort=full_name&per_page=100&page={page}"
        ))?;
        let done = batch.len() < 100;
        repositories.append(&mut batch);
        if done {
            break;
        }
    }
    Ok(repositories)
}

fn any_path_exists(
    client: &GithubClient,
    owner: &str,
    repository: &str,
    paths: &[&str],
) -> Result<bool> {
    for path in paths {
        if client.path_exists(&format!("repos/{owner}/{repository}/contents/{path}"))? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn source_portfolio_finding(repositories: &[&Repository]) -> Finding {
    let complete = !repositories.is_empty()
        && repositories
            .iter()
            .all(|repo| !repo.default_branch.trim().is_empty());
    Finding {
        rule_id: "org-source-portfolio".into(),
        factor: FactorRef {
            number: 1,
            slug: "codebase".into(),
        },
        state: if complete {
            EvidenceState::Observed
        } else {
            EvidenceState::Missing
        },
        severity: Severity::Critical,
        title: "Repository source portfolio".into(),
        rationale: "Active, non-fork repositories with default branches are the minimum visible source-history boundary for an organization.".into(),
        evidence: repositories
            .iter()
            .map(|repo| Evidence {
                kind: "github-repository".into(),
                location: repo.html_url.clone(),
                detail: format!("Default branch: {}", repo.default_branch),
            })
            .collect(),
        next_step: "Map every deployed or distributed artifact to exactly one repository and immutable revision; archive superseded repositories deliberately.".into(),
    }
}

fn repository_context_finding(repositories: &[&Repository]) -> Finding {
    let missing = repositories
        .iter()
        .filter(|repo| repo.description.as_deref().is_none_or(str::is_empty))
        .map(|repo| repo.name.as_str())
        .collect::<Vec<_>>();
    Finding {
        rule_id: "org-repository-context".into(),
        factor: FactorRef {
            number: 11,
            slug: "contract-first-interfaces".into(),
        },
        state: if missing.is_empty() && !repositories.is_empty() {
            EvidenceState::Observed
        } else {
            EvidenceState::Missing
        },
        severity: Severity::Important,
        title: "Repository purpose is discoverable".into(),
        rationale: "Repository descriptions are a small but useful public contract: they identify purpose before a consumer reads implementation.".into(),
        evidence: repositories
            .iter()
            .filter_map(|repo| {
                repo.description.as_ref().map(|description| Evidence {
                    kind: "github-description".into(),
                    location: repo.html_url.clone(),
                    detail: description.clone(),
                })
            })
            .collect(),
        next_step: if missing.is_empty() {
            "Review descriptions against actual consumer-visible responsibilities and link schemas from each README.".into()
        } else {
            format!("Add concrete descriptions for: {}.", missing.join(", "))
        },
    }
}

fn security_baseline_finding(exists: bool, owner: &str) -> Finding {
    Finding {
        rule_id: "org-security-baseline".into(),
        factor: FactorRef {
            number: 12,
            slug: "secure-by-design".into(),
        },
        state: if exists {
            EvidenceState::Observed
        } else {
            EvidenceState::Missing
        },
        severity: Severity::Critical,
        title: "Organization security reporting baseline".into(),
        rationale: "A default SECURITY.md gives every repository a discoverable reporting and response boundary; it does not substitute for a threat model.".into(),
        evidence: exists
            .then(|| Evidence {
                kind: "github-community-health".into(),
                location: format!("https://github.com/{owner}/.github/blob/main/SECURITY.md"),
                detail: "Organization-wide security policy".into(),
            })
            .into_iter()
            .collect(),
        next_step: "Publish SECURITY.md in the public .github repository, then add repository-specific threat and response detail where the common policy is insufficient.".into(),
    }
}

fn release_automation_finding(repositories: &[&Repository], workflows: &[String]) -> Finding {
    Finding {
        rule_id: "org-reviewed-automation".into(),
        factor: FactorRef {
            number: 14,
            slug: "supply-chain-integrity".into(),
        },
        state: if !repositories.is_empty() && workflows.len() == repositories.len() {
            EvidenceState::Observed
        } else {
            EvidenceState::Missing
        },
        severity: Severity::Critical,
        title: "Reviewed automation is present".into(),
        rationale: "Checked-in workflows provide a review surface for builds and releases; their existence does not establish pinning, isolation, or provenance.".into(),
        evidence: workflows
            .iter()
            .map(|location| Evidence {
                kind: "github-workflows".into(),
                location: format!("{location}/tree/main/.github/workflows"),
                detail: "Repository automation directory".into(),
            })
            .collect(),
        next_step: "Add CI to each active repository, pin third-party actions exactly, and verify release provenance and consumer installation.".into(),
    }
}

fn formal_methods_finding(repositories: &[String]) -> Finding {
    Finding {
        rule_id: "org-formal-methods".into(),
        factor: FactorRef {
            number: 20,
            slug: "formal-methods-functional-core".into(),
        },
        state: if repositories.is_empty() {
            EvidenceState::Missing
        } else {
            EvidenceState::Observed
        },
        severity: Severity::Critical,
        title: "Formal-methods evidence is discoverable".into(),
        rationale: "A checked-in model or formal-methods directory makes critical state and invariants reviewable; its existence does not prove correspondence with implementation.".into(),
        evidence: repositories
            .iter()
            .map(|location| Evidence {
                kind: "github-formal-methods".into(),
                location: location.clone(),
                detail: "Repository contains a formal model or formal-methods path".into(),
            })
            .collect(),
        next_step: "Model the highest-risk lifecycle, check safety and progress properties, and record how the model refines into exhaustive production transitions.".into(),
    }
}

fn safe_languages_finding(repositories: &[String]) -> Finding {
    Finding {
        rule_id: "org-safe-languages".into(),
        factor: FactorRef {
            number: 21,
            slug: "safe-languages-total-types".into(),
        },
        state: if repositories.is_empty() {
            EvidenceState::Missing
        } else {
            EvidenceState::Observed
        },
        severity: Severity::Critical,
        title: "Memory-safe language evidence is discoverable".into(),
        rationale: "A memory-safe toolchain or language-safety record is a review lead; it does not establish strict nullability, safe transitive dependencies, or correct unsafe boundaries.".into(),
        evidence: repositories
            .iter()
            .map(|location| Evidence {
                kind: "github-language-safety".into(),
                location: location.clone(),
                detail: "Repository contains a memory-safe toolchain or language-safety path".into(),
            })
            .collect(),
        next_step: "Prefer memory-safe languages, enable strict type and null checks, isolate unsafe or foreign code, and publish a risk-ranked migration path for legacy components.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revised_organization_signals_use_the_2026_3_factors() {
        let formal = formal_methods_finding(&["https://github.com/example/spec".into()]);
        let safe = safe_languages_finding(&["https://github.com/example/rust".into()]);

        assert_eq!(formal.factor.number, 20);
        assert_eq!(formal.factor.slug, "formal-methods-functional-core");
        assert_eq!(formal.state, EvidenceState::Observed);
        assert_eq!(safe.factor.number, 21);
        assert_eq!(safe.factor.slug, "safe-languages-total-types");
        assert_eq!(safe.state, EvidenceState::Observed);
    }
}
