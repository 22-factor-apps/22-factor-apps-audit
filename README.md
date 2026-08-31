# Twenty-Two-Factor Audit

`twenty-two` is an evidence-first Rust CLI for reviewing repositories and
GitHub organizations against the [Twenty-Two-Factor App][guide]. It discovers
review leads, initializes a contextual assessment, and validates the resulting
evidence record.

It deliberately does **not** calculate a “17/22” score. Architecture is not a
sticker collection: a missing authorization boundary can be existential while a
well-documented sustainability exception can be reasonable. Findings therefore
stay attached to individual factors, evidence, owners, and follow-up work.

## What it audits

```text
repository files + Git history ─┐
                                ├─> factor-scoped evidence findings
GitHub organization metadata ───┘          │
                                           └─> human assessment + evidence links
```

- `audit repo` scans a local checkout for versioned evidence and creates one
  finding for every factor. Runtime properties that static files cannot prove
  are marked `manual-review`, never guessed.
- `audit org` inspects source-portfolio, public contract, security, automation,
  formal-methods, and safe-language signals through the GitHub API.
- `assessment init` downloads the catalog generated from the canonical 22
  Markdown documents and creates an unscored, 22-entry JSON assessment.
- `assessment validate` enforces provenance, factor coverage, evidence URLs,
  review dates, and contextual rationale.

## Install

The project currently publishes source releases rather than a crates.io binary:

```sh
cargo install --git https://github.com/22-factor-apps/22-factor-apps-audit \
  --tag v0.2.0 --locked --bin twenty-two
```

To work from a clone:

```sh
cargo build --locked
cargo test --locked
```

## CLI contract

[`flags-2-env`](https://github.com/flags-2-env/flags-2-env) is the CLI parsing
authority. [`.cli-flags.toml`](.cli-flags.toml) declares the command tree,
options, types, defaults, environment keys, and help text. The Rust binary
audits that contract, rejects unknown options, applies
`dotenv < environment < dotenv_override < argv` precedence without mutating
the process environment, and uses the bundled parser so no shared native
library is required at runtime.

Installed binaries carry an embedded copy of the contract. Set
`TWENTY_TWO_FLAGS_CONFIG` only to test or deliberately supply an external
contract; the path must identify a readable file and is validated before any
command runs.

## Use

Audit the current checkout in readable text:

```sh
twenty-two audit repo .
```

Emit stable JSON for CI or later review:

```sh
twenty-two audit repo . --format json > audit-report.json
```

Make missing critical evidence fail CI only after your team has reviewed the
policy and accepted that boundary:

```sh
twenty-two audit repo . --fail-on critical
```

Audit a public organization. Set `GITHUB_TOKEN` for private repositories or a
higher API rate limit; the token is read from the environment and is never a
command-line argument.

```sh
twenty-two audit org 22-factor-apps --format json
```

Create and validate an assessment:

```sh
twenty-two assessment init --target github.com/acme/payments --output assessment.json
twenty-two assessment validate assessment.json
```

Initialization refuses to replace an existing evidence file. Use `--force` only
after deliberately preserving or reviewing the previous assessment.

Print the bundled contracts:

```sh
twenty-two schema audit-report
twenty-two schema assessment
```

## Interpretation boundary

An observed `SECURITY.md` proves that a file exists. It does not prove secure
design. A missing repository file can also be legitimate when the evidence is
held in an approved external system. The CLI records both cases as review
inputs; the assessment carries the contextual decision.

The current repository policy is versioned at
[`policies/default-v2.json`](policies/default-v2.json); the earlier v1 policy remains
available for edition 2026.2 assessments. The assessment and audit
report contracts are under [`schemas/`](schemas/). See
[`docs/checks.md`](docs/checks.md) for the factor-by-factor automation boundary
and [`docs/overlays.md`](docs/overlays.md) for AI, local-first, mobile, and
regulated-safety overlays.

## Security and privacy

- Audits are read-only.
- No telemetry is collected.
- Repository scans skip dependency, build, VCS, and vendor directories.
- HTTPS credentials in Git remote URLs are redacted before output.
- The GitHub token is only read from `GITHUB_TOKEN`; there is no token flag.
- The bundled `flags-2-env` FFI is isolated and documented in
  [`docs/language-safety.md`](docs/language-safety.md); ordinary audit logic remains
  safe Rust.

## License

[MIT](LICENSE) © 2026 The 22-Factor Apps Authors.

[guide]: https://22-factor-apps.github.io/
