# Implementation PRDs

The complete, ordered build plan for bumbledb v0. Written once, after the design closed
(commit `773de21`); amended in place if reality disagrees — never replaced by a new
suite. This suite exists to sequence *code*, not to make decisions: **every decision
already lives in `docs/architecture/`, which is the sole authority.** If a PRD and an
architecture doc disagree, the PRD is wrong; fix the PRD in the same change or stop.

## Rules of engagement (every PRD, every implementer)

1. Read the architecture docs cited in the PRD's header before writing code. The PRD
   gives direction; the docs give the contract.
2. Scope is exactly the PRD. No speculative generality, no `dyn` in hot paths, no
   modes, no dead code, no `#[allow(dead_code)]`. A mechanism with no reader in the
   current or an earlier PRD does not get built (README rule 3 of the architecture).
3. Unit tests are part of each PRD's code change and its passing criteria. **Excluded
   from this suite entirely (human-owned, per owner instruction): smoke tests,
   end-to-end testing, the SQLite oracle harness, golden examples, benchmarks, fuzz
   targets, and anything migration-shaped.** PRD unit tests exercise the module's own
   contract only.
4. Every PRD must leave the workspace green under the global commands below. No PRD
   may be "done" with a failing or `#[ignore]`d test.
5. Panics: only for programmer-invariant violations (`debug_assert!` where hot).
   Everything reachable from user input or disk returns the typed errors of
   `60-api.md`.
6. Unsafe code: only where a PRD explicitly sanctions it, with a `// SAFETY:` comment
   per block and Miri-clean tests for the touching module.

## Global commands (green after every PRD)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Order

Foundations: 00 workspace · 01 encodings & fact codec · 02 schema descriptors ·
03 fingerprint
Storage: 04 environment & keys · 05 dictionary · 06 delta transaction core ·
07 commit apply · 08 commit validate & counters · 09 point reads & scan
Images: 10 image builder · 11 image cache · 12 filtered views
Query model: 13 IR · 14 validation · 15 normalization · 16 planner ·
17 plan lowering
Execution: 18 COLT · 19 scalar executor · 20 sinks · 21 vectorized execution ·
22 NEON kernels · 23 guard-probe dispatch · 24 EXPLAIN
Surface: 25 prepared queries & results · 26 allocation discipline ·
27 schema macro · 28 public API assembly

Dependencies are strictly earlier-numbered PRDs unless a PRD states otherwise.
