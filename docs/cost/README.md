# Cost boundary

The CLI is a user-invoked process with no always-on infrastructure. Material cost
is bounded by local traversal and GitHub API calls. Organization audits make a
bounded repository listing plus policy/content probes; authenticated callers use
their own API allocation.

Any future hosted service must attribute compute, storage, and API consumption to
an audit outcome and introduce explicit budgets before launch. Until then, binary
size, build time, network requests, and scan duration are the reviewable unit-cost
signals.
