# Repository audit lifecycle model

`AuditLifecycle.tla` models the repository auditor's publication boundary:

- factors are the closed set 1 through 22;
- each factor begins pending and can be classified exactly once as `Observed`,
  `Missing`, or `ManualReview`;
- a report cannot be published until every factor is classified; and
- a published report cannot be reclassified.

TLC checks the full 22-factor model using factor permutation symmetry. CI downloads
the official TLA+ 1.7.4 tool artifact and verifies its pinned SHA-256 digest before
execution.

The model deliberately has no score variable. Individual evidence states survive
publication without being compressed into an aggregate maturity number.

## Refinement map

| Model element | Implementation boundary |
|---|---|
| `Factors` | `Policy::validate` requires exactly factors 2–22; factor 1 is always supplied by `source_history_finding` |
| `EvidenceStates` | `model::EvidenceState` is a closed Rust enum |
| `Classify` | `evaluate_repo_rule`, `manual_finding`, and `source_history_finding` each construct one immutable `Finding` |
| `Publish` | `repo_audit::audit` constructs `AuditReport` only after collecting all findings |
| no score variable | serialization tests reject the presence of `score` |

The TLA+ model does not prove filesystem traversal, Git command behavior, matching
quality, or the truth of discovered evidence. Those remain implementation and human
review boundaries. Counterexamples or model changes must be translated into Rust
tests before the implementation claim changes.

Run the model with a local TLA+ Toolbox or TLC installation:

```sh
tlc -config docs/formal-methods/AuditLifecycle.cfg \
  docs/formal-methods/AuditLifecycle.tla
```
