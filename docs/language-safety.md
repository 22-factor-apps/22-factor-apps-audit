# Language-safety boundary

The auditor is implemented in Rust 2024 and pins its minimum toolchain. Ordinary
audit, policy, assessment, GitHub, rendering, and filesystem logic uses safe Rust.
Absence and expected failure are represented with `Option<T>` and typed `Result`
values rather than ambient null references or sentinel strings.

## The one unsafe boundary

`src/cli_contract.rs` calls the bundled `flags-2-env` native help renderer through
a C ABI. The boundary is intentionally local:

- Rust owns all input `CString` values for the duration of the call.
- Interior NUL bytes are rejected before FFI.
- A null return is checked before constructing `CStr`.
- The returned NUL-terminated string is copied into owned Rust memory before release.
- The pointer is released exactly once with the matching `f2e_free` function.
- Callers receive a safe `Result<String, String>` and cannot access the raw pointer.

Every unsafe block carries its safety premise next to the operation. Crate-level
`unsafe_op_in_unsafe_fn` denial prevents future unsafe functions from silently
performing unchecked operations outside explicit blocks.

This boundary still depends on the native library honoring its allocation,
termination, and ownership contract. CI exercises parsing and help generation, but
that is not a proof of native memory safety. A future native update must review the
ABI contract, run the Rust tests, and use platform memory tooling where available.

## Dependency and migration policy

New privileged or network-facing implementation belongs in safe Rust. Native
dependencies require a documented safety boundary and must not receive credentials
or unvalidated repository content unless their contract requires it. If a future
dependency introduces material memory-unsafe code, record its exposed surface,
mitigations, owner, and migration or replacement plan here.
