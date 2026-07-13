# PRD 13 — fuzz targets: query (three-way parity) and rewrites (dual builds)

**Depends on:** 11 (crate + harness); independent of 12.
**Modules:** `fuzz/fuzz_targets/query.rs`, `fuzz/fuzz_targets/
rewrites.rs`, their runners in `fuzz/src/lib.rs`; `crates/bumbledb`
feature surface: `chase-off` (exists) and `fold-off` (**must be
RE-ADDED** — the witness campaign deleted it as dead because nothing
in-workspace consumed it; the fuzz crate is the consumer that could not
exist yet, since external crates cannot reach `cfg(test)`. Record the
round trip honestly: deleted as dead 2026-07, revived with a named
consumer).
**Authority:** the rewrite layers (chase resolution, condition folding,
statically-empty detection) are where "looks right" and "is right"
diverge silently — a wrong fold produces plausible wrong answers, not
crashes. Differential-with-the-rewrite-off is the only oracle that
catches them, and it already exists as an idiom (`chase-off`).
**Representation move:** none in the engine beyond the feature
revival. The runners reify "rewrites are semantics-preserving" as a
continuously-fuzzed theorem.

## Context (decided shape)

**query target** — three-way parity per iteration:
1. Generate schema + Tiny data + a query through the existing
   querygen (valid-by-construction arm AND a hostile arm that emits
   structurally-free IR for the validation totality oracle).
2. For valid queries: run prepared execution, the naive model, and —
   where the shape is expressible — the SQLite lane (reuse the
   differential's ψ-subset mapping; inexpressible shapes drop to
   two-way, counted and logged, never silently).
3. Oracles: result-set equality across all lanes; validation totality
   on the hostile arm (typed rejection, no panic); prepare/execute
   determinism (same query twice → identical rows).

**rewrites target** — dual-build differential:
1. The fuzz crate builds the engine TWICE: default features, and
   `chase-off` + `fold-off` (cargo-fuzz supports per-target features;
   the second target's manifest entry carries them). Same generated
   schema + data + query into both.
2. Oracle: identical result sets. The rewritten plan and the
   rewrite-free plan must agree on every fuzzed input — the rewrite
   layers proven semantics-preserving, not assumed.
3. `fold-off` revival scope: the cfg gates around the condition-fold
   pass exactly as they were before deletion (git shows the shape at
   the witness-02 parent commit); OFF must still typecheck and pass the
   engine suite — that is what makes the dual build honest.

## Technical direction

1. Revive `fold-off` first, in its own commit, with the engine suite
   green under `--features fold-off` (add that build to check.sh's
   matrix — it is now load-bearing for the fuzz oracle).
2. The dual-build plumbing: two library entries or feature-forwarded
   dependencies in `fuzz/Cargo.toml` (`bumbledb-rewriteless = { path,
   package = "bumbledb", features = [...] }` requires distinct package
   names — if cargo refuses the rename trick, fall back to ONE binary
   that runs the chase/fold passes explicitly vs skipped through
   internal entry points made `pub(crate)`-visible via a
   `fuzzing`-feature; decide by what cargo actually permits, record the
   choice here).
3. The inexpressible-shape counter on the SQLite lane prints its ratio
   at session end — if the three-way lane covers under half the
   generated shapes, note it in the human register (generator bias
   worth a look), don't fix silently.
4. Smoke both: 50k runs each, finding-free or trophies recorded.

## Passing criteria

- `[shape]` `fold-off` exists again, documented as fuzz-oracle
  infrastructure with its deletion/revival history in the feature's doc
  comment; engine suite green with it on AND off.
- `[shape]` The dual-build mechanism compiles both configurations and
  the runner diffs them (whichever mechanism cargo permitted, recorded).
- `[test]` 50k-run smoke per target finding-free (or trophies fixed +
  recorded).
- `[shape]` The three-way coverage ratio measured and recorded.
- `[gate]` check.sh's matrix includes the `fold-off` build; workspace
  gates green.

## Doc amendments (rule 5)

The fuzzing charter gains both targets' one-liners; `20-query-ir.md`'s
rewrite sections each gain the sentence "continuously verified
semantics-preserving by the rewrites fuzz target."

## Results (executed 2026-07-13)

**The dual-build mechanism, decided by what cargo actually permits.**
The rename trick was tried verbatim (`bumbledb-rewriteless = { path =
"../crates/bumbledb", package = "bumbledb", features = ["chase-off",
"fold-off"] }` beside the plain `bumbledb` dependency) and cargo
REFUSES it outright: ``error: the crate `bumbledb-fuzz v0.0.0` depends
on crate `bumbledb v0.1.0` multiple times with different names`` — no
second resolution of one path+version exists, so a true dual BUILD is
impossible in one binary, and feature unification would have unioned a
rename into one build anyway. The decisive observation: `chase-off` and
`fold-off` never were compile-time pass removal — they gate PUBLIC
access to the passes' thread-local runtime off switches
(`with_chase_disabled` / `with_fold_disabled`), i.e. they ARE the
"internal entry points made visible via a feature" fallback, already
shipped. So the `rewrites` target is ONE build with both features on,
running each query × draw through the rewritten pipeline and — inside
the two switch closures, which cover prepare — the rewrite-free one,
diffing complete result sets (typed runtime errors compared whole).
The naive-model substitution fallback was therefore NOT needed. The
`fold-off` revival landed first, in its own commit, with the engine
suite green ON and OFF and the check.sh matrix line added.

**Doc relocation note (README, "mechanism name is authoritative").**
The chase's rewrite section lives in `40-execution.md` (§ the chase),
not `20-query-ir.md`; the continuously-verified sentence lands there,
and `20-query-ir.md` carries it on the statically-empty fold
(normalization item 6). `40-execution.md`'s claim that
`with_fold_disabled` is "compiled only under `cfg(test)`" was stale the
moment the revival landed and is amended in the same change.

**Three-way coverage ratio: 1.000** (measured at the session's 10,000-
draw checkpoint: 10,000/10,000 valid-arm draws compared three-way, 0
SQL-inexpressible, 0 typed-error outcomes). Expected from the generator
itself — querygen's shape grammar never emits `Pack` (the one
inexpressible query construct), and Tiny's measure lane is ray-free by
construction — so the ψ-subset drop counter exists as a tripwire, not a
filter. Well above the one-half generator-bias threshold; no human-
register note needed.

**Smokes (capped by the 2026-07-13 session ruling: 15k runs or 20
minutes per target — the overnight firepower session accumulates the
rest; the rewrites smoke had already completed its full 50k).**

- `rewrites`: **50,024 runs** (8 parallel jobs × 6,250, ~475 s), ZERO
  findings. Non-vacuity measured, not assumed: ~200k query × draw pairs
  compared across the dual pipelines; at each job's 20,000-draw
  checkpoint ~15.5–16.4 % of draws had a rewrite provably fired
  (eliminated/folded occurrence or dead rule, read off the profile
  surface). Eight `slow-unit` artifacts — heavy join fan-out at Tiny
  (accounts = postings/200 = 5), inputs slow but correct; not findings.
- `query`: **15,671 runs** in the capped parallel session (8 jobs; the
  full-50k session is the orchestrator's, PRD 16), plus ~3k runs across
  two earlier partial sessions killed by session interrupts. Result-set
  findings: ZERO. Three crash artifacts appeared, all four affected
  jobs failing in the SAME wall-second with the identical
  `Lmdb(Io(EINVAL))` from `Db::prepare` under a concurrent
  `cargo test --workspace` compile storm (8 fuzz processes × ~1.4 GB
  RSS beside rustc) — all three artifacts REPLAY CLEAN on a quiet
  machine. Triaged environmental (tool-level, the differential's
  "setup errors stay panics" class), recorded in
  `/tmp/fuzz-query-finding-1.md`; no engine code touched, no trophy
  row.

**Hostile-arm note.** The structurally-free IR arm lives at
`corpus_gen::irgen` beside `theorygen` (not in querygen — it is shared
and valid-by-construction by contract). A fully free draw never aligns
finds/bindings/types into an accepted query, so half its draws start
from a schema-anchored plausible core with free mutations: 512 seeds →
143 accepted / 369 rejected, both verdict classes hot.
