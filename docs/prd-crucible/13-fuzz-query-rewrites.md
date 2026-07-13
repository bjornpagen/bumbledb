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
