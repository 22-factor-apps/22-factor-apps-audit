mod cli_contract;

use cli_contract::{
    AssessmentAction, AuditTarget, Cli, Command, FailOn, OutputFormat, ParseOutcome, SchemaKind,
};
use twenty_two_factor_audit::{
    DEFAULT_CATALOG_URL, assessment,
    error::{AuditError, Result},
    github::GithubClient,
    model::{AuditReport, EvidenceState, Severity},
    org_audit,
    policy::Policy,
    render, repo_audit,
};

fn main() {
    let argv = std::env::args().collect::<Vec<_>>();
    match cli_contract::parse(&argv) {
        Ok(ParseOutcome::Print(output)) => print!("{output}"),
        Ok(ParseOutcome::Run(cli)) => {
            if let Err(error) = run(cli) {
                eprintln!("error: {error}");
                std::process::exit(2);
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(2);
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Audit { target } => match target {
            AuditTarget::Repo {
                path,
                policy,
                format,
                fail_on,
            } => {
                let policy = policy
                    .as_deref()
                    .map(Policy::from_path)
                    .unwrap_or_else(Policy::embedded)?;
                let report = repo_audit::audit(&path, &policy)?;
                emit_report(&report, format)?;
                exit_if_required(&report, fail_on);
                Ok(())
            }
            AuditTarget::Org {
                owner,
                format,
                fail_on,
            } => {
                let report = org_audit::audit(&owner, &GithubClient::from_environment())?;
                emit_report(&report, format)?;
                exit_if_required(&report, fail_on);
                Ok(())
            }
        },
        Command::Assessment { action } => match action {
            AssessmentAction::Init {
                target,
                catalog,
                output,
                force,
            } => {
                let catalog_document = assessment::load_catalog(&catalog)?;
                let catalog_url =
                    if catalog.starts_with("https://") || catalog.starts_with("http://") {
                        catalog.as_str()
                    } else {
                        DEFAULT_CATALOG_URL
                    };
                let assessment = assessment::initialize(&catalog_document, catalog_url, target);
                assessment::validate(&assessment)?;
                assessment::write(&output, &assessment, force)?;
                println!("wrote {}", output.display());
                Ok(())
            }
            AssessmentAction::Validate { path } => {
                let assessment = assessment::read(&path)?;
                assessment::validate(&assessment)?;
                println!("valid assessment: {}", path.display());
                Ok(())
            }
        },
        Command::Schema { kind } => {
            match kind {
                SchemaKind::Assessment => {
                    print!("{}", include_str!("../schemas/assessment-v1.schema.json"));
                }
                SchemaKind::AuditReport => {
                    print!("{}", include_str!("../schemas/audit-report-v1.schema.json"));
                }
            }
            Ok(())
        }
    }
}

fn emit_report(report: &AuditReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Text => print!("{}", render::text(report)),
        OutputFormat::Json => {
            let output =
                serde_json::to_string_pretty(report).map_err(|source| AuditError::Json {
                    context: "audit report".into(),
                    source,
                })?;
            println!("{output}");
        }
    }
    Ok(())
}

fn exit_if_required(report: &AuditReport, threshold: FailOn) {
    let should_fail = match threshold {
        FailOn::Never => false,
        FailOn::Missing => report
            .findings
            .iter()
            .any(|finding| finding.state == EvidenceState::Missing),
        FailOn::Critical => report.findings.iter().any(|finding| {
            finding.state == EvidenceState::Missing && finding.severity == Severity::Critical
        }),
    };
    if should_fail {
        std::process::exit(1);
    }
}
