# PRD 09 — Commit: containment, target side

**Depends on:** 08.
**Modules:** `crates/bumbledb/src/storage/commit/judgment.rs`; **deletes** `crates/bumbledb/src/storage/commit/restrict.rs`.
**Authority:** `docs/architecture/50-storage.md` (§ commit step 3 target side), `30-dependencies.md` (§ enforcement).

## Goal

Deleting (or shrinking, via delete+insert) target facts cannot strand a source
fact: every deleted-and-not-reestablished target key tuple is checked against the
statement's reverse edges, with the interval form re-walking coverage for affected
sources. `restrict.rs` is deleted; this is its replacement, generalized.

## Technical direction

1. **Inputs:** the per-statement `deleted_guards` set from PRD 07 (keyed by
   `StatementId` of the *key* statement) minus the `inserted_guards` set — the
   difference is the check set, exactly as the old restrict logic computed it.
   Map each key statement to the containment statements that resolved to it
   (`Resolved::Containment { target_key }` — build a `target_key → Vec<StatementId>`
   index once at schema seal, in PRD 03; add it there if missing).
2. **Scalar form:** for each (containment stmt, disestablished key tuple):
   prefix-scan `R | stmt | key_bytes`; any surviving entry ⇒ fetch the source fact
   (`F` get on `source_rel | source_row` — the existing cold-path fetch from
   `restrict.rs`, ported) ⇒ `ContainmentViolation { direction: TargetRequired,
   fact: source fact bytes }`. Note the subtlety ported from restrict: a surviving
   R entry whose *source fact was itself deleted this commit* cannot exist,
   because step 1 removed its R entries — no re-check needed; comment this.
3. **Interval form:** a disestablished target guard is a segment
   `(prefix, ts, te)` leaving group `prefix`. Affected sources are R entries in
   `R | stmt | prefix` whose source interval intersects `(ts, te)` — range-scan
   the group (R key_bytes end with the source interval, start-ordered): seek to
   `prefix | first-start-that-could-intersect` — conservatively scan the whole
   `prefix` group and filter by intersection; the group is small and this is the
   delete path (comment the conservatism; an optimized lower bound needs the
   max source-interval length, which we refuse to track). For each intersecting
   survivor: re-run PRD 08's coverage walk against the **final** `U` state; a
   failed walk ⇒ violation naming that source fact.
4. **Ordering:** target-side checks run after all deletes and inserts (they read
   final state through the write txn). Batch per (stmt, prefix-group): if several
   segments of one group were disestablished, walk each affected source **once**
   (dedupe the affected-source set per group before walking).
5. Delete `restrict.rs` and its wiring; the `obs` span name `FK_RESTRICT` becomes
   a judgment-phase span (rename in `obs.rs`'s name registry).

## Out of scope

Any new namespace. Any caching across commits.

## Passing criteria

- `[shape]` `restrict.rs` no longer exists; `rg -i restrict crates/bumbledb/src`
  returns nothing.
- `[shape]` The affected-source dedupe per prefix group exists (no source is
  coverage-walked twice for one commit's changes to one group).
- `[test]` Scalar: deleting a referenced target alone aborts (`TargetRequired`,
  naming the source fact); deleting target + all sources in one delta commits
  (cluster demolition); deleting a target whose only source is also deleted but a
  *different* source (other statement, same key bytes) survives — aborts on the
  right statement id.
- `[test]` Interval: shrink a covering segment (delete `[0,10)`, insert `[0,7)`)
  under a source `[5,9)` — aborts; shrink under source `[2,6)` — commits; delete
  one segment of a two-segment chain covering a source — aborts; delete it and
  insert a replacement covering the hole in the same delta — commits.
- `[test]` `==` demolition: parent + child deleted in one delta commits; child
  alone deleted aborts on the totality direction.

## Conflict

Found while implementing technical direction 1; implemented exactly as specified,
flagged for the owner because the architecture is silent.

The check set `deleted_guards − inserted_guards` treats a key tuple as
re-established whenever *any* fact re-lands its guard bytes. A containment with a
**target selection** requires a target satisfying ψ, and re-establishment does not
check ψ: delete `Account(9, active=true)` + insert `Account(9, active=false)` in
one delta re-establishes the key guard `(id=9)`, so the tuple leaves the check set
— yet a surviving `Transfer(9)` under `Transfer(account) <= Account(id | active ==
true)` is stranded in the committed final state. Neither side catches it (the
source side probes only *inserted* source facts). The interval form has the same
shape: delete + re-insert a byte-identical segment whose σ-relevant non-key field
changed. Both this PRD ("the difference is the check set, exactly as the old
restrict logic computed it" — correct pre-σ) and `50-storage.md`/
`30-dependencies.md` ("deleted and not re-established") specify the unqualified
subtraction, so `judgment::check_target` does the unqualified subtraction.

A possible fix, not improvised here: refine per dependent statement — a
re-established guard counts only if the establishing fact satisfies that
statement's target σ (one `F` get per re-established guard per σ-carrying
dependent; dependents with empty σ keep the plain subtraction, which is every
currently-tested case).
