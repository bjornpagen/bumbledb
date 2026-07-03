# bumbledb

An embedded, typed, set-semantic relational database over LMDB, executing
conjunctive queries with Free Join. The engine is built: schema macro, delta
write path with commit-time constraints, columnar image cache, the Free Join
planner/executor with NEON kernels, prepared queries with a
zero-warm-allocation contract, and the `Db` embedding surface.

Five earlier implementations (v1–v5) were built and discarded; the current
engine was rebuilt docs-first, decision by decision, from the architecture
docs below. Prior implementations and the review that motivated the reset
live in git history before the reset commit (`1b65ae8`).

Contents:

- `crates/bumbledb/` — the engine.
- `crates/bumbledb-macros/` — the `schema!` proc macro (hand-rolled, no
  syn/quote).
- `docs/architecture/` — the normative design. When code and these docs
  disagree, one of them is wrong and the repo is broken until they agree.
- `docs/free-join-paper/` — Wang, Willsey, Suciu, *Free Join: Unifying
  Worst-Case Optimal and Traditional Joins* (arXiv:2301.10841v2), the
  algorithmic reference.

The gate suite (run `scripts/check.sh`, or the three commands plus the
release-mode allocation gate by hand):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --features alloc-counter --test alloc_gate --release
```

Not yet built (deliberately): the SQLite oracle and the ledger benchmark —
the external halves of `docs/architecture/50-validation.md`. No performance
claims are made until they exist.
