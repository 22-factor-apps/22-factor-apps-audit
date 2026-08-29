use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use twenty_two_factor_audit::{
    DEFAULT_CATALOG_URL, assessment,
    error::{AuditError, Result},
    github::GithubClient,
    model::{AuditReport, EvidenceState, Severity},
    org_audit,
    policy::Policy,
    render, repo_audit,
};

#[derive(Debug, Parser)]
#[command(
    name = "twenty-two",
    version,
    about = "Discover evidence for the Twenty-Two-Factor App without manufacturing a maturity score"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Audit evidence in a local repository or a GitHub organization.
    Audit {
        #[command(subcommand)]
        target: AuditTarget,
    },
    /// Initialize or validate a contextual, evidence-bearing assessment.
    Assessment {
        #[command(subcommand)]
        action: AssessmentAction,
    },
    /// Print a bundled JSON Schema.
    Schema {
        #[arg(value_enum)]
        kind: SchemaKind,
    },
}

#[derive(Debug, Subcommand)]
enum AuditTarget {
    /// Inspect a local repository. File matches are review leads, not compliance proof.
    Repo {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        policy: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
        #[arg(long, value_enum, default_value = "never")]
        fail_on: FailOn,
    },
    /// Inspect public GitHub organization metadata and common policy evidence.
    Org {
        owner: String,
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
        #[arg(long, value_enum, default_value = "never")]
        fail_on: FailOn,
    },
}

#[derive(Debug, Subcommand)]
enum AssessmentAction {
    /// Create a 22-entry assessment from the canonical generated catalog.
    Init {
        #[arg(long)]
        target: String,
        #[arg(long, default_value = DEFAULT_CATALOG_URL)]
        catalog: String,
        #[arg(long, default_value = "assessment.json")]
        output: PathBuf,
        /// Replace an existing output file. Omitted by default to prevent evidence loss.
        #[arg(long)]
        force: bool,
    },
    /// Validate assessment semantics in addition to its JSON shape.
    Validate { path: PathBuf },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum FailOn {
    Never,
    Missing,
    Critical,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SchemaKind {
    Assessment,
    AuditReport,
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("error: {error}");
        std::process::exit(2);
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
