# PRD 03 — Coverage evidence: the disjointness precondition becomes a proof object

**Depends on:** 02 (evidence structs name interval types; land after to
avoid double-churn in schema.rs).
**Modules:** `crates/bumbledb/src/schema.rs` (`Enforcement`, :366-378),
`schema/validate.rs` (`resolve_target_key` :645, the coverage flag
assignment :755-758, the pointwise-key acceptance branch :253-274),
`storage/commit/judgment.rs` (`check_coverage` :622 and its callers),
`verify_store/` (the offline re-derivation reads the same shape),
consumers of `Enforcement` in commit plan and introspect.
**Authority:** audit P0-2, verified current: `check_coverage` is sound
only under the precondition that the target guard group is disjoint and
start-ordered, and that precondition is carried in comments
(judgment.rs:604-609, 649-653) — established by the pointwise-key gate
at validation, then ASSUMED at commit. The audit's sharpest line: an
overlapping guard group would return a wrong commit verdict without
erroring.
**Representation move:** the precondition becomes a zero-size proof
token mintable only by the validator path that accepts a pointwise
interval key. Commit-time coverage cannot be CALLED without it. The
boolean `coverage` flag — a proof obligation hidden in a `bool` — dies.

## Context (decided shape)

```rust
pub(crate) enum Enforcement {
    /// Scalar containment: probe the target key image for existence.
    ScalarProbe { target_key: KeyId, key_permutation: Box<[u16]> },
    /// Interval coverage: sweep the target's segments. Carries the
    /// proof that the target key is pointwise — i.e. its guard group
    /// is disjoint and start-ordered by the interval-FD judgment —
    /// which is exactly the precondition check_coverage's forward
    /// pass is sound under.
    IntervalCoverage {
        target_key: KeyId,
        key_permutation: Box<[u16]>,
        disjoint: DisjointGuardProof,
    },
    Closed { members: [u64; 4] },   // PRD 04 retypes this field
}

/// Zero-size token. The ONLY constructor lives in the validator arm
/// that accepts an interval-position functionality (the pointwise-key
/// branch of validate_functionality) — its existence IS the proof.
pub(crate) struct DisjointGuardProof(());
```

- `check_coverage`'s signature gains `&DisjointGuardProof` (or takes
  the `IntervalCoverage` evidence whole). The prose preconditions at
  judgment.rs:604-609/649-653 are rewritten to cite the token.
- The `coverage: bool` flag and every `if coverage` dispatch die; the
  match on the two variants replaces them. Grep-zero on the field name.
- `verify_store`'s coverage pass documents that it re-derives the same
  fact offline (it is the auditor and may re-check disjointness from
  bytes — that stays).
- The token is deliberately NOT serialized, NOT fingerprinted (the
  fingerprint excludes enforcement data — fingerprint.rs:9-16 — and
  this PRD must keep it that way).

## Technical direction

1. Pin first: the existing coverage judgment tests (accept/gap cases)
   green before and after with unchanged values.
2. Introduce the token in the pointwise-key acceptance branch; thread
   it into the sealed `IntervalCoverage` variant at the exact line the
   `coverage: true` flag is set today (validate.rs:755-758 — the flag
   is set where the target projection carries an interval position AND
   resolved to a key; the key it resolved to passed the interval-FD
   gate, which is where the token is minted).
3. Split the enum; chase all consumers (commit plan construction,
   judgment dispatch, introspect/EXPLAIN rendering, verify_store).
4. Adversarial lock (in-scope, from audit test #10): a test that
   constructs an overlapping guard group THROUGH THE BACK DOOR (raw
   storage writes in a test harness, not the public API) and asserts
   the offline verifier flags it — pinning that the precondition is
   load-bearing and the auditor is its net.

## Passing criteria

- `[shape]` `grep -rn "coverage: bool\|coverage:bool" crates` → zero;
  `grep -rn "Enforcement::Probe" crates` → zero (both variants named).
- `[shape]` `DisjointGuardProof` has exactly one construction site,
  inside the pointwise-key acceptance arm (grep count = 1 outside the
  type definition).
- `[shape]` `check_coverage` is uncallable without the proof (its
  signature demands it; no test constructs the token directly).
- `[test]` Coverage accept/gap tests unchanged and green; the
  overlapping-guard verifier lock green; full engine suite green.
- `[gate]` Fingerprint pin byte-untouched; bounded fuzz smoke (ops)
  per policy 7; clippy; fmt.

## Doc amendments (rule 6)

`30-dependencies.md` § enforcement: the paragraph stating coverage
soundness rides a validator-minted proof, not an assumption.
`50-storage.md`'s coverage-sweep description cites the token. The
theorem↔evidence table's coverage row updates its evidence cell.
