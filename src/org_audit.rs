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

    let profile_exists =
        client.path_exists(&format!("repos/{owner}/.github/contents/profile/README.md"))?;
    let common_security = any_path_exists(
        client,
        owner,
        ".github",
        &["SECURITY.md", ".github/SECURITY.md"],
    )?;
    let common_owners = any_path_exists(
        client,
        owner,
        ".github",
        &["CODEOWNERS", ".github/CODEOWNERS", "docs/CODEOWNERS"],
    )?;

    let mut workflows = Vec::new();
    for repository in &active {
        if client.path_exists(&format!(
            "repos/{owner}/{}/contents/.github/workflows",
            repository.name
        ))? {
            workflows.push(repository.html_url.clone());
        }
    }

    let findings = vec![
        source_portfolio_finding(&active),
        repository_context_finding(&active),
        security_baseline_finding(common_security, owner),
        release_automation_finding(&active, &workflows),
        ownership_finding(profile_exists, common_owners, owner),
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

fn ownership_finding(profile_exists: bool, codeowners_exists: bool, owner: &str) -> Finding {
    let observed = profile_exists && codeowners_exists;
    let mut evidence = Vec::new();
    if profile_exists {
        evidence.push(Evidence {
            kind: "github-organization-profile".into(),
            location: format!("https://github.com/{owner}"),
            detail: "Public organization profile README".into(),
        });
    }
    if codeowners_exists {
        evidence.push(Evidence {
            kind: "github-codeowners".into(),
            location: format!("https://github.com/{owner}/.github"),
            detail: "Organization-wide ownership default".into(),
        });
    }
    Finding {
        rule_id: "org-outcome-ownership".into(),
        factor: FactorRef {
            number: 20,
            slug: "outcome-ownership".into(),
        },
        state: if observed {
            EvidenceState::Observed
        } else {
            EvidenceState::Missing
        },
        severity: Severity::Critical,
        title: "Organization purpose and review ownership".into(),
        rationale: "A profile plus default ownership makes purpose and review responsibility discoverable, while outcome authority still requires human verification.".into(),
        evidence,
        next_step: "Publish an organization profile and CODEOWNERS default, then verify that one empowered team owns each user outcome through retirement.".into(),
    }
}
