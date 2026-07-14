# Spec-fidelity report 08 — `Exec/Rewrites.lean` vs the prepare-time rewrites

Reviewer: pairing #8 (covenant PRD 15). Normative side:
`lean/Bumbledb/Exec/Rewrites.lean`. Rust surface: `crates/bumbledb/src/plan/ground.rs`,
`plan/ground/evaluate.rs`, `exec/dispatch/classify.rs`, `ir/normalize/fold.rs`,
`api/prepared/bind.rs`, `api/prepared/build.rs`, `ir/normalize/normalize.rs`.

## Per-theorem fidelity table

| Lean item | Rust site | Verdict |
|---|---|---|
| `AgreesWithAxioms` / `den_agrees` (Rewrites.lean:116-129) | sealed extensions, `Relation::extension()` (evaluate.rs:135-137, 576-590) | FAITHFUL — closed extensions are validate-sealed, instance-invariant. |
| `groundAtom` / `groundAtom_join_step` (:154-174) | `surviving_ids` (evaluate.rs:576-590) | FAITHFUL under the recorded generalization: the Lean fold is the full substitution; Rust's σ is its prepare-time constant half. |
| `Atom.foldableB` (:193-198) | `parse_resolvable` (evaluate.rs:379-485) | FAITHFUL as the modeled acceptance; the Lean side records that preservation never spends it. Rust's param/measure/`PendingIntern` refusals are strictly narrower — conservative. |
| `litSatB` (:203-207) | `surviving_ids`' row filter (evaluate.rs:582-587) + `row_satisfies` (:598-661) | FAITHFUL with a factoring difference: Rust's σ also evaluates range/Allen/membership filters at prepare; the model leaves those as rule conditions filtering the disjuncts at execution. Same denotation (conditions kill the extra survivors), different factoring — subsumed by the full-substitution narrowing. |
| `groundCondition` / `groundCondition_holds` (:228-273) | `attach_membership` (evaluate.rs:683-695), `Const::WordSet` | FAITHFUL under the narrowing: the `WordSet` is the one-live-variable projection; equivalence additionally leans on the id position being a whole key (row id = declaration index, evaluate.rs:568-573) and payload deadness (`payload_escapes`, :282-293) — both plan-shape conditions the narrowing names. |
| `groundSplit` first-foldable scan (:287-353) | `fold_step` (evaluate.rs:109-125) | FAITHFUL up to atom-list order: Rust may skip a `foldableB`-true atom that `payload_escapes` refuses and fold a later one; since `ruleAnswers` is atom-order-invariant, the Rust step is a `groundRewrite` step on the reordered list. No divergence. |
| `groundRewrite` / `grounding_preserves_answers` / `ground_refuted_empty` (:362-459) | `fold_positive` (evaluate.rs:128-199), rule-death channel (:178-190), `Program::Empty` | FAITHFUL. The `|S| ≥ 1` no-live-`k` gate-deletion arm (module doc, evaluate.rs:52-58) is the "vacuous fold / always-true condition" arm the Lean doc subsumes (:184-186); Rust's zero-binding requirement is the aggregate face (PRD 05), recorded. |
| `ElimStep.atoms_split/finds_eq/negated_eq/conditions_eq` (:483-487) | `Role::Eliminated` mark (ground.rs:144) — occurrence ids never move, nothing else touched | FAITHFUL. |
| `ElimStep.source` (:489) | `removable` source scan (ground.rs:177-183) | **DIVERGENCE F1 (class b)** — see below. |
| `ElimStep.join_covers` (:492-493) | `join_covers_full_key`, covering half (ground.rs:216-220) | FAITHFUL; Rust adds `shared_vars_pair_positions_only` (:221-227), an extra conjunct absent from `ElimStep` — strictly narrower acceptance, conservative. Full-key-ness of Y is acceptance-side and spent by the aggregate face — recorded (:77-84). |
| `ElimStep.carries_phi` (:497) | `source_carries_phi` (ground.rs:260-269) via `encoded_selection`/`lower_literal` (:563-568) | FAITHFUL — (field, encoded literal) set containment, both sides through one `lower_literal`; params/ranges fail equality, never inferred. |
| `ElimStep.target_bindings` (:501-503) | `selections_within_psi` (ground.rs:252-259) | FAITHFUL — only `Compare{Eq}` within ψ passes; every other filter shape (`FieldsCompare`, `PointIn`, `WordSet`, params) hits the `_ => false` arm. |
| `ElimStep.var_functional` (:507-508) | `lower_atom` pass 1 first-binding discipline (normalize.rs, `vars` dedupe) + pass 2 `FieldsCompare` lowering, refused by condition 2 | FAITHFUL — `Occurrence::vars` carries each variable at exactly one field by construction; the residual positions surface as `FieldsCompare` filters that `selections_within_psi` refuses. The premise's anchor is exactly as the module doc records (:85-90). |
| `ElimStep.join_or_dead` (:513-516) | `variables_join_or_dead` (ground.rs:281-299) + `var_is_dead` (:315-373) | FAITHFUL — Rust deadness (outputs, four residual kinds, anti-probe bindings, other non-discharged occurrences' vars and membership points) matches `v ∉ r'.allVars` over the surviving rule; discharged occurrences are correctly absent from `r'`. |
| condition 4, scalar-split premises (`RewriteStep.eliminate` hsrc/htgt, :1068-1069) | `Enforcement::ScalarProbe` gate (ground.rs:167-169) | FAITHFUL — `ScalarProbe` is minted only for scalar-position pairs (schema.rs:366-371; interval pairs mint `IntervalCoverage`), matching both `intervalSplit = none` premises. Rust additionally refuses `Closed`-target containments — narrower, conservative. |
| `elimination_sound` (:582-675) | the rewrite as a whole; differential `with_grounding_disabled` (ground.rs:106-116) | FAITHFUL as the projection-sink face; the aggregate face is PRD 05's, recorded (:77-84). |
| `Term.pinned`/`pinValue`/`pinAt` (:692-737) | `value_of` (classify.rs:77-87) | FAITHFUL — first Eq constant per field, literals and params both pinned; `KeyProbePlan` resolves per probe (a `PendingIntern` key constant resolves via `dict::lookup(..).unwrap_or(SENTINEL_ID)`, key_probe_fact.rs:45 — a miss probes the never-minted sentinel and misses, matching `keyProbeEval`'s empty find). |
| `KeyProbeShape` (:801-815) | `classify` (classify.rs:24-130), `key_probe_candidate` (:134-161) | PARTIAL — the single-occurrence/no-negated/no-residual/key-covered clauses match exactly (a lone occurrence is positive by the positives-first ordering, classify.rs:28-32; all four residual kinds checked, :33-39). **Divergences F2, F3** below. The full-fact `M` fallback (:153-160), closed refusal (:94), and measure/set-filter refusals (:44-75) are recorded narrowings — confirmed accurate. |
| `keyProbeEval` / `keyprobe_equiv_join` (:821-948) | `PreparedRule::KeyProbe` minted at build.rs:335; kernel `key_probe_fact.rs` | FAITHFUL on the modeled shape; `probeHitB`'s one-get-uniqueness argument matches the `U` determinant get. |
| `keyprobe_key_spent` (:955-964) | key statements accepted by the PRD 03 gate | PARTIAL — discharges scalar keys only; **F3**. |
| `StaticallyEmpty` / `statically_empty_sound` (:975-989) | fold.rs rules (a)-(f) (:151-213), `NormalizedQuery::dead`, `Program::Empty` | FAITHFUL in verdict soundness — every rule judged on constants only (params/`Ne` never fold, fold.rs:28-35, 233-235, 397-411), so every verdict is ∀ρ∀σ. **F5** (minor) on the shape claim. Completeness correctly not claimed. |
| negated handling in fold.rs (:83-95) | `participates()` skip | FAITHFUL to the narrowing "a negated occurrence's contradiction is NOT emptiness" — filters left untouched, no verdict, rule not killed. Adversarially confirmed sound: a contradictory anti-probe rejects nothing. |
| `EmptyAt.selectionMiss` / `.refuted` (:1001-1034) | `resolve_filters` `Ok(false)` (bind.rs:285-340), `resolve_selection_into` (:348-404), `resolve_filter_into` (:418-490) vs `Program::Empty` | FAITHFUL — the two verdicts are structurally distinct code paths. `Ok(false)` fires only for Eq-miss/empty-set on POSITIVE occurrences (bind.rs:441, 453, 462 all check `!negated`); a negated miss resolves to the sentinel and matches nothing (:442-445) — exactly the constructor's "sound on positive occurrences only" note. The latch rewrites the template on HIT only (:433-440; selection at :359-365); a miss short-circuits this execution and keeps the template pending — per-execution, never a plan verdict. Discharged occurrences resolve nothing and never count toward the latch (:294-304). |
| `RewriteStep` / `rewrite_composition` (:1050-1181) | the `ground` fixpoint loop (ground.rs:129-156) | PARTIAL — grounding/kill/eliminate steps compose as modeled; the negated-complement fold has NO constructor (recorded narrowing, :72-76) and chained-source eliminations fall outside `RewriteStep.eliminate` (**F1**). |

## The negated-complement fold, judged adversarially (unmodeled by record)

`fold_negated` (evaluate.rs:203-273) was read arm by arm against its
doc-side soundness argument (evaluate.rs:66-90):

- **`|S| = 0` deletion** (:213-224): sound unconditionally — no sealed row
  satisfies the σ'd filters, so the anti-probe rejects nothing on any
  store; any binding shape qualifies, as the comment argues.
- **Zero-binding gate death** (:225-233): sound — `parse_resolvable`
  already guaranteed constants-only, so "some sealed row satisfies" is
  instance-independent.
- **Keyed complement** (:234-272): the `k ∉ S ⟺ k ∈ complement` rewrite is
  gated on `domain_within_ids` (:518-537) exactly as the doc pins; both
  witnesses check out (an id-position binder of the same closed relation
  yields a row id by construction; the containment witness carries φ
  literally, condition-2 discipline, :543-566). The refusal direction
  (no witness → anti-probe stays) is correct.
- **Interaction with later rewrites** (the adversarial target): could a
  later elimination remove the domain witness or a membership home? No —
  every domain witness binds `k`, is therefore a membership binder, and
  receives the complement `WordSet` filter (:683-695), which fails
  `selections_within_psi` (ground.rs:252-259), blocking its elimination;
  a later positive fold of a binder re-applies the `WordSet` in its own σ
  (`parse_resolvable` accepts `WordSetEq`, evaluate.rs:392-395), so the
  constraint transfers. Independently, per-step soundness is judged on
  the current (already-rewritten) rule, so no cross-step invariant is
  actually needed. **No class-(a) found.** The rewrite remains outside
  `rewrite_composition`'s licence — recorded (:72-76) — and its only
  empirical arm is the rewrites fuzz differential.

## Divergences

- **F1 (class b)** — *Chained eliminations use discharged sources.*
  `removable` excludes only `Role::Negated` sources (ground.rs:178), so an
  `Eliminated` (or, theoretically, `Folded`) occurrence may serve as the
  pairing source — explicitly licensed by the Rust module doc's
  support-forest induction (ground.rs:66-81, `chain_reaches` :551-559).
  `ElimStep.source` requires `a ∈ r'.atoms` (Rewrites.lean:488-489), so a
  discharged-source elimination is an instance of NO modeled
  `RewriteStep`, and reordering does not repair it (the earlier state has
  more live readers, so `var_is_dead` can fail there). The induction
  argument lives only in the Rust doc; the Lean narrowings do not record
  it. (The folded-source case is practically unreachable: a folded
  source's only shareable variable is its id var, whose binders all carry
  the attached `WordSet`, which condition 2 refuses on the target.)
- **F2 (class b)** — *KeyProbe residual filters are unmodeled.* `classify`
  accepts non-key, non-Eq per-field filters — order compares, `Ne`,
  `FieldsCompare`, resolved `PointIn`/`FieldWithin`/Allen — into
  `remaining_filters` (classify.rs:127, 165-188), applied post-get
  (key_probe_fact.rs:253). `KeyProbeShape` demands `conditions = []` and
  `keyProbeEval` checks bindings only (Rewrites.lean:807-808, 821-828).
  The narrowing (:91-97) records the `M` path and the set/measure
  refusals but not this accepted surface. Sound-looking (post-get
  filtering only shrinks the one hit) but unproved and unrecorded.
- **F3 (class b)** — *Pointwise keys are key-probed; the spec discharges
  scalar keys only.* `key_probe_candidate` accepts any declared key,
  interval-final pointwise keys included (classify.rs:139-151;
  `KeyStatement::pointwise`, schema.rs:472-474; test
  `pointwise_key_point_lookup_uses_key_probe_and_is_image_free`,
  api/prepared/tests/key_probe.rs:245). `keyprobe_key_spent` discharges
  `hkey` only under `intervalSplit R K = none` (Rewrites.lean:955-964).
  Exact-tuple functionality of a pointwise key is semantically implied
  (two facts sharing the scalar prefix AND the exact interval would
  overlap pointwise), so no bug is claimed — but the discharge is nowhere
  proved or recorded.
- **F4 (class b, latent)** — *Type-blind sentinel trim in rule (d).*
  `set_refutes_eq` trims `SENTINEL_ID = u64::MAX` (dict.rs:80) from ANY
  `WordSet` (fold.rs:184-192) regardless of field type; a numeric-typed
  set containing a legitimate `u64::MAX` would be misjudged empty.
  Unreachable today: `lower_literal` never mints `WordSet`
  (lower_literal.rs:14-34), grounding attachments (row ids ≤ 256) happen
  after the fold, and the bind path correctly scopes its own trim to
  `ValueType::String` sets (bind.rs:216-220). Recorded as a hardening
  note, not a bug.
- **F5 (class c, minor)** — *The "every verdict is an instance" claim
  outruns `StaticallyEmpty`'s shape.* Rules (b)/(d) kill on
  occurrence-FILTER contradictions (fold.rs:247-258); `StaticallyEmpty`
  quantifies rule CONDITIONS (Rewrites.lean:975-977). Under the
  filters-as-lit-bindings reading of the bridge, an Eq-conflict kill is
  honest emptiness (the atom matches nothing on any instance) but not an
  instance of the condition-refutation shape; the narrowing's claim
  (:98-101) silently assumes a filters↦conditions mapping. Soundness is
  unaffected; the spec-side coverage statement is loose.

All module-doc `file:line` anchors in Rewrites.lean (:13-56) were checked
against the current sources and are accurate. Bridge rows for the four
PRD-08 theorems (Bridge.lean:384-402) cite the correct minting and test
sites.

## GRADE: B+

No class-(a) divergence survived adversarial reading — every rewrite the
Rust applies is either an instance of a modeled step (up to harmless atom
order), strictly narrower than the modeled acceptance, or a recorded
narrowing whose doc-side soundness argument checks out arm by arm
(including the negated-complement fold, this review's special target).
What keeps the grade from A is the gap between what the code accepts and
what the record admits it accepts: chained-source eliminations (F1), the
KeyProbe residual-filter surface (F2), and pointwise key probes (F3) are
real, reachable behaviors outside both the theorems and the recorded
narrowings — each looks sound, none is covered, and the module's own law
("narrow and record") says they should be. F4/F5 are latent or cosmetic.
