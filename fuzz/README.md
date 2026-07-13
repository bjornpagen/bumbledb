# fuzz — the fire

The generative fuzzing crate (docs/architecture/60-validation.md § the
fuzzing charter; docs/prd-crucible/11-fuzz-theory.md). Detached from the
workspace on purpose: workspace gates never build fuzz artifacts. Build
and run through `cargo fuzz` from the repo root; the pinned toolchain
(`rust-toolchain.toml`) owns every command.

```
cargo fuzz check                       # build every declared target
cargo fuzz run theory -- -runs=100000  # one smoke unit
```

## Targets

| target | PRD | drives |
| --- | --- | --- |
| `theory` | 11 | schema acceptance: random `SchemaDescriptor` (valid and deliberately-invalid shapes) → `Db::create` judgment, under the no-panic / typed-rejection / determinism+reopen+`verify_store` oracles |
| `ops` | 12 | the flagship lifecycle interleaver: generated op sequences (`corpus_gen::opgen`, ten verbs — insert/delete/mixed batch, commit, rollback, execute, re-prepare, view read, reopen, verify_store) against the live engine with the naive model in lockstep, under five oracles: verdict parity (typed violator, the multi-violation citation ruling), query parity, reopen equivalence, continuous `verify_store`, rejected-commits-change-nothing |
| `query` | 13 | three-way parity over a cached Tiny target corpus: querygen's valid arm compared across the prepared engine, the naive model, and the `SQLite` lane where expressible (ψ-subset drops counted, never silent) with prepare/execute determinism; plus the hostile structurally-free-IR arm (`corpus_gen::irgen`) under the validation-totality oracle (typed rejection, TOTAL `ValidationError` census, deterministic verdicts) |
| `rewrites` | 13 | the dual-pipeline differential: every query × draw executed through the rewritten pipeline (chase + statically-empty fold on) and the rewrite-free one (the `chase-off`/`fold-off` thread-local switches — ONE build; cargo refuses a dual-build dependency on one package), demanding identical result sets; non-vacuity tallied off the profile surface |

The remaining PRD adds `crash` (14).

## Corpus policy

`corpus/<target>/` is the checked-in seed corpus (small, deterministic
generator runs); `artifacts/` is gitignored — a crash artifact is triage
input, never a deliverable. A minimized counterexample (`cargo fuzz
tmin`) becomes a permanent regression test in the crate that owns the
bug, and a row here. `trophies/<target>/` holds the checked-in inputs
of recorded findings; `tests/replay.rs` replays every corpus and trophy
entry through its runner on plain `cargo test` — the regression-replay
slot.

## Trophy ledger

Every real finding, minimized and pinned, gets one row: date, target,
root cause, the regression test that now owns it.

| date | target | root cause | pinned by |
| --- | --- | --- | --- |
| 2026-07-13 | `ops` | multi-violation commits cite different statements on the two oracles: the engine convicts per affected tuple (`commit/judgment.rs` target checks), the model per statement id — a contract gap (`30-dependencies.md` pins citation identity, not the tie among simultaneous violations); no state effect. Ruled, not fixed: oracle 1 accepts any citation from the model's COMPLETE violation set (`NaiveDb::violations`), nothing outside it (docs/prd-crucible/12-fuzz-ops.md § conflict) | `trophies/ops/multi-violation-citation-order` via `tests/replay.rs`; `naive/tests/judgment.rs::citation_set` |
| 2026-07-13 | `ops` | generator hang, not engine: `querygen::shapes_interval::random_mask` rejection-sampled the vacuous EMPTY/FULL masks — non-terminating on the entropy seam's constant zero tail (`Rng::Bytes` exhaustion, PRD 10's exhaustion-is-legal contract). Fixed by total repair (EMPTY gains a bit, FULL drops one), loop-free; `contradict::contradiction_query` recorded as the same latent class, currently unreachable from fuzzer bytes | `shapes_interval::tests::random_mask_is_total_on_constant_streams` |
