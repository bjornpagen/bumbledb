# PRD 04 — Compiled subsets: statements over closed relations

**Depends on:** 01 (sealed extensions), 03 (virtual reads for the sweeper's
sake); independent of 02.
**Modules:** `schema/validate.rs` (`Resolved`), `storage/commit/` (plan.rs,
judgment.rs, applier.rs), `error.rs`.
**Authority:** `30-dependencies.md` (acceptance gate, enforcement summary),
the staging law.
**Representation move:** a containment into a closed relation is
stage-1-known on its target side, so its enforcement plan is not a probe
strategy — it is **the answer set itself**, compiled to a 4×u64 bitset at
open. The acceptance gate's O(log n) ceiling drops to O(1) for the entire
statement class, and the ψ-selected form gives sub-vocabularies
("`Escalation(severity) <= Severity(id | pages == true)`") the same O(1)
plan.

## Context (decided shape)

- **New `Resolved` arm**: `Resolved::ClosedContainment { members: [u64; 4] }`
  — a 256-bit bitset over row ids (the ≤256 roster cap from PRD 01 exists
  exactly to make this fixed-size). Computed at validate for every
  containment whose TARGET relation is closed: start from all rows, apply ψ
  (the target selection) against the sealed extension's pre-encoded values,
  set the surviving ids' bits.
- **Source-side judgment** (insert of a referencing fact): membership =
  `members[id >> 6] & (1 << (id & 63)) != 0` — replaces the guard probe
  entirely for this class. No `R` reverse edges are written for closed
  targets (the target side can never shrink — axioms don't delete), so the
  target-side judgment for these statements is **vacuous by construction**
  and the plan emits nothing for it.
- **Out-of-range ids** (a u64 field value ≥ extension len): membership is
  simply false → the same `ContainmentViolation` as any dangling reference;
  no special error.
- **Coverage FROM a closed source** (`Severity(id) <= Handler(severity)` —
  domain quantification): source side is constant, so source-side judgment
  never fires on commits (no closed inserts exist); target-side fires only
  when `Handler` deletes — the existing `dependents` machinery already
  delta-restricts this; verify and pin with a test rather than build
  anything. The re-establishment check compares against the compiled member
  bitset where applicable.
- **Interval positions on closed containments**: refused at validate v0
  (closed extensions may carry intervals as payload, but a pointwise
  containment *into* a closed target mixes the coverage walk with virtual
  storage — no census sighting; typed roster error, trigger recorded).
- **`==` with a closed side**: left-to-right into a closed target compiles
  as above; a closed relation as the `==`'s *source* ("every kind has an
  arm") is two statements as always — the closed-source direction is the
  domain-quantification case.

## Technical direction

1. `validate.rs`: in `resolve_target_key`, branch when the target relation
   is closed — no target-key FD search, no `key_permutation`; evaluate ψ
   against the sealed rows (reuse `satisfies`-equivalent comparison on the
   pre-encoded values; the encodings are already canonical so this is
   byte-compare per selected field) and emit the bitset arm. The
   `MAX_GUARD_WIDTH` check does not apply (no guard exists).
2. `commit/plan.rs`: edge derivation for a source fact under a
   closed-target containment emits a `CheckMembership { statement, id }`
   edge instead of guard bytes (the id is the referencing field's decoded
   word — it is already in hand during encode); skip reverse-edge emission.
3. `commit/judgment.rs`: `check_source` gains the bitset arm (one AND, one
   test, error path identical to the probe-miss path); `check_target`
   asserts (debug) that no dependent entry names a closed-target statement.
4. `verify_store`: the F↔R walk skips closed-target statements; add the
   assertion that no `R` entry names one.
5. Tests: fixture theory with (a) a plain closed reference, (b) a
   ψ-selected sub-vocabulary, (c) a domain-quantification coverage; exercise
   insert-accept, insert-violate (wrong subset member and out-of-range id),
   and the Handler-delete re-judgment for (c).

## Passing criteria

- `[test]` Sub-vocabulary: inserting a fact whose field holds a row inside
  ψ commits; outside ψ (and out-of-range) aborts with
  `ContainmentViolation` naming the statement — verdict-identical to the
  naive model once PRD 06 lands (record the pending cross-check in the test
  comment; the unit test here asserts the engine side).
- `[test]` Domain quantification: deleting the last `Handler` row for a
  covered severity aborts; deleting a non-last one commits.
- `[test]` The bitset is computed at validate (construct the schema, read
  `Resolved` directly, assert bits without any Db).
- `[shape]` No `R` namespace traffic for closed-target statements (grep the
  plan emission + the verify_store assertion); no guard-width check runs for
  them.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`30-dependencies.md`: the compiled-subset enforcement plan joins the
enforcement summary (O(1) row); the interval refusal with trigger; the
domain-quantification worked example.
