# Release and progressive exposure

The CLI has no hosted runtime to expose progressively. Releases use immutable
Git tags and start as GitHub release artifacts; consumers opt in by installing a
specific version with the locked dependency graph. A release is not promoted in
documentation until CI, fresh-install consumer testing, schema compatibility,
and checksum verification pass at the tagged commit.

Breaking command or contract changes require a new major version and a coexistence
window. A release with incorrect findings is withdrawn in documentation, not
silently replaced under the same tag. This is the project’s current exposure and
rollback boundary.
