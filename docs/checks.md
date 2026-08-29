# Audit checks and proof boundaries

The repository audit emits exactly one primary finding for each factor. These
findings answer “where should a reviewer look?” They do not answer “is this
system compliant?”

| Factor | Automated evidence cue | Required human or runtime review |
|---:|---|---|
| I | Git revision and credential-redacted origin | Deploy-to-commit mapping and source authority |
| II | Lock or resolution file | Isolation, verification, provenance, and update policy |
| III | Secret-free example or configuration schema | Startup validation and secret delivery |
| IV | Manual review | Service binding, replacement, failure, and migration contracts |
| V | Build, container, or release automation | Immutable artifact identity and byte-for-byte promotion |
| VI | Manual review | Termination/relocation test for durable state |
| VII | Manual review | Admission, backpressure, and saturation bounds at every layer |
| VIII | Manual review | Readiness, drain, abrupt termination, replay, and recovery |
| IX | Tests and CI | Whether production-significant differences are represented |
| X | Migrations, admin scripts, or runbooks | Shipped-release context, authorization, audit, and dry-run behavior |
| XI | Schema, IDL, or contract directory | Semantics, compatibility, failures, limits, and consumer tests |
| XII | Security policy or threat model | Identity, authorization, input, abuse, and fail-closed tests |
| XIII | SLO or observability material | User-visible objectives, causality, and actionable alerts |
| XIV | Dependency automation, SBOM, or provenance | Exact pins, build traceability, and admission verification |
| XV | Chaos, resilience, recovery, or fault material | Demonstrated bounds and recovery under disturbance |
| XVI | Privacy, retention, deletion, or governance material | Every copy from collection through verified deletion |
| XVII | Declarative infrastructure or policy | Validation, drift reconciliation, safety bounds, and approvals |
| XVIII | Flags, canary, rollout, or progressive-delivery material | Cohorts, health gates, stop conditions, rollback, and cleanup |
| XIX | Changelog, migrations, compatibility, or contract tests | Coexistence, support windows, migration, and rollback proof |
| XX | CODEOWNERS, contribution, or ownership material | One empowered team owns the user outcome through retirement |
| XXI | Cost, FinOps, budget, or unit-economics material | Resource attribution and design/incident decision boundaries |
| XXII | Sustainability, carbon, energy, or green-software material | Material impact, avoided harm, and documented trade-offs |

## Exit behavior

The default exit policy is `never`: the audit is useful before a team has agreed
on enforcement. `--fail-on missing` treats every missing evidence cue as an exit
failure. `--fail-on critical` limits that behavior to findings whose absence can
hide a high-impact boundary. `manual-review` is never converted into a passing or
failing status by the CLI.

## Custom policy

Pass `--policy path/to/policy.json` to change repository path cues. A policy is
valid only when rule identifiers are unique, factors are within 1–22, and each
factor has no more than one primary rule. Keep policy changes in review and pair
them with examples; a broad filename match can create false confidence.
