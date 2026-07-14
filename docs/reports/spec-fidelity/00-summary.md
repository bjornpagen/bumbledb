# Spec-fidelity fanout — consolidated summary

Covenant PRD 15, executed 2026-07-14 against the campaign close
(`09b12a17`). Ten blind reviews, one per spec↔implementation pairing;
every class-(a) candidate adversarially re-verified by the
orchestrator with independent code reads before entering this summary.
**Reports only — no code, spec, or doc was changed by this PRD.**
Acting on any finding is the owner's decision.

## Grades

| # | Subsection | Rust surface | Grade | (a) | (b) | (c) |
|---|---|---|---|---|---|---|
| 1 | Values | value/interval/encoding | B | 0 | 6 | 0 |
| 2 | Dependencies (acceptance) | schema/validate | C | 2 | 3 | 1 |
| 3 | Dependencies (judgment) | commit/judgment | B | 1 | 1 | 0 |
| 4 | Query denotation | ir + validate + normalize | A− | 0 | 5 | 1 |
| 5 | Aggregates | sink/aggregate + sweep + allen | A | 0 | 3 | 3 |
| 6 | Exec/Sweep | interval/sweep + check_coverage | A− | 0 | 4 | 0 |
| 7 | Exec/Dedup | sink seen-sets + fj witnesses | A− | 0 | 3 | 1 |
| 8 | Exec/Rewrites | plan/ground + keyprobe + fold | B+ | 0 | 4 | 1 |
| 9 | Txn | api/db + commit pipeline | B | 1 | 5 | 0 |
| 10 | The naive model | bench/naive | A− | 0 | 5 | 0 |

Class (a) = behavior the spec forbids (bug). (b) = spec-undetermined
behavior. (c) = spec claims no code implements / spec mis-statement.
The three (a) citations across reports 2/3/9 deduplicate to TWO
distinct findings (the preemption finding was independently converged
on by three reviewers); one of report 2's (a)s reclassified (c) on
re-verification. Net: **one confirmed engine bug, one confirmed
spec-vs-both-implementations divergence, one confirmed spec error,
and 38 recorded underspecifications/notes.**

## Findings, ranked

### F1 — CONFIRMED (a): validation panics where the contract promises refusal
**The one true engine bug.** `schema/validate.rs:518` —
`AxiomIndex::try_from(word).expect("sealed axiom index fits u16")` on
the closed-source-under-closed-target scan. The `word` is the SOURCE
ground axiom's projected field value — an arbitrary schema-author
`u64` — not a sealed index. A closed relation whose row carries a
referencing value > `u16::MAX` under a containment into a closed
target panics `SchemaDescriptor::validate` instead of returning
`ClosedStatementRefuted`. This contradicts (i) `AxiomIndex`'s own doc
contract (`schema.rs:395`: "values beyond u16 are absent"), (ii) the
constitution's PRD-04 rule that arbitrary u64s narrow FALLIBLY, and
(iii) the spec's typed-rejection totality (`ClosedStatementRefuted`
is the modeled outcome). Severity: moderate (validation-time crash on
a mis-authored schema; no data-plane exposure). **Recommended
disposition:** one-line fix — `try_from(...).map_or(true→refuted)`
shape: out-of-range narrows to refuted, exactly the contract; plus a
reject-suite lock with a >65535 referencing value. (Also worth noting:
the theory fuzz target never caught this because theorygen draws
closed-reference values from the small handle range — a
generator-coverage note.)

### F2 — CONFIRMED (a-divergence): phase preemption vs `rejection_is_complete`
Triple-convergent (reports 3, 9, 10). The engine (`commit/apply.rs:
54-58`) and the naive model (`naive.rs:321-325`) both preempt: a final
state violating a key AND a containment rejects citing only the key
phase — "never a mix," deliberately, recorded in
`30-dependencies.md:67-75`. `Txn.lean`'s `rejection_is_complete`
(:279-297) proves completeness over ALL statements, and its narrowing
record does not carry the preemption. The spec sides with neither
implementation. No invalid state can commit (payload-only divergence).
The deep argument FOR the implementations: containment judgment is
DEFINED over keyed final states — the probe machinery presupposes key
uniqueness, so cross-phase "completeness" is arguably ill-defined when
keys are violated. **Recommended disposition:** narrow the spec —
restate `rejection_is_complete` as per-phase completeness (complete
within the failing phase) and record the preemption as a Txn.lean
narrowing with the ill-defined-composition argument; alternatively
redefine both implementations to judge all phases (a semantics
decision with probe-machinery consequences). Owner's call; the spec
narrowing is the cheaper and arguably more honest fix.

### F3 — CONFIRMED, reclassified (c): the spec mis-reads permuted interval positions
Report 2's D1, re-verified: `resolve_target_key` (validate.rs:744-752)
counts interval positions as a SET and resolves coverage regardless of
where the interval sits in the WRITTEN projection order —
`key_permutation` bridges statement order to key order, exactly the
constitution's FieldSet doctrine ("validation compares sets; execution
keeps statement order"). The Lean model's `intervalSplit`
(Schema.lean:207-212) instead reads the WRITTEN order and gives
interval-not-final shapes the classical reading — so for an accepted
statement written `B(span, id)`, spec and engine assign different
semantics. The engine's reading is the deliberate, doctrine-consistent
one; the spec mis-encodes it. **Recommended disposition:** fix
`intervalSplit` to canonicalize on the field SET (interval position
order-independent), re-prove the affected judgment lemmas, and add the
permuted-written-order case to the conformance corpus. Also fold in
report 2's D2 (the same function's gate-refused-shape claim is wrong
in the other direction — `[interval, interval-final]` splits `some`
where the note claims `none`): one spec function, two mis-statements,
one fix.

### F4 — the noteworthy (b): measure group keys (report 10, D5)
Both implementations group aggregate answers by the PROJECTED measure
VALUE (query.rs:882-884 and the engine's find columns); the spec's
`Group` fibers over `VarId`s (Aggregates.lean:1146-1148) — two
bindings with colliding measure values merge in the implementations
and split in the model. Corpus-thin (no conformance case pins it).
Underspecification with real semantic content. **Recommended
disposition:** decide the intended contract (value-keyed grouping is
almost certainly it — answers are value tuples), align `Group`'s
statement, and add a colliding-measure conformance case.

### F5–F41 — the recorded (b)/(c) tail
Full detail in the per-section reports; the classes: unmodeled Rust
surface (AllenMask value variant, SENTINEL_ID, decode/corruption
boundary, WriteResult's mechanism-level failure modes, fresh-mint
flush); spec generality unspent (fixedBytes 0, heterogeneous-key
disjointness acceptances outside `syntactic_disjointness_sound`,
pointwise key-probe uniqueness proved for scalar keys only —
report 8's F3); acceptance narrower than the model (the benign
EmptyRuleSet/self-comparison/caps family); ordering conventions
(AllenRel constructor order vs bit order; Pack tie-order transfers by
argument not theorem — reports 5/6); one latent type-blind sentinel
trim (fold.rs:184-192, unreachable today — report 8's F4, worth the
one-line type guard when convenient); and the membership-resolution
seam both implementations re-derive independently of the spec
(report 10's D3 — the one place a shared misreading could still hide;
a conformance-corpus arm pinning bivalent resolution would close it).

## The instrument's own verdict

The fanout confirms the campaign thesis: after 260 theorems and a
217-case executable-denotation agreement, the residual divergences
live exactly where the spec was narrowed (preemption, measure
grouping, membership lowering) or where Rust carries mechanism the
model deliberately excludes. One reachable panic survived every
previous instrument class — found only by reading the code against a
formal contract that named the absent-by-contract rule. Median grade
A−/B+; the implementation is faithful to its mathematics to a degree
none of the previous audit generations could certify, and every
remaining gap is now named, cited on both sides, and dispositioned.
