use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    error::{AuditError, Result},
    model::{
        AuditReport, Evidence, EvidenceState, FactorRef, Finding, Severity, Target, TargetKind,
    },
    policy::{ManualRule, Policy, RepoRule},
};

const SKIPPED_DIRECTORIES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    "vendor",
    ".direnv",
    ".next",
    "dist",
    "build",
];

pub fn audit(path: &Path, policy: &Policy) -> Result<AuditReport> {
    let root = path.canonicalize().map_err(|source| AuditError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let files = collect_files(&root)?;
    let git = GitEvidence::read(&root);

    let mut findings = vec![source_history_finding(&git)];
    findings.extend(
        policy
            .repo_rules
            .iter()
            .map(|rule| evaluate_repo_rule(rule, &files)),
    );
    findings.extend(policy.manual_rules.iter().map(manual_finding));

    Ok(AuditReport::new(
        Target {
            kind: TargetKind::Repository,
            locator: root.display().to_string(),
            source_revision: git.head.clone(),
            dirty: git.dirty,
        },
        findings,
    ))
}

#[derive(Debug, Default)]
struct GitEvidence {
    head: Option<String>,
    remote: Option<String>,
    dirty: Option<bool>,
}

impl GitEvidence {
    fn read(root: &Path) -> Self {
        let head = git_output(root, &["rev-parse", "HEAD"]);
        if head.is_none() {
            return Self::default();
        }

        Self {
            head,
            remote: git_output(root, &["remote", "get-url", "origin"]),
            dirty: git_output(root, &["status", "--porcelain"]).map(|value| !value.is_empty()),
        }
    }
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn source_history_finding(git: &GitEvidence) -> Finding {
    let mut evidence = Vec::new();
    if let Some(head) = &git.head {
        evidence.push(Evidence {
            kind: "git-commit".into(),
            location: head.clone(),
            detail: "Current source revision".into(),
        });
    }
    if let Some(remote) = &git.remote {
        evidence.push(Evidence {
            kind: "git-remote".into(),
            location: redact_remote_credentials(remote),
            detail: "Authoritative origin candidate".into(),
        });
    }

    Finding {
        rule_id: "authoritative-source-history".into(),
        factor: FactorRef {
            number: 1,
            slug: "codebase".into(),
        },
        state: if git.head.is_some() && git.remote.is_some() {
            EvidenceState::Observed
        } else {
            EvidenceState::Missing
        },
        severity: Severity::Critical,
        title: "Authoritative source history".into(),
        rationale: "A commit plus a remote is the minimum evidence that a deploy can be mapped to one durable source history; neither proves that the deployed revision is actually mapped.".into(),
        evidence,
        next_step: "Record the deploy-to-commit mapping and verify that the named remote is authoritative for every promoted artifact.".into(),
    }
}

fn redact_remote_credentials(remote: &str) -> String {
    if let Ok(mut url) = url::Url::parse(remote)
        && matches!(url.scheme(), "http" | "https")
    {
        let _ = url.set_username("");
        let _ = url.set_password(None);
        url.set_query(None);
        url.set_fragment(None);
        return url.into();
    }

    let Some(scheme_end) = remote.find("://") else {
        return remote.to_owned();
    };
    let authority_start = scheme_end + 3;
    let Some(at_offset) = remote[authority_start..].find('@') else {
        return remote.to_owned();
    };
    let at = authority_start + at_offset;
    format!("{}{}", &remote[..authority_start], &remote[at + 1..])
}

fn evaluate_repo_rule(rule: &RepoRule, files: &[PathBuf]) -> Finding {
    let matches = files
        .iter()
        .filter(|path| rule_matches(rule, path))
        .take(8)
        .map(|path| Evidence {
            kind: "repository-path".into(),
            location: path.to_string_lossy().into_owned(),
            detail: format!("Matched evidence rule {}", rule.id),
        })
        .collect::<Vec<_>>();

    Finding {
        rule_id: rule.id.clone(),
        factor: FactorRef {
            number: rule.factor_number,
            slug: rule.factor_slug.clone(),
        },
        state: if matches.is_empty() {
            EvidenceState::Missing
        } else {
            EvidenceState::Observed
        },
        severity: rule.severity,
        title: rule.title.clone(),
        rationale: rule.rationale.clone(),
        evidence: matches,
        next_step: rule.next_step.clone(),
    }
}

fn manual_finding(rule: &ManualRule) -> Finding {
    Finding {
        rule_id: rule.id.clone(),
        factor: FactorRef {
            number: rule.factor_number,
            slug: rule.factor_slug.clone(),
        },
        state: EvidenceState::ManualReview,
        severity: rule.severity,
        title: rule.title.clone(),
        rationale: rule.rationale.clone(),
        evidence: Vec::new(),
        next_step: rule.next_step.clone(),
    }
}

fn rule_matches(rule: &RepoRule, path: &Path) -> bool {
    let normalized = format!(
        "/{}",
        path.to_string_lossy().replace('\\', "/").to_lowercase()
    );
    rule.any_path_suffixes
        .iter()
        .any(|suffix| normalized.ends_with(&suffix.to_lowercase()))
        || rule
            .any_path_fragments
            .iter()
            .any(|fragment| normalized.contains(&fragment.to_lowercase()))
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files_at(root, root, 0, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files_at(
    root: &Path,
    directory: &Path,
    depth: usize,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    if depth > 8 {
        return Ok(());
    }

    let entries = std::fs::read_dir(directory).map_err(|source| AuditError::Read {
        path: directory.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| AuditError::Read {
            path: directory.to_path_buf(),
            source,
        })?;
        let file_type = entry.file_type().map_err(|source| AuditError::Read {
            path: entry.path(),
            source,
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if file_type.is_dir() {
            if !SKIPPED_DIRECTORIES.contains(&name.as_ref()) {
                collect_files_at(root, &entry.path(), depth + 1, files)?;
            }
        } else if file_type.is_file() {
            files.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(&entry.path())
                    .to_path_buf(),
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_removed_from_https_remotes() {
        assert_eq!(
            redact_remote_credentials("https://token@example.com/org/repo.git"),
            "https://example.com/org/repo.git"
        );
        assert_eq!(
            redact_remote_credentials(
                "https://user:secret@example.com/org/repo.git?access_token=secret"
            ),
            "https://example.com/org/repo.git"
        );
        assert_eq!(
            redact_remote_credentials("git@github.com:org/repo.git"),
            "git@github.com:org/repo.git"
        );
    }

    #[test]
    fn repository_audit_covers_all_twenty_two_factors_without_a_score() {
        let temp = tempfile::tempdir().expect("temp directory");
        std::fs::write(temp.path().join("Cargo.lock"), "").expect("fixture");
        let policy = Policy::embedded().expect("embedded policy");

        let report = audit(temp.path(), &policy).expect("audit succeeds");
        let factors = report
            .findings
            .iter()
            .map(|finding| finding.factor.number)
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(factors, (1..=22).collect());
        let json = serde_json::to_string(&report).expect("serialize report");
        assert!(!json.contains("score"));
    }
}
