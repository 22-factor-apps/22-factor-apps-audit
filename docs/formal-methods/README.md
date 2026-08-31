# Repository audit lifecycle model

`AuditLifecycle.tla` models the repository auditor's publication boundary with a
finite count abstraction:

- exactly 22 factors begin pending;
- each classification moves one pending factor exactly once into the `observed`,
  `missing`, or `manualReview` bucket;
- the four bucket counts always sum to 22;
- a report cannot be published until every factor is classified; and
- a published report cannot be reclassified.

Weak fairness excludes an execution that stutters forever while classification or
publication is enabled, so TLC also checks eventual publication. CI downloads the
official TLA+ 1.7.4 tool artifact and verifies its pinned SHA-256 digest before
execution.

The model deliberately has no score variable. Individual evidence states survive
publication without being compressed into an aggregate maturity number.

## Refinement map

| Model element | Implementation boundary |
|---|---|
| `TotalFactors` | `Policy::validate` requires exactly factors 2–22; factor 1 is always supplied by `source_history_finding` |
| terminal buckets | `model::EvidenceState` is a closed Rust enum |
| classification actions | `evaluate_repo_rule`, `manual_finding`, and `source_history_finding` each construct one immutable `Finding` |
| `Publish` | `repo_audit::audit` constructs `AuditReport` only after collecting all findings |
| no score variable | serialization tests reject the presence of `score` |

The count abstraction proves lifecycle conservation and publication safety, not the
identity or ordering of individual factors. Exact factor-number uniqueness is a Rust
policy invariant covered by `embedded_policy_covers_every_non_source_factor_once`.
The model also does not prove filesystem traversal, Git command behavior, matching
quality, or the truth of discovered evidence. Those remain implementation and human
review boundaries. Counterexamples or model changes must be translated into Rust
tests before the implementation claim changes.

Run the model with a local TLA+ Toolbox or TLC installation:

```sh
tlc -config docs/formal-methods/AuditLifecycle.cfg \
  docs/formal-methods/AuditLifecycle.tla
```
