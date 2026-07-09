# PRD 13 — Elegance: storage

**Depends on:** 12 (sequential elegance passes — each inherits the previous
pass's normalized idioms).
**Binding constraints:** the README's elegance-pass block.
**Modules:** `crates/bumbledb/src/storage.rs` + `storage/` (env, keys, dict,
delta, commit — applier/judgment/write, read), `crates/bumbledb/src/verify_store/`
(fresh from PRDs 06–07 — built by a different unit than commit; the seams
between sweeper and commit path are prime targets), `crates/bumbledb/src/arena.rs`.

## Subsystem-specific hunt list (verify, don't assume)

- **Guard/key derivation call-sites:** `keys::guard_bytes` /
  `permuted_guard_bytes` are the single slicers by design — but the *call-site
  patterns* around them (derive → build key → probe) recur across applier,
  judgment, point reads (PRD 10 of the rebuild), and the sweeper. If three or
  more sites share the derive-build-probe shape, extract the shape; if only the
  slicer is shared, leave it (do not force an abstraction).
- **The coverage walk:** PRD 07 required one implementation shared by commit
  and sweeper. Confirm, and check the neighbor probe for the same
  lift-worthiness (it has one caller today — leave single-caller code inline;
  note the decision).
- **Delta bookkeeping:** the fact map, guard map (point reads), applied-inserts
  output (PRD 02), and serial marks grew in three different eras — check the
  delta module for parallel-but-divergent structures that should be one struct
  with named fields, and for last-disposition-wins logic implemented more than
  once.
- **Phase plumbing in `write.rs`:** the commit pipeline threads an accumulating
  set of outputs (deleted guards, inserted guards, applied inserts, pending
  interns) as separate arguments — if the signature has grown past ~4 threaded
  values, introduce the phase-output struct the signatures are begging for.
- **Judgment module size:** `judgment.rs` absorbed source-side, target-side,
  ψ-qualification, and the coverage walk across three PRDs — check for
  sectioning into submodules along the check-direction boundary if the file
  reads as three files concatenated (it may be fine; length alone is not a
  finding).
- **Test fixtures:** commit tests construct schemas/stores in at least three
  styles (PRD 07/08/09 eras). Converge.

## Passing criteria

As PRD 12's, applied to this subsystem: findings summary; no assertion changes;
zero unjustified dead weight; `[gate]` workspace gates green. Additionally:
- `[shape]` The coverage walk still has exactly one definition (grep).
- `[shape]` Any change under `storage/commit/` that touches the judgment or
  apply hot loops is flagged in the commit body for the closing re-bench.
