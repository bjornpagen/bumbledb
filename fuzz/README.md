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
| `query` | 13 | three-way parity over a cached Tiny target corpus: querygen's valid arm compared across the prepared engine, the naive model, and the `SQLite` lane where expressible (ψ-subset drops counted, never silent) with prepare/execute determinism; plus the hostile structurally-free-IR arm (`corpus_gen::irgen`) under the validation-totality oracle (typed rejection, TOTAL `ValidationError` census, deterministic verdicts) |
| `rewrites` | 13 | the dual-pipeline differential: every query × draw executed through the rewritten pipeline (chase + statically-empty fold on) and the rewrite-free one (the `chase-off`/`fold-off` thread-local switches — ONE build; cargo refuses a dual-build dependency on one package), demanding identical result sets; non-vacuity tallied off the profile surface |

Later PRDs add `ops` (12) and `crash` (14).

## Corpus policy

`corpus/<target>/` is the checked-in seed corpus (small, deterministic
generator runs); `artifacts/` is gitignored — a crash artifact is triage
input, never a deliverable. A minimized counterexample (`cargo fuzz
tmin`) becomes a permanent regression test in the crate that owns the
bug, and a row here.

## Trophy ledger

Every real finding, minimized and pinned, gets one row: date, target,
root cause, the regression test that now owns it.

| date | target | root cause | pinned by |
| --- | --- | --- | --- |
