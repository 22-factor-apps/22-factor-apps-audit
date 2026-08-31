//! The `twenty-two` command-line boundary, owned by flags-2-env.
//!
//! `.cli-flags.toml` is the only option and command schema. The native parser
//! resolves command paths, rejects unknown options, applies environment/argv
//! precedence, and coerces typed values before this module builds an
//! application invocation.

use std::collections::BTreeSet;
use std::ffi::{CStr, CString};
use std::io::Write;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};

use flags2env::BundledFlags2Env;
use serde::Deserialize;
use tempfile::NamedTempFile;

use twenty_two_factor_audit::DEFAULT_CATALOG_URL;

const EMBEDDED_CONTRACT: &str = include_str!("../.cli-flags.toml");
const CONTRACT_OVERRIDE_ENV: &str = "TWENTY_TWO_FLAGS_CONFIG";

#[derive(Debug)]
pub struct Cli {
    pub command: Command,
}

#[derive(Debug)]
pub enum Command {
    Audit { target: AuditTarget },
    Assessment { action: AssessmentAction },
    Schema { kind: SchemaKind },
}

#[derive(Debug)]
pub enum AuditTarget {
    Repo {
        path: PathBuf,
        policy: Option<PathBuf>,
        format: OutputFormat,
        fail_on: FailOn,
    },
    Org {
        owner: String,
        format: OutputFormat,
        fail_on: FailOn,
    },
}

#[derive(Debug)]
pub enum AssessmentAction {
    Init {
        target: String,
        catalog: String,
        output: PathBuf,
        force: bool,
    },
    Validate {
        path: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailOn {
    Never,
    Missing,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaKind {
    Assessment,
    AuditReport,
}

#[derive(Debug)]
pub enum ParseOutcome {
    Run(Cli),
    Print(String),
}

#[derive(Debug, Deserialize)]
struct TypedConfig {
    #[serde(rename = "FLAGS2ENV_COMMAND")]
    command: Option<String>,
    #[serde(rename = "TWENTY_TWO_VERSION", default)]
    version: bool,
    #[serde(rename = "TWENTY_TWO_FORMAT")]
    format: Option<String>,
    #[serde(rename = "TWENTY_TWO_FAIL_ON")]
    fail_on: Option<String>,
    #[serde(rename = "TWENTY_TWO_POLICY")]
    policy: Option<String>,
    #[serde(rename = "TWENTY_TWO_TARGET")]
    target: Option<String>,
    #[serde(rename = "TWENTY_TWO_CATALOG")]
    catalog: Option<String>,
    #[serde(rename = "TWENTY_TWO_OUTPUT")]
    output: Option<String>,
    #[serde(rename = "TWENTY_TWO_FORCE", default)]
    force: bool,
}

struct ContractFile {
    path: PathBuf,
    // Keeps the embedded fallback alive until the parser has finished reading
    // it. Installed binaries therefore do not depend on Cargo's source cache.
    _temporary: Option<NamedTempFile>,
}

pub fn parse(argv: &[String]) -> Result<ParseOutcome, String> {
    let environment = std::env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)));
    let contract = resolve_contract()?;
    parse_with_environment(argv, &contract.path, environment)
}

fn parse_with_environment(
    argv: &[String],
    contract_path: &Path,
    environment: impl IntoIterator<Item = (String, String)>,
) -> Result<ParseOutcome, String> {
    if help_requested(argv) {
        return help_table(contract_path, argv).map(ParseOutcome::Print);
    }

    let contract_path = contract_path
        .to_str()
        .ok_or_else(|| ".cli-flags.toml path is not valid UTF-8".to_owned())?;
    let parser = BundledFlags2Env::new();
    parser
        .audit_config(Some(contract_path))
        .map_err(|error| format!("flags-2-env configuration audit failed: {error}"))?;
    let parsed = parser
        .parse_structured(argv, Some(contract_path))
        .map_err(|error| format!("flags-2-env parse failed: {error}"))?;

    if !parsed.unknown_options.is_empty() {
        let names = parsed
            .unknown_options
            .iter()
            .map(|option| diagnostic_option_name(option))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("unknown command-line option(s): {names}"));
    }
    if !parsed.errors.is_empty() {
        return Err(format!(
            "invalid command-line value(s): {}",
            parsed.errors.join("; ")
        ));
    }

    // flags-2-env returns each precedence channel separately. Preserve the
    // contract's order without mutating the process environment:
    // dotenv < environment < dotenv_override < argv.
    let mut raw_config = parsed.dotenv;
    raw_config.extend(environment);
    raw_config.extend(parsed.dotenv_overrides);
    raw_config.remove("FLAGS2ENV_COMMAND");
    raw_config.extend(parsed.provided_flags);
    let typed = parser
        .coerce::<TypedConfig, _>(&raw_config, Some(contract_path))
        .map_err(|error| format!("invalid typed CLI configuration: {error}"))?;

    if typed.version {
        return Ok(ParseOutcome::Print(format!(
            "twenty-two {}\n",
            env!("CARGO_PKG_VERSION")
        )));
    }

    let command_path = parsed.subcommands.as_slice();
    let command = match command_path {
        [audit, repo] if audit == "audit" && repo == "repo" => {
            let path = optional_single_operand(&parsed.extras, "audit repo [PATH]")?
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            Command::Audit {
                target: AuditTarget::Repo {
                    path,
                    policy: typed.policy.map(PathBuf::from),
                    format: output_format(typed.format.as_deref())?,
                    fail_on: fail_on(typed.fail_on.as_deref())?,
                },
            }
        }
        [audit, org] if audit == "audit" && org == "org" => {
            let owner = required_single_operand(&parsed.extras, "audit org OWNER")?;
            Command::Audit {
                target: AuditTarget::Org {
                    owner,
                    format: output_format(typed.format.as_deref())?,
                    fail_on: fail_on(typed.fail_on.as_deref())?,
                },
            }
        }
        [assessment, init] if assessment == "assessment" && init == "init" => {
            no_operands(&parsed.extras, "assessment init")?;
            let target = nonempty_required(typed.target, "--target")?;
            let catalog = typed
                .catalog
                .unwrap_or_else(|| DEFAULT_CATALOG_URL.to_owned());
            let output =
                PathBuf::from(typed.output.unwrap_or_else(|| "assessment.json".to_owned()));
            Command::Assessment {
                action: AssessmentAction::Init {
                    target,
                    catalog,
                    output,
                    force: typed.force,
                },
            }
        }
        [assessment, validate] if assessment == "assessment" && validate == "validate" => {
            Command::Assessment {
                action: AssessmentAction::Validate {
                    path: PathBuf::from(required_single_operand(
                        &parsed.extras,
                        "assessment validate PATH",
                    )?),
                },
            }
        }
        [schema] if schema == "schema" => Command::Schema {
            kind: schema_kind(&required_single_operand(
                &parsed.extras,
                "schema {assessment|audit-report}",
            )?)?,
        },
        [] => {
            return Err(
                "a command is required: audit, assessment, or schema (pass --help for usage)"
                    .to_owned(),
            );
        }
        path => {
            return Err(format!(
                "incomplete or unsupported command path: {}",
                path.join(" ")
            ));
        }
    };

    if typed.command.as_deref() != Some(command_path.join(" ").as_str()) {
        return Err("flags-2-env command channels disagreed".to_owned());
    }

    Ok(ParseOutcome::Run(Cli { command }))
}

fn resolve_contract() -> Result<ContractFile, String> {
    if let Some(path) = std::env::var_os(CONTRACT_OVERRIDE_ENV).filter(|value| !value.is_empty()) {
        let path = PathBuf::from(path);
        if !path.is_file() {
            return Err(format!(
                "{CONTRACT_OVERRIDE_ENV} does not point to a readable file"
            ));
        }
        return Ok(ContractFile {
            path,
            _temporary: None,
        });
    }

    let manifest_contract = Path::new(env!("CARGO_MANIFEST_DIR")).join(".cli-flags.toml");
    let adjacent_contract = std::env::current_exe().ok().and_then(|executable| {
        executable
            .parent()
            .map(|parent| parent.join(".cli-flags.toml"))
    });
    if let Some(path) = adjacent_contract
        .into_iter()
        .chain([manifest_contract])
        .find(|candidate| candidate.is_file())
    {
        return Ok(ContractFile {
            path,
            _temporary: None,
        });
    }

    embedded_contract_file()
}

fn embedded_contract_file() -> Result<ContractFile, String> {
    let mut temporary = NamedTempFile::new()
        .map_err(|error| format!("could not create embedded CLI contract: {error}"))?;
    temporary
        .write_all(EMBEDDED_CONTRACT.as_bytes())
        .map_err(|error| format!("could not write embedded CLI contract: {error}"))?;
    Ok(ContractFile {
        path: temporary.path().to_path_buf(),
        _temporary: Some(temporary),
    })
}

fn output_format(value: Option<&str>) -> Result<OutputFormat, String> {
    match value.unwrap_or("text") {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        value => Err(format!("--format must be text or json, received {value:?}")),
    }
}

fn fail_on(value: Option<&str>) -> Result<FailOn, String> {
    match value.unwrap_or("never") {
        "never" => Ok(FailOn::Never),
        "missing" => Ok(FailOn::Missing),
        "critical" => Ok(FailOn::Critical),
        value => Err(format!(
            "--fail-on must be never, missing, or critical, received {value:?}"
        )),
    }
}

fn schema_kind(value: &str) -> Result<SchemaKind, String> {
    match value {
        "assessment" => Ok(SchemaKind::Assessment),
        "audit-report" => Ok(SchemaKind::AuditReport),
        value => Err(format!(
            "schema must be assessment or audit-report, received {value:?}"
        )),
    }
}

fn nonempty_required(value: Option<String>, option: &str) -> Result<String, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{option} is required"))
}

fn no_operands(extras: &[String], usage: &str) -> Result<(), String> {
    if extras.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{usage} accepts no positional operands; received {}",
            extras.len()
        ))
    }
}

fn optional_single_operand(extras: &[String], usage: &str) -> Result<Option<String>, String> {
    match extras {
        [] => Ok(None),
        [value] => Ok(Some(value.clone())),
        _ => Err(format!(
            "{usage} accepts at most one positional operand; received {}",
            extras.len()
        )),
    }
}

fn required_single_operand(extras: &[String], usage: &str) -> Result<String, String> {
    match extras {
        [value] => Ok(value.clone()),
        _ => Err(format!(
            "{usage} requires exactly one positional operand; received {}",
            extras.len()
        )),
    }
}

/// Strip any `=value` before diagnostics so an accidental credential flag is
/// never copied into process output.
fn diagnostic_option_name(option: &str) -> String {
    if let Some(long) = option.strip_prefix("--") {
        return format!("--{}", long.split('=').next().unwrap_or_default());
    }
    option.chars().take(2).collect()
}

fn help_requested(argv: &[String]) -> bool {
    argv.iter()
        .any(|argument| argument == "--help" || argument == "-h")
}

unsafe extern "C" {
    fn f2e_help_table_for_json_argv_from_file(
        config_path: *const c_char,
        command_name: *const c_char,
        argv_json: *const c_char,
        terminal_columns: c_int,
    ) -> *mut c_char;
    fn f2e_free(value: *mut c_char);
}

fn help_table(contract_path: &Path, argv: &[String]) -> Result<String, String> {
    // Constructing the bundled client anchors its static native object into
    // the link graph for the help symbols used below.
    let _parser = BundledFlags2Env::new();
    let contract = c_string(
        contract_path
            .to_str()
            .ok_or_else(|| ".cli-flags.toml path is not valid UTF-8".to_owned())?,
    )?;
    let command = c_string("twenty-two")?;
    let argv_json = c_string(
        &serde_json::to_string(argv)
            .map_err(|error| format!("could not encode arguments for help: {error}"))?,
    )?;
    // SAFETY: all C strings remain alive through the call. The returned value
    // is either null or an owned NUL-terminated string released by f2e_free.
    let raw = unsafe {
        f2e_help_table_for_json_argv_from_file(
            contract.as_ptr(),
            command.as_ptr(),
            argv_json.as_ptr(),
            terminal_columns(),
        )
    };
    if raw.is_null() {
        return Err("flags-2-env could not render help".to_owned());
    }
    // SAFETY: the core returned a valid owned C string. Copy before freeing.
    let text = unsafe { CStr::from_ptr(raw).to_string_lossy().into_owned() };
    // SAFETY: raw came from flags-2-env and has not previously been freed.
    unsafe { f2e_free(raw) };
    Ok(text)
}

fn c_string(value: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| "arguments must not contain interior NUL bytes".to_owned())
}

fn terminal_columns() -> c_int {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.trim().parse::<c_int>().ok())
        .filter(|columns| (40..=400).contains(columns))
        .unwrap_or(100)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".cli-flags.toml")
    }

    fn parse(tokens: &[&str], environment: &[(&str, &str)]) -> Result<ParseOutcome, String> {
        parse_with_environment(
            &tokens
                .iter()
                .map(|token| (*token).to_owned())
                .collect::<Vec<_>>(),
            &contract(),
            environment
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned())),
        )
    }

    #[test]
    fn parses_repo_defaults() {
        let ParseOutcome::Run(Cli {
            command:
                Command::Audit {
                    target:
                        AuditTarget::Repo {
                            path,
                            policy,
                            format,
                            fail_on,
                        },
                },
        }) = parse(&["twenty-two", "audit", "repo"], &[]).unwrap()
        else {
            panic!("expected repository audit")
        };
        assert_eq!(path, Path::new("."));
        assert!(policy.is_none());
        assert_eq!(format, OutputFormat::Text);
        assert_eq!(fail_on, FailOn::Never);
    }

    #[test]
    fn argv_beats_environment_and_preserves_positionals() {
        let ParseOutcome::Run(Cli {
            command:
                Command::Audit {
                    target:
                        AuditTarget::Repo {
                            path,
                            format,
                            fail_on,
                            ..
                        },
                },
        }) = parse(
            &[
                "twenty-two",
                "audit",
                "repo",
                "checkout",
                "--format=json",
                "--fail-on=critical",
            ],
            &[
                ("TWENTY_TWO_FORMAT", "text"),
                ("TWENTY_TWO_FAIL_ON", "missing"),
            ],
        )
        .unwrap()
        else {
            panic!("expected repository audit")
        };
        assert_eq!(path, Path::new("checkout"));
        assert_eq!(format, OutputFormat::Json);
        assert_eq!(fail_on, FailOn::Critical);
    }

    #[test]
    fn parses_assessment_init_from_the_contract() {
        let ParseOutcome::Run(Cli {
            command:
                Command::Assessment {
                    action:
                        AssessmentAction::Init {
                            target,
                            catalog,
                            output,
                            force,
                        },
                },
        }) = parse(
            &[
                "twenty-two",
                "assessment",
                "init",
                "--target=github.com/acme/payments",
                "--force",
            ],
            &[],
        )
        .unwrap()
        else {
            panic!("expected assessment init")
        };
        assert_eq!(target, "github.com/acme/payments");
        assert_eq!(catalog, DEFAULT_CATALOG_URL);
        assert_eq!(output, Path::new("assessment.json"));
        assert!(force);
    }

    #[test]
    fn rejects_scoped_and_unknown_options_without_echoing_values() {
        let scoped = parse(
            &["twenty-two", "audit", "org", "acme", "--policy=x.json"],
            &[],
        )
        .unwrap_err();
        assert!(scoped.contains("unknown command-line option"));

        let secret = parse(
            &["twenty-two", "audit", "org", "acme", "--token=do-not-print"],
            &[],
        )
        .unwrap_err();
        assert!(secret.contains("--token"));
        assert!(!secret.contains("do-not-print"));
    }

    #[test]
    fn renders_subcommand_help_from_flags_2_env() {
        let ParseOutcome::Print(help) =
            parse(&["twenty-two", "audit", "repo", "--help"], &[]).unwrap()
        else {
            panic!("expected help output")
        };
        assert!(help.contains("Command: twenty-two audit repo"));
        assert!(help.contains("--policy"));
        assert!(help.contains("--format"));
    }

    #[test]
    fn embedded_contract_is_a_self_contained_runtime_fallback() {
        let embedded = embedded_contract_file().unwrap();
        let outcome = parse_with_environment(
            &[
                "twenty-two".to_owned(),
                "schema".to_owned(),
                "assessment".to_owned(),
            ],
            &embedded.path,
            std::iter::empty::<(String, String)>(),
        )
        .unwrap();
        assert!(matches!(
            outcome,
            ParseOutcome::Run(Cli {
                command: Command::Schema {
                    kind: SchemaKind::Assessment
                }
            })
        ));
    }

    #[test]
    fn catalog_default_matches_the_library_contract() {
        assert!(EMBEDDED_CONTRACT.contains(&format!("default = \"{DEFAULT_CATALOG_URL}\"")));
    }
}
