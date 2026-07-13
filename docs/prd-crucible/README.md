# PRD set — the crucible: one toolchain, one authority, trial by fire

This directory is the complete, ordered work plan for the pass after the
witness campaign. **Baseline assumption: the witness set is fully landed**
(verified 2026-07-13 at `eba32a5`: `Statement` is a typed-witness sum with
`KeyId`/`ContainmentId` arenas and `Enforcement { Probe, Closed }`;
`Program`/`PreparedRule` sums replaced `ExecPlan`; `ResolvableFilter`,
`SinkSpec`, `ParamSpec` landed; the union elision is REVERTED with its
refutation recorded; the estimator pin is re-derived and the fixed
staleness cutoff replaced by the pull-based drift report; the engine's
non-test `unreachable!` census is 147, down from 201; suppressions are
`#[expect]` except the one recorded unsafe-policy site; the fingerprint
pin `63e3b480…` never moved). Where a file path below has moved, the
*mechanism name* is authoritative and the executor re-locates it.

## The organizing principle: melt it down, then torture it

Two moves, one campaign. **First, the melt**: every remaining dual notion
dies — the toolchain dual (a stable pin that forces a stable/nightly split
the fuzzer would otherwise need), the signature dual (`head_types` vs
`result_types` vs `column_types`, three derivations of the one thing a
query defines), the vocabulary dual (`PredicateTree` naming comparison
trees while the predicate concept goes nameless), and the classification
dual (`comparison_shapes` proving legality then discarding the proof for
`place_comparisons` to re-derive, ~27 asserts). One authority per concept,
zero lingering consumers of any old shape — grep-zero is the criterion
everywhere. **Second, the fire**: a complete generative fuzzing
infrastructure over the two oracles this engine already owns, structure-
aware and coverage-guided, saturating every core, with every
counterexample minimizing into a permanent regression. The engine was
built measurement-first; this set makes it adversary-first too.

The recursion question is settled by refusal, not silence: the closure
idiom ships as a cookbook recipe on the current engine, the refusal is
recorded with its trigger, and the full recursion design lands as a paper
proof with a seam ledger — so the punt is an engineering decision with a
reactivation condition, and the IR the fuzzer learns is the IR that
stays.

## Vocabulary discipline

The register extends the witness set's: *predicate* (the typed output a
query defines — anonymous, engine-internal, referenced by nothing),
*signature* (its column list), *condition* (a comparison tree — the word
"predicate" is hereby reclaimed), *dividend* (a nightly feature adopted
because it deletes code, never because it exists), *target* (one fuzz
entry point), *trophy* (a minimized counterexample, permanently a test),
*firepower* (all cores, fork-mode, every target). Banned: *shim*,
*compat*, *transitional*, *stable fallback* (there is one toolchain);
*TODO* (an obligation is a PRD criterion or it does not exist).

## Policy (read before executing any PRD)

1. **A PRD is a work-organizational unit, not an atomic passing-code
   state.** No transitional shims, no compatibility aliases, no feature
   flags for migration. Rip the old thing out and cut directly to the end
   state; the tree may fail to typecheck between PRDs — downstream
   breakage is the next PRD's job.
2. **Passing criteria are typed.** `[shape]` — checkable by reading or
   grep the moment the PRD lands. `[test]` — unit tests written in this
   PRD, co-located with the code they pin. `[gate]` — holds when the
   campaign closes: `cargo fmt --all --check`, `clippy --workspace
   --all-targets -- -D warnings`, `cargo test --workspace`,
   `scripts/check.sh`, and (new, from PRD 16 on) the corpus-replay lane.
3. **No migrations, ever.** Stores are regenerated; ETL is the human's
   story.
4. **No smoke-test or end-to-end-test PRDs.** Unit tests pinning this
   set's code are in scope where a PRD says so; running verify/bench
   harnesses and long fuzz explorations is human/orchestrator work (the
   register below).
5. **Conflict protocol:** if executing a PRD reveals the architecture
   docs are wrong or silent, stop and record the conflict in the PRD
   file.
6. **Doc amendments land in the same change.**
7. **The fingerprint is load-bearing.** Nothing in this set touches the
   declaration surface the fingerprint hashes. The pin `63e3b480…` and
   the corpus digest survive every PRD; PRDs that could conceivably move
   them carry the explicit criterion.
8. **Behavior-preserving refactors pin first.** Where a PRD replaces a
   derivation (the predicate, the comparison witness), its FIRST step is
   an exhaustive table test against CURRENT behavior; the refactor lands
   against the pinned table. Backwards is forbidden — a table written
   after the refactor pins the refactor's own bugs.
9. **Nightly features are dividends, not toys.** A feature is adopted
   only where it DELETES code or guards (the criterion names the
   deletion); every considered-and-rejected feature gets a one-line
   refusal in PRD 03's ledger. Every adopted feature is named in
   `rust-toolchain.toml`'s comment block so the pin's reason survives.

## The PRDs

Phase A — one toolchain (strictly ordered 01 → 02 → 03):
- [01 — The toolchain melts: pinned nightly, edition 2024](01-nightly-pin.md)
- [02 — The nightly dividend: guards deleted by the standard library](02-nightly-dividend.md)
- [03 — portable_simd, measurement-gated: adopt or refuse](03-portable-simd.md)

Phase B — one authority (04 → 05 ordered; 06/07 after 05):
- [04 — The predicate: one signature, one derivation](04-predicate.md)
- [05 — The vocabulary reclaimed: PredicateTree becomes ConditionTree](05-condition-tree.md)
- [06 — The closure idiom: recursion punted on the record](06-closure-idiom.md)
- [07 — The recursion design: a paper proof with a seam ledger](07-recursion-paper.md)

Phase C — the last parse (08; 09 closes the melt):
- [08 — The comparison witness: classification carries its proof](08-comparison-witness.md)
- [09 — The census floor: the melt's terminal reconciliation](09-census-floor.md)

Phase D — the fire (10 → 11 ordered; 12–15 after 11, freely; 16 closes):
- [10 — The entropy seam: fuzzer bytes drive the generators](10-entropy-seam.md)
- [11 — The fuzz crate and the theory target](11-fuzz-theory.md)
- [12 — The op-stream target: the flagship](12-fuzz-ops.md)
- [13 — The query and rewrite-dual targets](13-fuzz-query-rewrites.md)
- [14 — The crashpoint hook and the crash target](14-fuzz-crash.md)
- [15 — Proof by enumeration, and the UB lanes](15-exhaustive-miri.md)
- [16 — CI and the firepower orchestrator](16-ci-firepower.md)

Dependency spine: 01 first (everything downstream compiles on the new
toolchain once); 02–03 ride it. 04→05 strictly (the rename lands on the
new types); 06–07 any time after 05. 08 after 05 (it reuses the parse
conventions); 09 closes Phase B+C. 10 requires 01 only; 11 requires 10;
12–15 require 11, freely ordered; 16 last. Phases B/C and D interleave
freely — they touch disjoint files except where a PRD names the overlap
(13's fold-off note).

## The human work register (explicitly not PRDs)

(1) Long fuzz exploration sessions via `scripts/fuzz.sh` after PRD 16 —
the machine's idle hours are the campaign's real payload; (2) the
batch-size sweep (`set_batch_size` 64/128/256) — the standing D4 OPEN
item; (3) one `--trace` calendar bench run — the four dormant fold/mask
triggers; (4) the `#[ignore]`d microbench re-earn session ON THE NEW
TOOLCHAIN (PRD 01 changes codegen; every pinned margin is suspect until
re-asserted — this is the toolchain move's one real tax); (5) a full
bench regeneration if the microbenches move materially.

## Refusals (recorded with derivations — do not re-litigate)

- **No named predicates, no predicate references, no registry.** The
  predicate is anonymous and engine-internal; the algebra campaign's
  named-view refusal stands. The moment something REFERENCES a predicate,
  that is the recursion trigger firing — go through PRD 07's ledger, not
  around it.
- **No `AtomSource`/`PredId`/strata/frontier-hook code.** Recursion's
  cuts live in PRD 07's paper until its trigger fires. A one-inhabitant
  sum is a dead arm in every consumer.
- **No symbolic/resolved `Const` split; `missed_params` stays;
  `MembershipOp` symmetry stays; the leaf-evaluator arms stay** — the
  witness set's inherited rulings, unchanged.
- **The `NormalizedQuery.dead` rendered-string ruling stands** unless
  PRD 07's design finds a programmatic consumer (record there if so).
- **rusqlite/edition-of-bench-C-deps bumps stay deferred** to the next
  number-regeneration campaign. (The RUST edition moves in PRD 01 —
  that's toolchain, not baseline.)
- **`docs/brainlift-sources/` stays** (research record; its stale chapter
  citations are historical provenance, exempt from sweeps). PRD 09 rules
  on `docs/reference/` separately.
- **No fuzzing of the bench harness itself.** The oracles are the judge;
  a fuzzer for the judge has no judge. The naive model's correctness is
  the differential's axiom, guarded by its independence greps and its
  definitional simplicity — that is the recorded trust root.
