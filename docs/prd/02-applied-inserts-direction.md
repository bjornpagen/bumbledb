# PRD 02 — Net-disposition delta (judgment direction by construction)

**Depends on:** nothing (PRD 11 depends on this).
**Modules:** `crates/bumbledb/src/storage/delta/` (the representation change),
`crates/bumbledb/src/storage/commit/{applier.rs,judgment.rs,write.rs}`
(simplifications it licenses), `crates/bumbledb-bench/src/verify/` (op-stream
generator), `crates/bumbledb-bench/src/naive/differential.rs` (regression
stream).
**Authority:** `30-dependencies.md` (enforcement summary), `50-storage.md`
(the delta write path), `60-validation.md` (verdicts compared whole, including
`Direction`), `00-product.md` (representation over control flow).

## Context (decided — representation-first)

The two oracles disagree on a verdict *label* for a legal pattern: `A(x) <= B(y)`
holds, `a ∈ A` and target `b ∈ B` committed; one transaction does `insert(a)`
(a storage no-op) and `delete(b)`. The engine judges `a` source-side
(`SourceUnsatisfied`); the naive model judges the delete target-side
(`TargetRequired`). A latent verify-red. **The naive model is normative** —
"source side" means facts the transaction *actually added* — and the model stays
byte-for-byte untouched.

An applied-inserts list produced by the applier was considered and **rejected**:
it patches the trace (plumbing a second data flow to compensate for the delta
recording something false). The root is the representation. **The delta already
knows no-op-ness at op time** — `insert`/`delete` probe `M` through the
committed view to compute their `changed: bool` return (`50-storage.md` write
path) — and then throws the knowledge away by recording the disposition anyway.
Validation discards proof; parsing keeps it.

## Technical direction

1. **The delta records net dispositions against committed state.** At op time
   (the `M`+delta probe already performed for `changed`):
   - fact committed + `insert` → remove any pending `Delete` entry; record
     nothing (net no-op);
   - fact committed + `delete` → record `Delete`;
   - fact absent + `insert` → record `Insert`;
   - fact absent + `delete` → remove any pending `Insert` entry; record nothing.
   Last-disposition-wins becomes a *consequence* of these rules rather than a
   rule of its own. The single-writer mutex holds committed state stable for the
   delta's lifetime, so op-time probing is authoritative — say so in the module
   doc. The guard map (point reads) keeps its current behavior: reads compose
   delta-over-committed either way; verify with the read-your-writes tests.
2. **The invariant this buys, stated in the delta's doc comment:** the insert
   set contains exactly the facts commit will add; the delete set exactly the
   facts it will remove. "Judging a no-op insert" becomes unrepresentable.
3. **Simplifications it licenses (take them all):**
   - the applier's no-op short-circuit (`applier.rs:85-87`) dies — every entry
     applies by construction (the `MembershipDesync` corruption probes remain:
     they check storage agrees with what the delta *proved*, and a disagreement
     is now unambiguously corruption, which sharpens their meaning);
   - source-side judgment iterates the delta's insert set directly — no
     applied-list plumbing, no per-entry flags;
   - the wasted re-judging of redundant inserts disappears (observable:
     `JUDGMENT_SOURCE` probe counts drop for deltas containing them);
   - `bulk_load`'s changed-count is the delta size — check whether its counting
     code simplifies.
4. **Consequence check:** the pattern's violation now surfaces target-side via
   the standing R edge, naming the source fact. Audit tests pinning the old
   `SourceUnsatisfied` classification and update with a comment citing the
   normative rule. The `==`/totality corner needs no code (no-op parent
   re-insert + child delete → caught target-side via the parent's standing R
   edge) — pin it with a test.
5. **Generator coverage:** the op-stream generator gains the pattern class
   (redundant insert alongside a delete of its containment target).

## Passing criteria

- `[shape]` No applied-inserts list, no per-entry applied flag, no applier
  no-op short-circuit; the net-disposition rules live in the delta's op
  recording; the invariant doc comment exists.
- `[test]` Differential regression: pre-seed `{a, b}`, transaction
  `[insert(a), delete(b)]` — engine and naive verdicts identical **including
  `Direction`**.
- `[test]` Net-disposition algebra: `delete(x);insert(x)` on a committed `x` is
  an empty delta; `insert(y);delete(y)` on an absent `y` is an empty delta;
  both commit as no-ops (storage tx id unchanged). The existing
  either-order-idiom and read-your-writes tests stay green.
- `[test]` The `==`/totality corner: both oracles abort target-side.
- `[test]` A delta with a redundant insert and no violation commits with zero
  source-side probes for it (stats/trace surface).
- `[test]` Op-stream generator emits the pattern class (coverage assertion).
- `[gate]` Workspace gates green; the naive model's files show zero diff.

## Doc amendments (rule 5)

`50-storage.md` write path: the delta paragraph states net-disposition recording
and its invariant. `30-dependencies.md` enforcement summary: "per **genuinely**
inserted A-fact" — now true by representation. `60-validation.md`: the pattern
joins the covered-cases list.
