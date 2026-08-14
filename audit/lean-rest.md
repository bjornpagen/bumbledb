# Representation audit — rest of `lean/Bumbledb/` (wave-1 skip)

Brooks/Pike (SPOV 1–3, Insight 16). Scope: modules wave 1 did not
dump. No product-code fixes. Hunt: Program-shaped types, dual
denotations, flags reconstructing illegal states, validation that
throws away proofs, CQuery leftover, Idb/stratum/Program vocabulary.

Wave 1 already owns Syntax/Reach evalQuery, Denotation Theorem 9,
Dedup/Rewrites `Query.plain` wrapper, oddRec, dual decoders. This
pass does **not** re-file H1–L4 / lean-019 / lean-020. Sites that
only inhabit those findings are listed under Not filed.

Not filed (this-cut scope / essential / already owned):
mutual rec; Level 0 `evalQuery` vs Level 1 `evalQueryList`; `Safe` /
`WellTyped` as named premises; recursor name collision; private
`fueledLoop`; Plan staying over `Rule` (recorded in `Exec/Plan.lean`);
C5 Fin/Vector; CQuery/`CFind`/`plainQuery`/`Conformance.evalQuery`
(lean-008 — lives in `Conformance.lean`, **not** in Aggregates);
Dedup/Rewrites `Query` wrapper + `rulesAnswers ∘ edbEnv` (lean-005);
`oddRec` / `odd_not_stratified` (lean-002/015); Bridge
`translate/program.rs` (lean-019); "rec SCC" / "No strata" comments
(lean-020); `recDom` `idb` (lean-011); Schema.Atom staying `RelId`-only
(essential: statements cannot name interiors, Syntax.lean:100-103);
Sweep's unspent `Disjoint` (recorded narrowing); Txn / Admission /
Oracle / Capacity / Subsumption / Dependencies / Values / Decide /
Fresh / DeltaRestriction (no Program-shaped leftover).

**Aggregates hunt (PRD 04/05):** `Query/Aggregates.lean` is **not** a
second query IR. `AggOp` / `KeyTerm` are the recorded head-shape row
(Count has no `over`; Insight 4 already applied). `CFind`/`CQuery`/
`CQuery.dnf : Bool` live in `Conformance.lean` (lean-008). The rest
finding on heads is Dedup's `HeadSlot` (M2), a fourth encoding of
that same row.

**C5 splits:** none. No finding below asks for a Fin-telescope or
`Vector` rewrite. Dual coordinates still die.

---

## HIGH

### H1. `AtomSource` is a sum; Membership and key-probe collapse it to `RelId ⟨0⟩`

- **Where:** `Query/Membership.lean:333` (`SurfaceMatches` and ~40
  clones: `match a.source with | .edb R => R | .interior _ => ⟨0⟩`);
  `Typing.membership` / `Header.fieldType` are `RelId`-indexed
  (`176-197`, `120-121`); `Atom.lowerNegated` `1477-1484`
  (`AntiOccurrence.relation : RelId`, interior arm `{ relation := ⟨0⟩,
  domain := a.bindings, filters := [] }`); `Exec/Rewrites.lean:1286-1288`
  (`KeyProbeShape.declared` builds `Statement.functionality ⟨0⟩ K` for
  an interior atom); `keyProbeEval` `1302-1304` reads
  `factsOf W InteriorTables.empty`.
- **What's wrong:** The cut's atom is `AtomSource = edb RelId | interior
  InteriorId`. Membership typing never learned the second constructor:
  every interior atom is silently identified with stored relation 0,
  so an interval field on relation 0 makes interior membership fire
  against the **wrong header**, and a missing interval on relation 0
  makes it never fire. The negated lowering is a *different* lie —
  interiors become membership-free (filters emptied) rather than
  typed against ⟨0⟩. Key-probe acceptance is a third: an interior
  singleton-atom rule is "declared" iff relation 0 happens to carry
  key `K`, then evaluated against empty interior tables. Three
  independent reconstructions of one illegal identification.
  `Typing.membership` returns `Bool` and every lemma re-tests it
  (King: validate, discard, re-check). Parse-don't-validate inverted
  at the source coordinate the cut introduced.
- **What representation collapses it:** Membership and anti-probe
  keyed by `AtomSource`. Interior field types come from the derived
  head (after lean-002/006: `finds.length` / the body's bindings),
  never from `Header.sig ⟨0⟩`. `AntiOccurrence.source : AtomSource`.
  `KeyProbeShape.declared` is `a.source = .edb R ∧ functionality R K ∈
  T.statements` — interior key-probe unrepresentable (stored-relation
  probe, matching Syntax.lean:100-103). `keyProbeEval` takes the same
  `F` as `ruleAnswers`. The `⟨0⟩` arms, the Bool-retested membership
  screen, and `InteriorTables.empty` as a mode bit all die.
- **Essential vs accidental:** Accidental leftover of Program/EDB-only
  atoms. Bivalent membership on stored interval fields is essential;
  identifying an interior table with relation 0 is not. Key-probe on
  a stored key is essential; a well-typed interior key-probe is not.
- **Severity:** high
- **C5:** no split. Dual coordinate dies; identities stay dense `Nat`.

---

## MED

### M1. Plan denotes `edbEnv` — the per-rule theorem cannot be spent on interiors

- **Where:** `Exec/Plan.lean:300-301` (`Consistent` reads
  `edbEnv I a.source`); `runPlan` / `nodeStep` / `planBindings` /
  `planAnswers` take `I : Instance` only; `valid_plan_sound`
  `498-501` equals `ruleAnswers C r (edbEnv I)`.
- **What's wrong:** Plan staying over `Rule` is recorded essential.
  The environment is not. `ruleAnswers` is parameterized by
  `F : AtomSource → Set Fact`; Plan hardcodes the EDB-only instance
  of that parameter. A valid plan of an interior-atom rule is
  "sound" against unread interiors (empty), which is not
  `evalQuery`'s meaning of that rule. Dual denotation at the plan
  layer: the Free Join theorem inhabits the pre-cut Program
  coordinate. Cousin of lean-005 (that issue is the `Query` wrapper;
  this is the missing `F` on a theorem that already takes `Rule`).
- **What representation collapses it:** `Consistent` / `runPlan` /
  `planAnswers` take `F` (or `I` plus `InteriorEnv`), and
  `valid_plan_sound` is `planAnswers F = ruleAnswers F`. Reach spends
  it against `sourceDen`. `edbEnv` remains the EDB instantiation,
  not the definition. Plan still stays over `Rule`.
- **Essential vs accidental:** Accidental. Per-rule plan soundness is
  essential; baking `edbEnv` into the definition is leftover
  Program/EDB.
- **Severity:** med
- **C5:** no split.

### M2. `HeadSlot` is a fourth encoding of the head-shape row

- **Where:** `Exec/Dedup.lean:1428-1440` (`HeadSlot = key KeyTerm |
  fold VarId | foldMeasure VarId | count`); `Query/Aggregates.lean:1427-1430`
  (`KeyTerm`) and `2049-2056` (`AggOp`); `Conformance.lean:166-169`
  (`CFind = var | measure | agg AggOp`) — the last is lean-008.
- **What's wrong:** One head-shape row, four inductives. `AggOp` is
  the recorded op inventory (Count carries no `over` — good).
  `CFind` is the conformance wrapper lean-008 keeps as a thin layer
  around `Query`. `KeyTerm` duplicates `CFind`'s key faces.
  `HeadSlot` independently re-encodes the union-key *quotient* of
  that row (ops collapse to "fold this var") and can be inhabited
  without any `AggOp`/`CFind` existing — `HeadSlot.fold v` with no
  corresponding `Sum`/`Min`/`Max`/`Pack` is writable. Dual
  coordinate: the union-key law and the op inventory can disagree.
  Aggregates itself does **not** host `CQuery`.
- **What representation collapses it:** One head type (`CFind` after
  lean-008, or `AggOp` plus `KeyTerm` as *views*). `HeadSlot.of :
  CFind → HeadSlot` (or of `AggOp`) is a function, not a constructor
  family. Delete the independent `HeadSlot` inductive.
- **Essential vs accidental:** Accidental. The union key omitting
  Count's constant column is essential *as a reading*; storing it as
  a parallel datatype is leftover Program/head-projection plumbing.
- **Severity:** med
- **C5:** no split. Overlaps lean-008 (`CFind`); not a duplicate —
  HeadSlot is Dedup-local.

---

## LOW

### L1. Rest-of-tree comments still say "program"

- **Where:** `Exec/Dedup.lean` (module doc and theorem prose: "every
  rule of a program", "2+-rule program", "program-wide",
  "program-level", "HAND-WRITTEN multi-rule programs");
  `Exec/Rewrites.lean:58,107,1601,2299,2351-2355,2425,2454`
  ("prepared program", "rewrite step on a program", "program-level
  face", "lifted to the program"); `Query/Denotation.lean:933-935`
  ("set semantics at the program level");
  `Query/Aggregates.lean:1542` ("single-rule programs");
  `Bridge.lean:287,460` (obligation prose — not the `program.rs` path
  token, which is lean-019).
- **What's wrong:** C7: the spec-of-record teaches the deleted
  Program coordinate. Same defect as lean-020 (SCC/Tarjan/strata)
  for the other retired noun. Engine *path* tokens that still
  contain `ground_program` / `the_empty_program_*` are C8 — they
  move with engine-034, they are not this finding.
- **What representation collapses it:** Present-tense vocabulary:
  "query", "rule list", "prepared pipeline". Comment-only; no
  identifier changes (identifier-level Program is lean-005's
  `RewriteStep` over `Query`).
- **Essential vs accidental:** Accidental naming leftover.
- **Severity:** low
- **C5:** no split. Land after lean-005/001 touch Dedup/Rewrites so
  the comments rewrite against the restated theorems.

---

## Counts

| Severity | Count |
|----------|------:|
| high     |     1 |
| med      |     2 |
| low      |     1 |
| **total**|   **4** |

The rest of the tree learned `AtomSource` in Syntax and then kept
every consumer on the Program/EDB coordinate: membership and
key-probe reconstruct relation 0, Plan denotes `edbEnv`, Dedup
minted a fourth head inductive, and the comments still say
"program". Aggregates, Txn, schema, admission, and the write-side
modules are clean of this leftover. Dual coordinates die; no
finding asks C5 to reverse R-DENSE.

## Adversarial validation (2026-08-14)

H1/M1/M2/L1 (lean-021..024) survive as OPEN. lean-021's Fix was
rewritten: do **not** key `Header` by `AtomSource` or invent derived-head
membership — `Syntax.lean:48-49` records interior membership as
engine-only; `scalarAnchored` already takes `interior _ => false`. Kill
`⟨0⟩`; interiors stay value-equality in Lean; key-probe interior stays
unrepresentable. lean-023's Fix was rewritten: unify `HeadSlot` onto
Aggregates (`AggOp`/`KeyTerm`), not `Conformance.CFind` (Dedup must not
import the driver). No lean-025: Plan/Membership/HeadSlot/comments were
the remainder coordinates; no additional representation defect in
Txn/Admission/Oracle/Capacity/Subsumption/Values/Decide/Fresh.
