# Security policy

Please do not open a public issue for a vulnerability that could expose tokens,
private repository metadata, or assessment evidence. Use GitHub’s private
vulnerability reporting for this repository.

The CLI is read-only by design. It accepts a GitHub token only through the
`GITHUB_TOKEN` environment variable and redacts credentials embedded in HTTPS
Git remotes before serialization. Reports may still contain sensitive local
paths or private repository names; review them before publishing.

Supported releases are the latest tagged minor release and the current `main`
branch until the next release is cut.
