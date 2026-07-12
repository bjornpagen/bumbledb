# PRD set — the witness pass: the type carries the proof, the record carries the number

This directory is the complete, ordered work plan for the pass after the
comptime campaign: the 2026-07-12 audit's findings, executed. **Baseline
assumption: the comptime set is fully landed** (closed relations, compiled
subsets, the six-type roster, the folds, the latch, `ExecPlan::Empty`,
handles everywhere; the PRD directories retired; the benchmark numbers
regenerated 2026-07-12, verify stamp `0ed24dd9…`). Where a file path below
has moved, the *mechanism name* is authoritative and the executor
re-locates it.

## The organizing principle: proofs into types

The house axiom (`00-product.md`, Brooks → Pike → Raymond → Torvalds): the
biggest lever is the shape of the data, not the cleverness of the code.
This set applies it to the engine's own witness types. The audit's census:
201 `unreachable!` + 353 `.expect` sites in the engine crate, of which
roughly a quarter guard **parallel-discriminant agreement** — a
construction boundary sealed a proof, but the *type* doesn't carry it, so
~55 downstream sites re-assert with control flow what validate or prepare
already established. King's line is the diagnosis: a validator returns
nothing and forces every caller to re-check; a parser returns a type that
carries the proof, so the check happens once at the boundary. The witness
pass makes the four worst offenders parse:

1. **`Statement`** — a sum stored as three parallel fields (`descriptor` ∥
   `resolved` ∥ `checks`), 19 re-assertion sites (PRDs 09–11).
2. **`PreparedRule` and the `Empty` sentinel** — Option-pairs whose
   agreement is the plan kind, plus a dead program impersonating a rule
   (~11 sites, PRD 12).
3. **The chase evaluator's boolean gate** — `filters_prepare_resolvable`
   proves resolvability and throws the proof away; the evaluator re-parses
   and `unreachable!`s everything the gate refused (~15 sites, PRD 13).
4. **The sink's pre/post-rewrite shared enum** — `rewrite_measures` parses
   in place into the same type (~12 sites, PRD 14). Plus the small
   `ParamSpec` triple (~8 sites, PRD 15).

The second half of the pass is **the record**: the measurement discipline's
own ratchet, applied. A mechanism that measures as a loss is reverted, and
the record keeps the numbers *and the failure mechanism* — so the two
standing contradictions (the rule-disjointness elision at −32% p50, the
`≤ 3.3×` estimator pin against a measured 4761×) get isolated, diagnosed
in code, and closed (PRDs 06–08). The mechanical record-truth and residue
items land first (PRDs 01–05) so everything after edits a clean tree.

## Vocabulary discipline

The register extends the comptime set's: *witness* (a type whose
inhabitants prove a property, minted only at a construction boundary),
*total* (a match with no impossible arms), *discharge* (a proof obligation
moved from a call site into a type), *reckoning* (a measurement-adjudicated
ruling executed), *the record* (the architecture chapters — the only
normative text). Banned: *shim*, *compat*, *fallback path* (cut direct);
*TODO* (an obligation is a PRD criterion or it does not exist).

## Policy (read before executing any PRD)

1. **A PRD is a work-organizational unit, not an atomic passing-code
   state.** No transitional shims, no compatibility aliases, no feature
   flags. Rip the old thing out and cut directly to the end state; the
   tree may fail to typecheck between PRDs — downstream breakage is the
   next PRD's job.
2. **Passing criteria are typed.** `[shape]` — checkable by reading or
   grep the moment the PRD lands. `[test]` — unit tests written in this
   PRD, co-located with the code they pin. `[gate]` — holds when the
   campaign closes: `cargo fmt --all --check`, `clippy --workspace
   --all-targets -- -D warnings`, `cargo test --workspace`,
   `scripts/check.sh`.
3. **No migrations, ever.** No PRD writes store-conversion code. Stores
   are regenerated; ETL is the human's story.
4. **No smoke-test or end-to-end-test PRDs.** Unit tests pinning this
   set's code are in scope where a PRD says so; running verify/bench
   harnesses is human/orchestrator work (the human work register below).
5. **Conflict protocol:** if executing a PRD reveals the architecture
   docs are wrong or silent, stop and record the conflict in the PRD
   file.
6. **Doc amendments land in the same change** (architecture README
   rule 5). There are no doc-only PRDs in this set except 01, which IS
   the record-truth PRD the audit demanded; every other chapter change
   rides the code PRD that makes it true.
7. **The fingerprint is load-bearing.** No PRD in this set may move a
   schema fingerprint: the witness pass reshapes SEALED types, never the
   declaration surface (`SchemaDescriptor`, `StatementDescriptor`,
   `Side`, materialized order) that the fingerprint hashes. PRD 09 pins
   this with a criterion.

## Pre-execution conflict resolution

The 2026-07-12 execution preflight found and resolved five specification
defects before implementation began:

- Zero-hit searches in PRDs 01 and 07 searched this work-plan directory
  and therefore matched their own retired-vocabulary examples. Those
  checks now target the product code and normative record only:
  `crates`, `docs/architecture`, and the repository `README.md`.
- PRD 03 required production benchmark query builders to move into a
  `#[cfg(test)]` module and gave contradictory totals for `TempDir`.
  The shared module is now an always-compiled `fixture` module with
  test-only items individually gated, and the total is four.
- PRD 07 required a second owner interaction even though the owner
  delegated this complete campaign as an unattended ordered loop. The
  locked post-06 run now adjudicates mechanically: a measured loss takes
  branch R; a measured win takes branch E. No discretionary tie-break is
  delegated.
- PRD 08 named a staleness threshold constant that does not exist. The
  convention currently lives only in documentation and a code comment;
  those two statements must move together.
- PRD 14's broad `Duration` grep rejected the table test it requires.
  The criterion now checks the type boundary directly: symbolic measure
  variants may occur at the parser and in its tests, never in `SinkSpec`
  or post-parse matches.

These are corrections to make the already-decided work executable; they
do not add product scope or weaken a behavioral gate.

## The PRDs

Phase A — the record and the residue (independent of each other; 05 last):
- [01 — The record trued: the pipelined executor enters the doc](01-the-record-trued.md)
- [02 — Dead configuration: fold-off, skip_free, and the defunct dump](02-dead-configuration.md)
- [03 — Fixture deduplication: the bench testfix module](03-fixture-dedup.md)
- [04 — The identifier funeral completes: Variant, ordinal, seed](04-identifier-funeral.md)
- [05 — allow becomes expect: stale suppressions fail the gate](05-allow-to-expect.md)

Phase B — the reckonings (06 → 07 strictly; 08 independent):
- [06 — The elision sub-measurement isolated](06-elision-isolated.md)
- [07 — The elision ratchet: revert or re-earn](07-elision-ratchet.md)
- [08 — The estimator reckoning: the 3.3× pin vs the 4761× report](08-estimator-reckoning.md)

Phase C — the witness pass (09 → 10 → 11 strictly; the tree may be red
from 09 until 11 closes it):
- [09 — Statement becomes a witness: the sum and its ids](09-statement-witness.md)
- [10 — The commit pipeline consumes the witness](10-commit-consumes-witness.md)
- [11 — Sweeper, chase, and surface consume the witness](11-sweeper-chase-witness.md)

Phase D — the smaller parses (independent of each other and of Phase C):
- [12 — The program sum: PreparedRule and Empty stop impersonating](12-program-sum.md)
- [13 — Resolvable filters: the chase gate parses](13-resolvable-filters.md)
- [14 — SinkSpec: the rewrite returns a narrower type](14-sink-spec.md)
- [15 — ParamSpec: three vectors become one](15-param-spec.md)

Dependency spine: 01–04 freely ordered, 05 closes Phase A (it converts
suppressions the earlier PRDs may touch); 06→07 strictly (07 executes on
06's isolated number — see its precondition); 08 any time; 09→10→11
strictly (each consumes the previous's types); 12–15 any time after
baseline, in any order. Phases: A first, then B/C/D interleave freely
except as spined.

## The human work register (explicitly not PRDs)

Adjudications and measurements outside ordinary PRD implementation, in
the order they unblock: (1) the isolated elision bench run after PRD 06
(the owner's unattended-campaign delegation makes its sign the mechanical
PRD 07 branch ruling); (2) the batch-size sweep via `set_batch_size` 64/128/256 on the
ledger families — pins the D4 OPEN item; (3) one `--trace` calendar bench
run — settles four dormant fold/mask triggers' status; (4) the
`#[ignore]`d microbench session re-earning the surviving pinned margins;
(5) any future scale-L corpus (the p99 budget's binding scale, per
PRD 01's scoping). None of these blocks a Phase A/C/D PRD.

## Refusals (recorded with derivations — do not re-litigate)

- **No symbolic/resolved split of `Const`.** The template and resolved
  slots must share one type: the literal latch's in-place monotone
  rewrite is the recorded mechanism ("the latch IS the rewrite," 40-execution
  § the literal latch). The ~47 leaf-evaluator arms are its paid price.
- **`missed_params` stays a parallel bool.** Not derivable from the
  `Const` (a numeric param may legitimately hold the sentinel word);
  pooling rationale recorded at the field. One writer maintains it.
- **`MembershipOp` stays symmetrically derived on deletes.** Splitting
  `fact_op` adds a mode parameter to save a few word-pushes on the cold
  delete path — control flow purchased with nothing. The comment records
  the honesty.
- **No post-shapes `Term` operand sum** (the `ir/validate/context.rs` +
  `place_comparisons.rs` assert family). A parallel Term vocabulary
  tensions with the one-IR philosophy. *Trigger:* PRD 13's
  `ResolvableFilter` pattern landing well and the family still itching.
- **The "(PRD NN)" cohort labels are blessed vocabulary.** They are
  ticket numbers, not pointers; git history holds the mapping. Only
  narrating comments die (PRD 01/03).
- **`Interval::ray`/`is_ray` stay.** Public host API surface; the engine's
  internal word-level ray checks are the engine's business.
- **`docs/brainlift-sources/` stays.** Zero inbound references but it is
  the research substrate of record; deletion is the owner's call alone.
- **rusqlite/edition bumps deferred** to the next number-regeneration
  campaign: any SQLite bump voids the comparison baseline, and the
  numbers are days old. First item of that campaign, not this one.
