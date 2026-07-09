# 06 — `verify_store`: the offline coherence sweeper (the amcheck lesson)

**Kind:** coherence tooling — build the deferred "offline sweeper" before it is
needed. **Decided placement: engine-side, `Db::verify_store`** — the sweeper's
knowledge (namespace layouts, guard derivation, statement resolution) is engine
knowledge; a bench-side implementation would duplicate storage internals outside
the crate that owns them and drift. The bench CLI gets a thin subcommand wrapper.
Starts manual; wiring it ahead of every `verify` stamp is a later cheapness
question.

## Context

The commit path self-checks three of its four namespaces: on delete, `F`/`M`/`U`
misses are hard `MembershipDesync` corruption errors — but outgoing `R`
reverse-edges are deleted **without** verifying they existed
(`crates/bumbledb/src/storage/commit/applier.rs:64-68`), explicitly deferred to an
offline sweeper that does not exist. Meanwhile the target-side containment
judgment **trusts** `R` prefixes as the authoritative survivor set
(`storage/commit/judgment.rs:300-317`). The one unverified namespace is one the
correctness of commit verdicts leans on — and `50-storage.md` currently cites the
sweeper as the compensating control, i.e. the docs promise a control that is not
real. Postgres's history with exactly this shape is why `amcheck` exists: silent
desyncs discovered during incidents, at maximum distance from their cause.

## The work

`Db::verify_store` — read-only, quiesced store, O(store):

1. **F↔M:** every `F` fact's blake3 has an `M` entry pointing back at its row id;
   every `M` entry's row id resolves to an `F` fact with matching hash.
2. **F↔U:** for every FD statement, every fact's guard key exists in `U` with the
   fact's row id; every `U` entry resolves to a live fact whose fields re-derive
   the guard. Pointwise keys additionally re-verify per-group disjointness by an
   ordered walk (the invariant the neighbor probe assumes).
3. **F↔R:** for every containment statement, every source fact satisfying φ has
   its `R` edge; every `R` edge resolves to a live source fact still satisfying φ.
   The namespace with no online verification — the heart of the item.
4. **Judgments re-verified whole:** both judgment forms globally (not
   delta-restricted) against the committed state — the naive model's semantics
   over the real store. Catches "the incremental form was wrong once, long ago" —
   the class no delta-scoped check can see.
5. **Counters:** `S` row counts equal `F`-prefix cardinality; row-id high-water ≥
   max row id; dict ids dense below the next-id counter (dangling dict entries are
   the *accepted* leak — report, don't fail).

Every failure is a typed report naming namespace, statement id where applicable,
and the offending key bytes — same payload discipline as `CorruptionError`.
Scale sanity: at the ≤10⁷-fact axiom this is seconds.

## Acceptance

- Fixture stores with each desync class hand-injected (missing `M`, orphan `U`,
  missing `R`, orphan `R`, wrong `S`) each produce the typed report and a nonzero
  exit from the CLI wrapper; a clean store passes.
- The `applier.rs:64-68` comment pointing at "the offline sweeper" names the tool.

## Doc amendments (rule 5)

`50-storage.md`: the R-delete asymmetry paragraph names `verify_store` as the
compensating control (making the existing citation true). `60-validation.md`: the
tool joins the validation story as the third leg (oracles judge semantics; the
sweeper judges the store).
