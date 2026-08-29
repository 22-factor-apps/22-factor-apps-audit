# Resilience and containment

The CLI keeps effects at the boundary and fails without mutating the audited
target. Local traversal stops at eight directory levels, skips VCS/dependency/
build/vendor trees, and caps displayed path evidence per rule. GitHub listing is
bounded to ten pages of 100 repositories. Every HTTP operation has a 30-second
global timeout.

GitHub errors and malformed catalogs fail the command rather than producing a
partial “successful” report. Manual-review findings stay manual; a missing
runtime answer is never converted to observed because another file happened to
match. Assessment writes serialize and validate in memory before replacing the
requested output path.
