# Spec-fidelity report 04 — Query/Syntax.lean + Query/Denotation.lean vs ir.rs / ir/validate / ir/normalize

Pairing #4 of the PRD 15 fanout. Normative side: `lean/Bumbledb/Query/Syntax.lean`,
`lean/Bumbledb/Query/Denotation.lean` (+ `Countermodels.lean::unsafe_rule_infinite`,
Bridge rows 189–254). Implementation side: `crates/bumbledb/src/ir.rs`,
`ir/validate/{validate,context,finds}.rs`, `ir/normalize/{dnf,normalize,lower_literal,place_comparisons}.rs`.
Read-only review; no code changed.

## Per-theorem fidelity table

| Lean item (file:line) | Rust site (file:line) | Verdict |
|---|---|---|
| `Term`/`Atom`/`Comparison`/`Condition`/`Rule`/`Query` grammar (Syntax.lean:90–159) | `ir.rs:57–103, 262–326, 337–392` | FAITHFUL, modulo recorded narrowings: `finds : List VarId` vs `FindTerm` (aggregates are PRD 05, Syntax.lean:15–19); `Query.arity` vs `head : Vec<HeadTerm>` (Syntax.lean:17–19); `AllenMask` list vs bitmask (Syntax.lean:20–23). Variant-for-variant match on `Term`, `MaskTerm` (Syntax.lean:79–81 ↔ ir.rs:245–248), `CmpOp` (Syntax.lean:112–115 ↔ ir.rs:263–272), `ConditionTree` incl. `And([])`=true / `Or([])`=false readings (Syntax.lean:124–134 ↔ ir.rs:304–326). |
| `Matches` — THE matching equation (Denotation.lean:164–166) | `ir.rs:87–103` (bindings doc), realized `ir/normalize/normalize.rs:122–251` (`lower_atom`) | FAITHFUL for value-reading bindings: zero-binding atom = nonemptiness gate (ir.rs:85, tests:502–519); absence-is-wildcard by representation. Membership bindings: see divergence B1. |
| `repeated_var_unifies` / cross-atom (Denotation.lean:181–202) | `normalize.rs:174–188` (first field is the slot, later positions → same-fact `FieldsCompare Eq`); cross-atom shared vars are join constraints by slot identity | FAITHFUL. Repeated var whose first occurrence is a membership position correctly takes its first *domain* binding as the slot (normalize.rs:138–149) and composes `FieldsPointIn` same-fact (normalize.rs:163–172). Kernel-level word comparison not traced (exec is pairings 7/8). |
| `param_selects_not_binds` / `paramSet_selects_membership` (Denotation.lean:213–229) | `ir.rs:57–66`; `context.rs:405–416` (`note_param_kind`, `ParamScalarAndSet`); `normalize.rs:190–222` (param/set positions lower to filters, never slots) | FAITHFUL. Params/sets never enter `Occurrence.vars`, so they cannot bind — the hostile lock `a_param_position_does_not_bind_a_negated_variable_even_when_written_after_it` (ir/validate/tests/reject.rs) pins it. |
| `Safe` — positive range restriction (Denotation.lean:248–249) | finds: `finds.rs:102–104` (`UnboundFindVariable` vs `atom_vars`); negated: `context.rs:478–482` (`NegatedVariableUnbound`, `negated_vars ⊆ atom_vars`); conditions: `context.rs:821–827` (`ComparisonOnlyVariable`) | FAITHFUL. `atom_vars` = positive `Term::Var` bindings only (context.rs:499–505, 547–554) = `Rule.positiveVars`. A condition var bound only in a negated atom passes `comparison_var` (slots include negated anchors) but is caught by `NegatedVariableUnbound` — net acceptance equals `Safe`. Checked on LOWERED rules; a rule vanishing under `Or([])` escapes the check but denotes ∅ exactly (validate.rs:77–82). |
| `membership_only_unsafe` (Denotation.lean:269–272) | `context.rs:1319–1329` (`check_membership_domains`), resolution order documented context.rs:911–942 | FAITHFUL under the membership narrowing (B1): element-typed + positive-interval-bound-only ⇔ modeled var occurring only in conditions. Negated scalar bindings correctly do NOT populate `scalar_bound_vars` (context.rs:549–554). |
| `safety_order_independent` (Denotation.lean:298–303) | `context.rs:439–484` — positives-then-negated by construction, safety judged on sets after the walk | FAITHFUL. |
| `Rule.WellTyped` — shape discipline (Syntax.lean:243–273) | `DurationInBinding`: context.rs:526–531, 564–568 (both field kinds, both polarities); measure order-only/one-side: context.rs:697–767 (`DurationBothSides`, `DurationComparisonOperator`); set under `Eq` only, one side: context.rs:754–815 (`ParamSetComparison`; `Ne` vs set rejected via `negated: false` match at 799; set-vs-set → `ConstantComparison` at 812–815) | FAITHFUL; the validator additionally enforces the positional type rules the model narrows out (Syntax.lean:24–32, degenerate-arm denotations). |
| `cmpDen` (Denotation.lean:377–385) | `eq`/`ne` value identity ↔ canonical bytes; `gt`/`ge` mirrored reads ↔ mirror sealed at `OpClass::Order` (context.rs:93–128, `shaped_var_const` 233–238); `allen` mask membership ↔ `AllenMask::contains(classify)`; `pointIn` interval-left ↔ ir.rs:257–261 | FAITHFUL, two riders: interval `Eq`/`Ne` canonicalization (B4) and the `ne` doc claim (C1). |
| `pointIn_unfold` (Denotation.lean:448–457) | `ir.rs:258` ("`iv.start ≤ x < iv.end`"); cross-atom decomposition `place_comparisons.rs:184–204` — `Le(interval.start, point)` + `Lt(point, interval.end)` | FAITHFUL: exactly the half-open pair, on encoded words whose order embeds value order (Values pairing). Ceiling literals rejected (context.rs:58–86, 1209–1224) — the point-domain law, consistent with `Point`'s domain. |
| `allen_mask_denotation` (Denotation.lean:477–486) | `context.rs:1143–1172` (typed pass), `sealed_mask` converse for constant-first (context.rs:273–280), same-atom no-mirror (place_comparisons.rs:23–28) | FAITHFUL shape; converse correctness rests on `classify_swap` (PRD 05, pairing 5). Vacuous ∅/full masks rejected (context.rs:648–663) — recorded as unspent validator checks (Syntax.lean:22–23). |
| `Condition.lower`/`lowerAll`/`lowerAny` (Denotation.lean:502–526) | `dnf.rs:103–126` (`conjunction_terms`/`tree_terms`) | EXACT mirror including disjunct order (head-tree choice outermost, left-to-right leaves), `And([])`→one empty disjunct, `Or([])`→zero. |
| `Rule.lower` / `dnf_preserves_denotation` (Denotation.lean:692–735) | `dnf.rs:89–99` (`distribute`: finds/atoms/negated cloned, conditions = leaves); cap judged structurally first (dnf.rs:61–82, validate.rs:66–76); `collapse` dedup (dnf.rs:137–161) justified by `union_idempotent` | FAITHFUL. `nesting_depth` iterative (dnf.rs:40–53) — the boundary-cap narrowing (Syntax.lean:41–42). |
| `union_idempotent` / `answer_identity_canonical` (Denotation.lean:746–778) | `dnf.rs::collapse`; `exec/sink.rs` seen-set (Bridge rows 236–244) | Shape verified at this layer; sink internals are pairing 7. |
| `snapshot_single` (Denotation.lean:795–797) | one `ValidatedQuery`/read-txn per execution (Bridge row 246) | Out-of-scope internals (PRD 09); signature-level claim consistent. |
| `derives` anti-join (Denotation.lean:611–615) | negated atoms → `AntiProbe` descriptors (normalize.rs:47–56), `¬∃` by probe, never a complement | FAITHFUL by representation — no complement is constructible. |
| `eval_sound` premises (Denotation.lean:1558–1570) | `Safe` ↔ the three safety diagnostics; measure-free bindings ↔ `DurationInBinding` | The two premises are exactly the validator's discharge sites, as Bridge row 251 claims. |
| `unsafe_rule_infinite` (Countermodels.lean:399–407) | acceptance boundary: `NegatedVariableUnbound`/`ComparisonOnlyVariable`/`MembershipOnlyVariable` | FAITHFUL — the unsafe rule is unwritable downstream. |

## Divergences

**Class (a) — Rust behavior the spec forbids: NONE FOUND** under adversarial
reading, including: negated-only condition vars, membership-position repeated
vars, `Ne`-vs-set, measure-vs-set under order ops, `Or([])` vanishing rules,
constant-first mirror sealing, and the ceiling literals.

**Class (b) — behavior the spec does not determine:**

- **B1 — membership bindings are outside the modeled matching equation.**
  An element-typed term at an interval field means point membership in Rust
  (ir.rs:91–101; normalize.rs:109–172 binds no variable, lowers `PointIn`
  filters), while `Matches` (Denotation.lean:164–166) selects values only.
  The correspondence (membership binding ⇔ `PointIn` condition) is a recorded
  narrowing (Syntax.lean:34–40) realized by `resolve_bivalents`
  (context.rs:943–959) + `lower_atom`, but the surface-to-model transformation
  itself is unproven mechanism. Recorded; recommend a future lemma or a
  differential lock making the equivalence checkable.
- **B2 — acceptance is strictly narrower than `Safe ∧ WellTyped`.** The engine
  rejects programs the model denotes exactly: `EmptyRuleSet`/`EmptyFinds`/
  `NoPositiveAtoms` (validate.rs:35–44, 187), the all-vanished `Or([])` program
  (validate.rs:77–82), `SelfComparison`/`ConstantComparison` (context.rs:664–669,
  810–815), `DuplicateFindTerm` (validate.rs:199–203), Allen ∅/full vacuity
  (context.rs:648–663), and the caps `TooManyRules`/`ConditionNestingTooDeep`/
  `DnfExceedsRules`/`TooManyAtoms`/`TooManyVariables`. Caps and vacuity are
  recorded (Syntax.lean:22–23, 41–42); the "write the query you mean" refusals
  and the empty-edge rejections are not. Benign — never unsound (every theorem
  quantifies over arbitrary syntax or assumes only `Safe`/`WellTyped`).
- **B3 — `MeasureOfRay` is an effect the model does not carry.** Model:
  `Value.measure?` = `none` on rays, comparison false (Denotation.lean:21–26,
  98–101); engine: typed execution error (ir.rs:76–79). Recorded narrowing;
  conformance is restricted to error-free executions (Denotation.lean:25–26).
- **B4 — interval `Eq`/`Ne` canonicalize to `Allen(EQUALS)`/complement**
  (context.rs:253–259, 993–1033) while `cmpDen .eq` is value identity
  (Denotation.lean:378). Equivalence is deferred to PRD 05's `classify`
  refinement (Denotation.lean:373–376) — a cross-pairing dependency, not
  proven at this level.
- **B5 — `PendingIntern` string literals are unrepresentable in the model.**
  Model literals carry `StrId`; the engine's raw-bytes literal resolves per
  execution, a dictionary miss becoming the never-minted sentinel so `Eq`
  fails and `Ne` passes (lower_literal.rs:5–7, 19–21;
  exec/dispatch/key_probe_fact.rs:14–17, 52–55). This coincides with the
  model's exclusion reading (an absent string equals no stored value) but is
  outside the modeled fragment — no theorem covers it.

**Class (c) — spec claims no code implements / spec errors:**

- **C1 — the "ill-typed comparisons denote False" narrowing is wrong for
  `ne`.** Denotation.lean:32–36 claims ill-typed operand pairs "all fall
  through to the empty reading", but `cmpDen .ne a b = a ≠ b`
  (Denotation.lean:379) denotes TRUE on type-mismatched pairs (a `u64` vs a
  `bool` differ as `Value`s). Unreachable on accepted rules — the validator's
  `IllegalComparison` rejects mismatched `Ne` (context.rs:993–997) — and no
  theorem spends the claim, but the module-doc's total-and-empty story is
  false for `Ne`. Doc-level spec error; recommend correcting the comment (or
  guarding `ne` by `orderWord`/type agreement if the degenerate-arm story is
  to be kept uniform).

## GRADE: A−

No class-(a) divergence survived adversarial reading: the accepted fragment,
`Safe`, the shape discipline, the DNF lowering (including disjunct order, the
empty combinations, the structural cap, and collapse), the mirror/converse
sealing, and the half-open `PointIn` decomposition all match the spec clause
for clause, and the two `eval_sound` premises are discharged at exactly the
Bridge-claimed sites. The deduction is for B1 — the membership-binding
correspondence is the one semantic bridge carried by narrative rather than
proof or lock — plus the unrecorded portion of B2 and the C1 doc error. None
affects any accepted query's answers.
