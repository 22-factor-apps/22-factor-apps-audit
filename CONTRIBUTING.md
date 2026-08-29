# Contributing

Start with an issue that names the factor, the evidence gap, and the proof
boundary. Changes to policy cues must include a positive fixture, a negative
fixture, and an explanation of why the cue cannot be mistaken for compliance.

Before opening a pull request:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Commit generated dependency resolution in `Cargo.lock`. Never include repository
tokens, private assessment data, or live customer URLs in fixtures.
