# Operational and administrative boundary

The CLI has no server, database, migration, privileged repair command, or
background administrative process. Installation and local execution are the
only operational procedures in v0.1.0.

Releases are built from immutable tags. A maintainer runs the same locked test
and lint suite as CI, verifies installation from a fresh consumer environment,
and attaches checksums to the GitHub release. If stateful administration is
introduced later, its commands must be versioned with the shipped release,
support a dry run where meaningful, use typed failures, and leave an audit
record. This documented non-applicability should be revisited at every minor
release.
