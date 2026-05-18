# 01: Workspace Lints And Clippy Policy

**Goal**
- Make unused variables compile failures.
- Make Clippy a strict workspace gate.
- Ban panic-oriented and debugging smells across production, test, benchmark, and fuzz code unless a narrow documented exception is explicitly justified.

**Required Lint Policy**
Add the workspace lint policy to root `Cargo.toml`:

```toml
[workspace.lints.rust]
unused = "deny"
unused_must_use = "deny"
unfulfilled_lint_expectations = "deny"
unsafe_op_in_unsafe_fn = "deny"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"
dbg_macro = "deny"
undocumented_unsafe_blocks = "deny"
redundant_clone = "warn"
unnecessary_to_owned = "warn"
needless_collect = "warn"
large_heap_arrays = "warn"
large_stack_arrays = "warn"
box_collection = "warn"
vec_box = "warn"
```

Add this to each normal workspace package manifest:

```toml
[lints]
workspace = true
```

Mirror the relevant lint policy in `fuzz/Cargo.toml`, because `fuzz/` declares its own workspace and cannot inherit root `[workspace.lints]`.

**Clippy Config**
- Add root `clippy.toml` if needed.
- Set MSRV to the workspace Rust version if the current Clippy accepts the key.
- Do not add unsupported Clippy config keys; verify with the actual pinned nightly toolchain.
- Do not configure Clippy to allow unwraps in tests. Tests should be cleaned up too.
- Keep benchmark printing allowed in the benchmark binary if Clippy flags print-style concerns in future policy extensions.

**Required Script Updates**
- Update `scripts/bench-quick.sh` to use `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Update docs that list verification commands so they use `--all-features` consistently.
- If a new script is added for this suite, make it check that all normal crate manifests contain `[lints] workspace = true`.
- Keep `cargo check --manifest-path fuzz/Cargo.toml` in gates.

**Exception Policy**
- Prefer removing the smell over adding an exception.
- Use `#[expect(..., reason = "...")]` instead of `#[allow(...)]` when an exception is unavoidable.
- Every exception reason must explain why the code is correct and why a local refactor is worse.
- Exceptions for `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, or `dbg_macro` should be treated as temporary unless they are in generated code or impossible-to-reach test scaffolding.
- Do not add broad crate-level exceptions for these banned smells.

**Required Cleanup To Make Lints Pass**
- Convert tests from `.unwrap()`/`.expect()` to `Result`-returning tests and `?`.
- Convert `Option` unwraps in tests to `ok_or_else(...)` or direct `assert_eq!(..., Some(...))` patterns.
- Convert benchmark CLI parse failures from `panic!`/`expect` to typed errors.
- Replace production `expect`/`unwrap`/`unreachable!` sites before enabling the deny policy if that makes the patch easier to review.
- Remove unused bindings instead of suppressing them with `let _ = ...`.

**Passing Requirements**
- `cargo check --workspace --all-targets --all-features` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `cargo test --workspace --all-features` passes.
- `cargo check --manifest-path fuzz/Cargo.toml` passes with the mirrored fuzz lint policy.
- `rg 'unwrap\(|expect\(|panic!\(|todo!\(|unimplemented!\(|unreachable!\(|dbg!\(' crates fuzz --glob '*.rs'` returns no unapproved sites.
- `rg '#\[allow\(' crates fuzz --glob '*.rs'` returns no new broad exceptions for banned smells.

**Stop Conditions**
- Stop if lint fixes require changing query semantics.
- Stop if a lint exception seems necessary in production hot code; redesign the local API instead.
- Stop if the pinned nightly does not support a requested lint key; adjust the PRD implementation to equivalent enforceable lints and document the substitution.
