# 10 — One workspace

> **Decision.** `crates/bumbledb-log` joins the root cargo workspace as
> an ordinary member. Its private `[workspace]` stanza, its private
> `Cargo.lock`, and its private `.config/` die. After this document,
> `cargo test --workspace` at the repo root compiles and runs every
> Rust test in the repository, and the sentence "the root battery is
> green" has no footnote.

## The current representation

The repository holds **two cargo workspaces**: the root (eight engine
crates, with `ts/crate` and `crates/bumbledb-c` deliberately excluded as
foreign build systems) and `crates/bumbledb-log`, which carries its own
`[workspace]` marker, its own `Cargo.lock`, and now its own
`.config/nextest.toml`. Nothing on record justifies the split — no
ruling, no comment, no doc. Its observable effects are exactly the two
worst findings of the release audit:

- **The battery blind spot.** `cargo fmt --all --check`,
  `cargo clippy --workspace -- -D warnings`, and
  `cargo test --workspace` at the root **never touch the log crate**.
  Every green claimed by those commands during the cutover was a green
  about the engine only. The audit demonstrated it live: root clippy
  reported zero errors at the same instant the log crate's clippy was
  red.
- **The lockstep miss.** The version-lockstep gate walks the workspace
  it can see. `bumbledb-log` sat at `0.17.0` through the entire
  `0.18.0` release because it was not in the walked set. A fact outside
  the roster cannot be checked by the roster.

Both are the same defect: *"the set of crates that must build" has two
writers.* Every consumer — the battery, the gate, CI, a human at the
shell — must remember to consult both, and the audit proved they don't.

## Question the requirement

Why might the split have existed? The candidate reasons, each examined:

- *Dependency isolation* — the log crate pulls `tokio`, `object_store`,
  and S3 machinery the engine doesn't want. But cargo resolves
  dependencies per crate regardless of workspace membership; a workspace
  member's deps do not compile into its siblings. The isolation is
  already provided by the representation cargo has.
- *Lockfile churn* — engine builds re-resolving when log deps move. One
  lockfile is the point, not the cost: the repo ships these crates in
  lockstep (the gate exists precisely to force that), so their
  dependency universe should resolve as one fact too.
- *Lint profile drift* — the root workspace denies `unsafe_code` and
  carries `workspace.lints`. The log crate satisfies them, full stop:
  no per-crate override, no allow-list. The cutover already drove it
  through clippy `-D warnings`; whatever the root lints add, the code
  moves, not the lint.

No reason survives. The requirement was never written down, and the two
production failures it caused are. Delete it. **The merge is
unconditional** — there is no fallback design, no both-workspaces gate,
no contingency. If the merge surfaces friction (lints, features, a dep
conflict), the code and the lockfile move until it lands; the structure
does not.

## The target representation

1. `crates/bumbledb-log/Cargo.toml` loses its `[workspace]` stanza and
   gains membership in the root `members` list. It adopts
   `version.workspace = true` ([20](20-one-version.md)),
   `edition.workspace = true`, and the root `workspace.lints`.
2. `crates/bumbledb-log/Cargo.lock` is **deleted**; the root
   `Cargo.lock` becomes the one dependency-resolution fact. The root
   lockfile re-derives once, in the merge commit.
3. `crates/bumbledb-log/.config/nextest.toml` moves to the repo root
   `.config/nextest.toml` (nextest resolves configuration from the
   workspace root) and is **committed** — a battery config that exists
   only on one laptop is a battery with two writers.
4. The CI workflow drops its separate
   `--manifest-path crates/bumbledb-log/Cargo.toml` invocations wherever
   the intent is "the whole workspace"; the S3 smoke's targeted filter
   stays, expressed as a nextest filter, not a second manifest.
5. `ts/crate` and `crates/bumbledb-c` remain excluded — they are foreign
   build systems (napi, cbindgen) with their own drivers, and that
   exclusion *is* written down in the root manifest.

## What gets deleted

| Deleted | Because |
| --- | --- |
| the `[workspace]` stanza in `crates/bumbledb-log/Cargo.toml` | the crate set has one writer |
| `crates/bumbledb-log/Cargo.lock` | dependency resolution has one writer |
| `crates/bumbledb-log/.config/` | battery configuration has one writer, at the root |
| every `--manifest-path crates/bumbledb-log/...` whole-suite invocation in CI and scripts | `--workspace` now means what it says |
| the "its own workspace — root commands do NOT reach it" warnings in docs and prompts | the state they warned about is unrepresentable |

## The invariant

> **"The workspace" denotes every Rust crate this repository builds,
> minus the two written exclusions.** `cargo test --workspace` compiling
> a subset of the repo's testable crates is not a configuration to warn
> about; after this document it is a state that cannot be spelled.

Dissolves: the battery blind spot (audit A/§B-item), the lockstep miss
(audit B.1's root cause), the untracked-`.config` and stale-CI findings.
Enables: version inheritance ([20](20-one-version.md)) and the
one-battery script ([30](30-one-battery.md)), both of which need one
workspace to have one spelling.
