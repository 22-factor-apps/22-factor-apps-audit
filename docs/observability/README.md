# Observability boundary

`twenty-two` is a short-lived local process and intentionally emits no remote
telemetry. Human-readable findings go to standard output; operational errors go
to standard error; exit code `2` means invocation or execution failed, and exit
code `1` means the caller-selected missing-evidence threshold was crossed.

JSON mode is the causal record for automation. It includes the target, source
revision when available, edition, policy rule, factor, state, severity, evidence,
and next step. It excludes credentials and never hides factor-level results
inside an aggregate score. Callers own retention and alerting for those reports.
