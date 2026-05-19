# 01 Workspace Lints Results

**Implemented Policy**
- Added root `[workspace.lints.rust]` with `unused`, `unused_must_use`, `unfulfilled_lint_expectations`, and `unsafe_op_in_unsafe_fn` set to `deny`.
- Added root `[workspace.lints.clippy]` banning `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, `dbg_macro`, and undocumented unsafe blocks.
- Added warning-level Clippy policy for redundant clones, unnecessary ownership, needless collects, large stack arrays, boxed collections, and boxed vectors.
- Added `[lints] workspace = true` to every normal workspace crate.
- Mirrored the lint policy in `fuzz/Cargo.toml` because fuzz is a separate workspace.
- Added `clippy.toml` with `msrv = "1.96.0"`.
- Updated `scripts/bench-quick.sh` to run Clippy with `--all-features`.

**Policy Adjustment**
- `clippy::large_heap_arrays` is not available on the pinned nightly toolchain.
- The enforceable substitute in this PRD is `clippy::large_stack_arrays = "warn"` plus future heap observability in PRD 05.

**Cleanup Completed**
- Removed direct `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unreachable!`, and `dbg!` call sites from `crates/` and `fuzz/` Rust files.
- Replaced production panic smells in Datalog string lexing, sorted trie key access, hash trie insertion, aggregate finish, benchmark CLI parsing, and benchmark row conversion helpers.
- Converted tests to `Result`-returning tests or equivalent fallible helpers.
- Replaced broad `#[allow(...)]` sites with removals or targeted `#[expect(..., reason = "...")]` annotations.
- Added a safety comment for LMDB environment open unsafe block.
- Updated trybuild expected output after removing `.unwrap()` from the compile-fail UI fixture.

**Approved `#[expect]` Annotations**
- `Environment::path` keeps the environment path for diagnostics/debugging.
- `build_plan_candidate` mirrors the full optimizer planning context.
- Compiled-plan scaffolding remains reserved for specialization work.
- Reference recursion helpers carry explicit evaluator state.

**Verification**
- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`

**Next PRD**
- `docs/todos/observability_lints_allocation_hardening/02_panic_unwrap_and_smell_cleanup.md`
