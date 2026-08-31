# Changelog

## Unreleased

- Adopt edition 2026.3 and the v2 audit policy: Formal Methods & Functional Core
  and Safe Languages & Total Types replace the prior factors XX and XXI.
- Strengthen Admin Processes with separate network and administrative-identity
  trust domains.
- Add formal lifecycle and language-safety documentation, including the bounded
  `flags-2-env` FFI safety contract.
- Make `flags-2-env` and `.cli-flags.toml` the typed, fail-closed CLI parsing
  authority for commands, options, environment precedence, and generated help.

This project follows Semantic Versioning for the CLI and versioned JSON
contracts. A schema or policy version remains readable for its documented
support window even when a newer default is introduced.

## 0.1.0 — 2026-08-29

- Add evidence-first audits for local repositories and GitHub organizations.
- Add the versioned default repository policy and factor-scoped findings.
- Add assessment initialization and semantic validation with no aggregate score.
- Add JSON Schema contracts for audit reports and assessments.
