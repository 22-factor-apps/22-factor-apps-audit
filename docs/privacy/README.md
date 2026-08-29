# Data lifecycle and privacy

Audits are read-only and collect no telemetry. Local reports can contain absolute
checkout paths, commit identifiers, public or private repository names, and
evidence locations. Organization reports can contain any metadata visible to the
token in `GITHUB_TOKEN`. Treat generated JSON according to the sensitivity of the
target.

The tool never writes, logs, or serializes the token. Credentials embedded in an
HTTPS Git remote are removed before output. The CLI keeps no internal database,
cache, upload, or backup; deleting the caller-chosen report removes the tool’s
only durable output. Callers remain responsible for copies made by shells, CI
artifacts, evidence systems, or backups.
