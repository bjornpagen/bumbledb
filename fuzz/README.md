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
| `crash` | 14 | durability under torn commits: an ops prefix plus one victim commit (`corpus_gen::opgen::random_crash_scenario`) replayed in a CHILD process that aborts at a drawn crashpoint (the commit pipeline's named phase boundaries — engine hooks under the `crashpoint` feature, `BUMBLEDB_CRASHPOINT`-armed, compiled to nothing by default; the table in `storage/commit.rs` is the single authority); the parent proves all-or-nothing recovery: reopen, `verify_store`, full contents at the point's expected side (prefix before `mdb_txn_commit`, post after), victim replay landing the post state. The deterministic sweep (`tests/crash.rs`) kills every crashpoint × a small ops-prefix matrix under plain `cargo test`; child spawns cap throughput, so smoke budgets are lower than the in-process targets |


## Operations

`scripts/fuzz.sh` is the firepower launcher (docs/prd-crucible/
16-ci-firepower.md): no args runs all five targets time-sliced in
libFuzzer fork mode across 12 workers; `fuzz.sh <target> [minutes]`
bounds one target; `fuzz.sh --asan <target>` is the sanitizer lane
(query carries `-rss_limit_mb=4096` there — the ASAN quarantine
disposition, docs/prd-crucible/15-exhaustive-miri.md § Results). Every
session ends with `cargo fuzz cmin` on the target's corpus and one
summary line appended to `SESSIONS.md` (execs, rate, coverage, corpus
growth, findings — the honest zero is a recorded result). The launcher
REFUSES to start while `artifacts/` holds any file: untriaged findings
block new sessions.

## Corpus policy and the trophy pipeline

`corpus/<target>/` is the checked-in seed corpus, kept lean by the
launcher's post-session `cargo fuzz cmin`; `artifacts/` is gitignored —
a crash artifact is triage input, never a deliverable. The pipeline for
every artifact, enforced by the launcher's dirty-artifacts refusal:

1. **Reproduce and minimize** (`cargo fuzz tmin`).
2. **Real finding** → a permanent NAMED regression test in the crate
   that owns the bug (input inlined as bytes, or checked into
   `trophies/<target>/` where `tests/replay*.rs` replays it on plain
   `cargo test`), plus a trophy-ledger row below.
3. **Environmental** → the disposition recorded in `SESSIONS.md` with
   the replay evidence. Worked example: the `Lmdb(Io(EINVAL))` storms —
   artifacts appearing in the same wall-second across jobs under a
   concurrent compile, every one replaying clean on a quiet machine;
   triaged clean, recorded, deleted.
4. **The artifact is deleted.** Findings that live in an artifacts
   directory are not findings, they are homework.

## Trophy ledger

Every real finding, minimized and pinned, gets one row: date, target,
root cause, the regression test that now owns it.

| date | target | root cause | pinned by |
| --- | --- | --- | --- |
| 2026-07-13 | `ops` | multi-violation commits cite different statements on the two oracles: the engine convicts per affected tuple (`commit/judgment.rs` target checks), the model per statement id — a contract gap (`30-dependencies.md` pins citation identity, not the tie among simultaneous violations); no state effect. Ruled, not fixed: oracle 1 accepts any citation from the model's COMPLETE violation set (`NaiveDb::violations`), nothing outside it (docs/prd-crucible/12-fuzz-ops.md § conflict) | `trophies/ops/multi-violation-citation-order` via `tests/replay.rs`; `naive/tests/judgment.rs::citation_set` |
| 2026-07-13 | `ops` | generator hang, not engine: `querygen::shapes_interval::random_mask` rejection-sampled the vacuous EMPTY/FULL masks — non-terminating on the entropy seam's constant zero tail (`Rng::Bytes` exhaustion, PRD 10's exhaustion-is-legal contract). Fixed by total repair (EMPTY gains a bit, FULL drops one), loop-free; `contradict::contradiction_query` recorded as the same latent class, currently unreachable from fuzzer bytes | `shapes_interval::tests::random_mask_is_total_on_constant_streams` |
