# PRD 25 — Vocabulary sweep

**Depends on:** everything (last PRD; the tree compiles again before this one
completes — this is also the PRD where residual compile breakage from the whole
plan is driven to zero).
**Modules:** the entire workspace; `README.md` (root, code-example section only).
**Authority:** `docs/architecture/00-product.md` (§ deleted vocabulary), `docs/prd/README.md` policy rule 5.

## Goal

The deleted vocabulary is gone from the codebase as *identifiers and concepts*;
the tree builds green; the root README's code example speaks the new language.

## Technical direction

1. **Sweep targets**, workspace-wide (`crates/`, `scripts/`):
   `unique`, `fk`, `foreign`, `primary_key`/`pkey`, `constraint`, `cascade`,
   `restrict`, `replace` (the operation — the string API `str::replace` is
   obviously fine; judgment required, which is why this is last and explicit).
   For each hit: identifiers/type names/function names → renamed into the
   statement vocabulary; comments and doc comments → rewritten to the new
   concepts (a comment explaining *history* may keep the old word if it cites an
   architecture doc's tombstone); test names → renamed to what they now test.
2. **Systematic procedure** (do it this way, not ad hoc): for each target word,
   run `rg -i --pretty <word> crates scripts`, classify every hit into
   {identifier, comment, false-positive}, and fix the first two classes. Zero
   identifier hits is the bar; a comment may keep a deleted word only when it
   states the current refusal (e.g. "no cascade exists; delete the cluster in one
   delta"), never to narrate what used to be.
3. **Root README:** the code example (`schema!` block and the surrounding prose
   that names `fk`/serial-unique behavior) is rewritten in statement notation to
   compile against the new macro. Do not otherwise rewrite the README (owner's
   document).
4. **Compile-and-gate closure:** this PRD ends with
   `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   and `cargo test --workspace` green — every `[test]` criterion from PRDs 01–24
   is now live and must pass. Fix residual breakage here; if a fix requires a
   *semantic* decision, stop and file it as a `## Conflict` per the README policy
   rather than improvising.
5. `scripts/check-asm.sh` and `scripts/check.sh`: update invocations/module paths
   that moved; the asm gates' *properties* (no calls/`bcmp` in probe loops) are
   re-asserted, and any gate that no longer matches a renamed symbol is re-pointed,
   never deleted (deleting a gate is an owner decision).

## Out of scope

New behavior of any kind. Architecture-doc changes (only if a Conflict demands an
owner ruling).

## Passing criteria

- `[shape]` `rg -i 'unique|foreign|fkey|\bfk\b|constraint|cascade|restrict' crates scripts`
  yields only current-refusal comments and string-API false positives.
- `[shape]` The root README `schema!` example uses statement notation and the
  seven-type vocabulary.
- `[gate]` `cargo fmt --all --check` clean; `clippy --workspace --all-targets
  -D warnings` clean; `cargo test --workspace` green — with every PRD's `[test]`
  criteria included.
- `[gate]` `scripts/check.sh` runs to completion on the new tree.

## Execution record (2026-07-09)

Final `rg -i` hit summary over `crates/ scripts/` per target word — zero
identifier hits of any deleted concept:

| word | hits | classification |
|---|---|---|
| `unique` | 7 | 2 current-refusal (macro assertion + `compile_fail` example), 5 SQLite-DDL side (`UNIQUE` index emission and its comments in `sqlmap.rs` — the oracle's language) |
| `fk` | 1 | current-refusal (the macro's rejected-word assertion) |
| `fkey` | 0 | — |
| `foreign` | 24 | false positives: the another-environment sense (`ForeignPreparedQuery` and "foreign snapshot", normative in `70-api.md`; test names/locals in that sense) |
| `primary key` | 8 | SQLite-DDL side (`PRIMARY KEY` emission in `sqlmap.rs`) |
| `pkey` | 6 | substring false positives (`AggregateOverGroupKey`) |
| `constraint` | 3 | current-refusal ("field-level constraints do not exist; write a statement") |
| `cascade` | 0 | — |
| `restrict` | 51 | current vocabulary (Arg-restriction, PRD 18) and plain-English "restricts/restricted" |
| `replace` | ~25 | string/`mem::replace` API and the delete+insert replacement idiom (current mutation story) |

Surviving current-refusal comment sites: 4 (macro crate doc + diagnostic,
engine crate doc + `compile_fail` example).

Renames: `fk_walk` → `containment_walk` (family, functions, module
`sqlite_run/cold_containment_walk.rs`, golden `CONTAINMENT_WALK`,
`bench_viz.py` orders), `fk_target_db` → `containment_target_db`,
`point_lookup_by_unique_key` → `point_lookup_by_serial_key`,
`near_unique_maps_*` → `near_distinct_maps_*`,
`accepts_the_fk_walk_join_*` / `fk_walk_join_*` → containment-walk names.

Root README `schema!` example rewritten in statement notation
(`Account(holder) <= Holder(id);`, no field-level words) and aligned with
the real API (`tx.alloc()?`, `&mut` prepared query).

Residual breakage fixed here: 32 clippy `-D warnings` diagnostics
(doc-markdown backticks, `too_many_lines` allows with justifications,
items-after-statements moves, an `if let` destructure, elided lifetimes,
merged match arms, one `# Panics` section, `unused_self` allow), rustfmt
drift in `alloc_gate.rs`/`trace_out/tests.rs`/`verify/tests.rs`, and the
obs-only trace capture test re-bound through `families::param_args` /
`execute_args` (the PRD 20 surface).

Gates: `cargo fmt --all --check` clean; `clippy --workspace --all-targets
-- -D warnings` clean (obs feature included); `cargo test --workspace`
688 passed / 0 failed (+6 doc-tests); `scripts/check.sh` exit 0 end to
end (allocation gate included); `scripts/check-asm.sh` all three gates
green against a fresh release build — no gate re-pointing needed (no hot
symbol was renamed).

No conflicts.
