# Spec-fidelity review 02 — the acceptance half (covenant PRD 15, pairing #2)

Scope: `lean/Bumbledb/Schema.lean` + `Dependencies.lean` acceptance surface
(`TargetKeyAccepted`, the exact-field-set rule, selections, the pointwise gate
shape, the `no_closure` note) against
`crates/bumbledb/src/schema/validate.rs` and `schema.rs`. Lean is normative.
Zero code changes made.

## Per-definition / per-theorem fidelity table

| Lean item (file:line) | Rust discharge (file:line) | Verdict |
|---|---|---|
| `Selection`, equality-only by representation (Schema.lean:139-147) | `Side::selection: Box<[(FieldId, Value)]>` (schema.rs:184-193); no other predicate representable; literals typed via shared `value_matches` (validate.rs:691-710, schema.rs:144-179) | FAITHFUL. Rust additionally refuses duplicate bindings (validate.rs:645-651) and selection-on-projected-field (validate.rs:666-671) — acceptance strictly narrower, sound direction; see D5 |
| `sameFields` — set identity of projections (Schema.lean:129-130) | `FieldSet` — sorted, duplicate-refused canonical form (validate.rs:199-222); duplicates pre-rejected by `validate_projection` (validate.rs:616-621) | FAITHFUL. Sorted-unique equality ≡ extensional membership equality on duplicate-free lists; duplicate-free-ness is the recorded narrowing (Schema.lean:51-55) |
| `TargetKeyAccepted` — ∃ declared FD with set-equal projection (Dependencies.lean:135-137) | `resolve_target_key` (validate.rs:718-833): first set-equal `Functionality` on the target relation; existence≡first-match because `DuplicateFunctionality` rejects a second FD over one set (validate.rs:352-365) | FAITHFUL for ordinary targets, including permuted projections (`key_permutation`, validate.rs:787-797, licensed by `functionality_respects_field_set`, Dependencies.lean:488-494). Closed targets diverge — D3 |
| Acceptance/denotation split — target key a hypothesis, never a conjunct of `Containment` (Dependencies.lean:114-137) | `resolve_target_key` (validate.rs) structurally separate from `judgment.rs::Checker`; `Enforcement` carries the resolved key (schema.rs:362-394) | FAITHFUL; matches Bridge.lean:139-147 |
| Exact-field-set rule + `no_closure` note (Dependencies.lean:24-36, 476-483) | Exact set equality only (validate.rs:754-770); no closure computed; `SchemaWarning::RedundantSuperkey` diagnostics-only (validate.rs:227-249, schema.rs:579-589), warnings absent from `canonical_bytes` fingerprint inputs (fingerprint.rs:39-53) and never consulted by enforcement | FAITHFUL — entailment provably unspent, exactly as modeled |
| Pointwise FD gate — ≤1 interval, final (Schema.lean:47-50, 203-213 vs validate.rs:328-345) | `FunctionalityMultipleIntervals` (validate.rs:330), `FunctionalityIntervalNotLast` (validate.rs:340) | FAITHFUL for FDs. NOT enforced on containment projections — D1. `intervalSplit` doc-note inaccurate — D2 |
| `DisjointDeterminantProof` minting (Bridge.lean:164-167, 174-177) | Minted only by the accepted pointwise FD arm (validate.rs:427-430); re-derived for the resolved key and sealed into `Enforcement::IntervalCoverage` (validate.rs:811-826); zero-sized witness, consumed by signature (schema.rs:347-360; judgment.rs:671) | FAITHFUL — no boolean can license the sweep |
| `den_closed_constant` — closed-to-closed decided at validate (Schema.lean:283-295; Bridge.lean:134-137) | Sealed extension encoded once (validate.rs:1014-1096); closed-source scan against compiled member set (validate.rs:507-527); closed FD refuted at validate by byte-level scalar/half-open collision (validate.rs:393-425) | FAITHFUL in structure; panic path — D4 |
| `accepted_target_key_spent` premises (Dependencies.lean:515-527; Bridge.lean:144-147) | `hacc` ← `resolve_target_key`; `hI : holds` ← `judgment.rs::judge` + `Db::verify_store`; `hscalar` ← the `ScalarProbe` arm | FAITHFUL — premises and discharge sites match the Bridge row |
| `keyed_eq_unique_correspondence` premises (Dependencies.lean:307-329; Bridge.lean:154-157) | Both lowered containments independently pass `resolve_target_key`; `mirror_of` seals pairing (validate.rs:171-189); σ-subset keys via `functionality_selected` (Dependencies.lean:501-504) | FAITHFUL |

## Divergences

**D1 — class (a) BUG. No interval-finality gate on containment projections.**
`resolve_target_key` rejects only >1 interval position
(validate.rs:744-752) and never requires the single interval position be
final in the containment's *written* projection; a hand-built descriptor
(`SchemaDescriptor` is a public `Theory`, schema.rs:288-292) with sides
written `[interval, scalar]` over a set-equal pointwise key seals as
`Enforcement::IntervalCoverage` and is enforced as coverage in permuted
determinant order (judgment.rs:584 "already in target determinant order").
Normative `Statement.judgment` reads the written order:
`Header.intervalSplit` on `[interval, scalar]` yields `none`
(Schema.lean:207-212), so both sides fall to the classical `Containment`
arm (Dependencies.lean:226-233). A tiled target (`[0,5)`,`[5,10)` covering
source `[0,10)`) commits under Rust yet refutes the modeled `holds` —
committed states violating the normative judgment, the covenant's worst
divergence class. Fix direction: gate finality on containment sides at
acceptance, or respecify `Statement.judgment` order-free.

**D2 — class (c) SPEC ERROR. `intervalSplit`'s "every other shape splits to
`none`" claim is false.** Schema.lean:203-206 (and the narrowing note,
Dependencies.lean:55-59) state gate-refused shapes split to `none`/scalar,
but `intervalSplit` inspects only the last element: `[interval, interval]`
splits to `some ([interval], interval)` — a pointwise reading with an
interval inside the "scalar" prefix. Unconsumed today (Rust refuses the
shape: validate.rs:330, validate.rs:749-751), but the recorded narrowing
misdescribes the model's own total function.

**D3 — class (b) UNDERSPECIFICATION. Closed-target key resolution is
stricter than `TargetKeyAccepted`.** Rust demands the target projection be
exactly the synthetic `[FieldId(0)]` (validate.rs:734-742); a user-declared
non-id key on a closed relation is a sealable `Functionality`
(validate_functionality refuses nothing closed-specific,
validate.rs:314-431), so `TargetKeyAccepted` (Dependencies.lean:135-137)
holds for a containment Rust refuses. Sound direction (Rust ⊂ Lean), but
the closed-branch rule is unrecorded in the Lean narrowing notes.

**D4 — class (a) BUG. Panic path in the closed-source scan.**
`AxiomIndex::try_from(word).expect("sealed axiom index fits u16")`
(validate.rs:518) panics on a closed source whose projected u64 column
carries a value > `u16::MAX` — reachable via the public
`SchemaDescriptor::validate` with an out-of-range reference value. The
declared contract is the opposite: "values beyond `u16` are absent"
(schema.rs:396-398), and the commit path treats the miss as absent
(judgment.rs:217-218 "an out-of-range word is simply a miss"). The modeled
outcome is refutation (`ClosedStatementRefuted`, per the
`den_closed_constant` bridge, Schema.lean:283-289), not a panic.

**D5 — class (b) UNDERSPECIFICATION (borderline). σ shape refusals
unmodeled.** `SelectedFieldProjected` (validate.rs:666-671) and
`DuplicateSelectionField` (validate.rs:645-651) narrow the accepted σ
fragment below Lean's `Selection` (which happily models both,
Schema.lean:139-147). Sound direction; arguably covered by the blanket
"remaining shape checks" narrowing (Schema.lean:51-55), whose parenthetical
list names neither.

## Adversarial probes that came back clean

- **Duplicate field sets / permutations**: `DuplicateFunctionality` is a set
  rule (validate.rs:352-365), matching `functionality_respects_field_set`;
  permuted target projections resolve with a correct `key_permutation`;
  `normalize` sorts σ by field so statement identity matches σ's set
  semantics (validate.rs:293-310).
- **Superkey edges**: exact-set resolution spends no entailment; warning is
  outside fingerprint and enforcement — `no_closure` honored precisely.
- **Closed-side edges**: both-closed decided at validate
  (scan + `MemberSet`, id = row index, exactly `Containment` against the
  sealed extension); closed source under ordinary target correctly stays
  commit-judged (target can shrink); `ClosedContainmentInterval`
  (validate.rs:485-498) is a Rust-side v0 refusal, sound but Lean-silent
  (folded into D3/D5's class).
- **Closed FD collision**: half-open byte intersection
  `a[..8] < b[8..] && b[..8] < a[8..]` (validate.rs:405-414) is exactly
  `PointwiseKey` under the order-embedding encodings; scalar collision via
  canonical-byte equality is exactly `Functionality` via
  `value_eq_iff_encode_eq`.

## GRADE: C

The mainline acceptance machinery is a close, sometimes elegant, image of
the model — the exact-field-set rule, the acceptance/denotation split, the
unspent superkey entailment, and the proof-carrying pointwise plan all
implement their modeled predicates precisely, with premises discharged at
exactly the Bridge-claimed sites. But an adversarial reading finds two
class-(a) defects: D1 admits accepted theories whose committed states
refute the normative `holds` (the covenant's core promise), and D4 is a
reachable panic where the model and the code's own contract both demand a
typed refutation. Add one spec error in the model's recorded narrowing (D2)
and two unrecorded acceptance narrowings (D3, D5), and the pairing sits
well below the nothing-under-adversarial-reading bar, while the breadth of
faithful mainline structure keeps it clear of D territory.
